# Alpha 1, Prototype 007: Shared In-Memory Sensor Snapshot Cache

**Date:** 2026-11-17 (America/Los_Angeles). **Status: PROPOSAL — preparation Complete; implementation not authorized.**

Chris authorized this specification only. The recommended contract below requires review before coding. No cache, daemon, API, GUI or new production behavior has been implemented. Decision 43 remains **Pending**.

## 1. Repository baseline and preserved work

[PR #5](https://github.com/lgraak/Siliconwatch/pull/5) merged with regular merge commit **`52f92916430d3e515fa39477802106fef4c3620c`**, the fetched and verified `origin/main`. Prototype 006 contribution **`fd5a25e17a5c7bb0a6d6dccc729e3ed172321ae0`** is reachable; the expected merge parents and contribution tree match. No unexpected divergence was found.

A separate sibling worktree, `SiliconWatch-prototype007`, holds local branch **`codex/alpha1-prototype007`** at that merge commit, with no upstream. The original `SiliconWatch` checkout remains on Prototype 006. Its unstaged publication-status report is not copied into this milestone; its SHA-256 is preserved exactly:

`327485f986260c8612b7fc27b58693546ab990763645c7948e7862fc327b5325`

The original untracked `.agents/` and `skills-lock.json` are also preserved. The tracked Prototype 006 report in this worktree contains its historical pre-publication wording; PR #5 and the verified merge establish current repository status. Neither version is rewritten here.

The sole proposed repository addition is this document. No staging, contribution commit, push, PR or merge is authorized by this planning task.

## 2. Architecture review and objective

Provide one process-local acquisition owner and several read-only consumers of coherent, immutable published observations. Start with a small library module, without wiring the existing commands through a cache.

| Existing contract | Consequence for the cache |
| --- | --- |
| [Prototype 001](001-hwmon-prototype.md), [reader](../../src/hwmon.rs): `scan(root: &Path) -> io::Result<Snapshot>` returns owned data | Move the complete result into cache storage; preserve this function and all acquisition behavior. No general provider interface is necessary. |
| `Snapshot` holds schema version, root, ordered sensor rows and warnings; rows hold raw text/numbers, converted value/unit, status/error and source information | Preserve the whole object. Existing `raw_text` is trimmed by acquisition; preservation means exact acquired fields, not original sysfs bytes. |
| [Prototype 002](002-stable-sensor-identity.md), [identity](../../src/identity.rs): collision checks run over the complete vector | Publish after collision marking and warning ordering. Never key storage by label or candidate, deduplicate rows, select a collision winner or reconcile generations. |
| [Prototype 003](003-bounded-foreground-sampling.md), [sampling](../../src/sampling.rs): serial scans, owned `Record`, invocation-relative timestamps | Keep the current loop, ownership, CLI and JSON contracts intact. Its consuming JSON writer is not a cache adapter; avoid adding whole-snapshot cloning merely to connect them. |
| [Prototype 004](004-overhead-characterization.md) and [005](005-provider-latency-characterization.md) document variable scan latency | A completed scan can already be old at publication. Observed latency maxima are not deadlines or explanations of driver behavior. No benchmark rerun is required here. |
| [Prototype 006](006-diagnostic-reliability-ci.md) separates authoritative state from output | Publication must not depend on rendering or consumer output. Output errors must not rewrite acquisition data or its failure cause. |

Current scanning/sampling has no shared cache or concurrent-reader contract. Existing snapshots are not `Clone`; source-identity objects have their own independent status. `Ok(Snapshot)` includes empty inventories and snapshots whose sensors all report errors. It is not a guarantee of complete discovery, healthy hardware, trustworthy readings or protection eligibility.

## 3. Recommended structures and API boundary

Add a small `snapshot_cache` library module and export it from `src/lib.rs`; add module/fixture tests. These are proposed future files/changes, not changes made now. Keep Cargo dependencies and minimum Rust **1.85.0** unchanged.

Conceptual shape; names may be refined without changing this contract:

| Type | Responsibility |
| --- | --- |
| `CacheOwner` | Unique, non-`Clone` writer; private root and shared state. `acquire_once(&mut self)` performs one synchronous scan. No loop, worker or timer is created. |
| `CacheReader` | Cloneable handle to shared state. Cannot initiate acquisition, alter configuration or publish. |
| `Shared` | `RwLock<PublishedState>`, one common `Instant` origin, immutable positive `stale_after: Duration`, and an `AtomicBool` producer-live flag; shared through `Arc<Shared>`. |
| `PublishedState` | Latest published attempt: `Empty`, `Succeeded(ObservedSnapshot)`, or `Failed { attempt, error, previous: Option<ObservedSnapshot> }`. No history list. |
| `AttemptMetadata` | Checked sequence, scan-start/finish offsets, publication offset, nullable wall-start Unix milliseconds and explicit wall-time error. |
| `ObservedSnapshot` | Attempt metadata plus the unchanged owned `hwmon::Snapshot`. |
| `ReadView<'a>` | Private read guard exposing immutable references and a read-time assessment; private last-assessed time detects backward observations. No `Arc<Snapshot>`, mutable state guard or owned snapshot is returned. |
| Assessment/error types | Separate acquisition outcome, producer lifecycle, snapshot role and observation age. Typed acquisition, poisoned-state, invalid-clock and exhausted-sequence errors; no fabricated timestamps. |

A factory returns exactly one owner plus a reader. Readers may be cloned, but cannot manufacture another owner from their handles. Writer methods require `&mut self`; sharing the owner externally must preserve exclusive access. This enforces one acquisition at a time **per cache**, not a global prohibition on existing independent CLI processes. Root and freshness policy remain fixed for a cache instance; changing them creates a new cache rather than retargeting old observations.

## 4. Publication and concurrency contract

1. Validate owner liveness and sequence capacity before starting an attempt. Record monotonic start and wall correlation, invoke unchanged `hwmon::scan()` outside the state lock, then record finish. A private clock/scan test seam is sufficient; do not introduce a production provider trait.
2. Prepare result/error metadata outside the write lock. Take the write guard, observe publication time, validate timing order, and replace the complete published state in one critical section. A failure transition moves the existing successful payload into `previous`; consecutive failures preserve at most that one payload.
3. Success replaces the preceding state, including any historical snapshot. An empty or error-bearing `Ok(Snapshot)` replaces an earlier healthy snapshot unchanged. Sequence advances for every successfully published acquisition attempt, including whole-scan failures.
4. Release the guard before destroying displaced allocations. Do not scan, serialize, format, write output, invoke caller callbacks or sleep while holding the write lock. Publication time is the clock observation inside the critical section just before the swap, not a hardware sample timestamp.
5. `acquire_once` completes only after publication. Distinguish a published acquisition failure from an unrecoverable publication/control failure in its return type; never use a success-shaped return for either failure.

**Linearization:** consumers see the latest **published** attempt. A guard acquired before replacement sees the old coherent state; a read after replacement sees the new state. A scan that has returned but is waiting for a write lock has not yet published its result. There is no promise of immediate visibility of an in-flight scan or failure. This limitation must remain visible in API documentation.

Multiple read guards can coexist and borrow the same payload. Hold them briefly; release them before acquisition, another potentially recursive read, blocking output, sleeps or waiting on the owner. Same-thread acquisition while holding a read view can deadlock. Copying selected fields for later consumer work is a caller allocation, outside the cache's retention guarantee.

The standard library does not promise an `RwLock` fairness policy, so held guards can delay or starve publication. This is a deliberate review tradeoff for the minimal bounded-generation design, not a nonblocking or real-time claim. See [Rust's RwLock documentation](https://doc.rust-lang.org/std/sync/struct.RwLock.html). An `Arc<Mutex<State>>` would serialize readers without solving retained-payload ownership; returning `Arc<Snapshot>` would need a separate lease/backpressure policy to bound old generations. Do not introduce that policy or extra dependencies in this proposal.

Poisoned state returns an explicit error; do not silently recover it with `into_inner()` or hand out data as healthy. No public write guard is exposed. Sequence exhaustion or fatal timing/publication failure stops the owner and returns an error; an unpublishable incoming result is not relabeled successful or assigned invented timing. Previous published data remains inspection-only when accessible.

## 5. Empty, failure, recovery and freshness semantics

| Condition | Required reader meaning |
| --- | --- |
| No attempt published | Explicit `Empty`, no observation, no acquisition error and no age bounds; distinguish from a successful empty inventory. |
| Latest attempt succeeded | Expose its exact snapshot, including warnings, unsupported inputs and failed rows. Assess freshness separately from each row's status. |
| Latest attempt returned an I/O error | Publish `Failed` with actual error classification/diagnostic and attempt timing; no current snapshot. A previous success is labeled **historical after failure**, even if only microseconds old. |
| Repeated failures | Replace latest failure metadata, retain at most one previous observation; no accumulating error history. |
| Recovery | A new successful snapshot becomes the latest attempt; obsolete failure/historical state is replaced. Missing rows stay missing; do not synthesize prior readings. |
| Producer stopped or dropped | No snapshot is current. Retained data is explicitly historical with stopped lifecycle, independently of age and the latest attempt outcome. |

Expose historical data through an explicitly labeled inspection path, not a fallback in a “current snapshot” getter. Do not offer a bare getter that silently hides outcome/lifecycle/freshness. A latest successful snapshot whose age exceeds policy remains inspectable but stale. “Fresh” never overrides per-sensor or identity diagnostics, and never means physically validated.

Use a single cache-lifetime monotonic domain, separate from the offsets of any existing sampling invocation. Capture `started <= finished <= published`, also checking each owner's new start against its prior publication. Query assessments use checked `now >= published`. A held view additionally checks each assessment against its own previous observed time, rejecting a backward reading even if it still exceeds publication time; it must not turn stale data fresh again. An `assess(&mut self)` method can update that private scalar without mutating shared telemetry. A new view has the publication bound as its initial reference; no global reader-clock high-water tracker is proposed. At read-time assessment:

```text
observation_age_lower = now - finished
observation_age_upper = now - started
stale_by_age = observation_age_upper > stale_after
```

Equality is within the configured age budget. Require an explicit positive threshold at construction; no safety default, adaptive policy or configuration file is introduced. Compute age on each assessment, including through a held view; never freeze a fresh/stale boolean at publication. Return its `assessed_at` offset so callers know the verdict's time scope. A saved assessment is not a continuing guarantee.

Only nonempty sensor snapshots have observation bounds, matching existing sampling terminology. Empty successes retain scan timing but report `no_sensor_observations`; startup or first failure has no observation bounds. Historical snapshot bounds use that snapshot's original timing, never the failed attempt's timing. Its historical/stopped role overrides any young numeric age. A slow scan or delayed publication may already be stale on first read.

Wall-clock values are correlation metadata only. Jumps or wall-time conversion failures do not reset age or discard valid acquisition data; unavailable wall time remains explicit. Reject invalid monotonic ordering/overflow rather than saturating to zero. All physical sensor ages remain unknown; these bounds bracket sequential userspace discovery/reads, not hardware sampling instants.

Suspend inclusion is not guaranteed by `Instant`; do not infer resume from wall-clock deltas. This prototype does not implement Decision 33's resume detection. A caller aware of suspend must stop/discard the old cache and create a new one before treating subsequent data as current. Real suspend behavior remains unverified. See [Rust's Instant documentation](https://doc.rust-lang.org/std/time/struct.Instant.html).

## 6. Memory, stop and expected performance

The cache owns at most **one retained observation payload**, whether current or historical, plus one newly acquiring/replacing payload. Readers borrow the retained payload and cannot accumulate old shared generations through this API. At most two full snapshot payloads may coexist during replacement; scanner temporaries and fixed-count metadata, including one variable-length latest error, are additional. Memory does not grow with completed attempt count.

This is **O(size of the largest live scan plus the latest error)**, not an absolute byte/RSS ceiling. Existing discovery, strings, paths, warnings and identity evidence have no fixed size limits. Cloned reader handles and caller-owned copies also consume memory. A hard process-wide byte cap or bounded consumer population would require another approved contract; do not silently truncate observations or change the reader to claim one.

`stop(&mut self)` is terminal and prevents further acquisition. Stop and owner `Drop` set the shared producer-live flag to false without taking the state lock (Release store; read-time assessments use Acquire load). This lets an existing read view notice owner disappearance without a drop/write-lock deadlock. Do not freeze liveness in the read guard. The flag does not carry a precise stop timestamp or replace the locked acquisition result.

No cache-owned thread, channel queue, signal handler, file or child process needs shutdown. Dropping the last handles releases retained memory. Stop is not concurrent cancellation of `acquire_once`; exclusive writer access and blocking drivers mean it cannot promise cancellation or a deadline. Reader-held guards can delay publication indefinitely; leaked guards or malicious in-process callers are outside progress guarantees.

Expected costs: one unchanged scan per owner call, small synchronization/timestamp work, a move of the payload, and read-time arithmetic. Multiple readers cause no additional hardware scans. No per-reader whole-snapshot clone is required. Lock contention and retiring a large snapshot can still add latency; no measured overhead, fairness, performance improvement or production-readiness claim is made by this planning review.

## 7. Product requirements and compatibility

Chris's accepted product direction is recorded here explicitly:

- The installed SiliconWatch application is written in Rust and must not require Python to run. Python remains permitted for development, CI, testing and benchmarking.
- Preserve raw observations exactly as acquired. Existing native-unit `raw_value` and trimmed `raw_text` stay separate from canonical converted `value`/`unit`. Calibrated and derived values must remain distinguishable; none are computed by this cache.
- Celsius is the default temperature display. Fahrenheit will be supported through global and per-sensor display preferences. Preferences never modify shared telemetry or source identity.
- Keep all display preference storage/application outside Prototype 007. Per-sensor override, then user-global preference, then Celsius is a **recommended precedence** consistent with Decision 26, not newly implemented functionality or a resolution of permissions policy.

Recommended future design-document clarification: add a dated baseline statement about the Rust-only installed runtime and a temperature-display clarification under Decisions 18/26, linked to raw/canonical/calibration rules in 35/36. Keep numbering/history intact; these clarifications do not grant API configuration rights. This task does not edit `DESIGN_DECISIONS.md`. Decision 43 remains Pending.

Preserve `hwmon::scan`, row/warning ordering, diagnostic precedence, identity rules, raw/converted values, existing CLI arguments/output/exit codes and sampling semantics. Snapshot JSON remains schema **2**, identity schema **1**, sampling envelope schema **1**. Cache metadata is an internal experimental Rust contract, with no new serialized schema or wire API. Use Rust 1.85-compatible stable standard-library APIs, without async runtimes, unsafe code or dependency changes.

## 8. Deterministic acceptance tests for later implementation

Tests are required work after approval, **not tests executed during this proposal**. Use isolated hwmon fixtures, a private controllable clock/scan seam, and channels/barriers for concurrent ordering. Bounded test timeouts are deadlock guards, not timing-accuracy assertions.

| Test group | Acceptance evidence |
| --- | --- |
| Startup and empty inventory | `Empty` differs from successful zero-sensor scan; appropriate timing/null bounds; reads do not acquire hardware. |
| First success and replacement | Complete exact snapshot published with increasing attempt ID; all rows/warnings replaced together; removed sensors do not survive in latest data. |
| Whole-scan failure | Missing/unreadable fixture root produces latest `Failed`; previous success only through labeled historical inspection, even within threshold. |
| Repeated failure and recovery | One historical payload at most; new failure metadata replaces old; recovery replaces failure and old snapshot without stale field leakage. |
| Partial/all-failed sensors | Faulted, disabled, malformed, unsupported and permission-error rows retain values/status/primary error/warnings; `Ok` scan is not rewritten to global failure or healthy data. |
| Freshness boundaries | Just below/equal/above threshold; long scan and lock-delayed publication already stale; repeated held-view assessments advance age. |
| Clock domains | Wall jumps/unavailability do not alter age; invalid monotonic ordering and sequence/time overflow fail explicitly. A repeated assessment that moves backward but remains after publication is rejected, never stale-to-fresh; no invented zero ages. |
| Concurrent readers and atomicity | Multiple synchronized readers see one complete generation, timing and outcome; writer never exposes mixed old/new fields. Test success-to-failure publication boundary too. |
| Single owner and contention | No owner cloning/public read-side acquisition; maximum one scan active; retained guard prevents replacement until released, then exact pending attempt publishes. No sleep-based race assumptions. |
| Raw and identity preservation | Compare all fields with direct fixture scan: raw integers/text, zero values, conversions, duplicate/null candidates, collision members, label changes and registration renumbering. |
| Retention and lifetime | Many replacements/failures retain no history; compile-fail/doctest proves borrowed payload cannot outlive its view. Inspect the ownership bound and verify shared state is released after all handles/views drop (e.g. private `Weak` probe). No RSS claim inferred from this test. |
| Stop and cleanup | Stop/drop owner while a read view exists does not require a write lock; reassessment reports stopped/historical; later acquisition rejected; last-handle release frees shared state. |
| Poison and consumer failure | Inject a test-only writer panic; reads/acquisition report poison explicitly. Independent consumer formatting/output error cannot mutate published state. |
| Compatibility | Existing 53 Rust regressions remain passing, including CLI/sampling fixtures; Python 153-test strict suite and six DCO tests remain intact without skips. No hardware needed in CI. |

After implementation approval run formatting, Clippy with warnings denied, Rust tests on minimum 1.85.0 and the development toolchain, relevant Python/DCO tests, and compile-fail/documentation checks. Report actual new totals and all skips/limitations. Existing CLI fixture tests establish behavior preservation; no new live hardware or long benchmark authorization is implied.

## 9. Risks, review choices and implementation sequence

Chris should approve these specific choices before implementation:

1. **Borrowed read views with possible publication backpressure.** This is the smallest bounded-generation option, but unsuitable for retaining guards across slow consumer work. If prompt publication despite arbitrary reader retention is required, revise the proposal to define bounded leases/backpressure first.
2. **Library-only initial integration.** No CLI/sampling ownership/schema refactor or scheduler. Demonstrate through owner/reader fixture tests; future daemon integration remains separate.
3. **Explicit caller-selected age threshold and suspend limitation.** It is an observation-age policy, not a hardware-validity or protection budget. No automatic resume policy is claimed.
4. **Retention rather than hard byte bound.** A hard allocation cap is not achievable merely by replacing a shared pointer while preserving today's uncapped scan behavior.

Recommended implementation order after approval: define private states and ownership API; implement pure timing/assessment transitions and fixtures; add the synchronous scan-and-publish owner; add concurrency/lifecycle/retention tests; run existing compatibility suites and update the milestone report. Expected edits are limited to the new cache module/tests, `src/lib.rs`, and milestone documentation. Stop and propose any required broader change before making it.

No daemon/systemd service, D-Bus/network API, GUI, database/history, adaptive polling, alarm engine, controls or protective shutdowns. No automatic next milestone. A general provider interface is not justified by current requirements.

## 10. Verification performed for this planning task

- Read project instructions, README, architecture, relevant Prototype 001–006 reports, current Rust source and regression contracts; independent reviews covered acquisition/identity, timing/concurrency and product/measurement requirements.
- Fetched origin; verified exact main/merge ancestry and created the separate local worktree/feature branch. Original checkout fingerprints and diff/status are checked again at handoff.
- Reviewed official standard-library locking/time documentation; the documentation consulted describes current stable Rust. The proposal intentionally uses established APIs, but actual 1.85 compilation of a cache awaits implementation.
- Checked proposal Markdown, relative links, whitespace, sole-file worktree scope and protected-file preservation before handoff. No implementation, compilation, Cargo/Python regression execution, hardware access or performance measurement occurred. There are **no new executed cache tests or performance results** to report.
- Historical verification (53 Rust tests, 153 Python tests and six DCO tests) is recorded in the Prototype 006 publication/CI evidence, not presented as newly executed validation here.

## Revision history

| Date | Change | Status |
| --- | --- | --- |
| 2026-11-17 | Reviewed merged foundation; proposed a small Rust cache contract, bounds, failure semantics and acceptance tests in an isolated worktree. | Proposal for Chris's review; no implementation authorization |

## HANDOFF TO CHATGPT

Prototype 007 proposal preparation is complete; **implementation has not begun**. Chris authorized planning only after PR #5 merged. A fresh fetch confirmed `origin/main` at **`52f92916430d3e515fa39477802106fef4c3620c`**, with Prototype 006 contribution `fd5a25e17a5c7bb0a6d6dccc729e3ed172321ae0` reachable and no unexpected divergence.

A separate sibling worktree, `SiliconWatch-prototype007`, now holds local branch **`codex/alpha1-prototype007`** at that merge commit, without an upstream. Its only addition is this untracked proposal. The original Prototype 006 checkout remains in place; its intentional unstaged report, untracked `.agents/` and `skills-lock.json` are preserved. The report's expected SHA-256 remains `327485f986260c8612b7fc27b58693546ab990763645c7948e7862fc327b5325`. Nothing was staged, committed, pushed or published.

The recommended first cache is a standalone Rust library module, using a unique non-cloneable acquisition owner and cloneable read handles around private `Arc<Shared>` storage containing `RwLock<PublishedState>`. Each owner call performs unchanged `hwmon::scan()` outside the lock, then atomically publishes one complete result. Existing CLI commands, sampling ownership and all JSON schemas remain untouched. No general provider abstraction or new dependency is justified.

Readers borrow immutable views rather than receiving retainable snapshot Arcs. This bounds cache-owned observation retention to one current/historical payload plus one in-flight replacement. It does **not** impose a hard byte/RSS ceiling on existing variable-size scans or consumer allocations. Short read guards can delay publication; holding them across owner calls or blocking work is prohibited by the proposed usage contract. Chris must approve this contention tradeoff.

Startup, successful empty inventories, successful snapshots with row errors, failed acquisitions and recovery remain distinct. After a published failure, prior success is available only as explicitly historical data. Read-time freshness uses checked monotonic observation bounds, never wall-clock movement or publication time alone. A shared terminal liveness flag lets stop/owner-drop invalidate currentness without waiting for a read guard. Physical sensor age and automatic suspend/resume handling remain outside the guarantee.

The proposal defines deterministic transition, concurrency, clock, retention, raw-value, identity, poison and lifecycle tests. These tests are specified, not implemented or executed. This planning task performed source/document review, Git/ancestry and preservation checks, and document validation; no hardware measurements or application tests ran.

Accepted product direction is explicit: the installed application is Rust-only; Python is development tooling. Raw, canonical converted, calibrated and derived values remain distinguishable. Celsius is default, with future global/per-sensor Fahrenheit preferences confined to presentation. Design-document clarifications are recommended, not applied. Decision 43 remains Pending.

Chris should review the borrowed-view publication tradeoff, library-only scope, caller-selected age threshold and retention-only memory bound. After separate approval, implement states/tests first, then acquisition/publication and concurrency checks, preserving all existing regressions. Daemon, APIs, GUI, databases, adaptive polling, alarms and hardware/protective actions remain excluded. Stop for review.
