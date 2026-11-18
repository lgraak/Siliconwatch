# SiliconWatch native GUI

Prototype 010 is an experimental, native Linux client for SiliconWatch's existing read-only local API. It displays observations collected by a separately started daemon. Quitting the GUI leaves that daemon running.

The [Prototype 010 report](milestones/010-native-gui.md) records the actual build, desktop, accessibility and hardware checks, including untested environments. This guide describes the controls and data semantics; it is not a claim that every Linux desktop or screen reader has been qualified.

## Start the daemon and GUI

Use a normal, non-root account with Rust 1.85 or newer. From the repository root, start the daemon in one terminal:

```sh
cargo +1.85.0 run --locked -- daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

In another terminal, launch the GUI:

```sh
cargo +1.85.0 run --locked -p siliconwatch-gui -- --refresh-ms 1000
```

The native GUI needs a working Linux desktop and OpenGL support for its selected Slint renderer. Slint and its build helper remain pinned to 1.13.1; the project includes a narrowly scoped fontdue compatibility patch for Rust 1.85. See the milestone report for its provenance and verification. There is no software-renderer fallback or automatic dependency/framework upgrade.

The API must be enabled on a daemon owned by the same user. Its endpoint is under `$XDG_RUNTIME_DIR/siliconwatch/`; the client checks the existing namespace and peer identity. The GUI does not create or repair runtime directories, change socket permissions, remove stale endpoints, or start the daemon. See the [local API contract](api/local-v1.md) for endpoint requirements and daemon behavior.

`--refresh-ms` controls how often the GUI requests the daemon's cached snapshot. It defaults to 1000 ms and accepts one plain decimal integer from 1000 through 60000, without a sign or leading zeros. Duplicate or invalid options are errors. Requests are sequential; a slow request does not cause a catch-up burst.

For a daemon using a longer acquisition interval, choose a GUI refresh interval at least as long. The current API does not advertise acquisition cadence. Reading the cache more often does not acquire a new measurement, and refresh timing is deliberately absent from exported appearance profiles.

**Reconnect** retries the local API connection under the same checks and rate limits. It does not scan hardware. A missing endpoint, incompatible response or permission problem appears as a status message; correct the daemon/environment rather than running the GUI as root.

## System tray and closing windows

On a desktop with a working StatusNotifier host, SiliconWatch registers its existing project logo with the system tray. The menu provides **Open Sensors**, **Settings**, **Reconnect**, and **Quit SiliconWatch**. Left activation restores Sensors; the desktop host presents the right-click menu. Open Sensors and Settings reuse their existing windows. Reconnect uses the same cache-only API client.

**Minimize to Tray is off by default.** Closing Sensors exits the GUI and joins its owned workers. To keep the GUI running, enable **Settings → Appearance & Layout → Minimize to Tray** and apply the preference. Closing Sensors then hides it when a usable tray is available. Observations and session statistics continue while hidden; restoring the window preserves the session. **Quit SiliconWatch** always exits, regardless of this preference, and never stops the separately managed daemon.

Tray support is checked at runtime. If no usable host is available, closing the final visible window exits. If the host disappears while all application windows are hidden, Sensors reopens; a failed restore exits safely. Detection is bounded by the worker's periodic checks and call deadlines, rather than assuming startup registration lasts forever. The worker also responds to host disappearance signals. The status tooltip separates API connection state from observation currentness; a connected API does not certify recent sensor measurements or physical measurement age.

KDE Plasma is the primary target. GNOME and other desktops need a compatible StatusNotifier host; none is installed or configured by SiliconWatch. Hidden windows can be restored and unminimized. Raising or activating an already visible window is subject to the compositor and the pinned Slint API; desktop activation and accessible tray/menu exposure require separate qualification. The milestone report distinguishes protocol tests, native-window observations, and human accessibility checks.

## Read the Sensors window

The main window groups readings by the observed provider and source-device registration. Expand or collapse a group to inspect its sensors. Equal labels do not merge sources, and a familiar provider or sensor name does not prove a physical CPU, GPU or motherboard relationship.

The five columns are **Sensor**, **Current**, **Minimum**, **Maximum**, and **Average**. Numerical text is right-aligned. The last three columns describe this GUI session, not hardware-maintained extrema or a recorded history. A dash means no displayable value or no eligible statistic; it is never a substituted zero.

Read the observation banner and row quality together:

| Indicator | Meaning |
| --- | --- |
| Recent at last assessment | The validated observation remains within the client's conservative software-age budget. Physical measurement age and continuing daemon liveness are not certified. |
| Stale / freshness unconfirmed | The observation was stale at assessment, or its conservative age bound can no longer establish recency. Retained values and statistics remain qualified. |
| Historical | A retained earlier observation is being shown, for example after acquisition failure. It is not a new sample. |
| Failed / stopped / disconnected | Acquisition, producer state or connectivity prevents a current observation. A previously received observation can remain visible as last known. |
| Unsupported, unavailable, permission denied, invalid, fault or error | This row has a specific quality limitation. Other rows can still be usable. Original values and diagnostics remain accessible in Details. |
| Waiting / successful empty inventory | Waiting for a first acquisition differs from a completed acquisition that found no sensor observations. Neither state fabricates readings. |

A detected clock or suspend discontinuity clears positive freshness and freezes statistics for the affected daemon instance. Follow the displayed restart instruction to recreate the daemon's cache; reconnecting to the same instance does not clear this condition. The GUI's time bounds describe software observation intervals, not sensor-internal sampling times.

## Session statistics

Minimum, maximum and arithmetic average use distinct valid observations received by this GUI. A row contributes only when its canonical measurement/unit is supported, its status is `ok`, its value is finite, and its observation passes the currentness checks. Genuine zero RPM and valid negative temperatures remain valid values.

Repeated reads of the same cached publication count once. Redrawing, changing units, opening Details or expanding a group adds no samples. The selected row's valid-observation count is displayed. Acquisitions that occurred between GUI reads are not reconstructed, and averages are not time-weighted.

Failed, stale, historical or disconnected periods freeze qualified session statistics. A new daemon instance resets the statistical epoch. Changed or ambiguous source evidence also prevents transferring a statistic to another row. Statistics use bounded accumulators and do not retain a sample history.

Choose **Reset Statistics**, review the confirmation, and confirm to clear session statistics. The displayed cached publication is not counted again after the reset; a new eligible publication must arrive. Importing settings or resetting appearance defaults does not clear statistics or alter daemon behavior.

## Open Device Details

Double-click a sensor, select it and press **Enter**, or use **Open Details**. A single reusable native Details window shows all readings confidently associated with that current source registration. Opening another sensor explicitly retargets it.

Temperature, voltage and fan categories appear only when the API supplies those readings. Other or unsupported rows remain visible. The window shares the main window's observation and statistics; it does not create another API client or accumulator.

Select a reading to inspect its raw text, exact raw integer, canonical/display value, source evidence, identity issues, diagnostics and observation timing. These are selectable text. Source paths are evidence, not links to acquire hardware or authorize actions. The current API has no verified physical-device inventory or cross-provider component graph, so titles use source names rather than guessed component associations.

Closing Details returns focus to Sensors when it is visible. Closing Sensors follows the Minimize to Tray preference described above; quitting leaves the external daemon running.

## Keyboard operation

- **Tab / Shift+Tab:** move between ordinary controls, table and evidence areas.
- **Up / Down:** navigate rows. **Home / End** and **Page Up / Page Down** navigate longer lists.
- **Left / Right:** collapse or expand a group, or navigate to its group header.
- **Enter:** open a selected sensor's Details; on a group, toggle its disclosure. **Space** also toggles a group.
- **Escape:** close Details or an idle Settings window; cancel an unapplied confirmation when it is shown.

Refreshes should preserve focus. Explicit status words accompany color, and accessible row summaries include values, units, quality and statistical scope. Actual screen-reader and platform qualification is reported separately in the milestone report; toolkit support alone does not establish full accessible table-cell semantics.

## Appearance and layout

Open **Settings → Appearance & Layout**, edit preferences, and choose **Apply Preferences**. The available themes are exactly:

- **OLED Black**, the default, with black main/table/details backgrounds.
- **Dark**, using dark-gray surfaces.
- **Light**, using light surfaces.

There is no System theme. Focus and selection remain visible. Alternate-row backgrounds are suppressed in OLED Black so ordinary table surfaces remain black.

Choose `system-ui` or an installed font family and a logical font size from 10 to 24 pixels. A valid requested family that is unavailable on this installation uses a visible system-font fallback; its requested name remains stored for portability. Profiles cannot load font files, download fonts, supply CSS or execute styling.

Celsius is the initial/reset unit. The main **Fahrenheit (°F)** checkbox or Settings changes temperature display only and persists the choice. Conversion applies only to supported Celsius temperature readings; voltages, RPM, raw integers and diagnostics are unchanged. Both windows use the same display preference.

Settings also controls compact/comfortable rows, separators, alternate rows, default group expansion, normal window dimensions and main-window maximized state. Select a column, use **Move Left / Move Right**, and set its width. All five columns remain available; horizontal scrolling accommodates layouts wider than the window. At larger fonts, effective column minimums can exceed the stored widths for readability; this adjustment does not rewrite the profile. Direct header dragging, sorting, filtering and hidden sensors are not included.

Use desktop display scaling. Synthetic toolkit scale overrides that disagree with the native backend are not qualified; the milestone report records the observed capture failures.

Window sizes are logical pixels, without saved monitor identifiers, screen positions or scale factors. Desktop/compositor constraints can affect effective sizing. Explicit dimension settings remain the reliable way to carry a preferred size between installations; see the milestone report for small-screen and scaling limitations.

## Portable settings

The local file is:

```text
$XDG_CONFIG_HOME/siliconwatch/config.toml
```

When `XDG_CONFIG_HOME` is empty or unavailable, it falls back to:

```text
~/.config/siliconwatch/config.toml
```

A relative XDG path is ignored with a warning. If the configuration location is unavailable or unsafe, the GUI reports temporary/unsaved preferences instead of silently writing elsewhere.

Profiles use TOML with required `schema_version = 1` and `profile_kind = "siliconwatch.gui"`. Missing known settings take defaults; import replaces the whole preference set rather than merging selected fields. Unknown fields, unsupported versions, invalid types/ranges, malformed identities and invalid syntax reject the entire import. Maximum size is 256 KiB inclusive, UTF-8 only, with bounded structural nesting. Files must be regular local files; symlinks and special files are rejected.

Profiles contain GUI appearance, layout, and close-window behavior only. They do not store raw observations, session statistics, diagnostic logs, credentials, API tokens, endpoint identifiers, polling cadence, socket permissions, access policy or hardware controls. Per-sensor colors, custom labels, hidden sensors and per-sensor unit overrides are deferred and rejected by schema 1.

The backward-compatible schema-1 addition is:

```toml
[behavior]
minimize_to_tray = false
```

Existing valid profiles without this table or field retain the default close-to-exit behavior. The field accepts only a TOML boolean; strings, numbers, unknown behavior fields, and duplicate fields reject the entire candidate. No schema migration or reinterpretation of old values is needed. Export always includes the current behavior. Earlier GUI binaries with the strict original schema cannot import this newly extended profile without removing the new table; backward readability by this version does not promise forward readability by older binaries.

### Export

1. Open **Settings → Import, Export & Recovery**.
2. Enter the full absolute destination path, including the filename, for example `/chosen/directory/siliconwatch-settings.toml`. The destination directory must already exist and be safe for writing. There is no native filesystem picker in this release.
3. Leave **Include device group preferences in export** off for an appearance-only profile. Enable it only if you want to include source identity selectors, which can reveal hardware topology.
4. Choose **Export Settings…**, review the destination and replacement notice, then confirm or cancel.

Export uses the validated preferences held when the preview was prepared. Replacing an existing file requires the displayed confirmation; a changed destination is rejected for a new review. Export cannot replace the active configuration or its backup, and it does not alter active settings.

### Import

1. Enter the full absolute path of a local profile under **Import, Export & Recovery** and choose **Import Settings…**.
2. Review the validated incoming appearance/layout/close behavior, defaults, and pending device-group records. Invalid input shows an error and applies nothing.
3. Confirm replacement or cancel. File selection and preview alone do not apply settings.

The preview holds the parsed candidate; editing the source file after preview cannot substitute different settings at confirmation. A successful save commits the whole profile before applying it to the GUI. A failure before commit leaves active settings and the previous configuration intact.

### Review device groups after restart or import

Device selectors use existing SiliconWatch best-effort sensor identities, not labels or enumeration order. Those identities are not durable physical-hardware UUIDs: the same topology and channel can recur after hardware replacement or driver changes.

Saved/imported device-group overrides therefore start pending. While they are pending, groups remain expanded for inspection. With a recent live source inventory, choose **Review Device Group Matches…** to review proposed matches. Every anchor must resolve uniquely to the same source registration with consistent evidence. Missing, conflicting or ambiguous mappings stay inactive; they are never attached to the nearest-looking sensor. Offline imports apply global appearance only and keep device records pending.

A changed daemon/source inventory requires fresh review. You can still expand or collapse an actual displayed group directly. Groups without suitable unique identities have session-only layout changes. No imported “trusted” flag bypasses this review.

### Reset, conflict and recovery

**Reset to Defaults…** previews OLED Black, system font, Celsius, Minimize to Tray off, default dimensions/columns and removal of saved group overrides. Confirmation resets GUI preferences only. It does not clear session statistics, observation evidence or daemon configuration.

Writes use a same-directory temporary file and atomic rename. A previous valid active file is preserved exactly as `config.toml.bak`. New application configuration directories use mode 0700 and new files use 0600. The GUI does not change permissions on unrelated directories or the API endpoint.

Malformed, oversized or unsupported startup configuration remains untouched. The GUI uses temporary defaults, displays recovery state, and disables automatic persistence until you choose recovery. **Restore Validated Backup…** reads and validates the backup for a replacement preview. Explicit recovery preserves the unreadable original in a uniquely named local recovery file before replacement; it does not overwrite the good backup with invalid bytes.

If another editor or GUI changed the file, the write is rejected rather than overwriting that change. Use **Reload from Disk…** and review the result before deciding which preferences to keep. Reload pauses automatic saving while its replacement preview is unresolved. Cancelling or failing that review keeps the old appearance active and visibly unsaved; automatic saving resumes only after a successful, explicitly confirmed settings replacement. **Apply Preferences** also requires a preview while this pause is active. Ordinary direct unit/group/geometry changes can remain visible with an **Unsaved** notice after a save failure; a failed import/reset is not partially applied.

“Settings saved; crash durability could not be confirmed” means atomic replacement already occurred, but the final directory synchronization failed. The complete new preferences remain active and the backup remains available. Filesystem behavior determines power-loss durability; this warning does not mean the save was rolled back.

## Scope and remaining limits

The GUI consumes only `get_info` and `get_snapshot`. It performs no sensor acquisition, daemon supervision, fan/pump control, calibration, protective action, remote access or permission changes. It neither grants nor imports authority to change hardware.

Available measurements remain those supplied by the existing backend: supported hwmon temperatures, voltages and fan speeds, with source-dependent coverage and explicit errors. There is no persistent history, graph engine, dashboard builder, physical-component relationship database or guarantee that every acquisition reaches the GUI.

Only one Details window is reused. Grouped-list accessibility is application-defined rather than a claim of native table-cell semantics. Native backend, screen-reader, scaling and performance evidence—and any outstanding defects—belong in the [milestone report](milestones/010-native-gui.md). Prototype 010 technical acceptance remains Chris's decision.
