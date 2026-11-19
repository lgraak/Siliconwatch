# Preparing a SiliconWatch release

A candidate is identified by its version, source commit, GitHub Actions run ID and the SHA-256 of `candidate.json`. Keep those four values together throughout review. The first target is `v0.1.0-alpha.2`.

## Configure GitHub before the first hosted run

The gate supports classic branch protection on the personal `lgraak/Siliconwatch` repository. Protect `main`, including administrators. Require at least one pull request approval, dismiss stale reviews, require Code Owner review, block force pushes and deletion, and require up-to-date `Rust 1.85.0` and `DCO signoffs` checks from the GitHub Actions app. Protect the ownership file itself. For a release-sensitive PR authored by Chris, a second eligible Code Owner must be listed and available. Keep fork runs read-only. Select read-only default workflow permissions and disable approval of pull requests by Actions. Restrict release writers, scope allowed actions and use full SHA pins. Protect release tags against updates and deletion and allow the designated publisher to create fresh tags. Enable release immutability before publication. [Branch protection](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches) · [Immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)

Create these three environments. Configure **Selected branches and tags** on each, with exactly one **branch** rule for `main` and no tag rule.

| Environment | Access |
| --- | --- |
| `release-policy` | Store only the policy reader credential `RELEASE_POLICY_READER` here. Use a fine-grained, expiring credential limited to this repository, with read access to Administration, Actions, Environments, Secrets and Metadata. These permissions read policy and secret names. Keep all write permissions disabled. The repository owner must confirm token scope and expiry in GitHub when provisioning and rotating it. Successful metadata reads cannot attest the maximum authority granted to a token. |
| `release-candidate` | Store the 64-hex-character `CANDIDATE_KEY` here for encryption and review. |
| `release` | Store the same `CANDIDATE_KEY` here. Set `lgraak` as the sole required reviewer, prevent self-review, and disable administrator bypass. A different trusted maintainer must dispatch publication. |

Generate a random 256-bit key in the team's secret manager. Limit access to authorized reviewers and rotate both environment copies together. Keep the key until outstanding candidates expire, or generate fresh candidates after rotation. Keep both credential names out of repository secrets and all other environments. Public Actions readers can retrieve the encrypted artifacts. The public summaries contain candidate identifiers and digests. [GitHub environment protection](https://docs.github.com/en/actions/reference/workflows-and-actions/deployments-and-environments) · [Secret scope and precedence](https://docs.github.com/en/actions/reference/security/secrets)

A credential-free job first checks the repository, protected ref, caller and fresh dispatch. The policy job reads GitHub metadata before release jobs gain access to the encryption key or a write token. It checks the actual deployment rules and reviewer settings, as well as complete secret-name inventories. Missing permissions, ambiguous results and missing required fields stop the run. In the first hosted rehearsal, confirm that the environment API returns an explicit false value for `can_admins_bypass` and identifies the deployment rule type as `branch`. An omitted field cannot establish either control. A publication request repeats the check after verifying the candidate and requires release immutability to be enabled. Keep settings and release writers under administrative change control throughout the run and approval wait. These API reads are snapshots. GitHub enforces the configured environments when scheduling jobs. A credential holder with independent release authority remains a separate trust boundary.

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

Once Chris authorizes publication of that exact candidate, start a fresh **Release approval** run with the same four identifiers and **publish** selected. Verify the run summary before approving the `release` environment in GitHub. The publisher checks the approving GitHub account ID and refuses a dispatch initiated by that account. Only the publication job has `contents: write`. It verifies the candidate again, uploads the reviewed files to a new draft, checks each server-reported digest, creates the new tag at the approved source SHA, publishes the draft and verifies the final assets. A push, merge or tag alone cannot publish this workflow. The existing Alpha 1 release is preserved.

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
