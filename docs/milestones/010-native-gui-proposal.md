# SiliconWatch Alpha 1, Prototype 010: First Native GUI — proposal

Date: 2026-11-17. **Design revision complete; implementation not started or authorized.**

Revision scope: Chris approved revising the design to include the five-column hierarchical Sensors view, device-focused details and portable GUI profiles. This is planning authorization only. No GUI code, dependencies, daemon/API changes, Git publication or Prototype 011 work are authorized.

The proposed milestone delivers an actual native Linux desktop window showing live sensor observations from the accepted Prototype 009 local API. A client library, framework experiment or static mockup alone does not satisfy completion. This document requests review of the architecture, visual direction and acceptance gates; it does not authorize implementation, dependency installation, hardware measurements or publication.

## 1. Starting point and prior verified merge

The earlier planning record reports that Chris authorized a regular merge of [PR #8](https://github.com/lgraak/Siliconwatch/pull/8), followed by planning only. The merge is `3b6b85732b374d9d3dbc43ca052194475071bda9`. Its parents, in order, are previous main `231034f2822b9c91e8781c20579729374328280d` and the sole DCO-signed contribution `b3902b88b5f26f32a1b8b2e8699f0539a0401aa6`. Its tree, `8b61bcf45941312a6ec9a2100cac15858ebe0beb`, equals the reviewed contribution tree. Fetched `origin/main` matches the merge and contains the contribution. PR #8 is marked merged; no feature branch was deleted.

In that prior session, before merging, all 23 approved file hashes and paths, parentage, author-matching signoff, full-SHA DCO check and hosted checks were reverified. [Alpha 1 CI run 44145363315](https://github.com/lgraak/Siliconwatch/actions/runs/44145363315) passed formatting, configured Clippy, 166 Rust tests, two doctests, the release fixture build, 153 strict Python tests without skips, six DCO-checker tests and contribution signoffs. These are Prototype 009 results, not GUI validation. The two pre-existing supplemental all-target Clippy findings in `tests/hwmon.rs:217` and `tests/identity.rs:368` remain outside this proposal.

Planning worktree: `SiliconWatch-prototype010` (local worktree); branch: `codex/alpha1-prototype010`, created without upstream at the merge. The only repository file changed by this revision is this existing untracked proposal. Merge/hosted-CI statements below are preserved prior-session evidence, not rerun verification in this design revision. Existing README and historical milestone status statements retain their dated context.

## 2. Framework decision

Recommend **Slint `=1.13.1` with `slint-build =1.13.1`**, subject to an actual Rust 1.85.0 locked dependency/build check and native desktop/accessibility qualification during authorized implementation. This follows the README's existing Slint preference. Keep **egui/eframe 0.32.3** as the fallback requiring Chris's decision if Slint cannot pass those gates; do not silently raise the MSRV or change frameworks.

Current primary documentation and tagged manifests were reviewed on 2026-11-17. Pinning a compatible older release is intentional; the newest release is not interchangeable with the proposed version.

| Criterion | Slint 1.13.1 — recommended | egui/eframe 0.32.3 — alternative | Iced 0.13.1 — not selected |
| --- | --- | --- | --- |
| Rust integration | Declarative `.slint` compiled into Rust, Rust models/callbacks; separates visual structure from data/state. | Entire UI in immediate-mode Rust; particularly convenient for tooling and dashboards. | Rust message/update/view model; clear state transitions. |
| Declared MSRV | Rust 1.85. | Rust 1.85. | Rust 1.80, so top-level declaration fits 1.85. |
| Linux display | Winit supports Wayland and X11; proposed FemtoVG/OpenGL renderer. | Winit Wayland/X11 features; OpenGL/Glow or wgpu. | Winit Wayland/X11; default wgpu plus software rendering components. |
| Keyboard/accessibility | FocusScope/key/focus APIs and AccessKit mapping exist; grouped row semantics and Orca must be qualified. | Focus/repaint APIs and AccessKit integration available; customized table semantics and Linux Orca still need testing. | Focus helpers exist, but upstream screen-reader/AccessKit integration remains unfinished; insufficient basis for Decision 28. |
| Native behavior | Native desktop window and event loop, custom rendered controls; no claim of GTK/Qt widgets or automatic desktop-theme fidelity. | Native desktop window, deliberately custom visual controls. | Native desktop window, custom controls and application-managed keyboard behavior. |
| Idle/rendering | Event-loop updates, no animation required; bound row/model changes. OpenGL driver availability is a gate. | Immediate mode does not require continuous redraw; use requested/delayed repaints, not an unconditional loop. | Rendering/executor choices need review; newer reactive-rendering claims do not establish old-version behavior. |
| Build complexity | UI compiler/build helper plus windowing, rendering, font/image and accessibility dependencies. | No separate UI language/compiler; still substantial rendering/windowing dependencies. | Renderer/executor feature combinations and ecosystem components increase review surface. |
| Maintenance | Existing project preference and separated visual code; older pinned release carries upgrade/backport uncertainty. | Strong tooling/dashboard fit; fallback still uses an older MSRV-compatible version. | Good Rust architecture, but accessibility gap would invite out-of-scope custom/fork work. |
| Later graphs | Official Plotters/image-buffer example provides a future path, not a ready-made history system. | `egui_plot` ecosystem is appealing for later graphs. | Canvas drawing provides a future path. No plotting dependency is proposed now. |

Version evidence: tagged [Slint 1.13.1 workspace](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/Cargo.toml), [egui/eframe 0.32.3 workspace](https://raw.githubusercontent.com/emilk/egui/0.32.3/Cargo.toml) and [Iced 0.13.1 manifest](https://raw.githubusercontent.com/iced-rs/iced/0.13.1/Cargo.toml). Newer inspected versions raise the baseline: [Slint 1.14.1](https://raw.githubusercontent.com/slint-ui/slint/v1.14.1/Cargo.toml) and [egui 0.33.0](https://raw.githubusercontent.com/emilk/egui/0.33.0/Cargo.toml) require 1.88, as does [Iced 0.14.0](https://raw.githubusercontent.com/iced-rs/iced/0.14.0/Cargo.toml); inspected [Slint 1.19.0](https://raw.githubusercontent.com/slint-ui/slint/v1.19.0/Cargo.toml) and [egui 0.37.0](https://raw.githubusercontent.com/emilk/egui/0.37.0/Cargo.toml) require 1.95.

The future Slint configuration should disable default features and explicitly enable `std`, `backend-winit`, `renderer-femtovg`, `accessibility`, `compat-1-2`. In this release `backend-winit` includes both Wayland and X11; explicit selection avoids the default Qt selection. FemtoVG uses OpenGL; no software fallback is implied by this feature set. Match the build helper exactly. See the [pinned feature manifest](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/api/rs/slint/Cargo.toml), [build helper](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/api/rs/build/Cargo.toml) and [Winit backend graph](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/backends/winit/Cargo.toml).

The dependency gate is substantive: Slint's `image = "0.25"` range can select [image 0.25.10, which declares Rust 1.88](https://raw.githubusercontent.com/image-rs/image/v0.25.10/Cargo.toml). Exact top-level pins alone do not fix transitive drift. Cargo's [Rust-version-aware resolver](https://doc.rust-lang.org/cargo/reference/resolver.html#rust-version) helps select compatible packages but is not a build guarantee. Review the entire lockfile, build dependencies, features, target dependencies and license set; run the actual 1.85.0 build/test with that lockfile. Do not invent compatibility pins without inspecting the resolved graph. Runtime feature pruning does not remove all `slint-build` compiler dependencies.

The pinned [Slint accessibility adapter](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/backends/winit/accesskit.rs) maps groups, lists, list items, text and table, but does not establish table-row/cell semantics. Prefer a visually tabular grouped list with individually selectable, fully labelled accessible rows and a textual detail view. Use that release's [FocusScope/key APIs](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/compiler/builtins.slint), not newer `KeyBinding`/`@keys` examples. eframe's [native adapter graph](https://raw.githubusercontent.com/emilk/egui/0.32.3/crates/egui-winit/Cargo.toml) offers an alternative AccessKit path; the [Iced accessibility issue](https://github.com/iced-rs/iced/issues/552) and [draft integration](https://github.com/iced-rs/iced/pull/3111) explain its weaker fit. Availability of an adapter is not proof of this application's accessibility.

For maintainability, select Slint's declared GPL-3.0-only licensing option, matching this project's license, and retain required notices; review the resolved dependency licenses separately. Iced declares MIT; egui/eframe declare MIT OR Apache-2.0 in the manifests above. An older release does not imply an upstream backport commitment. [Slint's current style changes](https://slint.dev/blog/default-native-style-change) also mean current screenshots/examples must not be mistaken for pinned-version behavior.

All three can support future dashboards; examples are [Slint Plotters](https://github.com/slint-ui/slint/blob/master/examples/plotter/main.rs), [egui_plot](https://github.com/emilk/egui_plot) and Iced's Canvas feature. That does not justify adding graphs now. No framework was installed, built or benchmarked in this planning task, and no ranking of measured CPU/RSS/idle overhead is claimed. Slint's [event-loop guidance](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/api/rs/slint/lib.rs) and egui's [repaint scheduling](https://raw.githubusercontent.com/emilk/egui/0.32.3/crates/egui/src/context.rs) support event-driven designs; actual overhead remains a qualification result.

## 3. First-window scope and daemon ownership

Recommend **Option A: a manually started daemon** for Prototype 010. The daemon already owns acquisition, can outlive the GUI, and has explicit startup/shutdown and stale-endpoint behavior. Closing the GUI must leave it running. GUI-managed ownership would add executable discovery, duplicate-instance arbitration, launch failure reporting, stderr handling, process reaping and the distinction between a borrowed daemon and a child. A blocked acquisition has no hard shutdown deadline; the GUI must not kill a shared daemon to make its window close. Defer all launch/supervision behavior, including systemd.

The documented first-window profile pairs this existing command with a GUI cache-refresh interval of 1000 ms:

```sh
cargo run -- daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

The eventual GUI launch command will be `cargo run -p siliconwatch-gui -- --refresh-ms 1000`, after its package is implemented. This command does not exist yet. The default refresh is 1000 ms; accept one canonical unsigned decimal `--refresh-ms` value in the proposed 1000–60000 ms inclusive range, rejecting zero, duplicates, nonintegers and checked-conversion overflow. A user with a slower custom daemon must explicitly choose a GUI interval no shorter than the daemon's acquisition interval. `get_info` does not expose that interval; `stale_after_ns` is not a polling cadence and must not be used as one. Document the paired profile prominently rather than pretending cadence is negotiated. The GUI may know only what its user configured, so this remains a disclosed limitation with arbitrary external daemons. No API extension is proposed.

The first usable window supports connection state, cache refresh, expandable source-device groups, the Sensor/Current/Minimum/Maximum/Average columns, valid session statistics, temperatures/voltages/fans, row diagnostics, historical/stale/failed/unsupported states, restart reconnection, keyboard operation, a separate device details window, and persistent appearance/layout/unit preferences with validated import/export/reset. No artificial sensors are added for categories absent from the inventory.

On initial failure, show an actionable inline message and the selectable daemon command. Do not create/chmod runtime directories, unlink stale endpoints, elevate privileges or launch a daemon. Missing environment, invalid permissions and identity mismatch require user correction. Normal absence/restart triggers bounded reconnect attempts. A reconnect control retries the connection; it is never a hardware-refresh button.

## 4. Revised window layout — 2026-11-17

This revision implements Chris's **design direction only**: a dense HWiNFO-style hierarchy with **Sensor, Current, Minimum, Maximum, Average**, plus a separate device-focused details window. It supersedes the earlier Value/Unit/Status table, selected-row-only details, session-only unit preference, and exclusions of statistics, column preferences and settings. The original proposal and original SVG/PNG remain preserved outside the repository; no prior backend evidence or accepted decision text is rewritten.

The specific HWiNFO screenshot mentioned in the request was not attached. The official [HWiNFO screenshot](https://www.hwinfo.com/wp-content/uploads/2018/10/hwinfo-screen-1024x728.jpg), linked from its [home page](https://www.hwinfo.com/), was visually inspected instead. Reuse only its functional arrangement: compact device groups, disclosure controls, five aligned columns and per-cell units. Do not reproduce proprietary branding, icons, artwork, screenshots or graph assets in SiliconWatch.

### Main Sensors Window

Default 1120×680 logical pixels; minimum layout target 760×520. Native window-manager decorations, dark mode by default, visible keyboard focus, compact 28 px rows at the default 14 px font, and an optional comfortable 36 px density. Row heights grow with font/scaling; compactness must not clip text or reduce a disclosure target below 24×24 logical pixels.

```text
SiliconWatch — Sensors                       [Settings] [Reconnect]
Connection + qualified observation state       [°C / °F]
Sensor                       Current   Minimum   Maximum   Average
▾ provider · source device             Session · observed samples
  Sensor label · row status      — °C      — °C      — °C      — °C
  Sensor label · unsupported     —         —         —         —
▾ provider · source device
  Sensor label · row status      — V       — V       — V       — V
  Sensor label · row status      — RPM     — RPM     — RPM     — RPM
[Open Details] [Reset Statistics]    Valid observed samples: —
Read-only · Min/Max/Avg since session reset · physical age unknown
```

The em dashes are placeholders, not sample measurements. Revised review artifacts likewise contain no measured or fabricated numerical telemetry. In the eventual application, render only actual validated API fields; unavailable values remain dashes with an explanation.

| Element | Proposed behavior |
| --- | --- |
| Hierarchy | Expand/collapse source device groups. Header shows provider/source label and row/error counts. Collapse never hides the fact that a group contains failed or unsupported rows. Preserve source registrations separately, even if their labels match. |
| Five columns | Default order is exactly Sensor, Current, Minimum, Maximum, Average. Numeric strings and units align right with tabular glyphs. Sensor text aligns left. Statistics headings explicitly carry session scope. Columns have stable IDs and shared header/row widths. |
| Status | Row quality text belongs in the Sensor cell; observation freshness/role is shown globally and in details. Stale, failed, unsupported, historical and disconnected indicators use text and icons, never color alone. “Current” is a column heading, not a guarantee: retained values carry Last known/Historical qualification. |
| Statistics | Calculated from unique, valid GUI-observed publications as specified in §7.1. Display sample count and reset scope; dashes before the first eligible sample. Frozen aggregates retain a visible qualification during failures. |
| Interaction | Single click selects. Double-click or Enter opens the same details action; Open Details provides a discoverable keyboard-accessible equivalent. Refresh never steals focus or reorders rows by changing value. |
| Settings | A compact dialog contains theme/font/unit, window/table layout, Export Settings, Import Settings and Reset to Defaults. Keyboard Move left/right and numeric width controls configure columns; header drag/reorder may follow later. |
| Narrow view | Horizontal scrolling preserves all five columns and status access; vertical virtualization keeps long inventories usable. Never hide errors to fit a smaller window. Long labels have keyboard-accessible full text. |

Use dark canvas `#15191f`, panel `#1c222b`, header `#232b36`, separator `#394452`, primary text `#e6edf3`, secondary `#9eabb9`, focus `#8dbaff`, selection `#25394f`; green `#8dd4b2`, amber `#edc778`, red `#f2a4a4` supplement status words. Qualified green indicates recent observation, never hardware health. Light/system alternatives need their own contrast verification during implementation. Theme choices use application-owned palettes, not executable imported styling. Default font is the installed system sans; never download fonts.

### Hardware Details Window

Open one reusable native details window (default 960×720, minimum target 640×480). Selecting another row and invoking details retargets it explicitly. Closing it leaves the main window, shared API worker and daemon running; closing the main window closes GUI-owned windows without stopping the daemon.

The conceptual interaction **CPU Temperature → CPU Details** is supported only when component association is positively evidenced. **API v1 does not contain a physical device inventory or CPU relationship graph.** Prototype 010 titles therefore use the actual source label, e.g. “k10temp — Device Details” or “nct6775 — Device Details,” with “source group; physical association unverified” where necessary. A familiar provider name or “CPU fan” label alone does not establish hardware relationships.

Show every row belonging to the selected current source registration, grouped visually into Temperatures, Voltages and Fans only when corresponding API rows exist. Each category has a prominent current reading, compact session statistics and accessible status; multiple readings become small repeated cards/list items within the category. Do not create empty capability cards, guessed motherboard layouts, thermal thresholds, fan percentages or gauges with invented ranges. Unsupported source rows stay available in an “Other / unsupported” list. No graphs or history store are required for this release.

A permanently reachable evidence disclosure provides each selected reading's original label, raw text, exact signed raw integer, canonical value/unit, formatted display value, source paths, full identity evidence/issues, row diagnostics and observation timing/role. Evidence is selectable plain text, bounded and scrollable. Never drop diagnostics because a summary card is visually simpler. Preserve anomalous numeric values with their actual row error rather than replacing them with a plausible measurement. Main and details windows share the same observation, preferences and statistics; opening details creates neither an API request nor a second accumulator.

Group membership uses the exact observed provider and `device_path` of the current source registration. Parent-path or candidate equality is supporting evidence only, not permission to merge registrations. A duplicate/ambiguous row remains separate; cross-provider component aggregation requires future backend evidence. In-session selection uses source evidence plus duplicate occurrence, not a candidate alone. Clear selection on restart or uncertain remapping.

### Review artifacts

Two static, editable SVGs and PNG previews accompany this revision in the external deliverables directory:

- `010-main-sensors-window.svg` and `.png`.
- `010-hardware-details-window.svg` and `.png`.

Deliverables directory: external local-review outputs. These are original layout specifications, not implemented windows or hardware validation. The text above is the durable repository specification; the original `/tmp/siliconwatch010-planning/010-native-gui-wireframe.svg` and `.png` remain unchanged.

## 5. Accessibility and Slint compatibility for the revision

Decision 28 remains an acceptance gate. Keep the selected Slint `=1.13.1` architecture and Rust 1.85 requirement from §2. No framework switch, dependency addition or MSRV increase is authorized by this proposal.

| Requirement | Slint 1.13.1 design and limitation |
| --- | --- |
| Hierarchical five-column table | Flatten expanded headers and sensor rows into a Rust-owned visible-row model; use custom `ListView` delegates with shared widths and right-aligned formatted strings. Hierarchy, selection preservation, resize/reorder controls and full spoken row summaries are application work. Do not claim a turnkey accessible tree-table. |
| Session statistics | Bounded Rust accumulators; model row notifications update presentation. Slint receives formatted strings and never owns telemetry arithmetic or exact raw integers. |
| Details window | A second exported `Window`, created/shown on the UI thread under one event loop. Reuse one instance. Globals are per component instance, so Rust explicitly distributes observations and committed preference updates to both windows. |
| Fonts and theme | Bind validated family/size to window defaults and applicable text controls; use application palette tokens plus the pinned built-in palette's color-scheme setting. Some stock widgets have independent font behavior; verify them. Do not promise arbitrary runtime style replacement or downloaded font files. |
| Column persistence | Rust preferences own stable IDs, order and logical widths. Custom headers/rows bind to these values; Slint provides layout/model primitives, not persistence. Keyboard settings controls are the first-release path; direct dragging is deferred. |
| Import/export | Rust owns TOML parsing, validation, immutable preview data and transactional file operations off the UI thread; constructing/updating Slint preview models stays on the UI thread. Slint has no bundled file chooser in this release. Recommend an accessible absolute-path entry dialog for 010 to keep scope small; an asynchronous native picker is an optional later integration needing explicit dependency/MSRV review. |
| Keyboard/accessibility | Use pinned FocusScope key/focus APIs and TouchArea double-click callback routed to one action. AccessKit's list/list-item support does not prove complete table cell semantics. Use complete row summaries and test actual Linux AT-SPI/Orca behavior. |

Evidence reviewed: pinned [ListView](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/compiler/widgets/common/listview.slint), [Rust models](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/core/model.rs), [window/event-loop and globals documentation](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/api/rs/slint/lib.rs), [built-in elements](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/compiler/builtins.slint), [Fluent palette](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/compiler/widgets/fluent/std-widgets-impl.slint), [LineEdit](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/compiler/widgets/fluent/lineedit.slint), [AccessKit adapter](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/backends/winit/accesskit.rs), and upstream [file-dialog discussion](https://github.com/slint-ui/slint/discussions/1959) / [feature request](https://github.com/slint-ui/slint/issues/9781). These establish architecture feasibility, not a tested application. The full transitive lockfile, TOML parser and any future chooser still require actual Rust 1.85 builds.

Tab/Shift-Tab navigate toolbar, table, settings actions and details without visiting every numeric cell. Up/Down navigate visible rows, Left/Right collapse/expand a group (or reach its header), Home/End and Page Up/Down support long inventories. Enter/double-click opens details; Escape closes details or cancels an unapplied settings/import dialog and returns focus to the initiating row/control if it still exists. Space/Enter activate ordinary controls. No timed or mouse-only operation is required. The row's accessible name includes source, sensor, Current, Minimum, Maximum, Average, units, quality, observation state and statistics scope. Disclosure state, pending mappings and import validation errors must also be readable.

No focus stealing during updates, reconnect or errors. Do not announce every changing value. Slint 1.13.1 lacks the later `accessible-live-region` API, so automatic announcements are not promised; status remains reachable and readable. Do not add a custom accessibility bridge or raise MSRV silently. Test keyboard-only operation and Orca across both windows and import/export dialogs. Failure to meet basic reading/navigation is a framework qualification blocker, not an accepted omission.

Respect desktop scaling and resolved installed fonts; test 100%, 150%, 200%, both density modes, 10–24 px fonts, long labels and high contrast. No continuous animation/flashing. Target 4.5:1 text contrast and 3:1 meaningful non-text/focus contrast, following [W3C non-web guidance](https://www.w3.org/TR/wcag2ict-22/). These are design targets, not compliance certification.


## 6. Rust boundaries and bounded API client

Propose a separate `gui/` Cargo package, `siliconwatch-gui`, with root workspace `default-members` restricted to the existing headless package and resolver 3. This keeps ordinary CLI/daemon builds free of GUI dependencies and preserves Decision 14's eventual optional GUI distribution. A workspace is for build selection, not packaging infrastructure. No Python, JavaScript, browser, WebView or Node runtime is introduced. Slint markup is compiled at build time into the native Rust application; normal Linux windowing/graphics/accessibility system libraries may still be required.

| Proposed file/module | Responsibility |
| --- | --- |
| `gui/src/main.rs` | Parse the limited refresh option, create the main window and one reusable details window, start/stop the client worker and dispatch model updates. |
| `gui/ui/main.slint`, `gui/build.rs` | Main/details windows, accessible settings/import preview, grouped rows; compile-time UI only. A separate proposed `gui/ui/details.slint` may keep the second window readable. |
| `gui/src/client.rs`, small Linux OS boundary | Validate the local endpoint and server UID; bounded/cancellable connect, one LF-framed request/response and reconnect state. No acquisition ownership. |
| `gui/src/protocol.rs` | Typed, versioned DTOs and structural/semantic validation for API 1, snapshot 2, identity 1. Parse raw integers as `i64`, metadata as bounded `u64`/`u128`; no binary64 round trip. Keep exact DTOs in Rust and pass formatted strings, not toolkit numeric properties, for displayed readings and raw integers. |
| `gui/src/model.rs` | Pure grouping, stable selection, connection state, row status, display-age assessment and diagnostic presentation. |
| `gui/src/presentation.rs` | Celsius/Fahrenheit preference, formatting and escaped visible text; immutable raw data retained alongside formatted strings. |
| `gui/src/statistics.rs` | Bounded canonical session aggregates, publication deduplication, reset and continuity policy; no histories or acquisition. |
| `gui/src/preferences.rs` | Versioned TOML DTO, strict validation/defaults, pending identity matches, preview and transactional local persistence; no daemon configuration. |
| GUI tests/fixtures | Deterministic fake socket server, fake clock, protocol examples and pure presentation tests. No hardware access. |

These are proposed boundaries, not new files created by this planning task. Keep the first implementation small; do not add a generic transport framework, plugin architecture, separate client crate or async runtime without evidence. Client DTO duplication is a drift risk: test against published examples/schema and actual daemon fixture responses rather than changing the accepted API to accommodate the UI.

Use one background client worker with one in-flight request and a bounded latest-result mailbox; coalesce commands and UI wakeups so at most one notification is pending, not merely one payload. Update UI models only on the UI thread. Bound retained generations (current decoded frame/model plus at most one pending replacement), text, row counts and error messages. One separate, serialized preferences I/O worker may handle at most one bounded read/write job plus one coalesced pending save; it must never delay acquisition-client deadlines. Cancellation and orderly shutdown apply to both workers. Wake the event loop only for model changes, input and a needed age transition. Never perform socket I/O, JSON parsing, sleep or hardware access in UI callbacks. Window close signals cancellation and shuts down client-owned active I/O; never block the close callback joining uncancelled socket work. Join GUI-owned workers before process completion, without forced exit or signaling the external daemon. Kernel/filesystem stalls still preclude a hard real-time shutdown guarantee.

Connect only to the accepted pathname socket under the validated current user's runtime directory. Reject root/mismatched real/effective UID, invalid path/ownership/mode, or wrong server peer UID. Check peer credentials before sending a request, retain the existing same-UID/root trust limits, and never change endpoint permissions. A worker thread alone does not bound `UnixStream::connect`; use a reviewed nonblocking Linux connector with absolute deadline and cancellation. AF_UNIX `EAGAIN` needs a bounded close/retry on a fresh socket, not an assumption that TCP-style writable readiness proves connection. Keep any necessary `libc` use in the small OS boundary with deterministic failure tests. See [Linux connect semantics](https://man7.org/linux/man-pages/man2/connect.2.html) and [Rust UnixStream](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).

For each connection, send one complete request below 4096 bytes including LF, then accept one complete UTF-8 response at most 1 MiB including LF. Detect truncation, oversized frames and malformed encoding. Process the first LF and reject extra frame bytes already buffered with it, then close; do not wait indefinitely for EOF or claim to detect future bytes never read after closure. Discard incomplete data. Use an overall proposed 5 s transaction budget through connect/send/read/validation, stop-aware waits at most 50 ms, bounded decode input and deadline/cancellation checks between parse/model operations; parsing itself is not claimed preemptible. These client limits do not promise real-time scheduling or cancel arbitrary kernel stalls. The server has its own 1 s request and 2 s response budgets; neither is a client-side connect timeout.

On startup and reconnection: `get_info` first; require API 1, snapshot 2, identity 1, the two methods, read-only capability and supported limits, then `get_snapshot` on a new connection. A different `instance_id` on any response invalidates sequence/clock assumptions and triggers a new handshake; never concatenate epochs. Snapshot requests are sequential, with the next poll no earlier than the configured refresh interval after the preceding request completes; skip missed ticks and never catch up in a burst. Back off busy/request-timeout responses at least 2 s and absent/refused/disconnected/transport-timeout handshakes at least 5 s, always no faster than the configured refresh interval. These fixed connection-error delays do not change sensor polling. Reconnect coalesces with pending work and respects the same rate limit; it cannot start a concurrent request.

Honor explicit protocol error codes/retryability. Unsupported versions, invalid response shapes and unknown enums cannot become healthy/current state. Show a stable explanation for non-retryable incompatibility until explicit reconnect; For non-retryable `service_unavailable`, retain the rejected instance token and block further snapshot requests to it. Bounded reconnect probes may perform `get_info`, but automatic telemetry resumes only after a different valid instance token; explicit reconnect after user correction is the only same-instance retry path. Merely discarding local state must not defeat the non-retryable error. `response_too_large` and `serialization_failed` must never produce a truncated healthy table. Do not derive meaning by parsing diagnostic prose. Additive fields may be ignored per the contract; required fields, integer types, finite canonical numbers and semantic time/state relationships remain validated.

## 7. Freshness, failure and temperature presentation

Connection, lifecycle, scan outcome, observation age/role and per-row quality are separate dimensions. “Connected” must not mean “all sensors healthy.” Preserve every warning and diagnostic, even when acquisition succeeds.

| State | Window behavior |
| --- | --- |
| No handshake / incompatible / permission error | Clear currentness; show specific connection state and action, with no invented readings. |
| Empty cache | Waiting for first acquisition. A failed first attempt with no observation instead shows the acquisition error. |
| Successful empty inventory | No sensor observations; no fabricated zeros or healthy/current sensor set. |
| Recent valid observation with bad rows | Qualified recent-observation banner plus individual unavailable, permission-denied, unsupported, invalid, error and fault states. Retain all rows. |
| Latest acquisition failed, prior success retained | Historical after failure; display the successful observation's own attempt and diagnostics alongside the newer scan failure. Never relabel old values with the failed attempt's times. |
| Producer stopped or lifecycle stopping/stopped | Noncurrent; historical-after-stop where reported. EOF alone does not prove graceful shutdown. |
| Stale at daemon assessment | Explicit Stale at assessment state, even if just received. |
| Client's conservative age bound exceeds budget | Freshness unconfirmed / age budget exceeded. Do not imply that exceeding an upper bound proves physical staleness. |
| Disconnected after displaying data | At most one last observation remains visible, explicitly disconnected/last known, continuing to age. No continuing-current badge. |
| New instance | Discard epoch-dependent continuity, sequence comparisons and pending results; re-handshake. Old displayed data, if retained while reconnecting, is explicitly from the previous instance. |

Validate the required field types and time relationships before giving any positive state: started ≤ finished ≤ published ≤ assessed; observation bounds equal assessment minus its own acquisition finish/start; latest/observation presence, role, sequence, sensor count and freshness state agree. Recompute the transmitted `service_current` conjunction (running lifecycle, active producer, succeeded latest attempt, latest-attempt observation, within-budget freshness) and require agreement. An unknown enum, arithmetic overflow or inconsistent snapshot is a protocol failure. Duplicate sequence values are normal unchanged-cache reads; same-instance regressions are suspect. Historical role always prevents currentness, regardless of numerical age.

**Never compare daemon `assessed_at_ns` directly with a GUI clock or wall timestamp.** Its origin is private to the cache. Receipt-relative elapsed time alone misses pre-receipt encoding/transport/scheduling delay. Under the same-host Linux monotonic elapsed-clock model, absent known suspend/clock anomalies, record local `t0` before connect/request, `t1` after the complete LF response, and `t` when presenting it. For the daemon's software-age bounds `L`/`U`, compute with checked integer nanoseconds:

```text
display_lower = L + (t − t1)
display_upper = U + (t − t0)
```

The upper includes the full request round trip and time waiting for UI delivery; no half-RTT or clock-offset estimate is needed. Assessment occurs between request start and response completion. Reassess as the displayed data ages, even while the worker is blocked. Only show **Recent at last assessment** when the validated service conjunction remains true, no later disconnect/error invalidated it, and `display_upper <= stale_after_ns`. Equality is allowed. A stale server assessment stays stale; if only the client's upper crosses the threshold, currency is unconfirmed. If the lower also exceeds it, the entire software-observation interval is past the budget. None of these labels certifies physical measurement age, hardware health or continuing daemon liveness.

Example: daemon upper 2.8 s, budget 3 s, transaction 0.4 s gives a receive-time upper of 3.2 s. A zero receive-age cannot justify a recent badge. Repeated reads of one publication must not reset its age. Overflow, negative elapsed detection, invalid sequencing or a changed instance clear currentness.

This is an explicit conservative client inference, not a new API guarantee for arbitrary clocks or suspend. [Rust Instant](https://doc.rust-lang.org/std/time/struct.Instant.html) is opaque; [Linux monotonic time](https://man7.org/linux/man-pages/man2/clock_gettime.2.html) excludes suspend, and time origins must not be assumed comparable. On known suspend/resume or a detected clock anomaly, latch freshness-invalid for the affected instance, show freshness unknown and request a manual daemon restart to recreate its cache. Neither successful subsequent polls nor ordinary or explicit reconnect can restore that instance's positive freshness badge; only a validated different daemon instance clears the latch. Reconnecting alone cannot repair it. Reliable automatic suspend detection is not present in API v1 and is not silently added to Prototype 010. That residual limitation must remain visible in documentation and the qualified wording.

The accepted [local API contract](../api/local-v1.md) and [schema](../api/local-v1.schema.json) remain authoritative. Snapshot/identity data and paths are evidence, not permission to read source files or invoke hardware controls.

Temperature conversion is presentation only: Celsius is the first-run/reset default; Fahrenheit is an optional persisted GUI preference, `F = C × 9/5 + 32`, applied only to canonical Celsius temperature rows with the proper measurement/unit semantics. Finite-check the derived Fahrenheit output before formatting; an unrepresentable result is a separate presentation error, never a fabricated value or rewrite of source status. Preserve `raw_text`, exact `raw_value`, original Celsius and diagnostics unchanged. Do not apply conversion to voltages, RPM, unknown measurements or raw integers. Do not calibrate, clamp implausible readings, replace a fault with zero or silently correct hardware values. If an erroneous row retains a numeric value, keep it visibly marked with that error. UI rounding is labeled formatting; details retain the original canonical representation and raw integer. Test negative values, −40°C/°F, 0°C→32°F, 100°C→212°F, unavailable values, extreme integers and finite canonical values whose Fahrenheit conversion overflows.

## 7.1. Session Minimum / Maximum / Average

These are **client session statistics over valid observations received**, not all daemon acquisitions, hardware extrema, time-weighted averages, or persistent history. Keep canonical values in Rust; formatting/temperature conversion happens afterward. A numerical value alone does not establish validity.

1. An eligible observation must pass all protocol/currentness checks in §7, belong to the current daemon instance, be the successful `latest_attempt` observation, remain within the conservative age budget at UI processing, and have no suspend/freshness-invalid latch. An eligible row has `status = "ok"`, recognized measurement/unit semantics and a finite canonical value. Fault, unsupported, permission-denied, unavailable, invalid and error rows never contribute, even if they contain numbers.
2. Admit each `(instance_id, observation.attempt.sequence)` at most once. Use the **observation** sequence, not a newer failed attempt's sequence. Repeated cache reads, window refreshes, reopening details, collapse/expand and unit changes add no samples. Advance the watermark only from a structurally and semantically validated publication identifier belonging to the current instance, even when its rows or freshness make it ineligible for statistics. Malformed responses, invalid sequences and unknown instances never advance the watermark.
3. Maintain per-source-row count, minimum, maximum and arithmetic mean in canonical units with checked counters and finite, overflow-resistant arithmetic. Zero valid samples yields dashes; one yields the same min/max/average. Valid negative readings and genuine zero RPM are retained. Display “Valid observations received: N”; document skipped daemon acquisitions and the absence of physical measurement timestamps. No interpolation or fabricated zeros.
4. Stale/historical/disconnected/failed states freeze existing aggregates and qualify them as retained session statistics. A bad row is skipped while other eligible rows can contribute. Returning to an eligible later publication resumes only confident continuity. Historical success cannot become a new sample after a failed scan. Main and details use one shared accumulator.
5. The GUI session starts at launch or explicit Reset Statistics. A new daemon instance resets all statistics with a visible reason. Match continuity only where the observed provider/device-path/channel/measurement/unit tuple identifies exactly one row and its identity evidence remains consistent. A duplicate-occurrence index is for rendering only; it never proves continuity. Indistinguishable duplicates remain visible with statistics unavailable and an ambiguity explanation, rather than transferring aggregates between occurrences. Within-instance evidence/semantics changes, collision, or disappearance followed by reappearance reset the affected row; retain no unbounded disappeared-row history. A same-instance reconnect preserves the watermark and aggregates only if source evidence is unchanged. A known suspend/clock anomaly freezes accumulation until a validated new instance clears the existing freshness latch, which also starts a fresh statistical epoch.
6. Reset Statistics has its own confirmation (“Clear session Min/Max/Avg?”), clears aggregates, and watermarks the current observation so it waits for a new publication. It does not change daemon state or preferences. Import Settings and Reset to Defaults do not reset statistics. Units and fonts do not change counts or underlying values.
7. Bound accumulators to the accepted current inventory (proposed maximum 4096 source rows) and store no sample history. Count overflow or failed finite arithmetic produces a visible Statistics unavailable state until reset; never wrap, silently clamp, or publish a plausible mean. These arithmetic/resource choices require deterministic tests, not new backend behavior.

## 8. Portable GUI configuration — proposed schema v1

### Scope, location and authority

Use human-readable **TOML 1.0 syntax** with required integer `schema_version = 1` and exact `profile_kind = "siliconwatch.gui"`. This is a schema recommendation and example, not a parser implementation. Prototype 010 reads/writes **schema 1 only**; version 0, absent/noninteger versions and versions greater than 1 are rejected without conversion. A future explicit migrator must preserve a backup before migration. Never infer a version from filenames or silently downgrade.

Default file: `$XDG_CONFIG_HOME/siliconwatch/config.toml` when XDG_CONFIG_HOME is nonempty and absolute; otherwise `~/.config/siliconwatch/config.toml`, using the current user's home directory. An invalid relative XDG value is ignored with a warning, following the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir/latest/). Imported text never supplies a local destination or changes environment variables. If no trustworthy home/config directory is available, run with in-memory defaults and explain that persistence is unavailable.

Only GUI-owned preferences belong here. No acquisition intervals, GUI refresh cadence, socket path/instance token, socket permissions, service launch flags, API access policy, hardware controls, credentials, API tokens or machine secrets are in the profile schema. Do not serialize a shared application-state object or export runtime diagnostics. Import cannot change daemon acquisition, security configuration or hardware behavior. Appearance writes invoke neither API method beyond the existing refresh schedule and never touch the daemon configuration. The existing `--refresh-ms` setting remains a separate launch option, deliberately absent from portable appearance profiles.

Decisions 9, 26 and 29 support TOML, per-user UI preferences and validated destination mappings. Decision 9's daemon-validation rule remains applicable to live service configuration; this proposal scopes appearance preferences to a local GUI boundary without adding API writes. This is an explicit interpretation for Chris's review, not an edit to the accepted design record or authorization for service configuration.

### Complete portable example

The companion `010-gui-profile-v1.example.toml` contains the following complete schema-1 default profile. It is a review artifact, never installed as the user's live configuration.

```toml
schema_version = 1
profile_kind = "siliconwatch.gui"
group_overrides = []

[appearance]
theme = "dark"
font_family = "system-ui"
font_size_px = 14
temperature_unit = "celsius"

[windows.main]
width_px = 1120
height_px = 680
maximized = false

[windows.details]
width_px = 960
height_px = 720

[table]
column_order = ["sensor", "current", "minimum", "maximum", "average"]
density = "compact"
row_separators = true
alternate_rows = false
default_expanded = true

[table.column_widths_px]
sensor = 440
current = 152
minimum = 152
maximum = 152
average = 152
```

Except for the two required root keys, missing known fields use the defaults above; preview lists defaults that will replace current values. Import replaces the preference set as one transaction rather than silently merging with previous settings. `group_overrides` defaults to empty. A present `column_order` or width map must contain exactly the five IDs. Unknown fields are errors, including misspellings and attempted daemon/security fields; schema 1 has no opaque passthrough storage. Export serializes only validated known fields, omits comments/unknown metadata and never re-exports arbitrary imported bytes.

| Field / object | Validation and runtime behavior |
| --- | --- |
| `appearance.theme` | Exact enum `dark`, `light`, `system`. Dark is the first-run/reset default. System follows the pinned toolkit/platform selection, whose fallback may be light if desktop detection is unavailable. Explicit dark remains the default; do not promise detection or a fallback notice the toolkit cannot provide. See the [pinned palette implementation](https://raw.githubusercontent.com/slint-ui/slint/v1.13.1/internal/compiler/widgets/fluent/styling.slint). |
| `appearance.font_family` | `system-ui` or one installed family name, 1–128 UTF-8 bytes; no control characters, path separators, URI syntax, CSS/font directives or embedded font data. Invalid names reject the whole import. A syntactically valid but unavailable family is a preview warning: use system sans on this installation, keep the requested family for portability. Never download/load a file because a profile names it. |
| `appearance.font_size_px` | Integer logical pixels 10–24 inclusive. No floats, string coercion, zero, negatives, NaN or infinity. Scale controls/rows to fit. |
| `appearance.temperature_unit` | Exact `celsius` / `fahrenheit`; defaults to Celsius and persists across launches. Temperature conversion preserves source values and diagnostics. No conversion of unrelated units. |
| `windows.main` | Integer width 760–4096 and height 520–2160; `maximized` boolean. Save the normal dimensions separately from maximized state. No positions, monitor IDs or scale factors in portable profiles. |
| `windows.details` | Integer width 640–4096 and height 480–2160. Runtime clamps effective size to the available display work area if smaller, keeping controls reachable by scrolling. Preview reports size adaptation; stored requested size remains portable. The same applies to main. |
| `table.column_order` | Array of exactly five unique known IDs, any permutation. No omissions/duplicates. All columns remain available; sensor hiding is deferred. |
| `table.column_widths_px` | Exact five-key map of integer logical widths: Sensor 240–1024, each numeric column 112–512. Font fitting and horizontal scroll may increase effective minimum widths; never silently lose a column. |
| Other table fields | `density` exact `compact` / `comfortable`; `row_separators`, `alternate_rows`, `default_expanded` actual booleans. Row quality and freshness indicators cannot be disabled. Fixed stable source ordering in 010; sorting/filtering are deferred. |
| `group_overrides` | At most 256 records with unique selector sets. Each has only `expanded` (boolean) and `members` (1–16 member-selector records). A member has exactly `identity_schema_version = 1`, `candidate_id` (1–4096 UTF-8 bytes), `measurement` and `unit` from the allowed pairs below. At most 4096 total selectors, still within the overall file limit. Reject duplicate/contradictory selector records. |

### Identity selectors and portable group state

Use established SiliconWatch identity candidates; **never create a new identity by hashing a label, `hwmonN`, enumeration position or sysfs path alone**. The candidate is the exact compact JSON tuple already exposed by identity schema 1: `["siliconwatch.hwmon-source",1,provider,parent,channel,measurement_semantics]`. Treat its canonical string as an opaque comparison key after validating its expected tuple/type/semantic structure. Do not normalize or rewrite source paths in search of a match, and do not dereference them.

Allowed selector pairs are `temperature_input_millidegree_celsius` / `°C`, `fan_speed_input_rpm` / `RPM`, and `voltage_input_millivolt` / `V`. Here `measurement` is the exact API `identity.evidence.measurement` string, not an invented top-level sensor field; `candidate_id` stores API `identity.candidate`. Each tuple must contain nonempty string evidence and match its declared measurement. Unsupported input semantics cannot anchor a portable override in schema 1; their rows remain visible. API identity status and evidence are authoritative at match time, not claims made by a profile.

The optional group record shape, shown with a **fictional illustrative candidate**, is:

```toml
# Replace group_overrides = [] with this array-of-tables example, not both.
[[group_overrides]]
expanded = false
members = [
  { identity_schema_version = 1, candidate_id = '["siliconwatch.hwmon-source",1,"example_provider","/devices/example","temp1_input","temperature_input_millidegree_celsius"]', measurement = "temperature_input_millidegree_celsius", unit = "°C" }
]
```

A source-group override uses one or more actual member identities as anchors. Prefer all available eligible members up to the cap, chosen by candidate ordering, never volatile enumeration. Eligible anchors must fit the candidate/file limits; never truncate, rewrite or hash an over-limit identity to make it fit. Save no override for a group without eligible identities; disclose “Group layout cannot be restored safely.” Anchors must resolve uniquely to the **same current source registration**, have `best_effort` status with consistent evidence and semantics, and no collisions, conflicting registrations or incompatible live member evidence. All listed anchors must resolve; missing, ambiguous, mixed-device or duplicate matches leave the override inactive and the group expanded. Report each unresolved override with a reason. Never use a first-match winner or fallback by nearest label/path.

Identity v1 is best-effort topology/provider/channel evidence, not a physical hardware UUID. A candidate can recur after hardware replacement or driver channel reinterpretation. See [Prototype 002 identity format and limitations](002-stable-sensor-identity.md) and [local API](../api/local-v1.md). Consequently, even a unique candidate match on a new installation is only a **proposed source match**. Imported overrides start pending and require explicit review of source evidence in the import preview before activation. An ambiguous or conflicting match cannot be confirmed as an automatic match; leave it inactive. Users can expand/collapse the actual current group directly.

For the first release, the same conservative revalidation/review applies to restored group overrides at GUI launch or after a daemon/source identity discontinuity; no private machine ID, socket token or exported “trusted” flag bypasses it. During an uninterrupted, confidently matched live instance, an explicit group toggle changes the current view immediately and queues persistence as described below. Pending records remain stored but inactive; a settings notice shows their count and review action. Offline imports can apply globals and preserve pending records, with the preview stating that no device mapping was applied. Default unresolved groups expanded even if `default_expanded` is false, so uncertainty remains discoverable.

This deliberately limits automatic group restoration with API v1. It does satisfy portable storage and avoids silent reassignment; guaranteed cross-reinstall hardware matching requires future backend identity evidence/registry work. Sensor candidates can reveal hardware topology, so Export offers **Include device group preferences** (off by default) with a plain explanation; no raw paths outside the candidate, raw readings, diagnostic logs, instance IDs or secrets are exported. Excluding device overrides produces `group_overrides = []`. If included, the preview lists only the needed candidate selectors and states they require destination review.

### Future schema evolution — deferred, not accepted by schema 1

Reserve a future **schema 2** design for `[[sensor_preferences]]` records keyed by the same identity-schema/candidate/measurement/unit selector, with optional `color`, `label`, `hidden` and `temperature_unit`, plus an `appearance.extensions` table for individually specified appearance properties. These table/field names are a roadmap, not permissive unknown-field namespaces. No parser should apply them until a new schema and feature support are approved. Prototype 010 rejects nonempty or empty `sensor_preferences`, `appearance.extensions`, color keys and higher versions rather than silently dropping future settings.

Proposed future validation: color only `#RRGGBB` (six hexadecimal digits), finite bounded numeric appearance values, labels 1–128 UTF-8 bytes of plain text with no control/bidirectional formatting characters, hidden only boolean, and per-sensor units restricted by actual measurement semantics (Celsius/Fahrenheit only for temperature initially). Invalid color/font/value rejects an entire import with a field-specific error; color never replaces quality/freshness text, and inaccessible colors need a warning/correction before application. A valid but absent installed font remains the explicit fallback case above. Never accept executable expressions, URLs, CSS, file imports or scripts in appearance fields. Newly enabled per-sensor preferences inherit the same pending/ambiguous mapping policy; a custom label never becomes identity evidence.

### Untrusted input limits and failure behavior

Maximum input is **262,144 bytes (256 KiB), inclusive**, measured from the opened file; read at most limit+1 to detect overflow. Require a regular UTF-8 local file; reject symlinks, directories, device nodes, FIFOs, invalid UTF-8, duplicate TOML keys, invalid syntax and trailing unparsed data. Open without following the final symlink and validate the opened descriptor, not just a preceding path check. No archive decompression, include directives, environment interpolation, tilde/command substitution, URL fetching or filename-driven schema selection. Parsing is bounded by file size and schema container limits, with a maximum structural nesting of 8; choose/qualify a parser that enforces nesting during parsing rather than only after allocation. Input errors report bounded plain-text field/line/column context, not an entire file that may contain secrets.

Validate types, enum membership, numeric ranges, UTF-8 byte lengths, table keys, selector structure and aggregate counts before creating a preview. Raw TOML floating nonfinite values, dates, nested unknown structures or values in place of expected types are rejected. Unknown fields fail the complete import; do not partially apply recognized fields. Parse/validate/serialize work runs outside the UI thread and has cancellation; no parser dependency/version is selected or installed during planning.

On startup: missing config means defaults. Malformed/oversized/unsupported config means visible recovery state plus in-memory defaults; **do not automatically overwrite or rename the original**. Offer inspection of the error, restoration from a separately validated backup, or explicit Reset to Defaults after preserving the original. Disable automatic preference persistence until recovery is chosen. A syntactically valid file with unavailable fonts or pending mappings is not malformed; apply only the documented fallback/global behavior with notices.

### Atomic writes, conflicts, backups and recovery

Use a dedicated GUI configuration boundary, separate from API acquisition. Store new app directory with user ownership and mode 0700 and files with mode 0600. Validate existing directory/file ownership and reject unsafe symlink/nonregular-file destinations; never chmod unrelated XDG parents. Resolve/check directory components with descriptor-based no-follow operations rather than a check-then-open sequence. Export respects a chosen user directory but never overwrites a symlink or special file. Local appearance permissions do not alter API runtime/socket permissions.

For Settings Apply, import confirmation, reset and backup restoration, build a full candidate in memory, validate and serialize it, re-parse the serialized candidate, then perform one serialized write transaction. Ordinary group disclosure, window resize and direct display-unit changes may update reversible session view state immediately, then coalesce persistence after interaction settles; they remain usable even in recovery/read-only mode. If their background save fails, retain the visible session choice with an explicit Unsaved preferences notice. This exception does not apply to import/reset: their complete preference replacement is applied only after commit, and a precommit failure leaves both current session state and disk unchanged. Hold a cooperating-writer lock and compare the current file fingerprint against the version initially read; if another writer/editor changed it, report a conflict and require reload/review, never silently overwrite. Same-UID malicious processes remain outside the isolation boundary; advisory locking cannot constrain arbitrary external editors.

Write an exclusively created random temporary file in the **destination directory**, mode 0600, bounded bytes; flush and `fsync` it. Before replacing a valid active config, atomically refresh `config.toml.bak` with the exact previously validated bytes and sync it. Keep one last-known-good backup, never replace it with malformed input. If backup creation is required but fails, abort before commit. Commit by atomic same-directory rename over the validated destination and sync the parent directory. Apply the complete preferences model to both windows only after the rename succeeds. Temporary files belong to this operation; clean them up on precommit failure. Do not mutate the import source.

Before explicit recovery replaces a malformed/unsupported local file, preserve its exact bytes in a uniquely named recovery file outside the active config name with mode 0600; do not lose it or replace the valid backup. If preservation fails, retain the original and block the write. At launch, never auto-promote `.tmp` files or `.bak`: validate the backup and show a recovery preview. If both are invalid, use defaults in memory until the user chooses recovery. Backups may contain historical configuration fields; they are local recovery evidence and are never automatically exported.

Failure before rename leaves the old active config and live preferences untouched. Atomicity means readers see the complete old or complete new file, never a partially overwritten configuration. If the final directory sync fails **after** rename, the commit already occurred: show “Settings saved; crash durability could not be confirmed,” keep the complete new model consistent with the file and retain the backup. Do not falsely report that nothing changed or attempt an unsafe blind rollback. Crash/power-loss durability depends on filesystem support and successful sync; unsupported filesystems must be reported. A future export to an existing file likewise requires overwrite confirmation and atomic replacement; the source active config is untouched.

### Short import / export / reset interaction specification

| Action | Interaction and completion |
| --- | --- |
| Export Settings | Settings → Export Settings → choose destination via accessible absolute-path dialog (suggest `siliconwatch-settings.toml`, editable directory/name) → optional Include device group preferences, off by default → review included categories/pending mappings → confirm overwrite only for an existing regular file → atomic write → show destination/result. A destination equal to active config/backup is rejected to avoid accidental self-export replacement. Cancel/error leaves active settings intact. No session readings/statistics, endpoint data or secrets. |
| Import Settings | Settings → Import Settings → choose a local file → bounded read/parse/version/type/range/identity validation → error with field/line and no changes, or preview differences/defaults/font and size adaptations/pending mappings → explicit Replace Preferences / Cancel. Match confirmations occur in this preview or later settings review, never silently. Revalidate the retained candidate and destination state at commit, including current daemon instance/source evidence for every confirmed mapping. If source identity changed since preview, move affected mappings back to pending and require fresh review before activation; file selection or preview alone does not apply anything. The parsed candidate is held immutably, so changes to the source file after preview cannot substitute new input. |
| Successful import | Save a valid previous-config backup and atomically commit; update both windows together; return focus; display counts of applied global preferences and inactive device overrides. Unmatched overrides are a reported, supported pending state, not an input-validation failure. Save failure before commit leaves old disk/runtime settings intact. Never reset statistics or daemon state. |
| Reset to Defaults | Settings → Reset to Defaults → preview dark/system-font/Celsius/default geometry/columns/all groups expanded and removal of saved overrides → confirm Reset / Cancel → same backup/atomic transaction. Does not clear sensor observations, session statistics, diagnostic evidence or daemon configuration. Malformed-file recovery preserves the original first. |

No native picker dependency is silently implied. The proposed Slint path dialog must support keyboard entry, readable validation and explicit destination selection. If Chris prefers a native filesystem browser for 010, choose an asynchronous integration only after a separate Rust 1.85-compatible dependency review; synchronous blocking dialogs are not an acceptable shortcut.


## 9. Deterministic and practical acceptance tests

Completion requires the real native window, not only the following headless tests. Tests are proposed; none were executed for a GUI during this planning task.

| Area | Required evidence |
| --- | --- |
| Connect/handshake | Real temporary private Unix socket with fake server: good current-user connection, missing endpoint, refusal, timeout, peer/permission rejection, API/schema/limits mismatch. No root or permission changes. |
| Snapshot rendering | Valid typed fixtures with all three supported quantities; exact row text, grouping, status, diagnostics and values in the window model. Empty cache versus successful empty inventory are distinct. |
| Failures | Every row status, initial acquisition failure, later failure retaining success, stopped/starting producer, busy response, oversized/serialization error and partial frame closure. |
| Freshness | Fake clocks and delayed responses test threshold equality, request transit, render delay, historical role, false service-current, overlapping age interval, known suspend/anomaly latching across successful polls/reconnect and clearing only after a new daemon instance. Receipt never resets observation age. |
| Raw preservation | Exact signed i64 boundaries including values beyond 2^53, raw invalid text, negative values and conversion immutability. Unknown unit/measurement never gets guessed conversion. |
| Session statistics | Distinct observation sequence vs latest failed sequence, duplicate polls, redraw/details/unit invariance, valid zero/negative readings, exclusion of bad/historical/stale rows, one-sample/empty means, frozen states, resume/disconnect/new-instance/reset watermarks, evidence changes, bounded count/finite arithmetic failure and indistinguishable duplicate-row continuity. |
| Portable preferences | Round-trip known fields/defaults; export destination/cancel/overwrite; preview/replace/cancel; strict malformed/UTF-8/version/unknown-key/type/range/color/font rejection; missing-font fallback; 256 KiB boundary/excess; nested input/cap limits; source changes after preview, commit-time remapping, no daemon/security fields. |
| Configuration transactions | Inject temporary-file, backup, rename and sync failures; old-or-new complete file invariant, precommit runtime unchanged, postcommit durability warning, destination changed since preview, symlink/FIFO rejection, recovery preserving corrupt original, validated backup restore, unsupported newer file never auto-overwritten. |
| Portable identity | Reinstall/reordered inventories, same-label different-device, same-topology replacement, changed driver semantics, collision, missing anchor, mixed-source anchors and offline import; pending overrides never autoapply. Explicit review/direct toggle; unknown group expanded. |
| Both windows and layout | Details opens from Enter/double-click, all same-source rows retained, no guessed categories/relationships; shared model/counts/preferences; persistent order/width/geometry, both theme/font modes, unavailable-font and small-screen adaptation. No extra API request on open/import/reset. |
| Grouping/identity | Equal labels from separate devices, duplicate candidates, absent provider, collision/ambiguous identity, reordered inventories and disappear/reappear/restart selection rules. |
| Reconnection | Daemon/fake server restart, changed instance between info and snapshot, sequence rollback, bounded backoff/cancellation, no overlapping requests and no stale pending result overwriting a newer epoch. |
| Hostile/malformed input | Invalid UTF-8/JSON, wrong/null/missing fields, duplicate object keys, bad decimal grammar/overflow, unknown major/enum, inconsistent time/state, nonfinite/oversized fields, 1 MiB boundary and excess rows/text. Keep the UI responsive and fail closed on currency. |
| Scheduling/resources | Injected clock/transport proves one in-flight request, no catch-up burst, configured cadence, rejected invalid/duplicate refresh options, bounded retained frames and pending notifications, and cancelled shutdown with complete worker join. All socket and parse work stays off the UI thread. |
| No new acquisition | Run the existing daemon against a deterministic fixture root, publish attempt 1, then hold its already-admitted second acquisition on a synchronized fixture FIFO. Repeated GUI requests, unit changes and reconnect must still return attempt 1 until that existing worker is released, with no additional reader or acquisition worker. Record only the two exact API read methods; UI redraw and unit switch emit no extra request. Independently assert the GUI imports no hwmon/acquisition entry point and never reads `/sys/class/hwmon`. |
| Keyboard/accessibility | Drive actual window events and inspect exposed accessibility tree: tab order, arrows, group disclosure, details, focus restoration, no focus loss on refresh, names/roles/status. Keyboard-only manual path and Orca reading are mandatory qualification evidence. |
| Resize/scaling | Actual window at main 1120×680 and 760×520 plus details 960×720 and 640×480, 100/150/200% scale, long labels, 0/1/4096 rows and details open: no unreachable controls, hidden error statuses or clipped focus. |
| Regression/MSRV | On both Rust 1.85.0 and installed stable: locked GUI build/test, formatting and Clippy with warnings denied; retain all existing Rust/doctest/Python/DCO checks. Test headless default build excludes GUI dependencies. New tests must fail for meaningful regressions, not just mirror widget internals. |

Practical visual validation, when implementation is separately authorized:

1. Launch the actual GUI against a deterministic fixture daemon on a Wayland desktop; record compositor, backend, toolchain, scaling and renderer, excluding machine identifiers. First verify live changing sensor rows, not a screenshot of static design data.
2. Repeat the same script in an X11 session where available. Force/check the backend rather than assuming an XWayland window proves native Wayland support. If a session is unavailable, record it as unexecuted and leave that compatibility gate open.
3. Navigate only by keyboard; read controls/rows/details with Orca; inspect contrast, focus, row alignment, numeric jitter and long diagnostics. Resize/move across differently scaled displays and test 100/150/200% scaling.
4. Switch Celsius/Fahrenheit and inspect raw details; inject empty, unsupported, row fault, scan failure, stale and historical cases. Disconnect/restart the fixture daemon and confirm retained values stay qualified and recovery restores a new instance safely.
5. Close the GUI during stalled socket input; the window stays responsive and the external daemon remains running. Do not test by changing hardware settings.
6. Record screenshots and executed/unavailable steps. An eventual same-user live hardware smoke can show the existing daemon's readings only after its execution scope is approved; this planning authorization performs no such smoke or benchmark matrix.

## 10. Resource expectations, risks and alternatives

Proposed initial targets are reviewable budgets, not measurements or promises: event-driven idle behavior with no animation or forced 60 Hz repaint; at most 1 Hz snapshot requests under the paired profile; at most one API worker/request and bounded pending data; bounded per-row aggregates and no sample/history accumulation. Preference I/O adds at most one serialized bounded job plus a coalesced pending save; it is outside the API worker's deadline path. Keep display-age text coarse and schedule an immediate expiry transition so stale data is not left green between periodic labels.

On a later agreed reference desktop with release builds and deterministic fixtures, aim for ≤1% of one logical CPU averaged over 60 s for an idle unchanged 100-row window, ≤150 MiB application RSS at 1000 rows after settling, and p95 input-to-visible-response ≤100 ms under a 4096-row fixture. Report renderer/backend and sampling method; GPU/driver memory is separate, and shared-library RSS is not a framework-only cost. A budget miss triggers measurement and a scoped choice, not premature optimization or silent removal of accessibility. No framework performance advantage is claimed without like-for-like measurement. These small fixture checks are proposed for implementation qualification, not authorized hardware benchmark work now.

Main risks are the older MSRV-compatible GUI release and its transitive dependency resolution, platform graphics/accessibility variability, grouped-table accessibility effort, source-identity ambiguity, current-user socket races outside the trust boundary, and the API's single-client fairness/age limitations. Real cross-UID isolation remains untested in the accepted baseline; injected credential failures do not establish it. No total-process memory guarantee follows solely from a 1 MiB wire limit. Endpoint and permission policies, cooperative daemon shutdown and comprehensive JSON metaschema validation limitations remain those accepted for Prototype 009.


## 11. Scope boundary, implementation sequence and approval decisions

| Capability | Prototype 010 intent | Existing API versus future work |
| --- | --- | --- |
| Main table | Five required columns, compact hierarchy, quality/freshness states, keyboard operation, persistent layout. | Current snapshots contain the rows and evidence needed. All new behavior is GUI client work. |
| Min/Max/Avg | Bounded session aggregates of distinct eligible observations, sample counts, explicit reset. | Derived client-side. The API supplies no extrema, physical timestamps or complete stream; time-weighted/rolling/persistent history is deferred. |
| Device details | One reusable visual source-device window, all rows from that registration, raw/identity/diagnostic disclosure. | Current API provides temperature, voltage and fan readings when present. Verified CPU/GPU inventory, cross-provider physical associations, additional quantities and hardware capabilities require future backend evidence/providers. No synthetic capability cards. |
| Portable settings | Schema-1 TOML, theme/font/unit, main/details dimensions, five-column widths/order, table density/separators, default expansion and reviewed group overrides; import/export/reset. | Local GUI-only work. No API additions. Automatic reliable cross-reinstall device mapping is not available in identity v1. |
| Source-specific portability | Save established member identities; report pending/conflicting/unmatched overrides and review proposed matches. | Best-effort identities already exist. Durable physical identity/registry and richer matching are future backend work, not a 010 prerequisite solved by guessing. |
| Per-sensor appearance | Schema-2 direction only: colors, custom labels, hidden sensors, unit overrides and additional appearance fields. | Deferred GUI features; safe mapping remains dependent on evidence. Schema 1 rejects these fields rather than silently discarding them. |
| Convenience | Keyboard layout controls and accessible path-entry import/export; global units apply to both windows. | Header drag/reorder, native filesystem picker integration, search/sorting/filtering, multiple simultaneous details windows and dashboard builders deferred. A native picker may be considered separately only if Chris elects it. |
| Service/platform | Existing manual daemon workflow and read-only current-user socket boundary. | No supervision/systemd, remote or multiuser access, polling changes, controls, alarms, package installer or Prototype 011. |

After Chris separately authorizes implementation, keep the first vertical slice visible:

1. Resolve/lock reviewed Slint dependencies under Rust 1.85.0 and open the actual native dark Sensors window consuming `get_info`/`get_snapshot` from a fixture daemon. Show one changing API row, five columns, group disclosure and basic accessible navigation. Verify Wayland/X11 backend selection where available. If build/window/accessibility gates fail, report the blocker; do not silently switch frameworks or deliver infrastructure only.
2. Complete typed validation, bounded refresh/reconnect, source grouping, qualified freshness/error states and shared session statistics. Keep the daemon/API untouched.
3. Add the separate details window with shared state and exact raw/identity diagnostics. Implement validated global appearance/layout settings and the schema-1 import/export/reset transaction, including pending group matching and recovery. Review the TOML parser/dependency graph explicitly; no parser dependency is preselected by this design.
4. Execute deterministic, regression/MSRV and actual-window/accessibility checks; report exact passes, failures and unavailable platforms. Stop for Prototype 010 review. No next milestone begins automatically.

Chris's preferred five-column hierarchy, default dark mode, source-focused visual details and portable configuration **are already authorized design direction**. Remaining approval decisions before implementation are:

- Accept the selected Slint 1.13.1/features and Rust 1.85/full-lockfile/Orca gate; any framework fallback remains a separate decision.
- Accept schema 1's 256 KiB cap, strict unknown-field/version rejection, backup/atomic-write behavior, bounds and separation from acquisition/security configuration.
- Accept conservative pending group restoration, including review after restart/reinstall, given best-effort API identity; do not promise guaranteed physical matching without backend changes.
- Accept the accessible path-entry import/export dialog for the first release, or explicitly choose the extra reviewed asynchronous native-picker integration.
- Confirm deferred per-sensor customization, graphs/history, arbitrary component relationships and convenience features remain outside this first usable GUI.
- Authorize the actual Prototype 010 implementation as a separate action after this review. This document does not confer that authorization.

Manual daemon ownership, the documented acquisition/refresh pairing and proposed resource targets remain as previously proposed. No accepted design-decision text is silently rewritten. A single recommended next action is **review and, if satisfied, authorize Prototype 010's first native GUI slice** against the acceptance criteria in §9. It is not a recommendation to begin Prototype 011.

## 12. Actual verification, preservation and Git status

Design status: **Complete for this revision; Prototype 010 implementation remains not started.** Changes consist of this proposal, two external SVG/PNG wireframe pairs, a proposed TOML example and external continuation/review evidence. No product source code, dependencies, daemon/API changes, live preferences or software installations were made. Artifact drawing/check scripts stayed in the external scratch workspace.

Actual design checks: required repository guidance and original proposal read; API/identity source and contract inspected; pinned Slint capabilities reviewed against primary sources; official HWiNFO functional reference visually inspected; both revised wireframes rendered and visually inspected; SVG/XML and PNG integrity checked; complete TOML example and embedded snippets parsed; schema defaults compared with the example; document section/link/scope checks performed; independent API/identity and framework reviews performed. The example is syntactically valid TOML, not evidence that a Rust preference validator exists.

No `cargo fmt`, Clippy, Rust/GUI tests, native GUI builds, Wayland/X11 execution, Orca runs, hardware scans or benchmarks were executed in this design-only revision. Passed/failed implementation test counts: **not applicable; none run**. Proposed checks in §9 are future acceptance criteria. Prior Prototype 009 CI and hardware claims retain their historical scope and were not repeated. No runtime, portability, accessibility or performance certification is implied by a static wireframe.

Git remains on `codex/alpha1-prototype010` at `3b6b85732b374d9d3dbc43ca052194475071bda9`. Verified final status is exactly `?? docs/milestones/010-native-gui-proposal.md`; the existing untracked proposal was revised in place. Tracked diff and staged diff are empty. No stage, commit, push, PR, branch switch or ref update was performed.

Preservation comparison covers all five registered worktrees (427 tracked/nonignored working files), branch/remote refs and worktree registration, plus 247 pre-existing evidence/binary files from the earlier planning inventory. The only expected working-file content change is this proposal. The original checkout's intentional dirty AGENTS/Prototype 006/agent-skills state, Prototype 007/008/009 worktrees, historical reports/archives, old wireframe/proposal evidence and private measurements remain unchanged. Hash/size/mode/mtime and Git-state comparisons are saved externally; no claim is made that every ignored build-cache file was audited. Nothing was cleaned, removed, reconstructed or republished.

The explicitly requested repository **`/handoff` skill** was invoked from `.agents/skills/handoff/SKILL.md` in the original checkout, because it is absent in the Prototype 010 worktree. It requires a temporary-directory continuation outside the workspace, a suggested-skills section, references rather than duplicated specifications, and redaction of sensitive content. The continuation was saved at `/tmp/siliconwatch010-design-revision-20261117-3f_gk2rl/CONTINUATION.md`, outside every registered worktree; a review copy and the preservation report accompany the external outputs. The exact unchanged prior proposal is preserved alongside the continuation as `proposal-before.md`. The old handoffs and skill setup remain untouched.

## HANDOFF TO CHATGPT

Chris authorized a planning-only revision of SiliconWatch Prototype 010 to add the preferred Sensors table, visual device details and portable GUI appearance profiles. The revised proposal is `docs/milestones/010-native-gui-proposal.md`. No GUI source, dependencies, daemon/API changes, staging, commits, push, PR or Prototype 011 work were performed. Implementation remains unstarted and awaits Chris's explicit approval.

The design now uses an expandable, compact dark Sensor/Current/Minimum/Maximum/Average table with right-aligned values, keyboard navigation and explicit stale, failed, unsupported and historical states. Statistics are session-only arithmetic aggregates of distinct valid observations received by the GUI. Repeated cache reads do not count again; failed/historical/stale observations freeze qualified aggregates. Counts and reset scope remain visible, and unit conversion never changes raw readings.

Double-click or Enter opens one reusable visual Device Details window sharing the main window's observation and statistics. It includes every reading from the confidently associated source registration, with temperature/voltage/fan categories only where API data exists. API v1 does not verify CPU inventory or relationships between providers; source names are used without inventing physical associations. Raw values, identity evidence, diagnostics and observation state stay accessible.

The recommended portable format is schema-1 TOML at `$XDG_CONFIG_HOME/siliconwatch/config.toml`, falling back to `~/.config/siliconwatch/config.toml`. Theme, font, temperature unit, window dimensions, column order/widths and table/group preferences persist. Import validates a maximum 256 KiB file, previews changes and replaces preferences atomically with backup/recovery. Unknown fields and unsupported versions reject the import. Profiles contain no credentials, endpoint identifiers, polling policy or hardware controls. Best-effort sensor identities require conservative reviewed group mapping; uncertain matches remain inactive. Per-sensor colors, labels, hiding and unit overrides are deferred to a future schema.

Slint 1.13.1 remains selected subject to Rust 1.85 locked-build and Linux accessibility qualification. Hierarchy and persistence are application work; screen-reader table semantics, widget font behavior and transitive dependencies remain gates. An accessible path-entry dialog is proposed for import/export because the pinned toolkit has no bundled file chooser; a native picker needs a separately reviewed integration.

Two revised SVG/PNG wireframes and a TOML example are in the external outputs directory. They were rendered/inspected and contain placeholders, not fabricated measurements. TOML syntax and document consistency were checked; no GUI runtime or hardware results are claimed. Branch/base remain `codex/alpha1-prototype010` / `3b6b85732b374d9d3dbc43ca052194475071bda9`; only the existing untracked proposal changed and the index is empty. Preservation checks retain unrelated worktrees, intentional dirty files, historical evidence and old wireframes.

The repository `/handoff` skill was explicitly invoked and a concise continuation saved under `/tmp`, outside every worktree. Chris should review the schema/recovery rules, conservative mapping policy, path-dialog choice, framework gates and deferred scope, then separately decide whether to authorize Prototype 010 implementation. Stop here; do not begin implementation or Prototype 011.

> Publication privacy review — November 17, 2026: personal home-directory paths and local-only file links were replaced with portable descriptions. The original report is preserved unchanged in the private external publication-review archive. Historical dates, verification results, failures and counts are unchanged.
