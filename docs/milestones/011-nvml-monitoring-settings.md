# Milestone 011 — NVML and monitoring settings

## Formal acceptance — November 18, 2026

**Chris accepted Milestone 011, NVML and Monitoring Settings, as technically
complete on November 18, 2026. All approved implementation objectives were
satisfied.** Chris accepted the documented limitations, including synchronous
ACPI reads that can delay publication, physical validation on one P620 and
driver, simulated multi-GPU and loss coverage, and the process restart needed
after replacing the loaded NVML library.

This decision grants technical acceptance only. Commits, pushing the branch,
opening a pull request, merging, deployment, and public release each remain
subject to separate authorization. Provider isolation, AMD GPU support, and
live daemon configuration remain separate future milestones.

The engineering reports below preserve the measurements and decisions at each
review. The [publication preparation](#publication-preparation--november-18-2026)
records the final verification, file inventory, and proposed PR.

## Engineering record before formal acceptance

**Final engineering status, November 18, 2026. Complete and submitted for
Chris's acceptance review.** The initial handoff below records the first
implementation. The dated final review at the end supersedes its hardware
inventory, resource measurements, and test totals.

November 18, 2026. **Partial**, pending review. The local implementation and
Sabine hardware acceptance checks are complete. Provider isolation and write
authorization remain separate design work.

## Implementation

- `src/nvml.rs` discovers the optional driver library by its Linux SONAME
  `libnvidia-ml.so.1`. It resolves read-only symbols, initializes and shuts down
  NVML per inventory, and re-enumerates devices on each acquisition. UUIDs and
  metric semantics identify channels across polls. An index is diagnostic
  evidence for that enumeration when a UUID is unavailable. Such rows are
  excluded from statistics. Missing functions, absent drivers, permission
  errors, and lost GPU results are isolated from hwmon data. NVIDIA's
  [NVML device queries](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html)
  define the C signatures and raw units.
- `src/lib.rs`, `src/main.rs`, `src/sampling.rs`, and
  `src/snapshot_cache.rs` route default hardware acquisition through hwmon and
  NVML. Explicit alternate hwmon roots remain fixture-only inventories.
  `gui/src/model.rs` names GPU groups by product and source identity.
  `gui/src/presentation.rs` and `gui/src/statistics.rs` format and aggregate
  eligible observations. The categorical performance state has a current
  value. VRAM is displayed in MiB, with bytes preserved in the payload.
- `gui/ui/settings.slint`, `gui/src/app.rs`, and `gui/src/preferences.rs` add
  Monitoring and versioned TOML persistence. Older profiles load defaults.
  The settings page shows the saved request, effective values, pending restart
  state, and a selectable daemon command. The API timeout applies to new
  client transactions after saving, through `gui/src/client.rs`. Defaults are
  1,000 ms acquisition, 3,000 ms stale threshold, and 5,000 ms GUI API timeout.
  Reset Monitoring to Defaults changes the draft. Apply Preferences saves it.
- `src/daemon/api.rs`, `src/local_api.rs`, `src/local_api/wire.rs`, and
  `gui/src/protocol.rs` carry effective daemon timings in the optional,
  independently versioned `get_info.result.monitoring` object. API 1,
  snapshot 2, and identity 1 are retained. Units and measurements were
  extensible string fields. Older consumers following the documented
  additive-field convention can keep using the API. Consumers validating
  against the older exact JSON schema need the updated schema. Method set,
  frame sizes, peer credentials, and access policy retain their existing
  contract.
- `src/local_api/os.rs` names the occupied endpoint and potential causes.
  Existing namespace objects retain their identity, and the first daemon
  remains reachable. No endpoint takeover is attempted.
- No added Cargo, system, or packaging dependency. The NVIDIA driver package
  supplies the runtime library. The installed Alpha 1 package and its profile
  were left alone. The development GUI used an isolated configuration.

## Verification

- Rust 1.85.0 locked baseline workspace tests succeeded before editing the
  feature checkout. The updated workspace run recorded 277 passed, zero failed,
  and two ignored opt-in desktop tray tests outside the sandbox. The sandbox
  masks real device and Unix-socket ownership behavior and produced failures
  in credential and lifecycle integration tests before the outside-sandbox run.
- Formatting, workspace Clippy with warnings denied, and the locked optimized
  workspace build passed. The release build produced local binaries only. The permission-fixture literals in
  `tests/hwmon.rs` and `tests/identity.rs` use octal notation for Clippy.
- CLI JSON decoded at snapshot schema 2, and the release CLI emitted valid JSON Lines for the two-GPU fixture. A live daemon answered both read
  methods and published a current observation. A duplicate daemon exited
  with the occupied endpoint diagnostic, the original still answered, and
  normal shutdown cleaned up its test socket. Same-UID and framing regressions
  ran in the workspace suite.
- A temporary ABI fixture supplied two GPUs and 18 unique candidate channels.
  It verified normalized power, unsupported calls and symbols, and a GPU-lost
  result that discarded the failed call's output. A separate loader fixture
  simulated missing NVML. On the sandboxed host, real initialization returned
  NVML status 9. Both unavailable cases retained hwmon telemetry.
- The native application opened on Sabine. A probe exercised the production
  table row model, Settings callbacks, and config worker. It verified the
  Quadro P620 group and formatted GPU rows at 66 °C and 14%, with statistics, effective 6,000/15,000 ms timings, a pending saved
  7,000/15,000 ms request, a 2,400 ms effective client timeout, and reset of
  the draft. Restarting the GUI reloaded the persisted profile. The probe
  drove the controls programmatically. It did not exercise keyboard or
  pointer navigation, or restart the daemon with the saved request.

## Hardware validation

Sabine ran CachyOS x86-64, kernel `7.3.4-1-cachyos`, with a Quadro P620 on
driver 580.196.07. The CLI inventory had 18 hwmon rows and nine NVML rows.

| Metric | SiliconWatch | Immediately following nvidia-smi |
| --- | --- | --- |
| Temperature | 65 °C | 65 °C |
| GPU utilization | 7% | 7% |
| VRAM used | 1,075,970,048 B, about 1,026 MiB | 1,026 MiB |
| VRAM total | 4,294,967,296 B, 4,096 MiB | 4,096 MiB |
| Graphics / memory clocks | 734 / 2504 MHz | 734 / 2504 MHz |
| Fan speed / power draw | unsupported / unsupported | N/A / N/A |
| Performance state | P3 | P3 |

During development the legacy memory query read about 86 MiB above
`nvidia-smi`. In three further direct comparisons the legacy reading was
1,106.0625 MiB, the v2 used value was 1,019.4375 MiB, and v2 separately
reported 86.625 MiB reserved. The `nvidia-smi` rounded values were 1,020 MiB
used and 87 MiB reserved. The provider now prefers the raw v2 used field
and labels the legacy fallback explicitly. Different source semantics
have different identity evidence so their statistics cannot be mixed.
In the final CLI comparison the CPU package was 56 °C in both tools.
NVMe composite was 57.85 °C and the secondary sensor was 60.85 °C in both.


Only one physical NVIDIA GPU was available. Multiple identities and a lost
result were exercised in the fixture. Physical hot removal, NVML call
latency across different drivers, and the overhead of per-inventory
initialization remain unmeasured.

## Slow ACPI acquisition

The original path
`/sys/devices/virtual/thermal/thermal_zone0/hwmon0/temp1_input`
returned 27,800 millidegrees in 6.03 seconds on a direct read. One live daemon
scan took about 6.10 seconds, and the observation stayed within a 15-second
freshness budget. The daemon retains the previous complete publication while
a scan runs. The FIFO integration fixture verifies that API reads remain
responsive and never schedule additional acquisitions. The sampling tests
verify that age bounds describe observation windows. A full-scan timestamp
includes software acquisition time; the physical measurement age remains
unknown. The API attempt exposes start, finish and publication timestamps for
scan-latency diagnostics. Per-provider timing is not collected.

For a later isolation milestone, profile per-registration hwmon reads and
give the slow ACPI registration and NVML independent bounded acquisition
lanes. A single worker per lane should admit at most one in-flight operation,
coalesce missed ticks, and publish original start/finish evidence only after
completion. A coordinator should continue publishing healthy lanes and mark
overdue lanes explicitly. Use a versioned model for mixed-age observations,
per-source freshness, and diagnostics. Avoid refreshing a stale value's
timestamp when publishing another lane. Threads blocked in kernel I/O
cannot be forcibly canceled by a software timer. A helper process can isolate
daemon lifecycle, but even a signal may remain pending during uninterruptible
I/O. Any helper design needs a fixed worker and process limit, a reaping
strategy, and a rule against replacing an unreaped helper. The milestone
ships the existing single-owner scan and no provider-response timeout
control.

## Git status and next milestone

Local branch `feature/nvml-monitoring-settings` is based on
`eb7d03ce3302b7f426ef47a1106e8eb7e9555fb8`. Changes are unstaged. The work
has not been committed, pushed, merged, or deployed.

Recommend a bounded provider-isolation prototype. Acceptance requires a
deterministically blocked ACPI fixture that leaves CPU, NVMe, and NVML
publishing independently, with finite worker counts, original per-source
observation windows, and explicit shutdown semantics. Live write access
requires separate approval of authority, protocol, and persistence. See the
[configuration proposal](011-live-monitoring-configuration-proposal.md).

## Initial handoff to ChatGPT

SiliconWatch Milestone 011 implements optional read-only NVML acquisition,
native Monitoring settings, and occupied-endpoint diagnostics on Sabine.
The local feature branch is based on
`eb7d03ce3302b7f426ef47a1106e8eb7e9555fb8`. All changes are unstaged.
The installed Alpha 1 package and settings were left intact.

Sabine's Quadro P620 appeared in the development CLI, daemon API, and
native GUI. The final CLI reported 65 °C and 7% GPU utilization, matching
the next `nvidia-smi` query. Graphics and memory clocks, total and used
VRAM, and P3 also matched at the reference tool's display precision. Fan
and power were unsupported in both. Raw NVML v2 memory reads agreed with
the reference and accounted for the earlier legacy memory gap using the
separate reserved counter. CPU package and NVMe composite matched at 56 °C
and 57.85 °C. The native GUI showed a Quadro P620 group with 66 °C and 14%
in the rendered rows and session statistics. Fixture checks covered two distinct GPUs, missing library
and symbols, unsupported calls, and a lost GPU result.


The Monitoring tab saves requested sampling and stale timings and shows the
effective daemon timings separately. The daemon must be restarted using the
selectable command to apply that request. The GUI response timeout takes
effect on new transactions after saving. The native probe verified pending
status, reset, and TOML reload across a GUI restart. The optional versioned
timing metadata extends the existing read-only get_info response. Existing
frame limits, method set and peer credential policy remain in force.

The direct ACPI read returned 27.8 °C in 6.03 seconds and a full acquisition
took about 6.10 seconds. At 6-second sampling and a 15-second budget the
daemon published a current observation. This is scan-time evidence.
Physical measurement age remains unknown. The documentation proposes
bounded provider workers with separate observation windows. A provider
timeout control requires enforceable semantics and was omitted. No configuration
write authority was added. Chris's approval is needed for provider
isolation and separately for the live-configuration authority, protocol,
and persistence model.

The updated pinned workspace suite recorded 277 passes and two ignored
desktop tray tests. Formatting, workspace Clippy with warnings denied,
and the locked workspace release build passed. The recommendation is to qualify provider isolation against the
blocked-read fixture before adding a provider budget or changing the live
configuration protocol.

## Final engineering review — November 18, 2026

### Physical acceptance and timing

Tests ran as the desktop user on CachyOS x86-64, kernel `7.3.4-1-cachyos`
and driver 580.196.07. The release CLI returned 18 hwmon rows and ten NVML rows
for one Quadro P620. The private daemon exposed the same GPU identity candidates.
The following comparison used adjacent queries. NVML utilization and clocks
can change between them.

| Channel | SiliconWatch | Following nvidia-smi |
| --- | ---: | ---: |
| Temperature | 63 °C | 63 °C |
| GPU utilization | 13% | 13% |
| Used VRAM | 1,105,330,176 B (1054.125 MiB) | 1055 MiB |
| Reserved VRAM | 90,832,896 B (86.625 MiB) | 87 MiB |
| Total VRAM | 4,294,967,296 B | 4096 MiB |
| Graphics clock | 1303 MHz | 1303 MHz |
| Memory clock | 3003 MHz | 3003 MHz |
| Performance state | P0 | P0 |
| Fan speed and power draw | unsupported, null numeric fields | N/A |

A temporary optimized Rust probe used the production provider and collision
check directly, excluding sysfs, socket, GUI and process startup. Each iteration
opened the SONAME, initialized NVML, enumerated the device, queried the metrics,
constructed the rows, shut down and released the loader handle. Across one cold
and 100 warm iterations, all ten candidate strings stayed stable. Before the
loader fix, the warm median was 9.30 ms and the open descriptor count went from
10 after warmup to 110. On the corrected code, cold acquisition took 9.56 ms.
Warm minimum, median, p95 and maximum were 6.50, 6.64, 6.84 and 6.95 ms.
Descriptors remained at 10. Process RSS after the cold poll and after the last
poll was 21,848 and 21,864 KiB. This bounded run measures one machine under
its current load. The driver mapping intentionally remains resident.

In a separate run, a complete daemon acquisition took 6.08 s, and its upper
observation-age bound at assessment was 6.16 s against a 15 s threshold.
An independent direct ACPI read took 9.07 s and returned 27.8 °C. This
exceeded the original roughly six-second observation. Physical measurement
age remains unknown. The GUI continued servicing timers and accepting settings
during acquisition. A 100 ms GUI timer fired 50 times after opening Details
while the restarted daemon produced a second eligible sample. The largest
gap was 0.93 s, so this run supports responsiveness during polling but gives
no tight UI latency guarantee.

### Changes since the initial handoff

- `src/nvml.rs` and its native-ABI test fixture under `src/nvml/` cover
  mapping lifetime, balanced session cleanup, v2 reserved bytes, distinct
  legacy memory identities, function-not-found fallback, unavailable status,
  and preserving raw unsigned text beyond the signed wire range. Legacy
  queries cannot supply reserved bytes. Permission or GPU-loss failures never
  trigger a fallback that could mask the failure. The loader now uses
  `RTLD_NODELETE`. NVML initialization and shutdown remain paired on each
  inventory. Runtime dependencies and package manifests retain their prior
  contents. The fixture requires a C compiler at test time only.
- `src/main.rs` formats the GPU units in the ordinary CLI table.
  `gui/src/presentation.rs`, `app.rs`, `model.rs` and their tests cover
  complete Details categories, selection when GPU identity is uncertain,
  and clearing effective timings on explicit reconnect.
  `gui/src/statistics.rs` tests v2/legacy separation across transitions.
- `gui/src/protocol.rs` and its tests accept old daemons without the optional
  timing extension, validate revision 1, and skip unsupported future
  revisions. `gui/src/client/tests.rs` verifies that a saved 100 ms timeout
  expires the next request on an existing worker. The original exact JSON
  Schema predates the optional response field. Strict validators need the
  updated schema. API v1's documented additive-response rule, method set,
  peer checks, request framing and socket cleanup still apply.
- `docs/GUI.md` and `docs/api/local-v1.md` document GUI profile, fan, and
  memory semantics. Schema-1 portable group selectors remain hwmon-specific.
  NVML group layout can be adjusted in the current GUI session.

### Regression and native application evidence

The full pinned workspace suite increased from 277 to 287 passing tests.
The ten additions cover the CLI table (one), native ABI fixture (three),
Device Details and index-only selection (two), timing metadata (two),
client deadline application (one), and memory statistics (one). No tests
failed, and the two opt-in desktop tray tests remained ignored. Rustfmt,
Clippy across the workspace and all targets with warnings denied, the locked
release workspace build, and the Git diff whitespace check passed.

A separate private daemon with a simulated driver published two GPUs, a lost
temperature reading, disappearance, and recovery in reversed order. It published
null for the lost reading, removed the disappeared ten rows, and restored the
same 20 unique candidates on recovery. CPU and NVMe stayed available on each
publication. Separate missing-library and driver-unavailable runs retained 18
hwmon rows, including motherboard, CPU and storage telemetry. These are
failure simulations. Only the P620 tests above used the physical NVIDIA device.

The native Slint probe opened the P620's Device Details on utilization,
verified all ten rows with the selected row in view of the model, and checked
the two unavailable values. After changing the GUI timeout to 2400 ms and
saving a 7000/15000 ms daemon request, the current session's GPU sample count
stayed at two. The TOML reloaded in a fresh GUI process, alongside existing
legacy-profile test coverage. The probe copied the generated command, checked
the executable and arguments, and used those arguments with the local release
binary in the isolated runtime. Effective sampling changed from 6000 to
7000 ms, the threshold stayed at 15000 ms, and Settings reported the saved
request active. This exercised production callbacks programmatically.

The recommended next milestone is a separately approved bounded provider
isolation prototype with the slow-read and timestamp acceptance criteria
recorded above. Chris retains the acceptance and publication decisions.

## HANDOFF TO CHATGPT

1. **Implementation review.** The P620 provider discovered devices by UUID and preserved raw readings, but omitted reserved VRAM from its output. The CLI table also showed dashes for valid GPU metrics, and Device Details omitted most GPU rows. Repeated unloading of the driver library grew Sabine's descriptor count by one per scan.

2. **Corrections.** SiliconWatch now publishes reserved VRAM from NVML v2, distinguishes legacy memory evidence and statistics, and displays all ten GPU channels in Device Details. It clears index-only selections on re-enumeration. Optional-query and handshake regressions cover missing functions, driver errors, and future timing metadata. Monitoring preferences survived a GUI restart. Saving preserved the session count, and the generated launch command moved the GUI from pending to active after an isolated daemon restart.

3. **Physical hardware.** On Sabine's Quadro P620 with driver 580.196.07, SiliconWatch and the following `nvidia-smi` sample agreed on 63 °C, 13% utilization, 1303/3003 MHz clocks and P0. NVML returned 1054.125 MiB used, 86.625 MiB reserved and 4096 MiB total. The reference displayed 1055, 87 and 4096 MiB. Fan speed and power returned unsupported with null values.

4. **Tests.** On Rust 1.85.0 the workspace recorded 287 passed, zero failed and two ignored opt-in tray tests. Ten new passing regressions account for the increase from 277. Formatting, locked all-target Clippy with warnings denied, and the locked release workspace build passed. A simulated two-GPU daemon recovered from loss and reversed enumeration with the original 20 channel identities and continuously available CPU and NVMe readings.

5. **Performance.** Keeping the NVML library mapped while balancing initialization and shutdown held descriptors at 10 over 100 repeated acquisitions. The earlier run grew from 10 to 110. Final provider latency was 6.64 ms median and 6.84 ms p95. A full daemon acquisition took 6.08 s. A separate ACPI read took 9.07 s.

6. **Security.** Collection ran as the desktop user. The local API retained its two peer-verified read methods. A second daemon reported the occupied endpoint and the first kept serving. Testing used local binaries and isolated profiles.

7. **Limitations.** Physical validation covered one GPU and one driver. Loss and multi-GPU recovery used fixtures. The synchronous ACPI read can still delay publication. Restart the monitoring process after replacing the NVML library so it loads the new mapping. Provider isolation and live configuration remain separate proposals.

8. **Working tree.** Milestone 011 is ready for Chris's acceptance decision on `feature/nvml-monitoring-settings` at `eb7d03ce3302b7f426ef47a1106e8eb7e9555fb8`. All 34 changed or new files are unstaged. The installed Alpha 1 package remains in place.


## Publication preparation — November 18, 2026

### Final verification

The acceptance pass reran the pinned Rust 1.85.0 checks on the feature
checkout. Formatting, workspace Clippy with all targets and warnings denied,
the locked release workspace build, and the diff whitespace check passed.
The workspace suite matched the final engineering baseline exactly with
287 passed, zero failed and two ignored. Both opt-in tray tests passed when
run separately on a private bus. Feature-enabled GUI all-target Clippy,
four standalone vendored fontdue tests and 153 Python diagnostic fixtures
also passed. Python discovery included all four release-CLI fixture cases
and recorded zero skips. The P620 and timing figures in the proposed PR
come from the earlier physical acceptance run.

The changed paths contain source, tests and documentation. The review
found no accidental payload, debug probe, credential-shaped token, physical
GPU UUID literal or personal home-directory path in the contribution.
Sabine and the tested driver, hardware, kernel and sysfs sensor path are
intentional qualification evidence. The C fixture contains a simulated
ABI and standard C headers. It invokes the system compiler at test time.
The contribution adds no bundled NVIDIA library or SDK header. Both Cargo
packages retain GPL-3.0-only and the existing version. Dependency manifests,
lockfiles, third-party notices, packaging and service definitions have no
changes against the baseline.

The read-only API retains the existing socket ownership, permissions,
peer verification and cleanup rules. An exact validator using the original
response schema requires the updated schema for optional timing metadata.
The GUI loads Alpha 1 TOML with defaults. An Alpha 1 GUI rejects the new
monitoring section on downgrade. Direct Rust users of the GUI client helpers
also need to account for the added timeout argument and timing metadata on
`TimedSnapshot`. This is separate from the wire protocol.

### Proposed DCO commit sequence and complete file inventory

The four groups below cover the 34 repository-relative paths exactly once.
Of these, 29 are tracked modifications and five are new files. The new files
are the NVML provider, its two fixture files, and the two milestone documents.
Recording acceptance and correcting the existing GUI/API documentation added
no inventory paths. The first three proposed checkpoints each passed
`cargo +1.85.0 check --workspace --all-targets --locked --offline` on a
cumulative temporary copy of the baseline tree. This checked the file
boundaries without staging files or creating commits.

After explicit commit authorization, each contributor must use their own
authorized Git name and email and sign off each commit with `git commit -s`
under Decision 41. Neither author nor committer identity resolved in the
acceptance environment. The two DCO trailer parser tests passed. Actual
contribution signoffs can be checked only once commits exist, using
`python3 -B .github/scripts/check_dco.py <base-sha> <head-sha>`.

**1. `feat: collect read-only NVIDIA telemetry through NVML`**

Starts from the baseline. Adds optional acquisition, raw metrics, identity, the CLI display, and fixture coverage. The two octal permission literals in the existing tests satisfy all-target Clippy and preserve the fixtures’ access modes. Verify with CLI, sampling, cache, identity and NVML tests and core all-target Clippy.

```text
src/lib.rs
src/main.rs
src/nvml.rs
src/nvml/provider_fixture.c
src/nvml/provider_tests.rs
src/sampling.rs
src/sampling/tests.rs
src/snapshot_cache.rs
tests/hwmon.rs
tests/identity.rs
```

**2. `feat: report effective daemon timings and occupied endpoints`**

Follows commit 1 so the API documentation covers both GPU data and the new read-only timing metadata. Improves the collision diagnostic. Verify with wire-format and daemon integration tests, including same-user access, rejection of invalid requests, and preservation of the first daemon’s socket.

```text
src/daemon/api.rs
src/local_api.rs
src/local_api/os.rs
src/local_api/tests.rs
src/local_api/wire.rs
src/local_api/wire/tests.rs
tests/local_api_cli.rs
docs/api/local-v1.md
docs/api/local-v1.schema.json
```

**3. `feat: display GPU telemetry and monitoring settings in Slint`**

Consumes the inventory and timing contract from commits 1 and 2. Adds GPU presentation, statistics, TOML monitoring preferences, effective timings and client timeout handling. Verify the GUI protocol, preferences, model, category and statistics tests, and run feature-enabled GUI Clippy.

```text
gui/src/app.rs
gui/src/client.rs
gui/src/client/tests.rs
gui/src/model.rs
gui/src/model/tests.rs
gui/src/preferences.rs
gui/src/preferences/tests.rs
gui/src/presentation.rs
gui/src/protocol.rs
gui/src/protocol/tests.rs
gui/src/statistics.rs
gui/ui/settings.slint
docs/GUI.md
```

**4. `docs: record Milestone 011 acceptance and configuration proposal`**

Follows the implementation and verification commits. Preserves the engineering evidence, records Chris’s decision, and documents the separate future proposal. Check the Markdown links, evidence totals, inventory and full diff against the accepted scope.

```text
docs/milestones/011-live-monitoring-configuration-proposal.md
docs/milestones/011-nvml-monitoring-settings.md
```

### Proposed pull request targeting main

**Add read-only NVIDIA monitoring and native monitoring settings**

```markdown
## Summary

Implement the technically accepted Milestone 011. The CLI and daemon discover
NVIDIA GPUs through the driver's optional NVML library. Both the sensor table
and native Device Details expose ten channels with per-row quality, raw values,
units, and identity evidence. NVML v2 reserved memory is reported directly.
The legacy memory fallback uses separate identities and statistics.

The Slint Monitoring tab saves requested sampling and freshness settings in
versioned TOML, shows effective daemon settings, and provides a restart command.
The GUI response timeout applies to new requests after saving. A second daemon
now identifies the occupied endpoint and leaves its owner serving.

## Verification

Rust 1.85.0 formatting, all-target workspace Clippy with warnings denied, the
locked release build, and the diff whitespace check passed. The workspace
suite recorded 287 passes, zero failures and two opt-in tray tests ignored.
Both tray tests passed separately. Feature-enabled GUI Clippy, four vendored
fontdue tests and 153 Python diagnostic tests also passed locally.

On Sabine's physical Quadro P620 with driver 580.196.07, SiliconWatch and the
next `nvidia-smi` query agreed at 63 °C, 13% utilization, 1303/3003 MHz and
P0. Used, reserved and total VRAM were 1054.125, 86.625 and 4096 MiB.
The reference displayed 1055, 87 and 4096 MiB. Fan speed and power were
unsupported and carried null numeric values. Across 100 warm acquisitions,
descriptors stayed at 10. Median NVML time was 6.64 ms and p95 was 6.84 ms.

The native GUI showed all ten channels, preserved statistics across a
preference save, and reloaded the saved TOML. Running the generated restart
arguments against an isolated daemon moved its request from pending to active.
In a separate two-GPU fixture, one GPU disappeared and the devices returned
in reversed enumeration order. Recovery restored all 20 original channel
identities, and CPU and NVMe readings stayed available. Missing-library and inaccessible-driver checks
also preserved hwmon readings.

## Security and compatibility

Normal operation uses unprivileged read-only queries. The Unix socket keeps
its ownership, permission, peer-credential and cleanup checks. API v1 retains
`get_info` and `get_snapshot`. Optional versioned timing metadata in
`get_info` requires an updated schema for strict response validators. Snapshot
v2 and identity v1 are retained. The new GUI loads older profiles with
defaults. Alpha 1 rejects profiles newly saved with `[monitoring]`.

## Scope and release

The GUI guide, API contract and schema document these changes. The milestone
report records acceptance and physical and simulated evidence. The separate
live-configuration proposal identifies the authority and protocol decisions
still needed. Physical coverage is limited to one GPU and driver. Slow ACPI
reads still delay publication, and a process restart loads a replacement NVML
library. Provider isolation, AMD support and live daemon configuration are
separate future milestones.

Chris accepted all approved objectives and documented limitations on November
18, 2026. This source contribution retains version `0.1.0-alpha.1`. Merge,
deployment and public release require separate authorization.
```

### Acceptance handoff to ChatGPT

| Decision | Status on November 18, 2026 |
| --- | --- |
| Technical acceptance | **Approved by Chris**, including the documented limitations |
| Commit readiness | Content ready. Commit authorization and the contributor’s DCO identity are pending. |
| PR readiness | Title, description and inventory ready. Authorized signed commits, branch push and PR creation are pending. |
| Merge authorization | **Not granted**. Hosted checks and review of the eventual PR are pending. |
| Deployment and public release | **Not granted**. The installed Alpha 1 package remains in place. |

The feature branch, local `main` and GitHub `main` matched the original
baseline `eb7d03ce3302b7f426ef47a1106e8eb7e9555fb8` at review time.
All 34 contribution paths are unstaged. This pass made documentation-only
edits. The recommendation is to authorize the four DCO-signed commits once
the contributor identity is supplied, then authorize pushing the feature
branch and opening the proposed PR for review.
