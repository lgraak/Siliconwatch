# Milestone 003 — Alpha 1, Prototype 003: Bounded Foreground Sensor Sampling

**Date:** 2026-11-16

**Status:** Complete — implemented and verified locally; awaiting Chris's review. Experimental read-only monitoring, without hard real-time or physical-freshness guarantees.

## 1. Milestone summary

Implemented the approved finite foreground sampling command on `codex/alpha1-prototype003`, based on the merged Alpha 1 foundation. Each attempt performs a fresh sequential hwmon scan, records observation timing, and emits a table or one JSON line. No completed snapshots are retained by the CLI. Existing one-shot commands, raw readings, identity construction, and diagnostic precedence remain unchanged.

Chris's implementation authorization supersedes two exit rules in the [historical proposal](003-bounded-foreground-sampling-proposal.md): completing all requested attempts returns **0**, including recoverable scan/serialization failures; sampling output failures, including broken pipes, return **1**. The proposal now has an explicit authorization note; its original content is preserved.

## 2. Implementation details and usage

```bash
cargo run -- sensors
cargo run -- sensors --json
cargo run -- sample --count 5 --interval-ms 1000
cargo run -- sample --count 5 --interval-ms 1000 --json-lines
```

Run as an ordinary user. `sample` requires decimal integer arguments: **1–10,000 attempts** and **100–60,000 milliseconds**. Missing, duplicated, negative, signed, fractional, out-of-range, or overflowing numeric arguments are rejected before scanning. No unlimited mode exists. `--hwmon-root PATH` supports isolated fixtures. `sample --json` is rejected; `--json-lines` explicitly selects streaming output. `sensors --json` remains a single snapshot/error object.

### Files and dependencies

| File | Change |
| --- | --- |
| [src/sampling.rs](../../src/sampling.rs) | New bounded loop, checked clock observations, record/freshness types, and buffered JSON Lines writer |
| [src/sampling/tests.rs](../../src/sampling/tests.rs) | 13 deterministic clock, fixture, and writer tests |
| [src/lib.rs](../../src/lib.rs) | Export sampling module |
| [src/main.rs](../../src/main.rs) | New command parsing, sample table/exit handling, and one parser boundary test |
| [tests/sampling_cli.rs](../../tests/sampling_cli.rs) | Eight new CLI fixture tests |
| [README.md](../../README.md) | Current status, commands, and completion-versus-reading-success clarification |
| [Proposal](003-bounded-foreground-sampling-proposal.md) | Preserve pre-existing untracked planning document; annotate authorization and superseded exit policy |
| This report | Implementation, verification, limitations, and handoff |

No dependencies were added or removed; Cargo.toml/Cargo.lock are unchanged. The hwmon reader, identity module, original tests, existing milestone reports, workflow, accepted decisions, license, and artwork are unchanged. No thread pool, async runtime, scheduler framework, service, or persistence layer was introduced. A private clock interface and scan/output callbacks support deterministic loop tests; production uses the standard library and one synchronous loop.

### Scheduling and retention

The first attempt is eligible immediately after argument validation. Later targets are the **previous actual start plus the configured interval**. The loop rechecks after sleeping, waits again after an early wakeup, and runs only one attempt if late. It then anchors the following target to that attempt's actual start. There are no overlapping scans or catch-up bursts. Output is written and flushed before the next scan; there is no final sleep.

Each attempt invokes the unchanged `hwmon::scan`, which rediscovers sensors and recomputes source evidence/collisions. Failed attempts consume count. No previous values, source matches, or identity candidates are reused. Removed inputs disappear from subsequent snapshots without synthesized rows or cross-attempt reconciliation.

Only the current snapshot and its output buffer/rows, plus scalar loop state, remain live. Memory does not grow with completed attempt count. Memory still depends on the size of a single scan; no hard allocation limit is claimed. The public sink receives ownership of each record; the CLI consumes it immediately rather than retaining history.

### Timing and freshness contract

Scheduling uses checked `Instant`-relative durations. Every observed monotonic value is checked against its predecessor. Backward observations, unrepresentable nanosecond offsets, or deadline overflow return an error rather than wrapping or substituting zero.

All timing fields are integer nanoseconds relative to this invocation's origin: `target_start_ns`, `started_ns`, `finished_ns`, `prepared_ns`, `scan_duration_ns`, and `lateness_ns`. The interval remains milliseconds. A separate nullable `wall_start_unix_ms` records `SystemTime` for correlation; a conversion problem sets `wall_time_error` without discarding telemetry or changing scheduling. Wall timestamps may move backward. Offsets from different invocations are not comparable.

Freshness is a **userspace observation window**, not driver-cache age, physical sampling age, calibration, or protection eligibility:

- With sensor rows, `basis` is `current_attempt_observation_window`; `lower_bound_ns = prepared - finished`, and `upper_bound_ns = prepared - started`. The window includes discovery and metadata work and brackets sequential reads.
- `observation_window_exceeds_interval` is true only when the upper bound is strictly greater than the interval. This conservative advisory is not a physical-staleness verdict. Existing row statuses remain authoritative about read/parse failures.
- Empty snapshots use `no_sensor_observations`; failed/serialization-error attempts use `no_snapshot`. Both have null bounds and a null interval verdict.
- `physical_age_known` is always false. `prepared_ns` precedes serialization and output; transport/backpressure can increase age at receipt. No delivery-time freshness guarantee is made.

### Output schemas and errors

JSON Lines emits one compact, newline-terminated, flushed object per completed attempt. The envelope has `record_type: "siliconwatch.sample"`, `sampling_schema_version: 1`, sequence/count/interval, `timing`, `freshness`, `snapshot`, and `error`. Exactly one of snapshot/error is non-null. Nested snapshots retain **schema 2**, and identity objects retain **schema 1**, with raw values, source paths, warnings, and identity evidence unchanged.

Serialization is buffered before writing. A snapshot that cannot serialize, including a non-UTF-8 path, becomes a complete `serialization_error` record with no snapshot or freshness bounds. The next attempt can recover. Buffering cannot undo bytes already delivered when a transport fails midway. JSON stdout contains records only; fatal loop/output diagnostics go to stderr. Table output escapes control characters and includes the existing sensor table beneath each timing header.

| Outcome | Sampling exit / behavior |
| --- | --- |
| All N attempts emitted, including root I/O failures, empty/partial snapshots, or recoverable serialization errors | **0**; inspect each record, because completion is not successful sensor data |
| Invalid arguments | **2**, without scanning |
| Unrecoverable clock/setup/internal serialization error | **1**, stop; no fabricated timing or prior values |
| Write or flush failure, including broken pipe | **1**, stop before another scan |
| External interruption, including Ctrl-C | Normal OS signal termination; stream can be incomplete; no new signal handler or guaranteed final summary |

Existing one-shot root/serialization errors still return 1, and its deliberate broken-pipe success convention remains 0. These differences are intentional and covered by tests. No new claim is made about behavior if stderr itself fails.

## 3. Actual verification results

Executed locally on 2026-11-16 as UID 1000:

| Check | Result |
| --- | --- |
| `cargo fmt --check` with Rust 1.100.0 | Passed |
| `cargo clippy -- -D warnings` with Rust 1.100.0 | Passed |
| `RUST_TEST_NOCAPTURE=1 cargo test` with Rust 1.100.0 | **53 passed, 0 failed, 0 ignored**, no conditional permission skips |
| `cargo +1.85.0 fmt --check` | Passed |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings` | Passed |
| `RUST_TEST_NOCAPTURE=1 cargo +1.85.0 test --locked --offline` | **53 passed, 0 failed, 0 ignored**, no conditional permission skips |
| Existing CLI regressions | All seven passed, both in the targeted CLI run and both full suites |
| `cargo run -- sensors` and `cargo run -- sensors --json` | Both passed on available hardware; JSON independently parsed |
| `cargo run -- sample --count 2 --interval-ms 100` | Passed; two table records, exit 0 |
| `cargo run -- sample --count 3 --interval-ms 100 --json-lines` | Passed; three independently parsed records, exit 0; sequence, schemas, timing arithmetic, and freshness bounds checked |
| Read-only review | Scanner/identity code unchanged; new hardware path only calls the existing reader. Writes are to output streams; fixture writes stay in temporary directories |
| Independent implementation review | No blocking correctness or scope issue found |

The total is **31 original tests plus 22 new tests**: 13 sampling-loop/writer tests, one CLI parser test, and eight sampling CLI integration tests. Suite totals are 16 library tests, one binary test, seven original CLI tests, nine reader tests, twelve identity tests, and eight sampling CLI tests. Permission fixtures actually exercised denial; no privileged-user skip was reported.

**Simulated timing tests:** fake monotonic/wall clocks cover exact attempts, first eligibility/no final sleep, sequential scan/output order, early wakeups, oversleep, slow scans and output, reanchoring, backward clock rejection, timestamp overflow, wall jumps/unavailability, freshness bounds and threshold equality. Fixtures cover missing-root recovery, changed and removed values, malformed/unsupported rows, and serialization recovery without stale fields. Fake writers verify per-line flush before the next scan and immediate stop on write/flush errors. A 10,000-attempt fake-clock run streams to a discarding sink; this supports the retention design but is not a measured memory benchmark.

**Real process fixture tests:** validate argument errors/bounds, finite JSON Lines and table output, recoverable-failure exit 0, non-UTF-8 errors, nested schema compatibility, and escaping. Linux `/dev/full` exercises output failures in both modes. A preclosed Unix-domain stream deterministically exercises broken-pipe exit 1 for sampling and exit 0 for one-shot output. These tests use isolated fixtures, not real sysfs. Some end-to-end count tests use actual short waits; no wall-time precision assertion depends on them.

At the implementation handoff, no hosted CI had run for these changes. No real suspend/resume, blocked-driver cancellation, forced interruption, hardware removal, physical accuracy, long-duration memory profiling, or CPU/frame-time benchmark was performed. Those are not reported as tested.

## 4. Hardware validation

Platform: **CachyOS, Linux 7.3.4-1-cachyos, x86-64**, AMD Ryzen AI 9 HX 370 / Radeon 890M and NVMe storage. Reads ran unprivileged. No hardware settings, driver modules, or permissions were changed.

Providers exposed in this session: `acpitz`, `nvme`, `amdgpu`, `k10temp`, `iwlwifi_1`, and `hidpp_battery_0`; the battery provider exposed no input attributes. Each sampled snapshot contained **11 inputs: seven temperatures, two voltages, and two unsupported GPU inputs**. Nine readings were `ok`; `freq1_input` and `power1_input` remained unsupported with raw data retained. There were no discovery warnings or permission-denied inputs in these live samples.

**Difference from historical results:** Prototype 002 recorded 24 inputs including IT8613. IT8613 and its fan channels were not exposed in the current session. The later publication investigation below confirms the same 11 current inputs from direct sysfs enumeration and the accepted baseline; the historical cause of IT8613's disappearance remains unknown. No current live fan-reading validation is claimed. Existing fixture tests continue to cover fan readings/conversions. GPU voltage inputs again reported zero; `ok` means parsed, not physically validated.

Representative one-shot readings: NVMe 42.85°C (another channel 33.85°C), GPU 40.0°C, CPU 44.0°C. A table sampling excerpt, with identity/path details omitted:

```text
Sample 1/2; interval 100 ms
Scan duration 14836901 ns; start lateness 1123 ns
Observation-age bounds at preparation: 1242..14838143 ns
Physical sensor age is unknown; preparation precedes serialization and output.
nvme       Composite    42.9 °C  ok
amdgpu     edge         41.0 °C  ok
k10temp    Tctl         44.1 °C  ok
```

The separate three-record JSON Lines run observed scan durations **14.730, 14.850, and 15.709 ms**, start lateness **0.001, 0.063, and 0.062 ms**, and observation-age upper bounds **14.731, 14.850, and 15.710 ms**. These are short-run observations, not latency budgets or guarantees. All three snapshots had the same 11-input count and no attempt-level error. Late/failed scans and wall-clock jumps were verified through simulation, not induced on real hardware.

## 5. Architecture and design review

The implementation follows the approved incremental Rust/Linux-first, read-only CLI scope. It preserves source identity ambiguity, row statuses, raw data, and the distinction between read/parse success and physical validation. The local API remains foundational to the future daemon architecture; experimental JSON Lines does not implement that API. **Decision 43 remains PENDING.**

No accepted architectural decision was silently changed. Fixed foreground sampling does not complete adaptive polling/cache Decision 4, timing output does not satisfy performance-gate Decision 23, and rediscovery without reused values does not complete suspend/resume Decision 33. The two exit-policy changes are explicit instructions in Chris's latest authorization, not inferred architectural changes.

No daemon/systemd service, cache, storage/database, API, GUI, remote monitoring, adaptive polling, plugins, alarms, controls, or protective shutdowns were added. The loop uses existing reader boundaries, with no evidence that a broader architecture is needed for this milestone.

## 6. Known issues, limitations, and risks

- No confirmed new application bug remains from verification/review. Recovery and output failure behavior are explicit; exit 0 is intentionally not an all-sensors-healthy signal.
- Blocking sysfs reads and output writes can prevent timely completion or interruption. Count and interval bounds are not a hard wall-clock deadline. There is no timeout worker, cancellation mechanism, watchdog, or protection claim.
- `Instant` suspend accounting is platform-dependent; observation bounds do not promise suspend-inclusive physical age. Driver-cache age, exact per-sensor timestamps, and atomic cross-sensor snapshots remain unknown.
- Memory is independent of completed count but not capped per scan. Arbitrarily large alternate-root contents or diagnostics can consume memory. Allocation failure is not a recoverable attempt error.
- Full discovery/identity work repeats each attempt. The 100 ms minimum is a guardrail, not a measured safe hardware cadence or CPU/frame-time budget. No caching optimization was added.
- Source paths, labels, and identity evidence remain visible in JSON. It is a local diagnostic format, not a privacy-redacted export; no data is uploaded automatically.
- Output backpressure increases delay; preparation-time age excludes subsequent serialization/transmission. A failed write can leave a truncated final line, and an interrupted stream does not establish count completion.
- Source identities retain Prototype 002's topology, replacement, and driver-change limitations. Missing rows are not persistently tracked or automatically reconciled.

## 7. Git status at implementation handoff — historical

- Branch: `codex/alpha1-prototype003`, no upstream.
- HEAD remains the foundation merge: `b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d`.
- Three tracked files are modified: README.md, src/lib.rs, src/main.rs.
- Five files are untracked: src/sampling.rs, src/sampling/tests.rs, tests/sampling_cli.rs, this report, and the pre-existing proposal (now annotated). Nothing is staged.
- No commit, push, merge, or pull request was performed for Prototype 003. Future contribution commits require authorization and DCO signoff. Temporary hardware output remains outside the repository and is not included in this report.

## 8. Recommended next milestone — proposal only

**Prototype 004: characterize foreground sampling overhead.** If Chris approves, measure the existing command on available hardware at a small set of fixed intervals/counts, recording process CPU time, peak resident memory, scan duration/lateness, output destination, and environment. Keep it a reproducible measurement exercise before choosing any cache, scheduling optimization, or numerical budget.

Acceptance: documented repeatable commands, short bounded ordinary-user runs, results separated from guarantees, fixture-based validation of any result summarization, and no monitoring behavior/dependency/API changes. Compare at least two intervals and table versus JSON Lines output; explain measurement overhead and hardware variability. Hardware availability and suitable process measurement tools are dependencies. Gaming frame-time testing, architecture changes, and performance thresholds require separate agreement. No Prototype 004 work has begun.

## Revision history

| Date | Change |
| --- | --- |
| 2026-11-16 | Implemented approved bounded foreground sampling and revised exit policy; passed 53 tests on Rust 1.100.0 and 1.85.0; recorded current 11-input hardware validation and handoff. |

## Publication preflight — 2026-11-16

Chris authorized final review, a DCO-signed implementation commit, ordinary feature-branch push, and one PR targeting `main`; merging is not authorized. A fresh fetch confirmed `origin/main` remains the accepted foundation merge `b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d`. No remote Prototype 003 branch or PR existed at preflight. Local HEAD and ancestry are unchanged.

The full eight-file change set was reviewed, including all untracked files. Scheduling, clock invariants, exit behavior, JSON Lines compatibility, retention, one-shot preservation, dependencies, and security/regression risks were checked. An independent read-only reviewer found no blocking issue; no source correction was needed. The remaining limitations in Section 6 are intentional boundaries, not claims of production readiness.

### Fresh verification

Formatting, Clippy with warnings denied, and **all 53 tests** passed again on Rust 1.100.0 and 1.85.0. Each suite had **0 failures, 0 ignored tests, and no conditional permission skips**; all seven original CLI regression tests were included. The 1.85 checks used locked/offline dependency resolution.

Both one-shot commands and short table (two attempts) and JSON Lines (three attempts) hardware runs passed as UID 1000. An independent Python parser checked exact counts/sequences, envelope/snapshot/identity versions **1/2/1**, exactly one snapshot/error outcome, integer timing fields and their ordering/arithmetic, reanchored targets, and observation-age bounds. Two missing-root attempts produced explicit errors with no snapshot/freshness and exit 0; the same missing-root one-shot command retained exit 1. Two partial-fixture attempts preserved `invalid` and `ok` sensor statuses with exit 0. The 53-test suite separately verifies write/flush and broken-pipe failure semantics.

Current live runs again returned 11 inputs, nine `ok` and two unsupported, without warnings or permission failures. Three JSON scan durations were **17.706, 17.939, and 17.792 ms**, with start lateness **0.001, 0.063, and 0.063 ms**. These fresh observations supersede no earlier measurements and establish no performance guarantee.

### Hardware input-count investigation

This investigation used only reads; no driver was loaded/unloaded, no sysfs setting was written, and no system configuration or permission was changed.

| Current provider | Direct sysfs input attributes |
| --- | ---: |
| `acpitz` | 1 |
| `nvme` | 3 |
| `amdgpu` | 5 |
| `k10temp` | 1 |
| `iwlwifi_1` | 1 |
| `hidpp_battery_0` | 0 |
| **Total** | **11** |

The accepted baseline `b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d` was rebuilt offline in an isolated temporary directory with a separate target directory. Its `sensors --json` and the current implementation were run back-to-back. **Baseline, current output, and independent sysfs enumeration had identical 11 provider/channel pairs and no discovery warnings.** The current hwmon reader and identity source are byte-identical to that baseline. This establishes that Prototype 003 is not omitting currently exposed inputs in the tested environment.

IT8613 is absent from current hwmon registration. `it87` is absent from `/proc/modules` and `/sys/module`; no related IT87/IT86 platform device/driver was found. An installed kernel `it87.ko.zst` module exists, and its metadata describes IT87/IT86 monitoring support, so an installed module does not establish that a provider is active.

Historical kernel journal evidence records successful IT8613E chip discovery by `it87` on **2026-11-15 10:27:20 UTC**, in an earlier boot. The current boot began **2026-11-16 19:02:12 UTC** and has no matching `it87`/`it8613` kernel messages. The earlier signature warning reports kernel taint and is followed by successful discovery; it is not evidence of a failed load. Read-only searches of `/etc` and `/usr/lib` modprobe/modules-load configuration found no explicit IT87/IT8613 entries. These observations support changed provider availability across boots but do not establish the reason for the current absence or an unloading event.

The recorded category difference is **13 inputs: three temperatures, eight voltages, and two fans**, consistent with the missing IT8613 provider. The battery provider's registration-name change cannot explain the delta because it has no inputs. No retained earlier raw snapshot establishes the full historical channel mapping, so attributing every missing channel to IT8613 is an inference from reports/categories, not a fresh observation of the old state.

**Conclusion:** current provider/environment availability explains the observed reduction; no Prototype 003 discovery/count regression was found. The evidence cannot establish when or why IT8613 stopped being available, distinguish a prior manual module load from other environment changes, or validate the absent fan channels now. Restoring/investigating driver state further requires a separately approved task if Chris wants it; it is not necessary to publish this unchanged read-only scanner.

### Publication checkpoint

The reviewed commit will contain exactly README.md, src/lib.rs, src/main.rs, src/sampling.rs, src/sampling/tests.rs, tests/sampling_cli.rs, the preserved/annotated proposal, and this report. Application dependencies, original reader/identity/tests, CI, license, and artwork remain unchanged. No binaries, temporary hardware dumps, unrelated files, or apparent credentials belong to this set. Hosted results must be observed after publication; local passes do not imply CI success.

## Implementation handoff — historical

SiliconWatch Alpha 1, Prototype 003 is complete locally and ready for Chris's review. Chris authorized bounded foreground sampling on codex/alpha1-prototype003, preserving one-shot behavior and excluding services, persistence, APIs, GUI, adaptive polling, alarms, controls, and shutdowns. No Prototype 004 implementation has begun.

The new sample command requires --count (1–10,000) and --interval-ms (100–60,000), with optional --json-lines. It starts immediately, runs scans and output sequentially, schedules from the previous actual start, and reanchors after delays without catch-up bursts. Each attempt rediscovers hardware; failed attempts consume count and never reuse prior readings. The CLI retains only the current snapshot/output record, not history.

Records contain checked monotonic timing, a nullable wall timestamp, and userspace observation-age bounds at record preparation. Physical sensor age is explicitly unknown. JSON Lines uses sampling envelope version 1 with unchanged snapshot schema 2 and identity schema 1. Serialization errors produce complete error records without stale values or freshness fields. Raw data, source evidence, row statuses, and diagnostic precedence are preserved.

The latest authorization supersedes the proposal's exit policy: completing all requested attempts returns 0 even when scans or individual readings fail; consumers must inspect the records. Invalid arguments return 2. Unrecoverable timing/internal errors and output failures, including broken pipes, return 1. Existing sensors commands keep their prior behavior, including one-shot broken-pipe success. External interruption can leave an incomplete stream; no signal handler or guaranteed final summary was added.

Formatting, Clippy with warnings denied, and all 53 tests passed on both Rust 1.100.0 and the declared minimum 1.85.0. There were zero failures, ignored tests, or conditional permission skips. All 31 original tests remain, with 22 additions covering bounds, fake-clock scheduling, delays, wall changes, freshness, fixture recovery, streaming, and output/exit failures. Existing CLI regressions passed. No hosted CI run is claimed for uncommitted work.

Read-only ordinary-user hardware tests passed for both one-shot commands and short table/JSON Lines sampling. CachyOS kernel 7.3.4-1-cachyos on AMD Ryzen/Radeon/NVMe hardware exposed 11 inputs: nine successful readings and two unsupported GPU inputs, without permission errors or discovery warnings. IT8613/fans present in historical reports were absent; no current live fan validation is claimed. Three JSON scans took approximately 14.730–15.709 ms. Delays, clock jumps, and failures were simulated rather than induced on hardware.

Changed files comprise the sampling module/tests, CLI/library integration, README, annotated proposal, and this report. Dependencies and existing reader/identity/tests are unchanged. HEAD remains b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d; three tracked files are modified and five files untracked, with nothing staged. No commit, push, or PR was created.

Blocking drivers/output prevent hard runtime guarantees. Memory depends on one scan's size; suspend behavior, physical freshness, and performance budgets remain limitations. Decision 43 remains pending. Recommend Chris review this implementation before authorizing publication or a small Prototype 004 overhead-characterization milestone. Stop for review.
