# Prototype 010: Multi-distribution Alpha 1 package preparation

Date: 2026-11-17 (local date; some retained command records use the following UTC date).

## 1. Milestone Summary

**Status: Partial — native package preparation and the recorded local verification are complete; publication gates and final preservation verification remain pending at this precommit report snapshot.** This is a dated packaging continuation of Prototype 010, not the start of Prototype 011.

The objective was to prepare reviewable Linux x86-64 native packages for the existing Alpha 1 candidate without changing its monitoring implementation. Debian, Fedora and Arch packages were built with Rust 1.85 in isolated distribution environments. The exact final archives passed independent payload review and installation, fixture/API and removal checks on Debian 12, Ubuntu 24.04, Fedora 44 and the recorded Arch rolling snapshot.

Chris subsequently conditionally authorized experimental external alpha testing and a source pull request after the required gates pass. That authorization does not convert the candidate into production-ready or safety-critical monitoring. The two unresolved instrumented Wayland failures below remain qualification limits. No merge, public tag, binary upload or release publication has occurred at this report snapshot.

## 2. Implementation Details

### Package outputs and provenance

The external candidate directory is `siliconwatch-alpha1-packaging/artifacts`. It contains the three unchanged native archives, `PACKAGES.json`, `SHA256SUMS`, per-format payload manifests, `BUILD-PROVENANCE.json`, `SOURCE-PROVENANCE.json`, `DEPENDENCIES.json`, installation instructions and the license review/notice sidecars. Absolute local evidence locations belong in the private continuation, not this public report.

| Candidate | Bytes | SHA-256 |
| --- | ---: | --- |
| `siliconwatch-0.1.0-alpha.1-linux-amd64.deb` | 7,633,608 | `89874755a3e5ed9b87c4d8c1fb65de90a97988af52a63516ddaf9a90082cc6a8` |
| `siliconwatch-0.1.0-alpha.1-linux-x86_64.rpm` | 8,296,673 | `cd0c35c0f6a9763bdd9aa0a843d7b12d5276b0521c980a219011b95b4537e767` |
| `siliconwatch-0.1.0-alpha.1-linux-x86_64.pkg.tar.zst` | 10,242,322 | `8b4b606834079946aaaffe3a4ffc12067fa7448c40b26d2855fd2e8593f0c2b4` |

All candidates are unsigned. No optional portable AppImage was prepared or qualified in this task. The earlier local AppImage remains a separately preserved artifact with its original compatibility limits.

Each package contains exactly **730 application files**, including only two executable/ELF files: `siliconwatch` supplies the CLI and foreground daemon; `siliconwatch-gui` supplies the desktop client. The other **728 files are byte-identical across all three payloads**. They include the approved project icon, GUI-only desktop launcher, public usage/API/GUI documentation, source provenance and dependency notices. The optional native-check executable, fixtures, measurements, private captures, build tools and agent setup are excluded.

The shared stager checks an explicit payload; native package recipes preserve the reviewed binary bytes. Package-owned files use conventional application locations under `/usr/bin` and `/usr/share`. There are no maintainer scripts, RPM scriptlets/triggers, Arch `.INSTALL`, services, autostart entries, udev rules, module-loading scripts, privileged file modes or hardware-control actions. Installation launches nothing. The daemon remains a manual, foreground, normal-user process, and the desktop launcher starts only the GUI.

### Build targets and dependencies

| Native target | Native version | Build environment | Measured GUI GLIBC floor |
| --- | --- | --- | --- |
| Debian amd64 | `0.1.0~alpha.1-1` | Debian 12.15/bookworm; GCC 12.2, binutils 2.40, glibc 2.36 | 2.35 |
| Fedora x86_64 | `0.1.0-0.1.alpha.1.fc44` | Fedora 44; GCC 16.2.1, binutils 2.46.1, glibc 2.43 | 2.43 |
| Arch x86_64 | `0.1.0alpha.1-1` | Rolling image `20261108.0.623074`; GCC 16.2.1, binutils 2.47, glibc 2.44+r78 | 2.43 |

Official container image digests and the actual package inventories are retained in build provenance. Builds used Rust/Cargo 1.85.0, locked/offline release workspace builds, four jobs, non-root UID 1000, read-only source mounts and generic `x86-64` target CPU. The CLI GLIBC floor is 2.34 on all three targets. No executable has RPATH/RUNPATH. Generic code generation and ELF metadata do not establish compatibility with every physical CPU or distribution. Updated native repositories and retained inventories make these documented builds, not a claim of bit-for-bit reproducibility from a moving repository.

Runtime declarations cover fontconfig/fonts, libxkbcommon including X11 support, X11/xcb/Xcursor/Xi, Wayland client/cursor/EGL and the distribution GL/EGL/GLX stack. Arch uses its actual split `libgcc` provider; Fedora retains automatic ELF dependency generation. Session D-Bus supports accessibility/tray integration. CoolerControl is not required. Installation instructions distinguish build dependencies, runtime dependencies and external test tools.

Packaging added no Rust dependency or feature. Slint and slint-build remain pinned to 1.13.1, with the already-approved fontdue 0.9.5 six-cast compatibility patch for Rust 1.85. The current uncommitted contribution includes those earlier Prototype 010 source/lockfile changes; this packaging continuation did not alter them.

### Review files and bounded documentation updates

New reusable packaging files are `packaging/linux/{README.md,INSTALL.md,build-deb.py,stage_payload.py,siliconwatch.desktop}`, `packaging/linux/fedora/siliconwatch.spec`, and `packaging/linux/arch/{PKGBUILD,.SRCINFO}`. The workflow now installs GUI build/test dependencies and covers workspace tests, GUI all-target Clippy, explicit private-bus tests and standalone fontdue compatibility tests in addition to the existing Python/DCO checks. Its hosted execution is still pending.

README status/package guidance was reconciled with conditional alpha authorization. Five existing Prototype 010 reports/proposal received narrowly authorized privacy redactions; their original bytes are retained in immutable external publication evidence. Their historical results and unresolved failures remain historical records. The proposed contribution also includes this report, a publication-readiness report, draft `v0.1.0-alpha.1` release notes and public dependency-review/notice documents. The complete planned contribution path list is in Section 7; the final external `final-changed-files.json` is the later authoritative hash manifest, avoiding a self-referential report hash.

The dependency review found **no demonstrated material redistribution blocker from the identified `simd_helpers 0.1.0` standalone-license-text finding**. Its exact upstream manifest and registry declare MIT; the selected path is compile-time Slint compiler tooling. Inspection found a small procedural attribute macro that adds compiler attributes to caller-supplied tokens, with no bundled macro library or generated runtime implementation body demonstrated in the application payload. The review preserves the missing-text fact and its limited technical reasoning; it is not a legal certification of the entire dependency closure. The public review and external notice supplement preserve attribution and standard MIT reference terms without inventing a copyright holder/year. They supplement the unchanged candidates; the tested archives were not rebuilt or silently replaced to include them.

The exact fontdue source comparison found 43 regular files: 42 unchanged from the pinned upstream archive and only the six approved same-width casts changed in `src/table/parse.rs`. All original license texts, validation logic and upstream lockfile remain intact. The ignored `vendor/fontdue-0.9.5/Cargo.lock` must be explicitly included in the reviewed contribution.

## 3. Verification Results

### Rust, Python and checker verification

Fresh checks ran in Debian 12.15 with Rust/Cargo 1.85.0, Python 3.11.2 and D-Bus 1.14.10, using private synthetic fixtures and external writable build/test directories. Source was read-only and builds/checks used the locked dependencies.

| Executed check | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --workspace --locked --offline -- -D warnings` | Pass |
| GUI all-target Clippy with `ui-verification`, warnings denied | Pass |
| Workspace Rust tests | 166 headless + 106 GUI/CLI tests pass |
| Workspace doctests | 2 pass |
| Explicit private session-bus tests | 2 pass separately; normally ignored in the default suite |
| Strict Python CI discovery using the new Debian release CLI | 153 pass; zero failures, errors or skips |
| DCO-checker fixture tests | 6 pass; this is not yet verification of a new contribution commit |
| Standalone fontdue upstream tests from the exact external 43-file copy and upstream lockfile | 4 pass |
| Opt-in native-check release build | Pass; executable excluded from packages |

The first private-bus execution failed both tests because the isolated root could not resolve UID 1000 in its passwd database. Diagnostic stderr established that setup cause. Adding only the disposable build-user entry, without a home or service, made both tests pass. The initial failures and rerun are preserved; no host account or application source changed.

The supplemental root `--all-targets` Clippy check was not rerun for packaging. Two previously recorded `clippy::non_octal_unix_permissions` findings remain at `tests/hwmon.rs:217` and `tests/identity.rs:368`. They are distinct from the passing configured workspace check and passing GUI all-target check. A stable-toolchain rerun was not part of this packaging verification. Earlier dedicated parser boundary/conversion checks remain historical evidence; the current four standalone fontdue tests are accurately reported as upstream unit tests.

### Final package contents and native installation/removal

Independent archive inspection matched every application payload hash, size, mode and native root ownership to the per-format manifests. RPM's 465 hardlinked entries were resolved before comparison. All 730 Debian MD5 entries matched actual bytes. Native version comparators confirmed each alpha version sorts below stable 0.1.0. Targeted content/metadata scans found no private home paths, known workstation identifiers, credential/private-key patterns or excluded evidence. This is a targeted privacy review, not exhaustive secret-discovery assurance.

| Isolated install target | Exact final artifact and fixture/API test | Installed production GUI | Native removal |
| --- | --- | --- | --- |
| Debian 12.15 | Pass; `.deb` | Virtual X11 and Wayland, software rendering: pass | 730 application files removed; settings sentinel preserved |
| Ubuntu 24.04.5 LTS | Pass; same `.deb` | Virtual X11 and Wayland, software rendering: pass | 730 application files removed; settings sentinel preserved |
| Fedora 44 | Pass; `.rpm` | Virtual X11/software rendering: pass | All 1,149 owned paths, including directories, removed; settings sentinel preserved |
| Arch recorded rolling image | Pass; `.pkg.tar.zst` | Virtual X11/software rendering: pass | 730 application files removed; settings sentinel preserved |

All four installed CLI/daemon tests used synthetic hwmon files, four fixture sensors, raw/identity-preserving JSON, `get_info`/`get_snapshot`, current-user socket ownership, 0700 parent/0600 socket permissions and cooperative exit 0 with endpoint cleanup. GUI `--help` was checked separately and was not treated as graphical execution. Installed graphical checks verified disconnected-to-connected fixture status, 32/64/128 tray pixmaps, four expected menu actions, GUI quit 0, daemon survival after GUI quit and subsequent owned daemon cleanup. Debian, Ubuntu and Arch installed X11 checks read actual 1254 × 1254 window-icon dimensions; Fedora's container check established property presence. The later native-host checks verified those dimensions for all three production binaries. Arch `pacman -Qkk` reported 1,156 entries and zero altered.

Debian and Ubuntu minimal-image document filters initially omitted five application documents. Only the disposable test-image inclusion policy was adjusted; the exact unchanged `.deb` was reinstalled and complete payload/removal checks repeated successfully. Arch's minimal-image document exclusion was likewise adjusted only inside its disposable test root, with exact before/after evidence. Signature/dependency policies were not disabled. The uninstall sentinel retained identical 90 bytes, mode 0600 and SHA-256 `b01916f0f71ed570a16c467675435d7d4fe888a2b844e6f2ee5363c855f063cc`; it tests preservation of user preferences, not GUI-authored persistence.

Other retained setup failures include isolated-runner UID/script permissions, missing minimal-image test tools and an initial window-icon assertion that established only property presence. Those corrections affected the external harness/test environments. A provisional RPM's inherited build-host label was corrected before the final archive, which was tested again in a fresh root; the provisional archive is not a release candidate. No final candidate hash changed during final qualification.

### Native host production and instrumented GUI checks

The **final production executables** from Debian, Fedora and Arch each passed a private-fixture launch/tray/reconnect/quit test on the current KDE host under both Wayland and XWayland: **6 final cases pass**. These used the actual host display with software rendering and a private session bus. Initial X11 attempts failed because the helper lacked `xwininfo`; providing that external test tool and rerunning the same production binary bytes yielded three X11 passes. Initial failure records remain. These tests support production launch, protocol/tray actions and daemon independence, not physical panel interaction or a different distribution's native desktop session.

The separately built **instrumented native-check** executable produced a different, narrower result:

| Backend | Full workflow + profile restart | Default close | Tray lifecycle + profile restart |
| --- | --- | --- | --- |
| XWayland | Pass | Pass | Pass |
| Wayland | Failed: initial snapshot phase exceeded 60 seconds | Pass | Failed: initial snapshot phase exceeded 45 seconds |

Thus **4 of 6 instrumented cases pass; 2 fail with unresolved cause**. The failures reached fixture data/tray registration but did not complete the required first snapshot stage. They do not establish a defect in the separately tested production binary, and the production passes do not resolve them. Two XWayland profile restarts passed; the two requested Wayland restarts were not reached.

There were eight owned GUI executions: six exit 0 and two exit 1. All six fixture daemons survived GUI termination and later exited 0; all 14 owned GUI/daemon PIDs were confirmed absent and tray registrations removed. Of 23 application framebuffer captures, the initial visual review inspected five for OLED Sensors, Details, constrained Settings, Minimize to Tray and restored Sensors. Additional Dark and Light Sensors captures were visually inspected during release preparation; both retained readable fixture rows, controls and distinct theme surfaces. Private images remain outside this contribution. Application framebuffer review is not a screenshot of the actual desktop panel or human keyboard/screen-reader qualification.


The raw staged whitespace check flags preserved upstream CRLF endings in four
fontdue files and five single-space blank context lines in the verbatim unified
patch. The four files match pinned upstream bytes; the patch spaces are context
syntax. A per-command CRLF-aware check passes for every other file. These bounded
exceptions are recorded externally; vendor bytes and global Git settings were
not changed to silence the check.

### Publication and preservation checks still to run

The current local verification applies to the frozen implementation and candidate packages; it is not yet a final-contribution-HEAD result. Before publication, complete the reviewed file/hash manifest and final preservation comparison, create only the authorized signed-off contribution, check its full 40-character SHA and exact parent/manifest, and rerun required checks against that immutable commit. Only then may the authorized normal branch push and one PR targeting `main` proceed. Inspect completed hosted CI for the exact contribution and report mergeability. No hosted success, DCO contribution success, final preservation pass or PR URL is invented here.

## 4. Hardware Validation

**No new hardware measurement, calibration or benchmark matrix was performed during package preparation.** Tests used synthetic sensor fixtures and private runtime/configuration directories. They did not scan host hwmon or modify kernel drivers, modules, firmware, fan controls, voltages, service configuration or existing user applications.

Native display checks exercised the current Linux KDE Wayland/XWayland host, with software rendering; they do not qualify physical GPU acceleration or every desktop. The earlier IT8613E fan observation remains documented separately, with its raw-channel, driver and physical-mapping limitations. It is not repeated or promoted into new package-hardware evidence.

## 5. Architecture and Design Review

Packaging preserves the existing split: the foreground daemon owns acquisition, the opt-in current-user Unix socket exposes only read access, and the GUI reads cached observations without initiating scans. Package installation has no runtime lifecycle side effect. Cooperative shutdown, recoverable acquisition failures, raw-value/identity fidelity, explicit stale/historical states and presentation-only unit conversion remain unchanged.

The manual daemon and absent boot service deliberately follow the bounded Alpha 1 scope; packaging does not implement the broader eventual boot-service direction. Decision 43's approved Alpha 1 current-user read-only policy is unchanged, and broader permissions remain pending. No new public HTTP API, remote monitoring, hardware controls, calibration or protective actions were introduced.

Using native dependency resolution and separate distro builds avoids claiming that an earlier host-specific binary is portable. Explicit runtime-library declarations account for libraries loaded dynamically by the GUI. The GUI CI extension closes the previous headless-only coverage gap; its actual hosted result remains a publication gate. The additional runtime harnesses and notice review are external evidence, not new application abstractions or dependencies.

## 6. Known Issues and Risks

- The two fresh instrumented Wayland snapshot failures remain unexplained. Full-workflow/lifecycle/profile-restart qualification on Wayland is incomplete despite passing production launch/tray tests.
- Native package installation was not executed on Linux Mint, CachyOS or EndeavourOS; other Fedora releases, derivatives and unrecorded Arch snapshots are unqualified. Host launch is not package installation on that host distribution.
- Virtual displays/software rendering do not qualify physical GPU drivers, a separate Xorg session, GNOME, mixed monitors, human tray interaction or complete keyboard/assistive-technology behavior.
- Numeric CPU, RSS and latency targets were not measured here. Generic x86-64 compilation is not a test of every CPU. The differing measured glibc floors limit portability.
- Packages are unsigned candidates. Matching corresponding source must accompany any later distribution. The package provenance accurately records a precommit candidate; no public base commit is represented as containing the full GUI.
- The bounded dependency notice review is not a legal certification of the entire dependency closure. Its supplementary notice is external to the unchanged tested archives.
- Two historical supplemental root all-target Clippy findings remain; configured checks pass. No application source change was made to suppress them.
- Final preservation, exact-contribution checks and hosted results remain pending at this draft snapshot. No production or safety-critical readiness is claimed.

## 7. Git Status

At report preparation, branch `codex/alpha1-prototype010` remains at **`3b6b85732b374d9d3dbc43ca052194475071bda9`**, the reviewed Prototype 009 merge base. The index is empty, the candidate changes are unstaged/untracked, and the branch has no configured upstream. No contribution commit, push or PR exists at this snapshot. A separate publication-readiness report and external continuation will record later authorized publication outcomes without rewriting this historical state.

The planned contribution contains **99 paths**: the existing 85-file Prototype 010 candidate (including the ignored fontdue lockfile), eight packaging files, one workflow update and five publication/release/legal reports. Six existing publication-candidate documents—README and five Prototype 010 reports/proposal—have authorized consistency/privacy changes; the other 79 pre-existing candidate files retain their implementation bytes. Two milestone reports in the list are being prepared alongside this snapshot. The final manifest must check each exact path and SHA-256 before staging, including explicit inclusion of the otherwise ignored fontdue lockfile.

The intended complete repository-relative path list follows. It excludes external archives, installers, private evidence, prior local deliverables, website content, agent setup and all unrelated changes.

```text
.github/workflows/alpha1.yml
Cargo.lock
Cargo.toml
README.md
docs/GUI.md
docs/legal/NOTICE-simd_helpers-0.1.0.txt
docs/legal/alpha1-dependency-notice-review.md
docs/milestones/010-alpha1-publication-readiness.md
docs/milestones/010-alpha1-readme-website-update.md
docs/milestones/010-it8613e-fan-telemetry-verification.md
docs/milestones/010-local-user-test-package.md
docs/milestones/010-multidistro-package-preparation.md
docs/milestones/010-native-gui-proposal.md
docs/milestones/010-native-gui.md
docs/milestones/010-native-gui/details-font24-200pct.png
docs/milestones/010-native-gui/details-oled.png
docs/milestones/010-native-gui/import-preview.png
docs/milestones/010-native-gui/main-dark.png
docs/milestones/010-native-gui/main-light-font24.png
docs/milestones/010-native-gui/main-oled.png
docs/releases/v0.1.0-alpha.1.md
gui/Cargo.toml
gui/build.rs
gui/src/app.rs
gui/src/bin/native_check.rs
gui/src/client.rs
gui/src/client/os.rs
gui/src/client/tests.rs
gui/src/lib.rs
gui/src/lifecycle.rs
gui/src/main.rs
gui/src/model.rs
gui/src/model/clock.rs
gui/src/model/tests.rs
gui/src/preferences.rs
gui/src/preferences/tests.rs
gui/src/presentation.rs
gui/src/protocol.rs
gui/src/protocol/tests.rs
gui/src/statistics.rs
gui/src/tray.rs
gui/src/tray/tests.rs
gui/ui/common.slint
gui/ui/details.slint
gui/ui/main.slint
gui/ui/settings.slint
packaging/linux/INSTALL.md
packaging/linux/README.md
packaging/linux/arch/.SRCINFO
packaging/linux/arch/PKGBUILD
packaging/linux/build-deb.py
packaging/linux/fedora/siliconwatch.spec
packaging/linux/siliconwatch.desktop
packaging/linux/stage_payload.py
vendor/fontdue-0.9.5-PATCH.md
vendor/fontdue-0.9.5-rust185.patch
vendor/fontdue-0.9.5/.cargo_vcs_info.json
vendor/fontdue-0.9.5/.github/workflows/build.yml
vendor/fontdue-0.9.5/.github/workflows/format.yml
vendor/fontdue-0.9.5/.gitignore
vendor/fontdue-0.9.5/CHANGELOG.md
vendor/fontdue-0.9.5/Cargo.lock
vendor/fontdue-0.9.5/Cargo.toml
vendor/fontdue-0.9.5/Cargo.toml.orig
vendor/fontdue-0.9.5/LICENSE-APACHE
vendor/fontdue-0.9.5/LICENSE-MIT
vendor/fontdue-0.9.5/LICENSE-ZLIB
vendor/fontdue-0.9.5/README.md
vendor/fontdue-0.9.5/images/layout.png
vendor/fontdue-0.9.5/images/rasterize_cff.png
vendor/fontdue-0.9.5/images/rasterize_glyf.png
vendor/fontdue-0.9.5/licenses/rust-lang/libm
vendor/fontdue-0.9.5/licenses/xi-editor/xi-unicode
vendor/fontdue-0.9.5/rustfmt.toml
vendor/fontdue-0.9.5/src/font.rs
vendor/fontdue-0.9.5/src/hash.rs
vendor/fontdue-0.9.5/src/layout.rs
vendor/fontdue-0.9.5/src/lib.rs
vendor/fontdue-0.9.5/src/math.rs
vendor/fontdue-0.9.5/src/platform/float/as_i32.rs
vendor/fontdue-0.9.5/src/platform/float/atan.rs
vendor/fontdue-0.9.5/src/platform/float/atan2.rs
vendor/fontdue-0.9.5/src/platform/float/ceil.rs
vendor/fontdue-0.9.5/src/platform/float/floor.rs
vendor/fontdue-0.9.5/src/platform/float/fract.rs
vendor/fontdue-0.9.5/src/platform/float/get_bitmap.rs
vendor/fontdue-0.9.5/src/platform/float/mod.rs
vendor/fontdue-0.9.5/src/platform/float/sqrt.rs
vendor/fontdue-0.9.5/src/platform/float/trunc.rs
vendor/fontdue-0.9.5/src/platform/mod.rs
vendor/fontdue-0.9.5/src/platform/simd_core.rs
vendor/fontdue-0.9.5/src/platform/simd_x86.rs
vendor/fontdue-0.9.5/src/raster.rs
vendor/fontdue-0.9.5/src/table/gsub.rs
vendor/fontdue-0.9.5/src/table/kern.rs
vendor/fontdue-0.9.5/src/table/mod.rs
vendor/fontdue-0.9.5/src/table/parse.rs
vendor/fontdue-0.9.5/src/unicode/mod.rs
vendor/fontdue-0.9.5/src/unicode/tables.rs
```

The fresh packaging baseline covers all five repository worktrees, Git indexes/refs, original agent-skills setup, intentional Prototype 006 changes, historical/private measurement evidence, earlier packages and the currently preserved website artifacts. Authorized documentation redactions preserve originals externally. The final comparison must distinguish permitted candidate/docs/Git publication changes from all protected material; its pending state is not a preservation pass. Historical user-authorized cleanup of older website exports is a prior recorded event, not an action of this package task.

## 8. Recommended Next Milestone

**Recommended next authorization: Chris's separate merge approval for the reviewed Alpha 1 source PR, after the current conditional publication work and hosted verification finish.** This is a review gate within Prototype 010, not implementation of Prototype 011.

Acceptance requires the exact 99-file contribution and preserved bytes to be reviewed; the signed-off commit's parent, full SHA, author/trailer match and DCO checker to pass; all required tests to pass against that immutable commit; one normal branch push and one PR targeting `main`; completed hosted Rust/Python/DCO results tied to the published HEAD; and an explicit account of GitHub mergeability, known Wayland limitations, package hashes and preservation findings. The PR must remain open and unmerged for Chris. Public tag creation, package upload/release publication and any subsequent prototype need their applicable separate authorization. Draft release notes may prepare those later decisions but do not perform them.

## 9. HANDOFF TO CHATGPT

Chris requested native Alpha 1 package preparation for the existing Prototype 010 implementation, followed by conditional source publication only if the required gates pass. Three unsigned Linux x86-64 candidates were prepared: Debian `.deb`, Fedora `.rpm` and Arch `.pkg.tar.zst`. Their exact hashes, sizes, native versions, dependency declarations and external provenance are recorded above and in `siliconwatch-alpha1-packaging/artifacts`. No public tag, binary upload, merge or next prototype is authorized by this work.

Rust 1.85 built the unchanged implementation in Debian 12, Fedora 44 and a recorded Arch rolling environment with generic x86-64 code generation. Each package has 730 application files and only the CLI/foreground-daemon and GUI executables. The 728 nonbinary files match across all three packages. Installation starts no daemon and installs no service, autostart, driver configuration or hardware control. Debian/Ubuntu, Fedora and Arch installation, synthetic CLI/API/permission tests and native removal passed; user settings sentinels survived. Minimal-image document filters were corrected only in disposable test environments before complete-payload verification.

Fresh Debian Rust verification passed formatting, configured workspace Clippy, GUI all-target Clippy, 166 headless tests, 106 GUI/CLI tests and two doctests. Two normally ignored private-bus tests passed separately after an isolated UID-database correction. Strict Python ran 153 tests without skips, DCO-checker fixtures passed six tests, and standalone fontdue passed four upstream unit tests. Two historical supplemental root all-target Clippy findings remain distinct from these passing checks. No new hardware measurements or benchmarks were performed.

The three final production binaries each passed current-host KDE Wayland and XWayland launch/tray/quit checks: six final passes. Initial X11 helper failures were resolved by supplying the missing external test tool. Separately, the instrumented native probe passed four of six cases: XWayland workflow, close and lifecycle plus two profile restarts, and Wayland default close. Wayland full-workflow and lifecycle initial-snapshot stages exceeded their budgets; their cause remains unresolved and those profile restarts were not reached. These limits must remain visible.

The license review found no demonstrated material blocker from the specific MIT-declared, compile-only `simd_helpers` missing-text finding; public review and notice supplements preserve the evidence without changing tested packages. The approved fontdue patch and original licenses remain intact. README and five prior documents received authorized consistency/privacy updates, with originals retained externally; GUI CI coverage was added.

At this precommit snapshot, `codex/alpha1-prototype010` is still at `3b6b85732b374d9d3dbc43ca052194475071bda9`, with an empty index, unstaged/untracked changes and no upstream or PR. The planned contribution is 99 explicit paths including the ignored fontdue lockfile. Final preservation, exact-commit tests and completed hosted CI are still required. If those gates pass, finish the authorized signed contribution and PR, then stop with it open and unmerged. Chris's separate merge decision is the next review action; do not begin Prototype 011 or publish release binaries automatically.
