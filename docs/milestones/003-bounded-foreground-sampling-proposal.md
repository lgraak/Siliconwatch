# Prototype 003 proposal — Bounded Foreground Sensor Sampling

> **Implementation authorization update — 2026-11-16:** Chris approved this scope with revised exit semantics: completed sampling returns 0 even after recoverable scan/serialization failures; output failures, including broken pipes, return nonzero. Existing one-shot exit behavior is preserved. These instructions supersede the historical pending status and conflicting exit rules below. See the [implementation report](003-bounded-foreground-sampling.md) for current behavior and verification; the original proposal remains preserved as planning history.

**Date:** 2026-11-16

**Status:** PROPOSAL — awaiting Chris's approval; no Prototype 003 implementation is authorized or started.

**Planning milestone:** Complete. PR #1 is merged; the specification below is proposed behavior, not an implemented capability.

## 1. Milestone summary and foundation merge

Chris authorized a final verification and regular merge of [PR #1](https://github.com/lgraak/Siliconwatch/pull/1), followed by a local planning branch and this proposal. The foundation is now on `main`. This does not establish production monitoring, validated physical sensor identity, or protective-action readiness.

| Item | Verified commit |
| --- | --- |
| Main before merge | `a14d133072bbdb48f897afea266a9f3cd8d6e0f2` |
| Accepted architectural decisions | `61043906faddb1a05e8182ebf41eb2b008c8ce2d` |
| Prototype 001: read-only hwmon discovery | `bed3f17d7fb03ce3e500ecf5874c257a96241815` |
| Prototype 002: best-effort source identity | `ee12c50c56221de4e8cd252478d7f794c33a7d8b` |
| Alpha 1 CI; reviewed PR HEAD | `86b4e2d907eb8f8ceb86266d155859d4c552af40` |
| Final regular merge commit | `b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d` |

GitHub recorded the merge at **2026-11-16 20:27:08 UTC**. The merge's parents are the former main and reviewed PR HEAD, in that order. Its tree equals the reviewed feature tree. All four contribution commits remain unchanged, retain DCO signoffs, and are reachable from `origin/main`; the merge message also contains a DCO signoff. Neither squash nor rebase was used. The Prototype 002 branch remains on GitHub and locally.

## 2. Implementation details — proposed, not implemented

**Objective:** add finite, opt-in foreground sampling to the existing Rust CLI, with explicit observation timing and failure information, without changing the one-shot reader or introducing a service.

The existing [reader](../../src/hwmon.rs) already returns a new `Snapshot` for each `hwmon::scan(&Path)` call. The [CLI](../../src/main.rs) handles output and process exit status. The [identity module](../../src/identity.rs) derives candidates and checks collisions within each snapshot. Reuse these boundaries; do not retain previous sensor values or create a provider framework.

The only file created in this planning task is this proposal. No source, tests, dependency files, workflow, README, accepted design decisions, or existing milestone reports were changed. No dependencies were added. If approved, a small sampling-loop module, CLI integration, and isolated tests should suffice, using the standard library and existing `serde`/`serde_json`. Keep timing/scan/output injection local to the loop for tests; no async runtime, worker pool, general scheduler, or public clock framework is proposed.

### Proposed CLI contract

Existing commands and their output/exit behavior remain unchanged:

```bash
cargo run -- sensors
cargo run -- sensors --json
```

New commands would be explicitly bounded:

```bash
cargo run -- sample --count 5 --interval-ms 1000
cargo run -- sample --count 5 --interval-ms 1000 --json-lines
cargo run -- sample --count 3 --interval-ms 100 --hwmon-root /path/to/fixture
```

- `sample` requires both `--count N` and `--interval-ms M`. Count means scan **attempts**, including failures. There is no unlimited mode or hidden retry.
- Proposed initial limits are **1–10,000 attempts** and **100–60,000 milliseconds**. These are conservative CLI guardrails for review, not measured safe hardware rates or performance budgets. Chris may revise them before implementation.
- Accept decimal integers only. Reject missing, negative, zero, fractional, duplicated, out-of-range, or overflowing numeric options before accessing hardware. Use checked conversions and deadline arithmetic.
- `--json-lines` explicitly selects streaming output. Do not repurpose `sensors --json` or add multi-object output to it. `sample --json` is a usage error; help explains the stream flag.
- Preserve `--hwmon-root PATH` for read-only fixtures. Print help/version without scanning. Invalid arguments use exit 2.
- Human output appends a numbered sample header, timestamps, scan duration/lateness, observation-age information, and the existing table or attempt error. No terminal redraw or interactive UI. Retain escaping of untrusted labels and diagnostics.

### Sequential sampling contract

1. Establish a process-local monotonic origin; attempt 1 is eligible immediately after argument validation. Record the actual start just before invoking the scanner.
2. For attempt `i > 1`, the target is `actual_start(i-1) + interval`. Wait until that target if early, rechecking the clock after waking.
3. Only after the preceding scan and record write/flush finish may the next attempt start. If late, start one attempt and record the positive lateness; anchor the following target to this new actual start. Do not run catch-up bursts or count missed nominal intervals as extra attempts.
4. Each attempt rediscovers devices and rereads values/identity evidence through the unchanged reader. Scans never overlap. Newly appearing devices can appear; removed devices disappear. No cross-attempt identity reconciliation or tombstones are promised.
5. Serialize, emit, and flush one record, then release it and the snapshot. Keep only counters and timing scalars between attempts. Do not sleep after the final attempt.

For example, with a 1,000 ms interval, a first scan plus output taking 1,400 ms makes the next attempt start at approximately 1,400 ms, 400 ms late. Its following target is approximately 2,400 ms. The loop does not attempt to restore the original time grid.

### Timing, timestamps, and freshness

Use `std::time::Instant` and checked durations for scheduling; wall-clock changes must never determine waits or elapsed durations. Expose integer nanosecond offsets from the local origin for target start, actual start, scan finish, and record preparation; derive scan duration and start lateness from those offsets. Offsets are process-local, not comparable across invocations. Arithmetic overflow or a detected backward monotonic observation is an explicit fatal timing error, never a wrapped or silently zeroed interval.

Separately capture a wall-clock Unix-millisecond timestamp at scan start for human correlation. If the timestamp cannot be represented (for example, a clock before the Unix epoch), emit null plus a diagnostic; do not discard usable telemetry or alter the schedule. Clock adjustments can make successive wall timestamps decrease. The standard-library [Instant](https://doc.rust-lang.org/std/time/struct.Instant.html) and [SystemTime](https://doc.rust-lang.org/std/time/struct.SystemTime.html) documentation distinguishes these clock roles; suspend accounting is not a portable `Instant` guarantee.

**Freshness describes userspace observation, not physical sensor acquisition or accuracy.** A snapshot brackets sequential reads, not an atomic hardware sample. Drivers may cache values, and the prototype cannot determine their physical age.

- For an emitted snapshot with sensor rows, report an observation window from scan start to scan finish and lower/upper bounds on observation age at record preparation: `prepared - finished` and `prepared - started`. The window also includes discovery and metadata work; these are not measured per-sensor ages.
- Include an explicit basis such as `current_attempt_observation_window`, and `physical_age_known: false`. These bounds describe this scan's returned observations, including any questionable/raw readings; existing row status still determines whether a value was usable.
- Flag `observation_window_exceeds_interval` when the upper observation-age bound is greater than the configured interval. This conservative advisory can flag a scan whose actual input reads are newer; it is not a safety threshold or an assertion that any sensor is physically stale.
- For an attempt with no snapshot, expose no value-freshness bounds and use basis `no_snapshot`. Never substitute a prior successful value, identity, or timestamp.
- Empty snapshots retain scan timing but have basis `no_sensor_observations`, null age bounds, and no interval-freshness verdict. A failed row remains a failure regardless of the scan's observation window.
- Record preparation is before serialization/output. Transport delay can make a received record older; these fields do not promise freshness at receipt. Slow output is reflected in the next attempt's lateness. Output is backpressured, not queued.

### Failed, partial, and delayed scans

| Condition | Proposed behavior |
| --- | --- |
| Root-level scan error | Emit one attempt error with timing and no snapshot, consume the attempt, continue within the finite count; overall exit 1 even if later attempts recover |
| Per-sensor failure, unsupported channel, permission denial, or discovery warning | Emit the new snapshot with existing statuses, warnings, raw data, and identity evidence unchanged; these conditions alone retain existing exit-0 behavior |
| Empty readable root | Emit a valid empty snapshot; do not invent unavailable sensors from previous attempts |
| Scan or output exceeds interval | Report timing/lateness; remain serial, without hidden retries or catch-up loops |
| Snapshot cannot serialize, including a non-UTF-8 path | Emit a buffered serialization-error envelope for that attempt, no stale snapshot; continue count and return exit 1 overall |
| Timing arithmetic/clock invariant failure | Stop with a clear error and exit 1; do not emit fabricated timing as valid |
| Output/flush failure | Stop further scanning; return exit 1, except retain the existing deliberate broken-pipe exit-0 convention |
| User interruption | Normal OS termination behavior; no new signal handler or guaranteed terminal summary |

In the ordinary path, emit exactly one record per attempted scan. Fatal timing/output failure or external interruption can end the stream early; a truncated stream is not proof that all requested attempts completed. Error messages remain escaped in tables and encoded in JSON. A partially written transport record cannot be undone; buffer serialization first so *serialization* errors cannot leave partial JSON fragments.

### Experimental stream format

Use compact JSON Lines, one independently parseable object and newline per completed attempt. Keep stdout exclusively for records. There is no accumulated array, persistent history, or final JSON summary. A simple proposed envelope contains:

| Field | Meaning |
| --- | --- |
| `record_type` | `siliconwatch.sample` |
| `sampling_schema_version` | `1`, separate from existing nested schemas |
| `sequence`, `requested_count`, `interval_ms` | One-based attempted scan number and configured bounds |
| `timing` | Target/start/finish/preparation offsets, scan duration, lateness, nullable wall start timestamp and any wall-time diagnostic |
| `freshness` | Observation basis, nullable lower/upper observation-age bounds and interval comparison, and unknown physical-age indicator |
| `snapshot` | The unchanged experimental snapshot schema 2, including identity schema 1, or null |
| `error` | Attempt-level kind/message, or null; exactly one of snapshot and error is non-null |

Write complete error envelopes through the same output path. A fatal loop error can be reported on stderr and terminate with exit 1 without adding a fake attempt record. Snapshot warnings remain inside the nested snapshot. Identity and path metadata remain local diagnostic information, not a redacted export. Neither the envelope nor existing JSON constitutes the future local API.

## 3. Actual verification results

Final pre-merge fetch confirmed the exact four-commit history and unchanged main. GitHub reported PR #1 open, non-draft, base `main`, expected HEAD `86b4e2d907eb8f8ceb86266d155859d4c552af40`, `MERGEABLE`, and `CLEAN`. No unexpected commit, file, local modification, or divergence was found. `gh pr checks --required` reported no administratively required checks; both configured Actions jobs nevertheless passed. No protection was bypassed or changed.

[Alpha 1 CI run #1](https://github.com/lgraak/Siliconwatch/actions/runs/43948378451) is associated with that exact HEAD:

| Check | Observed result |
| --- | --- |
| Rust 1.85.0 `cargo fmt --check` | Passed |
| Rust 1.85.0 `cargo clippy --locked -- -D warnings` | Passed |
| Rust 1.85.0 `cargo test --locked` | 31 passed, 0 failed, 0 ignored; no conditional permission skips |
| Python DCO checker tests | 6 passed, 0 failed |
| DCO contribution check | All four contribution commits passed; rerun locally during merge preflight |
| Post-merge graph/tree verification | Expected two parents, all four signed-off ancestors reachable, reviewed tree unchanged, feature branch retained |

These are observed CI results, not newly executed local Cargo checks. No Rust files changed, so Cargo and live commands were not rerun for this merge/planning task. The workflow is PR-only; no new post-merge CI run is claimed. The proposed sampling implementation and every proposed test below remain unimplemented and unverified.

Proposal validation passed Markdown/relative-link review, whitespace checks (including the untracked file), and confirmation that this is the only working-tree addition. A separate read-only review found no blocking contract contradictions or scope expansion. No application code, accepted architecture text, workflow, or dependency changes are part of this proposal.

## 4. Hardware validation

No new hardware test occurred during merge/planning. [Prototype 002's report](002-stable-sensor-identity.md) records prior read-only CachyOS/AMD validation: 24 inputs, 22 successful readings with best-effort identities, and two unsupported GPU inputs. Those observations are historical, not verification of sampling, freshness, or performance. Physical sensor accuracy, reboot/re-enumeration behavior, other hardware, and CPU/memory/frame-time budgets remain unverified.

If implementation is approved, perform a short finite ordinary-user hardware run in both table and JSON Lines modes, report actual scan duration/lateness and errors, and avoid publishing sensitive paths or identifiers. Never write to real sysfs.

## 5. Architecture and design review

This continues the explicitly approved incremental CLI sequencing in [Prototype 001](001-hwmon-prototype.md) and [Prototype 002](002-stable-sensor-identity.md). It does not replace the eventual daemon-owned acquisition/cache and local API architecture. Read-only Rust/Linux monitoring, explicit uncertainty, and existing identity ambiguity semantics remain intact.

Relevant [design decisions](../DESIGN_DECISIONS.md): 2 (questionable readings), 4 (eventual adaptive polling), 10 (identity), 15/19 (least privilege/read-only boundary), 23 (performance gates), 24 (versioned experimental interfaces), 33 (resume freshness), and 41 (DCO). No accepted decision is amended. The proposed observation-age flag is not protective validation. Fixed foreground intervals do not complete Decision 4; timing observations are not Decision 23 benchmarks; rediscovery without reused values does not complete suspend/resume handling under Decision 33. **Decision 43 remains PENDING.**

**Excluded:** background daemon/systemd service; persistent sensor cache; database or historical storage; D-Bus/network API; GUI; alarm engine; fan/pump control; protective shutdowns; adaptive polling. No persistent identity registry, cross-attempt reconciliation, plugin work, or permissions policy is needed for this proposal.

## 6. Known issues, risks, and limits

- **Confirmed existing constraints:** sysfs reads and stdout can block; snapshots are sequential; driver cache age is unknown; JSON source paths can be sensitive. No new application bug was identified by this planning review.
- **Execution bound:** finite attempt count and validated interval arithmetic bound scheduled work. They cannot guarantee a hard wall-clock completion deadline while a driver or output write is blocked. Do not add abandoned read threads, watchdogs, or a process supervisor to imply such a guarantee.
- **Memory bound:** retain at most the current scan, its rendering/serialization buffer, and constant counters. Memory is independent of attempt count but depends on a scan's sensors, strings, warnings, and the existing scanner. There is no new hard byte limit against arbitrarily large alternate filesystem fixtures.
- **Suspend and clock limits:** monotonic suspend accounting varies; do not infer suspend solely from wall/monotonic disagreement. Every attempt rediscovers and has no carried-forward values, but reliable suspend detection/elapsed-time freshness is deferred. Use after resume is not protection-ready.
- **Performance assumptions:** the proposed 100 ms minimum is a reviewable guardrail, not a proven safe cadence. Repeated full discovery/identity resolution can be expensive. Measure first; do not add a cache as an unapproved optimization.
- **Output limits:** backpressure delays sampling; the final record cannot report time spent delivering itself. Buffered JSON prevents serialization fragments but cannot guarantee an atomic write to a failing pipe.
- **Identity limits:** best-effort candidates retain Prototype 002's ambiguity and replacement/topology limitations. No prior row is automatically matched, merged, or restored after a failure.

## 7. Git status

- PR #1 is merged with the regular merge commit recorded above; no force-push, squash, rebase, branch deletion, or protection change occurred.
- `codex/alpha1-prototype003` was created locally with no upstream from the updated `origin/main`, at `b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d`.
- The only pending change is this new, untracked proposal. Nothing is staged. No Prototype 003 commit, push, or PR was created. Existing tracked files are unchanged.
- Later implementation and any proposal commit/publication require separate authorization and the project's DCO policy.

## 8. Recommended next milestone and acceptance criteria

Recommend implementing **Alpha 1, Prototype 003: Bounded Foreground Sensor Sampling** only after Chris approves or revises this contract. No prerequisite service, cache, database, or permission decision is needed. Approval should specifically settle the new command/stream flag, proposed numeric limits, start-to-start interval policy, observation-age terminology, and aggregate failure exit behavior.

Acceptance criteria and required tests:

1. **Compatibility:** all existing 31 Rust tests remain valid; unchanged `sensors` table, JSON schema 2, identity schema 1, error semantics, and raw/source metadata. Add checks that sampling options cannot accidentally change one-shot output.
2. **Argument bounds:** tests cover both required arguments, endpoints, missing/duplicate/zero/negative/fractional/oversized numbers and conversion/deadline overflow; invalid invocations do not call the scanner.
3. **Finite scheduling:** controlled clock/sleeper proves immediate first eligibility, exactly N attempts including failures, no final sleep, and no overlapping scan/output. Cover early wakeups, oversleep, scan overrun, output delay, and reanchoring without catch-up bursts.
4. **Clock semantics:** injected monotonic and wall clocks prove wall-clock jumps never affect waits; null wall time preserves telemetry; backward monotonic observations/overflow fail explicitly. Test timing arithmetic and observation-age bounds without real sleeping.
5. **Freshness and recovery:** deterministic fixtures change/remove inputs and make the root disappear/reappear between attempts. Failures contain no previous values, recoveries reflect new data, and absent rows are not invented or silently matched. Preserve malformed/permission/unsupported row statuses and diagnostic precedence.
6. **Streaming:** each emitted line parses independently with one outcome, correct sequence/timing, unchanged nested schemas and identity fields, and no stdout prose. Cover empty snapshots, non-UTF-8 serialization failures, escaping, output/flush errors, broken pipe, and aggregate exit status.
7. **Bounded retention:** a long fake-clock run writes to a discarding sink and proves output is consumed record-by-record without retaining snapshot history; review ownership and loops for memory independent of N. Do not confuse this with a hard allocation limit or a performance benchmark.
8. **Quality and hardware:** run Rust 1.85 formatting, Clippy with warnings denied, all old/new tests, and the existing DCO CI. Perform short ordinary-user table/JSON Lines runs when hardware is available, report actual results and any skips, and confirm no real sysfs writes. No dependency change is expected.

Required tests are hardware-independent and deterministic; fixture mutation happens inside controlled callbacks between attempts. Use injected writers for transport failures and fake time for timing assertions. Avoid sleep-based race tests and broad abstractions.

## Revision history

| Date | Change | Approval state |
| --- | --- | --- |
| 2026-11-16 | Verified and regularly merged PR #1; created local Prototype 003 branch and bounded foreground sampling proposal. | Merge authorized and complete; proposal awaits Chris's review; implementation not authorized |

## HANDOFF TO CHATGPT

Chris authorized merging SiliconWatch PR #1 after exact-head verification, then preparing a Prototype 003 proposal without implementation. The merge and planning task is complete; Prototype 003 remains proposed and unimplemented.

PR #1, https://github.com/lgraak/Siliconwatch/pull/1, merged on 2026-11-16 using regular merge commit **b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d**. Its parents are the previous main and reviewed HEAD **86b4e2d907eb8f8ceb86266d155859d4c552af40**. The merged tree equals the reviewed tree. All four original commits remain reachable from origin/main with their DCO signoffs: architectural decisions **61043906faddb1a05e8182ebf41eb2b008c8ce2d**, Prototype 001 **bed3f17d7fb03ce3e500ecf5874c257a96241815**, Prototype 002 **ee12c50c56221de4e8cd252478d7f794c33a7d8b**, and CI **86b4e2d907eb8f8ceb86266d155859d4c552af40**. The merge also has a DCO signoff. The feature branch was retained; no history rewrite or protection override occurred.

Before merging, GitHub confirmed the expected head/base, MERGEABLE/CLEAN state, and both successful CI jobs for that exact head. Rust 1.85 formatting and Clippy passed; 31 Rust tests passed with zero failures, ignored tests, or conditional skips. Six DCO checker tests passed, and all four contributions passed signoff verification. No administratively required checks were reported, but every configured Actions job passed. These are observed existing CI results; Cargo and hardware tests were not rerun for unchanged application code during this planning task.

The new local branch is **codex/alpha1-prototype003**, based on the verified merge commit, with no upstream. Its only working-tree addition is the untracked **docs/milestones/003-bounded-foreground-sampling-proposal.md**; nothing is staged, committed, or pushed for Prototype 003. Proposal whitespace, links, and contract review passed. Source, tests, dependencies, and existing documents are unchanged.

The proposal introduces a separate sample command requiring a finite count and interval, with an explicit --json-lines stream option. It preserves one-shot sensors commands. Proposed guardrails are 1–10,000 attempts and 100–60,000 ms intervals. Sampling is sequential, scheduled from the previous actual start, and reanchors after delays without catch-up bursts. Monotonic timing controls scheduling; separate wall timestamps support correlation. Freshness means bounded userspace observation age, never physical sensor accuracy or driver-cache age. Failed attempts consume count, never reuse old values, and contribute to a nonzero aggregate exit status; existing per-sensor statuses remain unchanged.

Only one current snapshot/record is retained. Blocking drivers/output prevent hard runtime guarantees, and memory still depends on one scan's size. Suspend accounting, performance budgets, and physical freshness remain limitations. Deterministic fake-clock, fixture, and writer tests are specified but not implemented.

Recommendation: Chris should approve or revise the CLI, numeric guardrails, timing/freshness contract, and exit semantics before authorizing this narrowly scoped implementation. No daemon, cache, storage, API, GUI, adaptive polling, control, alarms, or shutdowns are included. Decision 43 remains pending. No further implementation or publication is authorized; stop for review.
