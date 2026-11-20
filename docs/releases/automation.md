# Preparing a SiliconWatch release

A candidate is identified by its version, source commit, GitHub Actions run ID and the SHA-256 of `candidate.json`. Keep those four values together throughout review. The first target is `v0.1.0-alpha.2`.

## Configure GitHub before the first hosted run

The owner of this public personal repository, `lgraak` (GitHub user ID `9121312`), dispatches release workflows and authorizes publication. The project-scoped Codex reviewer evaluates substantive changes on their exact diff before merge. Record its receipt and the disposition of findings in the PR. The maintainer makes the merge and release decisions. Both `lgraak` and `lgraakcg` belong to the same person. [Independent review procedure](../../AGENTS.md#independent-review-before-merge)

1. **Protect `main`.** In **Settings → Branches**, add a **classic** rule for exactly `main`. Turn on **Require a pull request before merging**. Leave **Require approvals**, **Require review from Code Owners** and **Require approval of the most recent reviewable push** off. The protection API must report a PR rule with an approval count of **0**. CODEOWNERS routes reviews to `lgraak`. Develop on feature branches and merge through a PR.
2. **Enforce CI for the owner too.** Require status checks and up-to-date branches. Select both `Rust 1.85.0` and `DCO signoffs` with **GitHub Actions** as their expected source (app ID `15368`). Run PR CI first if the checks are missing from the selector. Enable **Do not allow bypassing the above settings**. Leave **Allow force pushes**, **Allow deletions** and **Lock branch** off. Check for overlapping rulesets that demand another reviewer or prevent the owner from merging a green PR. Verify the settings against a future owner-authored PR. [Classic branch settings](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/managing-a-branch-protection-rule) · [Protection API and zero-review setting](https://docs.github.com/en/rest/branches/branch-protection#update-branch-protection)
3. **Limit Actions authority.** In **Settings → Actions → General**, select read-only default workflow permissions and disable permission to create and approve pull requests. Keep fork runs read-only and withhold secrets. Allow the SHA-pinned GitHub actions used by the checked-in workflows. Restrict write access to the repository and release-capable credentials. Only the publication job requests `contents: write`, alongside `actions: read`.
4. **Create the three environments below** in **Settings → Environments** before dispatch. On each, choose **Selected branches and tags** and add exactly one **Branch** named `main`, with no tag rule. The policy and candidate environments need no approval. [Environment settings](https://docs.github.com/en/actions/how-tos/deploy/configure-and-manage-deployments/manage-environments)

| Environment | Environment secrets and protection |
| --- | --- |
| `release-policy` | Store `RELEASE_POLICY_READER` here. Use an expiring fine-grained PAT restricted to this repository, with read access to Administration, Actions, Environments, Secrets and Metadata. Verify its scope and expiry when provisioning and rotating it. The preflight reads metadata and secret names. |
| `release-candidate` | Store `CANDIDATE_KEY` here for encryption and read-only verification. |
| `release` | Store the same `CANDIDATE_KEY` here. Under **Required reviewers**, select only the user `lgraak`. Clear **Prevent self-review** and **Allow administrators to bypass configured protection rules**. This lets the owner explicitly approve their own dispatch through **Review deployments**. |

Generate a random 256-bit candidate key in the maintainer's secret manager and store it as 64 hex characters. Rotate both copies together. Keep the key until outstanding candidates expire, or rebuild those candidates after rotation. Keep both secret names out of repository secrets and all other environments. The reusable policy workflow receives the named secret through its environment job. The encrypted candidate and public run summary can be downloaded by Actions readers. [Secret scopes](https://docs.github.com/en/actions/reference/security/secrets) · [Secrets in reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)

5. **Protect release objects.** In **Settings → General → Releases**, enable release immutability before publication. Protect release tags against updates and deletion while permitting the publisher to create a fresh tag. Review the identities that can edit settings or create releases directly. [Immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
6. **Qualify the hosted controls.** Run a fresh **Release candidate** on protected `main` as `lgraak`, then use **Release approval** with **publish** unchecked. The credential-free origin job checks the caller, repository, protected ref and first attempt. The policy reader checks effective CI, PR and deployment rules and complete secret-name inventories. Missing fields or permissions stop the run. Confirm the release environment returns explicit `can_admins_bypass: false`, `prevent_self_review: false`, the owner as sole reviewer and a deployment rule of type `branch`. A publication request repeats policy verification, including immutability, before GitHub schedules the approval. Use the simulation for the write path until a real release is separately authorized.

An owner credential compromise is a single point of trust. Keep the owner account and settings secure throughout candidate review and the approval wait. Policy reads are snapshots. GitHub enforces environment approval when scheduling the job, and the publisher independently reads that run’s approval history. A maintainer or credential with separate administrative or release-writing authority remains trusted.

## Build and validate a candidate

1. Use a reviewed source commit on `main`. Set the CLI and GUI Cargo versions and their workspace entries in `Cargo.lock`. Add `docs/releases/<tag>.md` with the first heading `# SiliconWatch <tag>`. Review features, bug fixes, hardware coverage, API and configuration compatibility, limitations, installation, upgrades and verification evidence. A dependency change requires a new notice review.
2. In Actions, run **Release candidate** on `main` with the exact tag text. The job checks the selected commit and version. It reuses the Rust 1.85 workflow for formatting, all-target Clippy, Rust tests, opt-in tray tests, release build and diagnostic fixtures. Every run tests the DCO checker. PR runs also check the contributed commit range.
3. Source preparation archives the Git revision, the locked registry sources needed for the supported Linux build, the local fontdue patch and reviewed notices. SOURCE-NOTICES.json maps the distributed registry archives to their attributions and separately lists entries retained only for dependency resolution. The native jobs compile that archive offline. Distribution package managers supply build and runtime dependencies under their normal trust policies. Logs record the base image digest and the packages selected at build time. Mirrors can advance independently of image pins.
4. Inspect the build and install logs for all rows below. Verification checks payload bytes, permissions, dependencies, desktop files, ordinary-user CLI and daemon operation, socket cleanup, a virtual X11 window, upgrade from Alpha 1 and preference retention on removal. Each successful runtime writes a receipt for the SHA-256 of the package it installed. Sealing and promotion require all four receipts to match the package bytes. Rebuild a candidate produced before install receipts were introduced. GPU and native desktop qualification evidence is recorded separately.

| Binary format | Build environment | Clean install and upgrade environments |
| --- | --- | --- |
| amd64 Debian package | Debian 12 | Debian 12 and Ubuntu 24.04 |
| x86-64 RPM | Fedora 44 | Fedora 44 |
| x86-64 Arch package | Pinned Arch rolling image | Matching Arch image |

The Arch build supports testing on compatible CachyOS systems. A clean CachyOS package installation needs separate qualification. The Debian, Fedora and Arch jobs each produce their own binaries.

## Review the exact bytes

Start a fresh dispatch for every candidate, including after a failed run. Reruns of an existing attempt are ineligible. Record the four candidate identifiers in the successful run summary. Download `candidate-<tag>` within 14 days. Check the manifest digest from the summary independently before relying on anything inside the artifact. Use a trusted checkout of the release tools and provide the review key privately in the process environment.

```bash
gh run download "$RUN_ID" --repo lgraak/Siliconwatch --name "candidate-$TAG" --dir encrypted
python3 scripts/release/envelope.py unpack encrypted/candidate.tar.gpg reviewed
python3 scripts/release/candidate.py verify --candidate reviewed \
  --version "$TAG" --commit "$SOURCE_SHA" --digest "$MANIFEST_SHA256"
(cd reviewed && sha256sum -c SHA256SUMS)
tar -tzf reviewed/verification-reports.tar.gz
```

Review the three packages, corresponding-source archive, release notes, native payload inventories and logs. Compare the documented hardware coverage with the physical acceptance report. Use **Release approval** on `main` with **publish** unchecked to verify a candidate with a read-only token. The job checks the canonical repository, candidate workflow, event, branch, source SHA, successful run, live artifact and its archive digest. It decrypts the artifact and rejects altered files, missing or unexpected assets, existing tags and releases visible to its token. GitHub restricts draft listings to callers with push access. The approved publication job repeats the collision check with draft visibility. Start a fresh verification dispatch whenever another read-only check is needed. [GitHub release visibility](https://docs.github.com/en/rest/releases/releases#list-releases)

## Approve publication separately

Once Chris authorizes publication of that exact candidate, sign in as `lgraak` and start a fresh **Release approval** run on `main` with the same four identifiers and **publish** selected. After verification, compare the run summary with the reviewed candidate, open **Review deployments**, select `release`, then **Approve and deploy**. The publisher requires both the owner’s dispatch and GitHub’s recorded approval by the owner for the `release` environment on this run. Only the publication job has `contents: write`. It verifies the candidate again, uploads the reviewed files to a new draft, checks each server-reported digest, creates the new tag at the approved source SHA, publishes the draft and verifies the final assets. A push, merge or tag alone cannot publish this workflow. The existing Alpha 1 release is preserved.

A failed verification stops before promotion. If workflow files on the default branch have changed since the candidate was prepared, GitHub can reject release creation for its older source SHA. Prepare a fresh candidate from reviewed `main` and repeat approval. [GitHub release creation permissions](https://docs.github.com/en/rest/releases/releases#create-a-release)

### Reconcile an interrupted publication

A failed run can leave a partial draft, a draft and tag, or a published release. A lost response can hide a successful last operation. Before requesting another run, use the reviewed candidate directory and the original recorded digest. Use an operator identity able to see drafts. The command performs only GET requests. If the log supplied a release ID, include `--release-id ID` to read that exact release; lack of access to a known ID is an error.

```bash
python3 scripts/release/reconcile.py --candidate reviewed \
  --version "$TAG" --commit "$SOURCE_SHA" --digest "$MANIFEST_SHA256"
```

| Observed state | Operator action |
| --- | --- |
| `draft_without_tag` or `draft_with_tag` | Inspect the draft ID and every reported difference. A matching draft still needs a separate recovery decision. |
| `public_verified` | GitHub confirms the published release is immutable and all eight approved assets, release metadata and tag match. Record the release as published and stop. |
| `public_mismatch`, `ambiguous`, or a visible tag without its release | Stop and investigate the tag, draft visibility and all differences. Preserve the existing objects. |
| No visible tag or release | Verify the operator has draft visibility and check the failed run's ID before concluding the version is free. |

Only `public_verified` exits successfully. A fresh publication refuses an occupied version. Any cleanup or subsequent publication requires a separate decision. While a draft exists, prevent other release writers from editing it. The version concurrency group covers these workflows, and immutable-release protections begin at publication.

After publication, use the verified release URL, actual asset download URLs, SHA-256 manifest and approved notes to prepare the external Cloudflare site's download page. Check the links and version in preview and obtain separate website deployment approval. Package installation remains an operator action.

## Local verification

Use `cargo +1.85.0 fetch --locked` and a clean test checkout. The source tool can use a cached, checksum-pinned Alpha 1 Arch package via `--reviewed-package`. Keep outputs outside the checkout. Set `DOCKER_HOST` if using a private rootless daemon. The native commands install packages only inside disposable containers.

```bash
python3 scripts/source_release/prepare.py --source . --commit "$SOURCE_SHA" \
  --version "${TAG#v}" --output "$WORK/source"
bash packaging/linux/run-native.sh build debian "$TAG" "$SOURCE_SHA" "$WORK/deb" "$WORK/source"
bash packaging/linux/run-native.sh verify debian "$TAG" "$SOURCE_SHA" "$WORK/deb"
bash packaging/linux/run-native.sh verify ubuntu "$TAG" "$SOURCE_SHA" "$WORK/deb"
```

Repeat the build and verification pair for `fedora` and `arch` with separate output directories. The complete sealing step lives in `.github/scripts/seal_candidate.sh`. Local promotion regression tests use a simulated GitHub release service, including an approved publication, duplicate versions, damaged candidates and refused writes. They require no live GitHub credential.

```bash
python3 -B -m unittest discover -s scripts/release -p 'test_*.py' -v
python3 -B -m unittest discover -s scripts/source_release -p 'test_*.py' -v
python3 -B -m unittest discover -s packaging/linux -p 'test_*.py' -v
python3 -B -m unittest discover -s .github/scripts -p 'test_release*.py' -v
```
