# Prototype 010 — Alpha 1 publication readiness

November 17, 2026. **Pre-commit review: ready for the conditionally authorized
contribution and PR, with documented experimental-alpha limits.** This record
does not assert a hosted CI result, merge, tag or release. Exact final contribution
SHA, PR and hosted results belong in the external post-publication handoff.

## Authorization and scope

Chris authorized finishing package qualification and preparing the first public
experimental alpha, `v0.1.0-alpha.1`, subject to verification gates. A DCO-signed
contribution and PR may proceed after those gates pass. Merge requires separate
explicit approval. No public tag, binary upload, GitHub release publication,
Cloudflare change, service deployment or Prototype 011 is authorized here.

The GUI implements the approved Slint 1.13.1 path: OLED Black/Dark/Light, Sensors
and session statistics, Device Details, conservative source identity/currentness,
portable preferences, reviewed import/export, and optional Minimize to Tray.
The separately started Rust daemon remains the acquisition owner. The GUI uses
only current-user read-only API methods. No new runtime implementation or Rust
dependency change was made during packaging/publication preparation.

## Package and local gates

The [package report](010-multidistro-package-preparation.md) and external
`siliconwatch-alpha1-packaging/artifacts/` directory contain exact package sizes,
checksums, native versions, 730-file manifests, dependency reports and provenance.
All 93 recorded build/documentation input hashes still match the proposed source.
The intentionally ignored upstream fontdue Cargo.lock is separately verified
and explicitly included below.

- Fresh Rust 1.85 target builds: Debian 12, Fedora 44 and Arch, generic x86-64
  application code generation, locked/offline dependency resolution.
- Clean native installation, fixture CLI/JSON, daemon/API, permissions, shutdown
  and removal: Debian 12.15, Ubuntu 24.04.5 LTS, Fedora 44 and the recorded Arch
  rolling image. User preference bytes/modes survived removal. Build tools were
  separated from initial runtime dependency verification.
- Package audit: exactly two executables and 728 identical nonbinary payload
  files across formats. Icons, desktop metadata, documentation and notices are
  present. No app installation hooks, services, autostart, drivers, privileged
  modes, test fixtures, private captures or agent setup. Supplemental attribution
  is separate; tested package bytes are unchanged.
- Formatting and configured workspace Clippy pass; GUI all-target Clippy with
  `ui-verification` passes. Workspace tests: 166 headless, 106 GUI/CLI, two
  doctests. Two private-bus tests pass separately. Strict Python: 153 tests with
  zero skips. DCO-checker tests: six. Standalone fontdue tests: four on Rust 1.85.
  The two historical supplemental root all-target Clippy findings remain
  documented in `tests/hwmon.rs:217` and `tests/identity.rs:368`.

Package minimum referenced GUI glibc is 2.35 for Debian and 2.43 for Fedora/Arch;
CLI maximum references are 2.34. Dynamic library checks include Winit/FemtoVG's
dlopen libraries and fonts, not just DT_NEEDED. Distribution CPU, kernel and
graphics requirements still apply. Signed package distribution, Fedora 45,
Linux Mint, CachyOS/EndeavourOS installation and portable AppImage compatibility
are not established.

## Native desktop and hardware limits

Production package binaries passed six native-host launch/tray/connection/quit
checks: all three binaries on the available CachyOS KDE Wayland and XWayland
backends. Exact package binary hashes were used; this was not a host package
installation. Virtual X11 checks passed on all four installation environments;
virtual Wayland checks also passed on Debian/Ubuntu. All GUI-owned test resources
were cleaned up and separate fixture daemons survived GUI Quit before their
owned cooperative shutdown.

The deeper native probe passed **4/6 cases**: XWayland full/default-close/lifecycle
and Wayland default-close. Two XWayland profile restarts passed. Wayland full and
lifecycle timed out at their initial screenshot phases, at 60 and 45 seconds.
Their causes remain unproven. Successful production launches and XWayland runs
do not resolve those failures. Current XWayland captures verify all three themes,
Details, settings and readable fixture data. Human keyboard/screen-reader
qualification, physical tray/menu interaction, GNOME, native Xorg and mixed
monitor behavior remain incomplete.

No new hardware measurements occurred during this task. The retained IT8613E
case demonstrates the architecture's driver boundary: after the kernel exposed
two fan channels through hwmon, unchanged SiliconWatch displayed their RPM.
It does not establish universal Super I/O/motherboard coverage, calibrated
readings, physical fan mapping, reboot persistence or future driver compatibility.
The existing documented third-party module/signature/taint limitations remain
part of that historical case. No driver forcing or hardware-control advice is
introduced.

The implementation remains suitable only for the authorized experimental alpha
review/testing scope. No production-readiness, protective function or complete
accessibility claim is made. The current-user API does not isolate hostile
same-UID applications; real cross-UID isolation and comprehensive JSON metaschema
qualification retain their earlier documented limits. Cooperative shutdown can
wait for blocked driver/output operations; stale endpoint handling is manual.

## License and source review

The [dependency notice review](../legal/alpha1-dependency-notice-review.md) records
why no material blocker was identified from the simd_helpers finding: the exact
upstream crate declares MIT, has no existing copyright notice to preserve or
invent, and is confined to compile-time tooling outside the runtime payload.
The standard-terms [attribution supplement](../legal/NOTICE-simd_helpers-0.1.0.txt)
is a separate proposed release asset. The original package's conservative notice
record remains unchanged. This is a bounded finding, not blanket legal certification.

The fresh fontdue comparison covers all 43 files: only the approved six
same-width conversions in `src/table/parse.rs` differ; the other 42 files,
original licenses, parser validation and upstream Cargo.lock match. The ignored
lock is 3,081 bytes, SHA-256
`89a337d215e4c22cc8604a6e5f7e2398ca3c02066de8e878538d56c68f2a5762`.
It must be staged explicitly by path, not by forcing an entire ignored tree.

The eventual tag must identify the separately approved merged source matching
the recorded build inputs. The prior public base alone lacks the GUI. Prepared
[release notes](../releases/v0.1.0-alpha.1.md) retain the exact tested package
hashes, manual startup and driver/desktop boundaries. The Cloudflare follow-up
is prepared separately and must wait until a real release exists.

## Publication preparation and preservation

Pre-contribution branch: `codex/alpha1-prototype010`. Parent/base and fetched
`origin/main`: `3b6b85732b374d9d3dbc43ca052194475071bda9`. No divergence or equivalent
open PR was found. The index was empty before staging. Repository requirements
are AGENTS, Decision 41 and the existing DCO checker; no separate CONTRIBUTING
file or required branch-protection contexts were found. User verification gates
remain binding even without enforced GitHub contexts.

CI is explicitly expanded to workspace formatting/Clippy/tests, GUI all-target
Clippy, private-bus tests, standalone fontdue tests and workspace release builds,
while retaining strict Python and DCO tests/signoff verification. This prevents
the root package's `default-members` setting from silently omitting GUI checks.
Those hosted results must be inspected for the exact final contribution HEAD.

Five historical reports received only reviewed privacy/path redactions and dated
notes. Exact originals remain in the private external archive; dates, findings,
failures and counts are retained. README now reflects conditional alpha
authorization and measured compatibility limits. Among the original 85
contribution paths, the 79 non-document inputs remain byte-identical; the six
authorized document updates are README plus those five reports. New files are
packaging recipes, current reports, the license review/supplement and release
draft. Original/007/008/009 worktrees, agent skills, intentional Prototype 006
changes, historical benchmarks, private evidence and previous user-test artifacts
remain protected by the preservation audit. No source target or evidence was
cleaned.


The raw staged whitespace check flags preserved upstream CRLF endings in four
fontdue files and five single-space blank context lines in the verbatim unified
patch. The four files match pinned upstream bytes; the patch spaces are context
syntax. A per-command CRLF-aware check passes for every other file. These bounded
exceptions are recorded externally; vendor bytes and global Git settings were
not changed to silence the check.

## Remaining gates and recommended action

Create the reviewed signed-off contribution with the exact manifest below;
verify parent, author/committer, matching DCO signoff and full-SHA checker results.
Run required checks against that commit, push normally, open one PR against
`main`, and inspect all hosted jobs and final mergeability. If CI exposes a
material problem, preserve the failure and resolve it within authorization or
stop for review. This report does not pre-claim those future results.

Stop with the PR open and unmerged. Chris's next authorization is merge approval
after reviewing the exact contribution, final hosted results and disclosed
qualification failures. Release/tag/upload approval remains separate. Do not
begin Prototype 011 or change Cloudflare.

## Exact proposed publication manifest

The list includes every proposed changed path, including the ignored upstream
vendor lock. The external final manifest records exact SHA-256/size/mode values
and the staged Git blob IDs after these reports are frozen. No generated build
output, private evidence or agent-skills setup is included.

**99 paths:**

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
