# SiliconWatch Prototype 010 — Local User-Test Package

Date: November 17, 2026. This separate packaging report preserves the earlier `010-native-gui.md` implementation/tray report without modification.

## 1. Milestone Summary

**Packaging: Complete. Native human qualification: Partial. Technical acceptance: Pending.** Prepared the authorized local CachyOS KDE Plasma AppImage and a separate unchanged prebuilt daemon/CLI so Chris can evaluate without compiling Rust. Actual artifact execution and payload verification passed. No public release, system installation, service deployment or next prototype was performed.

Delivery directory: external `SiliconWatch-Prototype010-CachyOS-UserTest-2026-11-17/` folder. Final filename: `SiliconWatch-Prototype010-CachyOS-x86_64.AppImage`; 10,738,168 bytes; SHA-256 `5de775797f2abc7d9a71f60a743a25af8df3de713dbdb763c27ce49ff3652bc2`.

## 2. Implementation Details

No existing implementation was recreated, recompiled, cleaned, stripped or patched. The AppImage copies the existing production Rust 1.85 release GUI exactly (SHA-256 `c33d5b3f861700f03a57b75e128b9105e2d2d23aab2c6015534e4c4e12f48a70`). The separate daemon sidecar remains `e00f76af18db8e9ba6dec0b4a91afd0dc4e292b21dd5fc0e30914f222c284a4d`. The original user-supplied logo remains `e987efdaf116348e7c2dbc89496a28bfc3336a65763409939bd3d5844f49be59`.

Slint/slint-build remain exactly 1.13.1 with std/backend-winit/renderer-femtovg/accessibility/compat-1-2; MSRV stays 1.85. Cargo's local fontdue 0.9.5 patch, its exact six conversions, parser soundness source, upstream licenses/provenance and intentionally ignored vendor Cargo.lock remain unchanged. Locked/offline Linux metadata still resolves the same 445 nodes; no Cargo graph/version/feature change was made.

The only repository addition for this task is `docs/milestones/010-local-user-test-package.md`. The delivery folder contains exactly eight user-facing files: AppImage, `siliconwatch`, `siliconwatch-icon.png`, `README.md`, `KNOWN-LIMITATIONS.md`, `VERIFICATION.md`, `LICENSE`, `SHA256SUMS`. `README.md` covers launch, manual daemon, connection checks, Sensors/Details, tray settings, XDG settings/export/recovery, exit/removal, optional KDE launcher and troubleshooting. No private evidence, test fixture or capture is distributed.

### Packaging method, dependencies and provenance

An isolated official appimagetool 1.9.1 (commit `8c8c91f762b412a19f4e8d2c4b35afb98f2d7c81`, SHA-256 `ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0`) assembled a type-2 AppImage using its bundled SquashFS 4.6.1 writer, zstd and explicit runtime release 20251108 (commit `dd6cebedcbddde9c82f89b011e8e1d40b6e43868`, SHA-256 `2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d`). Tool/runtime hashes matched official release metadata. Signature checking used an isolated downloaded key and is not claimed as independent key trust. No installed tooling was changed and no update/publication configuration was added.

The AppRun shell establishes only its relative library path and executes the production GUI. The desktop file and exact original icon are embedded; no launcher is installed. Eleven non-driver libraries are bundled: libXau, libXcursor, libXdmcp, libXext, libXfixes, libXi, libXrender, libffi, libxcb-xkb, libxkbcommon-x11, libxkbcommon. Core loader/glibc/libgcc, matching graphics drivers/GL/EGL/GLX, core X11/Wayland, fonts/font configuration, keyboard data and desktop services remain explicit host dependencies following standard AppImage platform exclusions. This passes the gate for the tested local target, not universal Linux portability.

Full available top-level notices plus 48 nested Cargo LICENSES files and 18 runtime notice/provenance files are retained. The 445-node Cargo closure attribution includes build/optional packages; it does not assert every package is linked. Some published crates provide only SPDX/source metadata. Runtime notices cover MIT/MD5 public-domain, musl, libfuse, squashfuse, zstd/xxhash, zlib and mimalloc. Runtime source/bytes are pinned; upstream unversioned Alpine package resolution limits exact transitive build reproducibility. No complete corresponding-source/public-release certification is asserted. The fontdue patch note and exact diff remain embedded and unchanged.

Tooling/source references: [appimagetool 1.9.1](https://github.com/AppImage/appimagetool/releases/tag/1.9.1), [runtime source](https://github.com/AppImage/type2-runtime/tree/dd6cebedcbddde9c82f89b011e8e1d40b6e43868), [AppImage concepts](https://docs.appimage.org/introduction/concepts.html), [reviewed exclusions](https://raw.githubusercontent.com/AppImageCommunity/pkg2appimage/15a64c20dc23a0154622ba25829364323903b6b5/excludelist). Download provenance and exact assembly command are retained in the new external evidence/scratch paths below.

## 3. Verification Results

All newly executed final checks passed. Full twelve-request qualification detail is in the external delivery's `VERIFICATION.md`; each partial/unexecuted native check is explicit.

| Check | Actual result |
| --- | --- |
| Payload/extraction | All 810 manifest/AppDir/fresh-extraction entries match; 759 Cargo notice source files, 18 runtime files, 19 provenance source hashes and 17 runtime-checksum entries verified. No external symlinks, privileged payload or private development files. |
| Runtime integrity | Expected 16-byte `.digest_md5` AppImage metadata change only; runtime code otherwise matches download. Runtime help/version/extract pass. |
| Shell/desktop entry | `sh -n` and `desktop-file-validate` pass. Optional user desktop integration was documented, not installed or qualified. |
| Actual GUI matrix | 6/6 final cases pass: Wayland/XWayland × default false/saved true; full GUI extraction fallback; offline missing-endpoint. 11/11 GUI launches exit 0, 5/5 exact-profile restarts match. |
| Native windows/icon | Exact-process mapped Sensors/Settings XWayland windows and 1254×1254 icon property observed; SNI 32/64/128 pixmaps verified. Rendered pixels, panel/menu appearance and Wayland surface visibility unobserved. |
| API/cache | Actual production AppImage receives valid temperature/fan rows through unchanged protocol. 35 recorded cache responses; Connected tooltip. Held acquisition retains publication 1; releasing it yields publication 2 and expected raw 99000. |
| Quit/acquisition | All ten connected GUI exits leave separate daemon alive. Five owned daemon cleanups exit 0. Only get_info/get_snapshot forwarded; one FIFO reader/three daemon threads while held; no sampled GUI fixture/sysfs acquisition descriptor. Bounded evidence, not exhaustive tracing. |
| Offline | Disconnected/Waiting, no endpoint creation or observed daemon/hwmon descriptor. |
| Bundled dependencies | All eleven libraries loaded from AppImage mount and hashes matched on XWayland; four used on Wayland. Host graphics/runtime paths observed separately. |
| Permissions/cleanup | Private directories 0700, config/proxy 0600; owned workers joined, daemon endpoints and runtime mounts removed; no owned process remains. |
| Delivery | Exact-byte relocated AppImage and daemon exercised directly in XWayland, full extraction and offline cases. All seven checksum entries pass. |
| Source build/test checks | Not rerun: no source/dependency/build-output changes. Retained previous Rust 1.85/stable fmt/configured Clippy/locked tests/release, 166 headless, 105 GUI, 1 CLI and 2 doctests, separately executed 2 platform tests per toolchain, Python 153, DCO 6 and fontdue gates are documented in unchanged `010-native-gui.md`. These are preceding results, not new packaging executions. |

No final package-runtime case failed. Candidate1's exploratory Wayland test also passed but is separate from the final artifact. An initial independent audit incorrectly treated three added BUILD-INFO provenance fields as forbidden; original report retained, final audit validates those exact fields and passes. A checksum-preparation command initially used a duplicated relative directory and made no changes; corrected invocation passes. These were tooling checks, not hidden GUI failures.

Device Details, rendered OLED/readings/icon appearance, physical keyboard/mouse/close/hide/restore, UI export/import, Orca and actual accessibility tree are unexecuted for this production package because suitable native interaction/inspection tools were unavailable. No input-control bypass, synthetic probe or prior screenshot was substituted. User authorization permits recording unavailable qualification separately; it does not imply acceptance.

## 4. Hardware Validation

Executed on CachyOS KDE Wayland, kernel 7.3.4-1-cachyos, glibc 2.44, normal non-root account. The existing GUI ELF requires GLIBC_2.43, declares x86-64-v4 and Linux ABI 6.1.0. Those requirements are explicit, not removed or disguised. Runtime tests use private temperature/fan fixtures and independently started unchanged daemons; no new physical hardware acquisition was performed for packaging. Prior non-root read-only hardware smoke remains recorded in the unchanged native-GUI milestone and its private evidence. X11 here means XWayland inside KDE Wayland, not a separate Xorg session.

## 5. Architecture and Design Review

Packaging preserves the approved Slint architecture, daemon protocol, security policy and sensor semantics. A separate unmodified prebuilt daemon removes a compilation requirement for Chris while retaining explicit manual startup. AppRun neither starts a daemon nor repairs endpoints/configuration. No alternative framework, dependency downgrade, privileged service, unsupported renderer or Arch fallback was introduced. Host graphics/glibc requirements are the bounded compatibility contract of this local artifact. General Linux portability would require separately scoped build/qualification work.

## 6. Known Issues and Risks

See the delivered `KNOWN-LIMITATIONS.md`. Local runtime qualification does not establish full visual/accessibility quality. Fresh default/persistence checks establish profile values, not rendered colors or physical close behavior. Device Details and UI import/export await hands-on testing. Different CPUs, older glibc, independent Xorg, other desktops, mixed-monitor scaling and broader graphics/performance remain unqualified. Wayland taskbar association/foreground activation are compositor-dependent. Prior unknown intermittent timeout/visibility causes and large-font layout limits remain documented; no new GUI changes were made to hide them. No blockers prevent testing on the verified host; broad portability and complete native qualification remain open.

## 7. Git Status and Preservation

Branch `codex/alpha1-prototype010`, HEAD `3b6b85732b374d9d3dbc43ca052194475071bda9`, empty index. No staging, commit, push, PR, merge, system install, service deployment or Prototype 011. Preflight verified all five worktrees and exact previous 81-file contribution; all existing source, approved proposal, vendor, intentional modifications, agent skills, historical/private evidence and old build outputs were frozen. Only this new report is added in the repository; complete current contribution is 82 paths including the unchanged intentionally ignored vendor Cargo.lock.

**Final preservation audit: PASS, zero unexpected differences.** All five worktrees/refs and the empty index are unchanged; the exact previous 81-path implementation is preserved, with only this new report added (82 contribution paths total). Fresh locked/offline metadata matches all package and resolution entries, including 445 Linux nodes. Delivery allowlist is exactly eight files, and all seven checksum entries pass. No owned build/daemon process remains. The final audit was refreshed after this report was finalized; its exact report hash is recorded in `final-changed-files.json`.

Fresh private evidence: `/tmp/siliconwatch010-package-7ptfhv2a`. Important records: `preflight-report.json`, `preservation-before.json`, `dependency-review.json`, `appdir-v2-independent-review-final.json`, `appdir-v2-payload-manifest.json`, `packaged-runtime-final-results.json`, `packaged-runtime-permissions.json`, `delivery-manifest.json`, `preservation-after.json`, `final-changed-files.json`. Exact final manifest records every contribution path/status/size/hash. The baseline protects 504 existing working files, 9,419 ignored entries / 1,552 directories, 1,446 earlier external files, and 38,048 existing generated work entries / 8,660 directories. Generated files include 36,982 hashed regular files, 1,064 large metadata-only records and 2 symlinks; metadata-only preservation is not claimed as byte hashing.

All new scratch/tooling/candidates are isolated in the external packaging workspace. Previous targets and candidate1 are retained. The distributable contains none of the private evidence or testing tools. This report's user-facing copy is retained externally as `010-local-user-test-package-report.md`.

## 8. Recommended Next Action

**Chris's Prototype010 hands-on evaluation**, with no further GUI changes before feedback. Follow the delivered guide to confirm actual OLED appearance/readings, Details, Settings, physical keyboard navigation, tray close/restore/quit and settings export/import/restart. Report defects and platform observations. The requested outcome is feedback, not technical acceptance. Any subsequent fix, broader portability, publication or acceptance action requires the appropriate later authorization; Prototype 011 has not begun.

## 9. HANDOFF TO CHATGPT

The local Prototype 010 AppImage (external local-test delivery) is ready for hands-on testing. The installation guide in that external delivery provides exact commands; no Rust compiler is needed. The delivery folder also contains the separate prebuilt daemon, your original SiliconWatch icon, checksums, verification results and known limitations. Nothing was installed system-wide, published or configured as a service.

The AppImage is `SiliconWatch-Prototype010-CachyOS-x86_64.AppImage`, 10,738,168 bytes. Its SHA-256 is:

```text
5de775797f2abc7d9a71f60a743a25af8df3de713dbdb763c27ce49ff3652bc2
```

Packaging used official appimagetool 1.9.1 and a pinned type-2 runtime, with eleven ancillary libraries and upstream notices. The unchanged Rust 1.85 production GUI retains Slint 1.13.1, the approved fontdue patch, OLED Black defaults, Settings, Device Details, tray support and portable TOML preferences. The daemon remains separate and must be started manually using the guide. This build requires x86-64-v4, glibc 2.43 or newer, Linux 6.1 or newer and working desktop graphics; other Linux environments are unqualified.

Six final packaged-runtime cases passed across Wayland, XWayland, full extraction fallback and offline startup: eleven GUI launches, five preference restarts and thirty-five recorded cache responses. Sensors and Settings were confirmed through mapped XWayland window metadata. Window/tray icon properties contained the supplied artwork. Fresh profiles saved OLED Black. GUI Quit left every separately started daemon alive, and bounded fixture checks found no extra acquisition. All owned test processes were stopped afterward.

These results do not complete native human qualification. Rendered appearance/readings, Device Details interaction, physical keyboard and tray close/restore behavior, screen-reader access and settings import/export through the packaged UI remain unexecuted. They are explicitly distinguished from passing checks in the external delivery's verification notes. No packaged screenshots are claimed, and private development evidence is excluded from the delivery folder.

The branch remains `codex/alpha1-prototype010` at `3b6b85732b374d9d3dbc43ca052194475071bda9`, with an empty index. Existing implementation, worktrees, build outputs and earlier evidence are preserved; packaging adds only its new repository report. No staging, commit, push, PR, merge, deployment or Prototype 011 work occurred. The repository `/handoff` skill was invoked and its continuation saved outside every worktree. Technical acceptance remains **Pending**. The next action is Chris's hands-on feedback; no further GUI changes or acceptance request will begin before that feedback.

The explicitly invoked repository `/handoff` skill saved the continuation at `/tmp/siliconwatch010-package-7ptfhv2a/CONTINUATION.md`, outside all repository worktrees and the current workspace. It references this report, delivered guide and private evidence, includes suggested skills, and does not duplicate specifications or secrets.

> Publication privacy review — November 17, 2026: personal home-directory paths and local-only file links were replaced with portable descriptions. The original report is preserved unchanged in the private external publication-review archive. Historical dates, verification results, failures and counts are unchanged.
