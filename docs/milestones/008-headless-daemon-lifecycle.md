# Alpha 1, Prototype 008: Headless Daemon Lifecycle

## 1. Milestone summary

**Status: Accepted by Chris — November 17, 2026.**

**Completion: Complete — implemented Rust foreground daemon, with documented limitations.**

Implemented November 17, 2026 under Chris's authorization of the [proposal](008-headless-daemon-lifecycle-proposal.md), including Section 10 and **Option 2: cooperative shutdown**. This is an experimental foreground lifecycle, not a production service installation. No commit, push, PR, merge or Prototype 009 implementation is authorized or performed.

The Rust application now continuously acquires read-only snapshots through one dedicated owner, handles SIGINT/SIGTERM, preserves failed/historical observations, and joins its worker before reporting termination. The proposal itself remains unchanged as the reviewed historical specification.

### Formal acceptance — November 17, 2026

Chris formally accepts **Prototype 008: Headless Daemon Lifecycle** as technically complete. Acceptance applies to the implemented **Rust foreground daemon**, including **Option 2: cooperative shutdown**. All documented limitations remain accepted, not resolved.

Acceptance does not establish production readiness or authorize deployment, systemd installation, API/GUI development, hardware controls or protective actions. This review authorizes acceptance documentation, deterministic verification, preservation checks and publication-readiness review only. Staging, committing, pushing, PR creation/merge and Prototype 009 remain unauthorized. Decision 43's local API permissions remain **Pending**.

The original proposal is preserved byte-for-byte as historical planning. The [publication-readiness report](008-publication-readiness.md) records the subsequent fresh review and revised publication manifest.


## 2. Implementation and usage

```bash
cargo run -- daemon --interval-ms 1000 --stale-after-ms 3000
```

Run as the normal user. Ctrl-C or SIGTERM requests cooperative shutdown. For a fixture directory, append `--hwmon-root PATH`. This command does not install anything, fork into the background, write sysfs, or expose a telemetry API.

Both timing arguments are required. The interval accepts **100–60,000 ms**. The freshness threshold accepts a positive decimal `u64` number of milliseconds; its conversion to `Duration` does not overflow. Missing, duplicate, malformed, overflowing and unknown arguments are rejected before worker creation. The root defaults to `/sys/class/hwmon`. Existing `sensors`, `sensors --json`, table `sample` and JSON Lines commands retain their contracts.

### Ownership and lifecycle

The main thread coordinates signals; one named worker exclusively owns `CacheOwner`. Standard-library `Mutex`/`Condvar` coordination holds only lifecycle/admission state. The existing scanner, identity implementation, sampler and cache are unchanged. There is no generic provider interface, asynchronous runtime, second acquisition owner, snapshot queue or accumulated log history.

| State | Implemented meaning |
| --- | --- |
| `Starting` | Worker/control initialization, with no claim of sensor health. Setup failures do not start acquisition. |
| `Running` | Worker transitioned under the control mutex and emits its startup acknowledgement before admitting its first scan. Main immediately listens for signals; it never waits separately for readiness. |
| `Stopping` | Stop acknowledged under the same mutex used for admission. New admission is closed and an idle worker is notified. An already admitted attempt may still begin, finish and publish. |
| `Stopped` | Owner cleanup and worker join completed, or thread creation failed without creating a worker. Execution success/failure is reported separately. |

The startup notice records the `Running` transition; it is not an enduring liveness or healthy-sensor guarantee and can race a later stop. If control state is poisoned, cleanup still closes signal delivery and joins the worker, but the report says **lifecycle unavailable**, rather than inventing a valid `Stopped` state.

### Scheduling, freshness and failure

Polling uses checked monotonic offsets and a deadline based on each actual owner-call dispatch start. Timed waits recheck both stop state and remaining time after wakeups. Delayed attempts reanchor the next deadline; there is no backlog, catch-up burst or overlap. Scheduler offsets are distinct from cache-relative acquisition metadata and are never substituted for sensor observation timestamps.

Cache metadata remains unchanged: sequence, userspace start/finish/publication offsets, optional wall-clock correlation and wall-clock errors. Age bounds remain `[assessment - finish, assessment - start]`; the upper bound exceeding the configured threshold is stale. Equality remains within budget. Physical hardware age remains unknown.

Successful scans preserve every raw reading, converted value, identity, source path and diagnostic, including empty/partially failing snapshots. Recoverable whole-scan failures publish an explicit failed attempt and continue at the configured interval. Any retained success is historical. A subsequent successful acquisition recovers through normal cache replacement. No previous success is fabricated as the result of a failed attempt.

The private service assessment requires `Running`, an active owner, latest-success status, latest-attempt observation role and freshness within budget before reporting service-current data. Individual sensor status remains authoritative. During `Stopping` or `Stopped`, data is non-current regardless of age or an underlying cache assessment temporarily still reporting an active owner. This service assessment does not change cache roles or expose a new public read API.

### Signals, cleanup and reporting

Only SIGINT/SIGTERM are registered, before the worker starts. Signal processing that uses locks runs in ordinary main-thread context. A worker exit guard closes the signal iterator on normal return or panic, including startup failure before first admission. Unexpected iterator wait unwinding is also caught narrowly and converted into a fatal coordinator result; the iterator is not resumed afterward.

All coordinator paths after a successful spawn stop admission, close delivery and join. Worker fatal errors retain precedence over concurrent stop/reporting errors. The signal registration and handles remain alive through joining; repeated signals are idempotent. Closing the iterator wakes its wait, not a hardware read. Signal-hook unregistering does not promise restoration of previous/default OS handlers, so this entry point is documented for the dedicated CLI process rather than concurrent library hosting.

No lifecycle mutex is held across cache-lock acquisition, scanning, output or joining. Combined inspection acquires the cache view before the lifecycle mutex; read guards are released before output/join. Fatal lock/clock/cache errors fail closed. Cleanup never detaches a worker and reports successful completion.

Normal stderr output is a startup line and one final outcome; stdout has no daemon telemetry. For example, the actual short hardware smoke ended with:

```text
SiliconWatch daemon running
SiliconWatch daemon stopped; outcome: graceful; latest attempt: succeeded; observation: historical; service-current: false
```

| Exit | Meaning |
| --- | --- |
| `0` | Requested cooperative shutdown completed, including owner cleanup/join. Recoverable acquisition/row errors do not change this exit policy. |
| `2` | Invalid daemon arguments, if the error could be reported. |
| `1` | Setup, fatal control/cache/worker/coordinator, or required-output failure. If reporting an argument error itself fails, exit is `1`. |

Startup-output failure ends the worker and wakes main. Final reporting occurs only after join; write/flush failures still return nonzero. Daemon errors bypass the older one-shot broken-pipe-success exception. Exit status remains authoritative for reporting failure even if part of a final execution-outcome line was written.

### Changed-file manifest

The original implementation manifest contains these ten paths; none are staged:

1. `Cargo.toml` — narrow signal dependency.
2. `Cargo.lock` — exact dependency resolution.
3. `README.md` — concise status/command/link update.
4. `src/lib.rs` — daemon module export.
5. `src/main.rs` — separate daemon parsing/dispatch and fallible diagnostics.
6. `src/daemon.rs` — lifecycle, worker, coordination, inspection and cleanup.
7. `src/daemon/tests.rs` — 21 deterministic unit/fixture tests.
8. `tests/daemon_cli.rs` — seven isolated-process CLI/signal tests.
9. `docs/milestones/008-headless-daemon-lifecycle-proposal.md` — pre-existing untracked proposal, preserved byte-for-byte.
10. `docs/milestones/008-headless-daemon-lifecycle.md` — this report.

The acceptance review separately adds `docs/milestones/008-publication-readiness.md`, making the revised intended publication manifest **eleven files**. Its exact list is recorded in the [readiness report](008-publication-readiness.md).

One additional parser-boundary test is in `src/main.rs`: **29 new executable Rust tests** in total. No application-contract deviations from the approved proposal were required. The private setup/clock/acquisition/inspection seams and guarded signal wait implement its deterministic testing and cleanup requirements.

### Dependencies

`signal-hook = { version = "0.4.5", default-features = false, features = ["iterator"] }` resolves to **0.4.6**. Only its iterator/channel features are enabled; extended signal information and the C compiler build feature are not enabled. Local exact-version source and manifests were inspected.

| New locked package | Version | Declared minimum Rust | Used on Linux |
| --- | --- | --- | --- |
| signal-hook | 0.4.6 | 1.66 | Yes |
| signal-hook-registry | 1.4.8 | 1.26 | Yes |
| libc | 0.2.191 | 1.65 | Yes |
| errno | 0.3.15 | 1.56 | Yes |
| windows-sys | 0.61.2 | 1.71 | No; target-specific transitive entry |
| windows-link | 0.2.1 | 1.71 | No; target-specific transitive entry |

All six declare `MIT OR Apache-2.0` and include both license texts. All 11 pre-existing external lockfile records remain unchanged. The Rust 1.85.0 builds/tests establish actual Linux compatibility beyond these declared minimums. No Python runtime, logging framework, networking stack, async runtime or other direct dependency was introduced. This is a source/license/feature review, not a comprehensive upstream security audit.

## 3. Initial implementation verification

The results in this section are the original implementation checks. The subsequent acceptance review reruns deterministic verification independently and records it in the [readiness report](008-publication-readiness.md).

All required local checks passed on **Rust 1.85.0 and installed stable Rust 1.100.0**. Logs were retained outside the repository in `/tmp/siliconwatch008-checks/`; no hosted CI is claimed for uncommitted changes.

The following commands were executed for each `TOOLCHAIN` value, `1.85.0` and `stable`:

```bash
cargo +"$TOOLCHAIN" fmt --check
cargo +"$TOOLCHAIN" clippy --locked --offline -- -D warnings
cargo +"$TOOLCHAIN" test --locked --offline
```

| Check | Actual result |
| --- | --- |
| Formatting, both toolchains | Exit 0 |
| Configured/default Clippy, both toolchains | Exit 0, warnings denied |
| Rust tests, each toolchain | **105 executable tests + 2 compile-fail doctests passed; 0 failed, 0 ignored** |
| Rust 1.85.0 release build | `cargo +1.85.0 build --release --locked --offline --target-dir target/provider-latency`: exit 0 |
| Rust 1.100.0 release build | `cargo +stable build --release --locked --offline`: exit 0 |
| Existing CLI and sampling regressions | All included tests passed on both toolchains; schemas and legacy behavior unchanged |
| Strict Python harness suites | **153 passed; 0 failed, 0 skipped**, against the final Rust 1.85.0 release fixture binary |
| DCO checker tests | **6 passed; 0 failed** |
| Diff/document/manifest checks | Whitespace, relative links, expected file set and empty index verified |

Python execution used `python3 -B /tmp/siliconwatch008-checks/strict-python.py`, an exact extraction of the unchanged [workflow's strict Python stanza](../../.github/workflows/alpha1.yml). It enforces discovery, required fixture tests and failure on any skip. Reproduction should build `target/provider-latency/release/siliconwatch` with Rust 1.85.0 and run that workflow stanza. DCO checker execution was:

```bash
python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v
```

These were fixture tests, not benchmark harness measurement modes. No historical matrix, extended stress test, or outlier investigation ran. Existing permission tests ran without conditional skips. No Windows or other architecture build was tested.

### Coverage and corrected review findings

Tests cover startup acknowledgement before admission, cancellation before readiness, normal/spurious/delayed timing, checked arithmetic, no additional admission after stop, explicit acquisition failure/recovery, raw/identity preservation, freshness/lifecycle classification, held-reader publication delay, late publication while stopping, idle condition-variable wakeup, startup/output/fatal errors, poisoning, worker panic and coordinator wait panic. Actual signal handling is exercised only in isolated fixture child processes. Repeated SIGINT/SIGTERM are tested while a FIFO fixture explicitly holds an admitted read open; children and fixture helpers have bounded readiness waits and kill/reap cleanup on test failure. No signal targets the developer shell or unrelated processes.

The 10,000-attempt simulated loop verifies attempts, scheduling and constant lifecycle output; it does not measure RSS. Payload retention relies on the unchanged cache and its existing retention tests.

Review identified that signal-hook can unwind on an unexpected iterator-wait I/O error. A narrow unwind guard now converts that into the normal stop/close/join failure path, with regression coverage. Supplemental linting also caught a duplicate fixture-module import introduced by the new tests; a local fixture helper corrected it. Formatting and complete Rust tests were rerun on both toolchains after that test-only correction.

### Separate pre-existing supplemental Clippy failures

```bash
cargo +1.85.0 clippy --locked --offline --all-targets -- -D warnings
```

The final supplemental run exits **101**, with exactly the two previously documented `clippy::non_octal_unix_permissions` errors at unchanged `tests/hwmon.rs:217` and `tests/identity.rs:368`, both using `from_mode(0)`. The new duplicate-module finding is gone. No unrelated test edit or lint suppression was made. This remains separate from the passing configured/default Clippy checks; no blanket all-target Clippy success is claimed.

## 4. Hardware validation

During implementation, a short, read-only functional smoke used the final Rust 1.85.0 release binary as a **non-root user** on **CachyOS, Linux 7.3.4-1-cachyos, x86-64**. One `sensors --json` scan succeeded. Then `daemon --interval-ms 1000 --stale-after-ms 3000` ran briefly after startup acknowledgement and received SIGTERM directed only at that test process. It exited 0, emitted two lifecycle lines and zero stdout bytes, and reported the retained observation historical/non-current after joining. No forced smoke cleanup was needed.

| Provider | Inputs in the independent one-shot scan |
| --- | ---: |
| acpitz | 1 |
| nvme | 3 |
| amdgpu | 5 |
| k10temp | 1 |
| iwlwifi_1 | 1 |
| Total | **11** |

Seven temperatures and two voltages had status `ok`; `freq1_input` and `power1_input` remained explicitly unsupported. No fan readings were exposed in this observation. There were no snapshot warnings or permission-denied readings. Examples: acpitz raw `20000` → `20.0 °C`, NVMe raw `42850` → `42.85 °C`. These are unvalidated hardware reports, not proof of sensor accuracy.

The daemon does not expose an attempt count; none was inferred from elapsed time, requested interval or exit status. This short smoke does not establish sustained-operation reliability, timing budgets, shutdown deadlines, provider completeness or physical sensor identity stability. No missing-provider investigation, driver change, performance setting change or sensor-value correction was performed. Detailed local smoke output remains outside Git; only sanitized observations are recorded here.

## 5. Architecture review and acceptance criteria

The implemented design follows the approved proposal without changing sensor/cache schemas or scheduling semantics of existing commands. Lifecycle and health remain separate; standard-library coordination and one signal dependency were sufficient. No general provider abstraction was needed.

**All scoped implementation/local-verification acceptance criteria are satisfied**, subject to the explicit cooperative-shutdown and cache limitations below and the separate pre-existing supplemental lint failures. Chris formally accepted the milestone with these limitations on November 17, 2026; acceptance does not authorize deployment or further development. The installed application remains Rust-only; Python is development/test tooling. Celsius remains the future default display unit, with Fahrenheit a presentation preference outside acquisition/cache state. Decision 43's permissions policy remains **Pending**.

## 6. Known limitations and risks

- No immediate read cancellation or hard shutdown deadline. An admitted attempt may start/publish after stop acknowledgement. Blocking driver reads, held cache guards or output can delay progress/join indefinitely; repeated signals do not force termination.
- Retention is bounded by payload generations and scalar control state, not bytes/RSS. Provider counts and payload size vary. No daemon CPU/memory performance guarantees or long-duration validation are established.
- Freshness describes userspace observations, not physical sensor age. Automatic suspend/resume detection remains unimplemented; recreate the cache after known suspend under its existing contract.
- Signal handling is process-global and intended for this command's lifetime. Unregistration does not guarantee restored OS dispositions. SIGKILL, abort, allocation failure and loss of all reporting channels have no graceful-cleanup guarantee.
- No external read endpoint or live health dashboard exists. The cache remains internal; absence of per-scan logging avoids unbounded output but is not a substitute for future health reporting.
- Two baseline supplemental Clippy errors remain. No other confirmed correctness defect remains from this review; untested platform/performance scenarios are limitations rather than demonstrated bugs.
- No API, GUI, database/history, systemd installation, deployment, adaptive polling, alarms, notifications, calibration, hardware controls or protective actions were added.

## 7. Git state and preservation

Work is local on `codex/alpha1-prototype008`, still based at **`28d7177cc334e357646023e5a2292b0cc434e0a7`**. The acceptance/readiness update leaves five modified tracked files and six untracked files in the revised eleven-file manifest. The index is empty, there is no upstream, and no new commit, push, PR or merge was made. Future authorized contribution commits require DCO signoff.

The original checkout and its intentional modifications, and the clean Prototype 007 worktree, were preserved. The original 78-file and Prototype 007 75-file baselines match. The Prototype 006 report's protected hash remains unchanged. Historical archives, recorded source/capture hashes and evidence exclusions were preserved; no benchmark evidence was regenerated or edited. The proposal SHA-256 remains **`c37dfcd9b076a07b03150503dd01932d4030223e75fa0e2c6c08830dd843135c`**. Core scanner, identity, sampler/cache implementations and their existing tests, CI workflow and Python harnesses match the base. Preserve the older Prototype 004 audit limitation: recorded raw-capture size checks cannot prove historical byte identity without capture hashes.

## 8. Recommended next action and milestone

Chris has accepted Prototype 008. Review the publication-readiness report and obtain separate publication authorization before staging, committing, pushing or creating a PR. Do not automatically publish it.

The next planning candidate is **Prototype 009: Versioned Local Read-Only API**, scoped to consuming the existing shared observation and lifecycle state. Before implementation, review the local permissions decision, schema/currentness/error contract and deterministic access/serialization tests. Acceptance criteria should require no second acquisition owner, no stale/failure masking, preserved raw observations/identities and bounded reader access. Decision 43 is a dependency requiring explicit resolution, not a policy selected here. No Prototype 009 proposal file or implementation was started.

## Revision history

| Date | Update |
| --- | --- |
| 2026-11-17 | Implemented the authorized cooperative foreground lifecycle; recorded local verification, brief hardware smoke, preserved evidence and outstanding limits. Awaiting Chris's review. |
| 2026-11-17 | Recorded Chris's subsequent technical acceptance with all documented limitations; added a separate publication-readiness review and revised eleven-file manifest. No publication authorized. |

## Implementation handoff — November 17, 2026 (before acceptance)

Chris authorized local implementation of SiliconWatch Alpha 1 Prototype 008, using the approved cooperative-shutdown proposal. **Implementation and local verification are complete; formal acceptance/publication await Chris's review.** No commits, pushes, PRs, merges or Prototype 009 work occurred.

The new Rust `daemon` command requires `--interval-ms` (100–60,000) and positive `--stale-after-ms`, with optional fixture root. One dedicated worker owns the existing cache. Main coordinates SIGINT/SIGTERM; startup, running, stopping and stopped are distinct from acquisition health. Polling uses checked monotonic dispatch timing, wakeable waits and sequential acquisition without catch-up queues. Recoverable failures publish explicit failed attempts; prior success remains historical, and later success recovers normally.

Shutdown closes admission and wakes an idle worker. An already admitted acquisition may finish and publish after stopping starts, but service assessment reports data non-current throughout stopping/stopped. Owner cleanup and join precede final reporting. Fatal cache/control/worker errors and required-output failures return nonzero. Review also closed an iterator-wait panic path that could otherwise detach the worker. Existing scanner, identities, cache, sampling and JSON contracts remain unchanged.

On **Rust 1.85.0 and 1.100.0**, formatting, configured Clippy, release builds, **105 executable Rust tests and two doctests per toolchain** passed with no failures or ignored tests. Strict Python tests passed **153/153 without skips**; DCO-checker tests passed **6/6**. Real signals were tested in isolated fixture children, including repeated requests during a blocked FIFO read. Supplemental all-target Clippy still reports only the two pre-existing permission-literal errors; these remain separate and untouched.

A brief non-root hardware smoke on CachyOS x86-64 found **11 inputs: seven temperatures, two voltages and two explicitly unsupported inputs**, across five providers. The foreground daemon exited cleanly after SIGTERM with historical/non-current final data and no stdout telemetry. No attempt count was inferred, and no benchmark matrix, stress test or outlier investigation ran.

The only new direct dependency is `signal-hook`, resolved to **0.4.6** with iterator/channel features. Six lockfile additions include two Windows-only transitive entries; all prior packages remain unchanged. The ten-path manifest comprises Cargo files, README, library/CLI wiring, daemon implementation/tests, the preserved proposal and this report.

Branch **`codex/alpha1-prototype008`** remains at **`28d7177cc334e357646023e5a2292b0cc434e0a7`**, with five modified tracked and five untracked files, an empty index and no upstream. Both earlier worktrees, the proposal hash and historical evidence remain preserved. Blocking reads, cache guards or output can delay shutdown indefinitely; no production readiness or physical-age guarantee is claimed. Decision 43 remains Pending. Recommended next action: Chris's implementation review, followed only by separately authorized acceptance/publication.

## Acceptance verification update — November 17, 2026

Following Chris's formal acceptance, a fresh deterministic review passed formatting, configured Clippy, release builds, 105 executable Rust tests and two doctests on each of Rust 1.85.0 and 1.100.0. The strict Python suite passed 153 tests without skips and DCO-checker tests passed six. Isolated-process SIGINT/SIGTERM fixtures passed on both toolchains. Only the two existing supplemental all-target Clippy errors remain. No application behavior was modified and no new hardware measurements were performed.

The [publication-readiness report](008-publication-readiness.md) contains the exact eleven-file manifest, versions/commands, dependency audit, DCO readiness, preservation findings and remaining limits. The source, tests, Cargo configuration, README and approved proposal remain identical to the accepted implementation. Only this acceptance update and the new readiness report change during this review. Publication remains subject to Chris's separate authorization.

## HANDOFF TO CHATGPT

Chris formally accepted **SiliconWatch Alpha 1, Prototype 008: Headless Daemon Lifecycle** on **November 17, 2026**, with all documented limitations. Acceptance covers the implemented Rust foreground daemon and cooperative shutdown. It does not establish production readiness or authorize deployment, API/GUI development, hardware controls or Prototype 009.

Acceptance documentation and publication-readiness review are complete. The milestone report now records the acceptance and preserves the original implementation evidence/handoff; the approved proposal remains byte-for-byte unchanged. A separate readiness report adds one documentation file to the original ten-file implementation manifest. **No unresolved local technical publication blocker was found, and no accepted application behavior was changed.**

The exact intended publication manifest is:

1. `Cargo.toml`
2. `Cargo.lock`
3. `README.md`
4. `src/lib.rs`
5. `src/main.rs`
6. `src/daemon.rs`
7. `src/daemon/tests.rs`
8. `tests/daemon_cli.rs`
9. `docs/milestones/008-headless-daemon-lifecycle-proposal.md`
10. `docs/milestones/008-headless-daemon-lifecycle.md`
11. `docs/milestones/008-publication-readiness.md`

Fresh checks passed on **Rust 1.85.0 and installed Rust 1.100.0**: formatting, configured Clippy, release builds, and **105 executable Rust tests plus two doctests per toolchain**, with zero failures, ignored tests or conditional skips. Actual SIGINT/SIGTERM and repeated-signal behavior passed in isolated fixture processes. The strict Python suite passed **153 tests without skips**; all **six DCO-checker tests** passed. Supplemental all-target Clippy still fails only on the two pre-existing permission-literal findings; neither was changed or suppressed. No new hardware measurements, benchmark matrices or historical-outlier investigation occurred.

The dependency audit confirmed locked `signal-hook 0.4.6`, iterator/channel-only features, declared licenses/MSRVs, locked archive checksums and unchanged prior packages. Configured author/committer identity matches existing published DCO attribution. No new commit exists, so no new contribution signoff or hosted CI result is claimed.

Branch **`codex/alpha1-prototype008`** remains at **`28d7177cc334e357646023e5a2292b0cc434e0a7`**, with five modified tracked and six untracked files, an empty index and no upstream. All eleven intended files are the only worktree changes. Both earlier worktrees, agent-skills setup, the proposal hash and historical evidence remain preserved; private captures and generated outputs remain excluded.

Cooperative shutdown can wait indefinitely for blocked hardware reads, cache guards or output. An admitted scan may publish after stopping begins, while service data remains non-current. Physical sensor age, suspend-aware freshness and production performance are not guaranteed. Decision 43 remains Pending. Recommended next action: Chris's separate authorization to publish the exact eleven-file manifest through a DCO-signed commit, normal push and PR with hosted verification. Nothing has been staged, committed, pushed or submitted; stop for publication approval.
