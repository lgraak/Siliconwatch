# Alpha 1, Prototype 007: Shared In-Memory Sensor Snapshot Cache

**Date:** 2026-11-17 (America/Los_Angeles). **Status: Accepted by Chris. Acceptance date: November 17, 2026.**

## 1. Milestone summary

Chris formally accepts the implemented **library-only snapshot cache** as technically complete, subject to the limitations below. Those limitations remain **accepted, not resolved**. Acceptance does not authorize production deployment, daemon/API/GUI work, hardware controls, publication, or Prototype 008. Decision 43 remains Pending. See the dated [publication-readiness review](007-publication-readiness.md) for the current seven-file manifest and fresh verification.

Implemented the Rust library-only cache authorized after the [proposal](007-shared-snapshot-cache-proposal.md). That proposal is preserved as the historical planning record; its earlier “implementation not authorized” wording predates Chris's implementation authorization. No existing command uses the cache yet.

One acquisition owner publishes whole observations to concurrent read-only consumers. Startup, empty inventories, row errors, whole-scan failure, historical data, recovery, stale observations and stopped ownership remain distinguishable. This milestone does not establish production monitoring or protective-action readiness.

## 2. Implementation details

| File | Purpose |
| --- | --- |
| [src/snapshot_cache.rs](../../src/snapshot_cache.rs) | New cache, owner/reader interfaces, timing, publication and lifecycle handling. |
| [src/snapshot_cache/tests.rs](../../src/snapshot_cache/tests.rs) | Controlled-clock, synchronization, lifecycle, retention and poison tests. |
| [tests/snapshot_cache.rs](../../tests/snapshot_cache.rs) | Filesystem fixtures exercising the unchanged reader through the public cache API. |
| [src/lib.rs](../../src/lib.rs) | Exports the new module. |
| This report | Implementation, verification and review handoff. |

The previously prepared proposal remains an untracked planning artifact; it was not rewritten. No dependencies, Cargo configuration, workflows, existing tests or other production modules changed. No general provider interface was introduced.

### Public contract

`snapshot_cache::new(root, stale_after)` requires a positive age threshold and returns one non-cloneable `CacheOwner` and a cloneable `CacheReader`. Construction does not scan or validate the root. Root and threshold remain fixed for the cache's lifetime.

- `CacheOwner::acquire_once(&mut self)` synchronously calls unchanged `hwmon::scan()`. `Acquisition::Succeeded(sequence)` and `Acquisition::Failed(sequence)` distinguish the published outcome. **`Ok(Acquisition::Failed(_))` means a scan failure was published, not that acquisition succeeded.** A fatal control/publication error returns `Err(CacheError)` and stops the owner.
- `CacheReader::read()` returns a borrowed `ReadView`; `assess(&mut self)` returns immutable references together with latest-attempt, producer, observation-role and age information. It never scans. No mutable published guard or retainable snapshot `Arc` is exposed.
- `CacheOwner::stop()` and owner drop are terminal. They invalidate currentness without acquiring the state lock. There is no cache-owned thread, scheduler, process, file or cancellation service.

Private `Arc<Shared>` contains `RwLock<PublishedState>`, an `Instant` origin, immutable threshold and an atomic producer-live flag. `PublishedState` is empty, successful, or failed with at most one preceding observation. Owner methods require exclusive mutable access. Multiple independent caches/processes are not prohibited.

Acquisition, error formatting and filesystem access happen outside the publication lock. The writer records publication time and swaps complete state under the write guard, then drops displaced allocations after releasing it. Readers observe the latest **published** attempt: an in-flight scan, including a completed scan waiting for the lock, has not yet replaced the preceding state.

### Failure, time and lifecycle

| Event | Result |
| --- | --- |
| Before first publication | Explicit empty state, no observation. |
| `Ok(Snapshot)`, including no sensors or all failed rows | Exact snapshot replaces preceding data; warnings and row diagnostics remain authoritative. Success is a scan result, not a sensor-health assertion. |
| Whole-scan I/O error | Latest failed attempt contains error kind, optional OS error and message. Any preceding success is explicitly historical, even if its numerical age is small. |
| Repeated failure | Replace error/timing metadata, retain only the same preceding observation with its original timestamps. |
| Recovery | Replace failure and history with the complete new snapshot; missing rows remain missing. |
| Stop, owner drop or fatal acquisition control error | Retained observations become historical after stop; further acquisition is rejected. |
| Poisoned state | Return `CacheError::Poisoned`; never silently recover the poisoned guard or present its data as healthy. |

Attempt metadata records checked sequence, cache-relative `Duration` start/finish/publication offsets, optional Unix wall-start milliseconds and a wall-time error when unavailable. `started <= finished <= published`; new starts cannot precede previous publication. Wall-clock jumps affect correlation only. Durations avoid narrowing monotonic time into a serialized integer; there is no new wire schema.

Every assessment checks monotonic ordering against publication and the same view's preceding assessment. For nonempty observations, userspace age bounds are `[now - finished, now - started]`. The observation is stale when the upper bound exceeds the caller's threshold; equality is within budget. Empty successful scans have timing but `NoSensorObservations`. Historical ages use the original successful attempt, never failure timing. Historical role, stopped lifecycle and row errors cannot be overridden by a young age bound.

Physical sensor age is always unknown. Assessments describe one instant, not a continuing validity guarantee. `Instant` does not establish suspend-aware elapsed time; automatic resume detection is absent. A caller aware of suspend must stop/discard the old cache and create a new one before treating later data as current.

### Locking and retention

Readers must release views before acquisition, recursive reads, blocking I/O, long processing or waiting on the owner. Held guards can indefinitely delay or starve publication; the standard lock provides no promised fairness. Stop/drop can still be observed through an already-held view because liveness uses a separate atomic flag. Stop is not concurrent cancellation of a blocking scan.

At most one observation payload is retained plus one incoming/replacing payload. Repeated attempts do not accumulate history, and borrowed readers cannot retain prior shared generations. This is a **retention bound, not a fixed byte/RSS ceiling**: inventories, diagnostics, strings, scanner temporaries and caller-owned copies vary in size. There is no truncation or new arbitrary sensor limit.

## 3. Verification results

| Actual command/check | Result |
| --- | --- |
| `cargo +1.85.0 fmt --check` and `cargo +stable fmt --check` | Passed both; stable was Rust 1.100.0. |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings` and the same with `+stable` | Passed both. |
| `cargo +1.85.0 test --locked --offline` and the same with `+stable` | Each: **76 executable tests + 2 compile-fail doctests passed; 0 failed, 0 ignored**. |
| `cargo +1.85.0 clippy --locked --offline --lib --test snapshot_cache -- -D warnings` and the same with `+stable` | Passed both for the new cache integration target and library. |
| `cargo +1.85.0 build --release --locked --offline --target-dir target/provider-latency` | Passed; used for the existing Python CLI fixture tests. |
| Exact CI Python runner, `python3 -B /tmp/siliconwatch007-checks/strict_python.py` | **153 passed, 0 failed, 0 skipped**, Python 3.14.10. |
| `python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v` | **6 passed, 0 failed, 0 skipped**; DCO-checker tests, not a new contribution signoff check. |
| `cargo +1.85.0 test --locked --offline --test hwmon --test identity permission -- --nocapture` | Both conditional permission tests passed their assertions; neither reported a skip. |
| Supplemental `cargo +1.85.0 clippy --locked --offline --all-targets -- -D warnings` | **Failed** on unchanged `tests/hwmon.rs:217` and `tests/identity.rs:368`: `clippy::non_octal_unix_permissions` rejects pre-existing `from_mode(0)`. No unrelated fix or lint suppression applied. |
| `git diff --check`, new-file whitespace/local-link checks, source-copy and preservation audits | Passed. |

Checks ran in a disposable copy including all new source/tests, with build outputs outside both project checkouts. The strict Python runner is the block extracted from the unchanged CI workflow, including required-module/CLI discovery and rejection of skipped tests. Python tests use temporary fixtures and synthetic processes; they do not execute hardware characterization. Existing CLI and foreground sampling regressions run within the Rust and Python suites.

The original **53** executable Rust regressions remain, plus **16** cache unit tests and **7** cache integration tests. Two compile-fail doctests verify non-cloneable ownership and borrowed-snapshot lifetime. Controlled tests cover threshold boundaries, backward clocks, maximum `Duration`, wall-time errors/jumps, sequence exhaustion, failed publication, caught panics, poisoning, stop/drop, shared-state cleanup and repeated historical retention. Channel-coordinated readers cover both success replacement and success-to-failure publication, including lock-delayed stale data. No arbitrary sleeps establish ordering.

Full snapshot comparisons preserve every acquired field, including raw/converted data, warnings, diagnostic precedence, duplicate/null identity candidates, collisions, label changes and renumbering. Synthetic fault/permission-denied rows test cache preservation independently of OS privilege. Existing one-shot and sampling fixtures cover terminal/JSON/JSON-Lines output, exit behavior, timing and output failures.

Required checks pass; the supplemental all-target lint finding remains explicitly open in the unchanged baseline tests. There is no hosted CI result for this uncommitted milestone. Local logs are retained outside the repository at `/tmp/siliconwatch007-checks/`. No benchmark or historical evidence was regenerated or modified.

## 4. Hardware validation

No live hardware acquisition, benchmark matrix, suspend test or extended performance investigation was performed. Cache acquisition integration was exercised only against isolated temporary filesystem fixtures. This library-only change adds no CLI behavior; fixture regressions preserve the prior commands. Lock overhead, real driver blocking and production contention have not been measured, and no new sensor counts or latency claims are made.

## 5. Architecture and design review

The implementation follows the approved ownership, publication, freshness and retention contracts. Two API clarifications do not change scope: offsets use `Duration` instead of a narrowed integer, and a typed published `Acquisition::Failed` is distinct from `Err(CacheError)` for an unpublishable attempt. A private unwind guard also stops currentness if a scan/clock panic is caught by an outer caller that retains the owner; it neither catches panics nor invents failed-attempt metadata.

All existing snapshot fields, sensor order, raw readings, conversions, identity candidates/collisions and diagnostic precedence pass through unchanged. Snapshot schema 2, identity schema 1 and sampling envelope schema 1 remain unchanged. The cache adds no serialized metadata, presentation preference storage or permissions policy.

The installed application remains Rust-only; Python remains development/test tooling. Native raw values and acquired trimmed raw text remain separate from canonical conversions. No calibration or derived telemetry is introduced. Celsius stays the default; future global/per-sensor Fahrenheit preferences belong to presentation and must not modify shared telemetry. Design-document clarifications recommended in the proposal remain future documentation work. **Decision 43 remains Pending.**

## 6. Known issues and risks

No confirmed cache defect remains from local review and verification. The supplemental all-target Clippy failure in two unchanged baseline tests remains a known repository quality issue; the requested default Clippy checks pass. The following are contractual limits or unverified operational concerns, not measured failures:

- Misuse of held read guards can deadlock or starve publication. No hard real-time, bounded wait or cancellation guarantee exists.
- Scans and old-payload destruction may take unbounded wall time; stop requires exclusive owner access.
- No absolute memory ceiling, physical-age inference, automatic suspend detection, live-hardware validation or performance result is claimed.
- Poisoning is deliberately fail-closed; recreate the cache after addressing the underlying failure. Rust abort, allocation failure or process termination is not recovered.
- The public Rust cache API is experimental. Future daemon integration must preserve these contracts and avoid exposing internal paths/error messages indiscriminately to remote clients.
- Cache lifecycle is library-only. No daemon, API, GUI, database, adaptive polling, alarms, hardware control or protective shutdown was implemented.

## 7. Git status and preservation

Worktree: `SiliconWatch-prototype007`; branch **`codex/alpha1-prototype007`**; unchanged HEAD **`52f92916430d3e515fa39477802106fef4c3620c`**. Nothing is staged, committed, pushed or published; no new PR exists. The five implementation/report files above, the existing proposal and the new publication-readiness report form the seven-file local review set. Only the two report documents changed during acceptance/readiness review; implementation and proposal bytes are preserved.

The original checkout contains separate agent-skills setup work on `codex/agent-skills-setup`; it was not moved or edited. Its intentional Prototype 006 report, `.agents/`, `skills-lock.json`, modified `AGENTS.md` and `docs/agents/` remain separate from this milestone. A final comparison verified all 78 original tracked/protected files against their task-start content, modification time and mode; none changed. All tracked files in the Prototype 007 worktree match its HEAD except the authorized one-line module export. The approved proposal hash is unchanged.

Protected Prototype 006 report SHA-256 remains:

`327485f986260c8612b7fc27b58693546ab990763645c7948e7862fc327b5325`

## 8. Recommended next milestone

Only after separate authorization, prepare a **Prototype 008: Minimal Headless Monitoring Lifecycle** specification. Define one acquisition owner serving the shared cache, explicit startup/stop/failure handling and a bounded, deterministic lifecycle harness. Keep APIs, GUI, persistence and protective actions separate. Before implementation, agree on scheduling integration, interruption handling, suspend policy and reader guard discipline.

Acceptance criteria for that later authorized work should include fixture-proven single acquisition ownership, complete publication, honest failure/stale states, bounded retention, orderly shutdown where drivers permit it, and unchanged one-shot/foreground commands. Use controlled timing and lifecycle tests; real driver cancellation and a service deployment policy are unresolved dependencies. Prototype 008 is **not started or authorized** here.

## Acceptance and publication-readiness review — November 17, 2026

Chris accepted the library-only implementation with its existing limitations. The initial implementation verification above remains historical evidence; this subsequent review independently reran formatting, requested Clippy and all 76 executable Rust tests plus two doctests on Rust 1.85.0 and 1.100.0. All passed. Cache-target Clippy, the Rust 1.85 release fixture build, 153 strict Python tests, six DCO-checker tests and both explicit permission assertions also passed without test failures or skips.

The supplemental all-target Clippy command reproduced the same two unchanged baseline test findings; no unrelated fix or lint suppression was made. Source/contract review identified no required correction. Cargo files, dependencies, existing commands and JSON schemas remain unchanged. No hosted CI result is claimed.

The [readiness report](007-publication-readiness.md) records the exact manifest, reproduction commands, DCO requirements, public-disclosure review and detailed historical-evidence audit, including the Prototype 004 raw-capture size-only verification limitation. Both worktrees and all original intentional changes remain preserved. No hardware measurement, evidence regeneration, staging, commit, push or PR occurred. Publication requires separate approval and fresh remote verification.

No design-decision history update is required: acceptance does not alter an accepted architecture decision. Decision 43 remains Pending. The earlier proposal is intentionally unchanged as the historical specification.

## Revision history

| Date | Change | Status |
| --- | --- | --- |
| 2026-11-17 | Implemented authorized library-only cache, deterministic regressions and local compatibility verification. | Complete; awaiting Chris's acceptance and separate publication authorization |
| 2026-11-17 | Recorded Chris's formal acceptance and completed publication-readiness, compatibility and preservation review. | Accepted by Chris; limitations accepted, not resolved; publication not authorized |

## HANDOFF TO CHATGPT

Chris formally accepted **Prototype 007: Shared In-Memory Sensor Snapshot Cache** on **November 17, 2026**, with its documented limitations. The milestone now records **Accepted by Chris**. This review is complete and the seven-file change set is locally ready for publication approval. Acceptance does not authorize production deployment, daemon/API/GUI work, hardware controls, or Prototype 008. Decision 43 remains Pending.

This task changed only the acceptance/milestone documentation and added the publication-readiness report. Source and tests are unchanged from the accepted implementation. The exact proposed publication manifest is:

- `src/lib.rs`
- `src/snapshot_cache.rs`
- `src/snapshot_cache/tests.rs`
- `tests/snapshot_cache.rs`
- `docs/milestones/007-shared-snapshot-cache-proposal.md`
- `docs/milestones/007-shared-snapshot-cache.md`
- `docs/milestones/007-publication-readiness.md`

The cache preserves one acquisition owner, concurrent borrowed read handles, atomic complete publications, explicit startup/failure/historical/recovery states, raw readings, identities and diagnostics. Existing acquisition, CLI, sampling and JSON contracts remain intact. The installed runtime remains Rust-only. No dependency, lockfile, workflow or architectural-decision change was needed.

Fresh deterministic verification passed on **Rust 1.85.0 and 1.100.0**: formatting, requested Clippy, **76 executable tests and two compile-fail doctests per toolchain**, with zero failures or ignored tests. Cache-target Clippy and the Rust 1.85 release build also passed. The strict Python suite passed **153 tests without skips**, and all **six DCO-checker tests** passed. Permission assertions ran without conditional skips. No hardware measurements or benchmark matrices ran.

The supplemental all-target Clippy command still fails on two pre-existing permission literals in unchanged hwmon/identity tests. This is separate baseline cleanup, not a cache defect or a failure of the configured CI command. No fix or lint suppression was added. Hosted CI and any future contribution signoff remain unverified because nothing has been committed or published.

Both worktrees are preserved. All 78 original checkout fingerprints remain unchanged, including the intentional Prototype 006 report and agent-skills work. Historical benchmark archives and private evidence remain excluded from the publication manifest and unchanged within the preservation audit's stated coverage. No credentials, private home paths or sensitive hardware evidence were found in the seven reviewed text files.

The feature branch remains **`codex/alpha1-prototype007`**, HEAD **`52f92916430d3e515fa39477802106fef4c3620c`**. Its review state is one modified tracked file and six untracked files; nothing is staged. No commit, push, PR, merge or history rewrite occurred.

Accepted limitations remain unresolved: reader-held locks can block publication; retention is not a fixed RSS bound; physical sensor age, automatic suspend detection, cancellation and real-time guarantees are absent. The next authorization should cover only staging the seven reviewed files, a DCO-signed contribution, normal branch push and one PR to main, after fresh remote verification and followed by hosted CI inspection. No merge or next-milestone implementation is recommended without separate approval.
