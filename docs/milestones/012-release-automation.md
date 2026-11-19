# Milestone 012 — Release automation

## Milestone summary

November 19, 2026, Sabine. Release automation for the proposed `v0.1.0-alpha.2` passed local qualification. The resulting packages are review artifacts. Hosted candidate execution and release publication are distinct operations.

## Implementation and boundaries

The candidate workflow runs on manual dispatch from the canonical repository's `main`. It checks the source version, calls the existing Rust CI, prepares a matching offline-buildable source archive and assembles three native packages. A fourth clean-root test installs the Debian package on Ubuntu. The seal job requires every distribution and CI job to succeed before writing a candidate manifest and SHA-256 checksums. Actions artifact storage receives encrypted source, package and final candidate bundles. The verification archive includes the native payload inventories and build and install logs. The [operator guide](../releases/automation.md) covers configuration, inspection, recovery and website updates.

The approval workflow defaults to read-only verification. The only write-capable job uses the protected `release` environment on an explicit publication dispatch. It checks GitHub's recorded approval by `lgraak`, binds the requested digest to a successful candidate run and source SHA, and verifies every release asset before and after publishing. An existing tag or release stops promotion. A partial draft is left for explicit operator review. Neither workflow deploys software. The contributor branch contains the workflows, source and packaging tools, tests, release notes, and documentation. The two workspace package versions changed to `0.1.0-alpha.2`. The locked dependency versions and the runtime monitoring code are identical to the accepted baseline.

## First local rehearsal, before independent review

All production source, workflow and packaging files in the final test checkout matched the working branch byte-for-byte, including executable modes. A disposable test revision supplied the Git SHA required by source provenance. The qualification report was added afterward. The test used a dedicated rootless Docker socket and native distribution images. Native package managers acted inside disposable containers. The executables ran under an ordinary user account.

| Local gate | Result |
| --- | --- |
| Rust 1.85.0 | Formatting, workspace and GUI all-target Clippy, locked release build passed. The workspace suite recorded 287 passes and two ignored tests. Both private-bus tray tests passed in their dedicated run. Four vendored font tests passed. |
| Python and workflow checks | 153 diagnostic fixtures, 16 promotion and encryption tests, three source tests, four package checks, three workflow tests and six existing DCO checker tests passed. actionlint and ShellCheck passed. |
| Corresponding source | Captured 214 repository files, 590 locked registry crates and the reviewed notice inventory for 383 selected Linux packages. Verified the archive against Git and the SHA-256 provenance record. |
| Debian and Ubuntu | Built an amd64 package on Debian 12. Clean install, virtual desktop, Alpha 1 upgrade and removal passed on Debian 12 and Ubuntu 24.04. |
| Fedora and Arch | Built and verified x86-64 packages in Fedora 44 and pinned Arch. Both passed clean install, virtual desktop, Alpha 1 upgrade and removal. |
| Release seal and verification | Verified eight assets from the final build and the encrypted Actions-format ZIP. The production verify-only code read candidate metadata from a local simulated GitHub service. The operation trace recorded zero writes. Publication success and fail-closed cases used isolated fixtures. |

Each installation checked the 731-file payload, ordinary-user CLI, daemon, peer-checked local socket, shutdown, packaged desktop file and visible X11 window. The runtime tests checked that preferences retained their bytes and permissions through upgrade and removal. An early Arch check detected the slim image's documentation filter. The final test enabled an exception for SiliconWatch documentation in its disposable installation root and verified the full inventory. An early harness retry also replaced live-mounted development scripts with a frozen checkout before making the final measurements.

**First rehearsal evidence**

| Identifier | Value |
| --- | --- |
| Local test revision | `8e7b62f0981bcd31ffec4771772b0ee9fcc70b65` |
| Source archive SHA-256 | `1d86f4e43cedbaf59810e137aecc9be3408f3b2a69f80cc34a3924baeaaaedc0` |
| Sealed manifest SHA-256 | `071e3c3ef499ba9158bc3de99973f0427891547c95988155adce989dd568247e` |

## Qualification limits and Git status

GitHub's live approval, token and upload services have not executed these local workflows. The repository had no configured release environments when inspected. Fedora emitted a nonfatal warning because its optional RPM changelog has no timestamp. The reviewed RPM passed content and upgrade checks. Candidate promotion compares exact hashes, and reproducible binary rebuilds were outside this qualification. The Docker desktop checks used virtual X11 and synthetic hwmon fixtures. They do not establish physical NVIDIA, native Wayland or clean CachyOS package qualification. The [Milestone 011 acceptance record](011-nvml-monitoring-settings.md) contains the independent physical Quadro P620 evidence.

Local `main` was fast-forwarded to the verified PR #10 merge `d2364d500e008bbed0c44fd3829051a7c162f80c`. `codex/release-automation` was created from that commit. Implementation and documentation remain unstaged on the local branch. Alpha 1 remains the existing published release.

## Initial handoff, before remediation

The local build, package qualification, sealing and approval implementation is ready for code review. The final rehearsal exercised the three native package formats, exact source provenance, all four installation and upgrade environments, and the same verification code the manual approval workflow will run. The candidate includes the Debian, RPM and Arch packages, corresponding source, release notes with the per-version changelog, verification reports, checksums and manifest. The manifest digest above identifies only the local rehearsal.

The independent review and second qualification below supersede the first rehearsal's setup and acceptance recommendation.

## November 19 independent review and remediation

The earlier qualification table and digests describe the first rehearsal. The independent review found incomplete attribution of distributed sources, missing evidence binding installed packages to sealed bytes, and reliance on unverified GitHub policy.

The revised candidate ships a Cargo local registry for the supported Linux target. Its source notice inventory covers the actual crate archives and records the metadata-only entries used to resolve the original lockfile. The source-build guide identifies the native prerequisites. Each successful install row now records the exact archive it installed, including both Debian and Ubuntu receipts for the Debian package. Sealing and verification check all four receipts.

A separate read-only policy job verifies the canonical protected branch, CI and review rules, deployment branches and scoped secrets before candidate access. Publication requires a fresh check, enabled release immutability and Chris's approval. Each candidate requires a fresh dispatch. Verification-only reports visible version collisions. The recovery command compares the approved candidate with the release, tag and server-side asset digests and requires an immutable published release for a successful result. The [operator guide](../releases/automation.md) specifies hosted setup, visibility limits and recovery.

## HANDOFF TO CHATGPT — remediation qualification

The independent security and supply-chain reviewers found all three local blockers addressed. Their review led to additional fixes for embedded asset notices, secret forwarding in reusable workflows, and handling of ambiguous or mutable releases after an interruption. The recorded tests use the corrected implementation.

| Evidence | Result |
| --- | --- |
| Corresponding source | Independently compared all 236 frozen project files, 442 original crate archives and 148 metadata-only lock entries. The crate archive hashes match Cargo.lock. Checked the 383-record binary inventory, 731 binary notice files, nine font digests and 76 Slint asset mappings. Installed notices identify the Cantarell font, the adapted MPL-covered headerbar code and the Fluent artwork. Two independently prepared source archives from the same tree had identical bytes. |
| Native installation | Built Debian, Fedora and Arch packages from the frozen source with locked Rust 1.85.0 dependencies. Installed the Debian package on Debian 12 and Ubuntu 24.04, the RPM on Fedora 44 and the Arch package on its pinned image. The Ubuntu default archive mirror timed out while fetching its indexes. The successful Ubuntu retry used the same pinned image and package with the signed Ubuntu Azure archive mirror. The inventories contained 741 payload files per package. All four rows verified metadata, clean install, ordinary-user runtime and virtual X11 GUI, Alpha 1 upgrade, preference retention and removal. The four receipts match the exact three package digests in the sealed manifest. |
| Rust and workflow regression | Formatting, both deny-warnings Clippy runs and the locked release build passed. Workspace tests recorded 287 passes and two opt-in private-bus tests ignored by default. Both passed separately. Four fontdue tests, 153 diagnostic tests and 63 release, source, payload, policy, workflow and DCO tests passed. actionlint, ShellCheck and the DCO check on the preceding five contribution commits passed. |
| Release controls and recovery | All 42 independent offline cases passed, including altered and rehashed candidates, stale receipts, wrong source, missing keys or approval, unauthorized branches, same-actor approval, collisions, interrupted publication and repeated requests. The final encrypted, Actions-format rehearsal verified all eight assets with five simulated GETs and zero writes. Recovery accepts a public release only when its immutable status, tag, metadata and asset digests match. |

The following identifiers describe the local rehearsal. The Git source identity is a tree object constructed from the uncommitted production files. Final reporting text was appended after qualification.

| Local evidence | Identifier |
| --- | --- |
| Git fixture tree | `31c6d4c6ef7e224bc5819a47b62363b2c995afe8` |
| Corresponding source SHA-256 | `5cd9cc2bb2c0aa6c79ad73e156bd8e602049a4ba568dc4f5b2e3446ab217df43` |
| Source-notices index SHA-256 | `e255cc11fed61acacfce37f237ee0c254eceffbb4f9b8065eba4fa3c95175b7f` |
| Sealed candidate manifest SHA-256 | `822beb0809341e586781cce3a451016e0db128126214e00b2b61a597b02082b0` |
| Debian and Ubuntu tested package SHA-256 | `a6cf49a8c6f71f46d0dc43b9a40b836a210864e6f30d2910ad15d1283239a6ab` |
| Fedora tested package SHA-256 | `60db97785be8d40f37860616ae9858ce7c08caceb1d4c4d6737de8be4a7f47f4` |
| Arch tested package SHA-256 | `10ce50f603669a5842591a495a9602761c3ba4364ef90e1a8d44487e13df8b13` |

**Recommendation.** Accept the local remediation for commit review. Hosted release readiness remains pending. On GitHub, the settings reader must be able to affirm each required protection, including disabled administrator bypass. GitHub's published REST environment schema omits the bypass field. Its Terraform provider reads the field when supplied by the service. The first authorized hosted rehearsal must obtain an explicit Boolean false. An absent field stops the workflow. Confirm environment secret scope and the reader's maximum permissions. Qualify the hosted artifact with publication disabled, then exercise the actual approval and denial controls separately under an authorization that precludes publication. The [operator guide](../releases/automation.md) lists the three environments, reviewer roles and setup. [GitHub REST schema](https://raw.githubusercontent.com/github/rest-api-description/main/descriptions/api.github.com/api.github.com.2022-11-28.json) · [GitHub Terraform provider](https://github.com/integrations/terraform-provider-github/blob/main/github/resource_github_repository_environment.go#L196-L220)

The branch remains at `d2364d500e008bbed0c44fd3829051a7c162f80c` with an empty index. The local rehearsal created no release or candidate commit. It establishes package integrity and the Linux redistribution inventory for this build. Hosted permissions, native Wayland, a clean CachyOS install, and reproducibility of independently rebuilt binaries fall outside this rehearsal.

## Formal local acceptance — November 19, 2026

Chris accepted Milestone 012, Automated Release Infrastructure, as locally technically complete on November 19, 2026. The second independent engineering, security and supply-chain review passed. All three original blockers involved incomplete third-party source and asset notices, missing SHA-256 evidence for the exact installed packages, and insufficient verification of GitHub release protection settings. The reviewers verified the corrections to all three. The review history and local rehearsal identifiers above remain the engineering record.

## HANDOFF TO CHATGPT

Chris accepted the local package and release-pipeline verification for the proposed `v0.1.0-alpha.2`. Debian 12 and Ubuntu 24.04 qualified the same Debian package. Fedora 44 and pinned Arch qualified their native packages. All four checks covered clean installation, ordinary-user execution, a virtual X11 desktop, upgrade from Alpha 1, removal and preference preservation. Each receipt identifies the package bytes that passed. Sealing and verification check those hashes against the candidate manifest. The local encrypted rehearsal verified eight assets against a simulated GitHub service.

The corresponding source supports the qualified Linux build. It includes the locked crate archives that build requires, the fontdue patch, and the reviewed licenses and asset credits. Two source builds from the same frozen tree produced the same archive digest. Chris accepted the documented limits on native binary reproducibility, distribution mirror stability, virtual-desktop coverage, and clean CachyOS qualification. The physical NVIDIA evidence remains in the Milestone 011 acceptance record. The Ubuntu mirror retry and the nonfatal Fedora changelog warning remain part of this record.

GitHub-hosted qualification is pending. A separately authorized hosted rehearsal must demonstrate that the policy reader can verify branch and environment protections, including an explicit false value for administrator bypass. Missing or ambiguous policy data stops the workflow. Human approval and denial, scoped secret delivery, hosted artifact verification, and actual service behavior still require hosted evidence. Recovery and publication failure cases were exercised locally.

Chris authorized local signed-off commits and preparation of a proposed pull request against `main`. Production publication remains unauthorized. Pushing the branch, opening or merging the PR, changing GitHub settings, creating a release tag, publishing Alpha 2 and deploying the website each remain outside this acceptance.
