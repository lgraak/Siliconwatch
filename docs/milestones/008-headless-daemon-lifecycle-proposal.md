# Alpha 1, Prototype 008: Headless Daemon Lifecycle

**Status: PROPOSAL — awaiting Chris's implementation approval.**

Planning completed on November 17, 2026. PR #6 has been merged; no Prototype 008 application code, dependencies, commits, pushes, or PRs have been created. This proposal is not a claim of implementation, production readiness, or approval to install a service.

## 1. Verified foundation and scope

PR [#6](https://github.com/lgraak/Siliconwatch/pull/6) was open, mergeable, and targeted `main` with the expected base `52f92916430d3e515fa39477802106fef4c3620c` and sole contribution `d671b6684798b4aef83fe1f9fdae51531e2b3ed0`. Its seven-file manifest matched the reviewed Prototype 007 contribution. The author-matching DCO signoff passed the repository checker.

GitHub's regular merge produced **`28d7177cc334e357646023e5a2292b0cc434e0a7`**. Verification established:

- PR #6 is marked merged.
- Its two parents, in order, are the expected old `main` and contribution above.
- The merge tree equals the reviewed contribution tree, `6241ecb38156a8e281d310e3f14f33793120930a`.
- The contribution remains reachable from fetched `origin/main`, which points to that merge.
- Existing feature branches were retained. No squash, rebase, force-push, or branch-protection change occurred.

The objective for Prototype 008 is a foreground Rust process that continuously acquires read-only observations into the existing cache, with explicit lifecycle and cooperative shutdown. There is no external cache consumer interface in this milestone.

## 2. Current architecture and compatibility

Review covered [README](../../README.md), [design decisions](../DESIGN_DECISIONS.md), the [sampling report](003-bounded-foreground-sampling.md), [cache proposal](007-shared-snapshot-cache-proposal.md), [accepted cache report](007-shared-snapshot-cache.md), and [publication review](007-publication-readiness.md).

| Existing component | Contract to preserve |
| --- | --- |
| [hwmon scanner](../../src/hwmon.rs) | `pub fn scan(root: &Path) -> io::Result<Snapshot>`; read-only discovery, ordering, raw values, source paths, labels, and diagnostic precedence |
| [identity](../../src/identity.rs) | Versioned evidence, ambiguity/collision handling, and existing identity semantics |
| [snapshot cache](../../src/snapshot_cache.rs) | One non-cloneable `CacheOwner`, cloneable readers, borrowed immutable views, atomic complete publication, explicit failure/history and freshness |
| [foreground sampler](../../src/sampling.rs) | Existing bounded `sample` command, scheduling, timestamps, output and exit behavior |
| [CLI](../../src/main.rs) | Existing `sensors`, `sensors --json`, table sampling and JSON Lines contracts |
| [Cargo configuration](../../Cargo.toml) | Rust 2024, minimum Rust 1.85.0; existing `serde` and `serde_json` dependencies |

`CacheOwner::acquire_once()` scans and publishes before returning. `Ok(Acquisition::Failed(sequence))` is a successfully published acquisition failure, not successful sensor data. `Err(CacheError)` is a terminal cache/control failure. A successful scan may legitimately contain no sensors, row errors, or snapshot diagnostics.

The cache creates no threads. An acquisition runs outside its publication lock, then waits for the write lock. `stop()` requires exclusive owner access and cannot interrupt an in-flight acquisition. Borrowed read guards can delay publication indefinitely. The existing sampler owns a separate scan/output loop; reusing it here would introduce a second acquisition path or require changes to its contracts. Keep it unchanged.

## 3. Proposed command and module boundaries

Proposed invocation, **not currently implemented**:

```text
siliconwatch daemon --interval-ms 1000 --stale-after-ms 3000
siliconwatch daemon --interval-ms 1000 --stale-after-ms 3000 --hwmon-root FIXTURE_PATH
```

Require both timing arguments explicitly. Accept an interval of **100–60,000 ms**, consistent with the foreground sampler. Require a positive integer freshness threshold in milliseconds, parsed with checked conversion; it is a reporting budget, not a promise that scans finish within that time. Reject malformed, zero, overflowing, duplicate or unknown options. Default the root to `/sys/class/hwmon`; permit a fixture root for tests. No runtime configuration reload.

The process remains in the foreground until shutdown or a fatal error. No double-fork, PID file, privilege elevation, background mode, count limit, systemd installation, or new telemetry output format. Existing one-shot and bounded sampling commands remain available separately.

Proposed implementation boundaries:

- `src/daemon.rs`: options validation, lifecycle controller, sequential worker loop, and signal/coordinator integration. Keep helpers private unless needed by the CLI.
- `src/daemon/tests.rs`: controlled clocks, waits and acquisition closures for deterministic lifecycle tests; no general provider interface.
- `tests/daemon_cli.rs`: temporary hwmon fixtures and isolated child-process CLI/signal tests.
- Minimal CLI dispatch/module exposure and documentation changes. Cargo/lockfile changes only for the reviewed signal dependency if approved.

The CLI currently acquires stdout and has a one-shot broken-pipe success path. Dispatch `daemon` independently so it neither holds a stdout lock for its lifetime nor inherits that success exception. Preserve existing commands' behavior.

Recommend a constant number of stderr lifecycle messages: one startup acknowledgement, one final outcome, and necessary fatal diagnostics; no per-scan progress stream or retained log history. The worker emits the startup acknowledgement after entering `Running`, before first admission, with no cache/control locks held. It records that transition, not successful readings or continuing liveness, and can race a later stop request. The coordinator immediately services signals after spawning; it does not wait separately for readiness. Only the coordinator writes the final outcome, after joining. A failed startup write terminates the worker and wakes the coordinator through the close guard; any failed required write causes a nonzero exit after cleanup. Blocked startup output can delay worker exit/join, but does not prevent coordinator stop acknowledgement. Blocked final output can delay process exit. These are explicit limitations, not bounded shutdown guarantees.

## 4. Ownership, lifecycle and publication contract

Use two application threads: the main thread coordinates signals and joining; one acquisition worker exclusively owns the `CacheOwner`. Use standard-library `Arc`, `Mutex` and `Condvar` for a small shared control object. Do not introduce an async runtime, worker pool, polling signal thread, message backlog or second acquisition owner.

Conceptual control state contains a lifecycle enum, stop request/reason, and at most one admitted-attempt marker. Keep execution outcome separate from lifecycle. Do not duplicate snapshots, observation timestamps, diagnostics or sensor-health state in the controller.

| Lifecycle | Meaning and allowed transition |
| --- | --- |
| `Starting` | Validate options, register signals, create empty cache and start worker. No successful observation is implied. Transition to `Running`, or `Stopping` on cancellation/setup failure. |
| `Running` | Worker initialized; sequential acquisition may begin. Recoverable scan failures do not change lifecycle. |
| `Stopping` | New admission closed; waiting worker is notified. One already admitted attempt may still finish and publish. Remain here until cleanup/join completes. |
| `Stopped` | Owner stopped/dropped and worker joined, or startup ended without creating a worker. A separate outcome records graceful or fatal termination. |

A lifecycle mutex protects both stop acknowledgement and admission. Reserving one attempt while `Running` is the admission linearization point. Release that mutex before calling the cache. A stop request sets `Stopping` under the same mutex and wakes the condition variable; no later reservation is allowed.

**An admitted attempt may start its actual scan after stop acknowledgement because of thread scheduling, and may publish after `Stopping`.** The cache cannot cancel or discard that publication through its existing interface. Signal arrival itself is not the acknowledgement point: the coordinator processes signals in ordinary thread context. Do not promise no further sysfs reads immediately after signal delivery.

During `Starting`, `Stopping` or `Stopped`, lifecycle-aware inspection must not present data as service-current. In particular, during `Stopping` the underlying cache may temporarily still report `Active/LatestAttempt`; the service lifecycle takes precedence. Keep bare cache readers internal to this milestone. A private combined assessment for tests/inspection can pair the lifecycle with the unchanged cache assessment; it must never strip historical roles or diagnostics. No public read API or cache schema redesign is required.

After an acquisition returns, the worker checks stop before admitting anything else. On exit it stops/drops its owner. The coordinator joins the worker before reporting `Stopped`. Never detach a blocked worker and report successful shutdown. Dropping a Rust join handle detaches rather than cancels the thread. [Rust JoinHandle documentation](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html)

## 5. Timing, freshness and acquisition failures

The first attempt is immediately eligible after `Running`. Subsequent eligibility uses the previous **actual owner-call dispatch start** plus the configured interval, calculated with checked monotonic arithmetic. Admission occurs first; record dispatch time immediately before `acquire_once()`. If a scan or scheduling delay passes the deadline, permit one next attempt, then reanchor to its actual dispatch start. Do not queue missed deadlines or issue catch-up bursts. There is never more than one acquisition in flight.

Wait on the condition variable using the remaining deadline and a stop predicate. Recheck both after every early or spurious wakeup. Use checked subtraction/addition; invalid time order or deadline overflow is fatal. Rust's condition-variable timeout does not itself establish the predicate and can wake spuriously. [Rust Condvar documentation](https://doc.rust-lang.org/std/sync/struct.Condvar.html)

Scheduler timestamps and cache-relative timestamps have different origins. A dispatch timestamp is not the cache's scan-start timestamp and must not be presented as hardware acquisition timing. Preserve `AttemptMetadata` exactly: sequence, cache-relative start/finish/publication offsets, optional wall-clock start and wall-clock error. Wall time is correlation information, never a scheduling clock.

For nonempty observations, existing userspace age bounds remain `[assessment - finish, assessment - start]`; an upper bound greater than the threshold is stale, equality remains within budget. Physical sensor age stays unknown. Empty snapshots have no sensor observations. Freshness does not turn a failed sensor row into a successful reading. Until an in-flight attempt publishes, readers see the preceding complete publication with its actual age, never a partial replacement.

| Outcome | Proposed service behavior |
| --- | --- |
| Successful scan, including empty/partial snapshot | Publish unchanged through cache; continue at configured interval. Preserve every row status and diagnostic. |
| Whole-scan I/O failure, including unavailable root | Publish explicit failed attempt; previous success is historical, never current. Continue at the same interval without adaptive backoff. |
| Recovery after failure | Next successful publication replaces failure state normally; retain original raw values/identity. |
| Fatal cache/control error or worker panic | Close admission, stop owner, wake coordinator and join. Report fatal failure; no automatic restart or fabricated snapshot. |
| Wall-clock discontinuity/unavailability | Preserve cache diagnostics; monotonic scheduling/freshness remain authoritative. |

A service-current observation requires `Running`, an active producer, the cache's latest-success role and age within budget. Per-sensor status remains independently authoritative. A point-in-time assessment is not a continuing liveness guarantee. Known suspend/resume requires stopping and recreating the cache under its existing contract; automatic suspend detection is not part of this proposal. Do not claim suspend-aware freshness.

## 6. Signals, cleanup and exit semantics

Rust's standard library is sufficient for ownership, scheduling and joining, but not a safe portable SIGINT/SIGTERM registration interface. Recommend evaluating one narrow runtime dependency, **`signal-hook` 0.4.x with default features disabled and `iterator` enabled**, rather than writing custom unsafe signal-handler plumbing.

The inspected crate manifest declares Rust 1.66 and MIT/Apache-2.0 licensing; its iterator feature uses channel support and Unix support includes `libc`/`signal-hook-registry`. This is a candidate dependency, not a pinned or validated lockfile addition. Resolve the exact release and transitive versions, review licensing/features, and compile/test the resulting lockfile on Rust 1.85.0 before accepting it. [Inspected signal-hook manifest](https://docs.rs/crate/signal-hook/latest/source/Cargo.toml)

Recommended sequence:

1. Validate arguments and register only SIGINT/SIGTERM before starting acquisition. Registration failure exits without scans.
2. Create the empty cache/control state and spawn the single worker with its unwind/wake guard installed before fallible startup work. Handle thread-spawn failure without leaving a worker or claiming `Running`. The worker checks cancellation before entering `Running` and emitting its startup line. The coordinator enters the signal wait immediately; there is no competing startup-readiness wait. A stop acknowledged before readiness suppresses both startup reporting and acquisition; panic before readiness still closes the iterator.
3. The coordinator handles signal-iterator events in normal thread context, requests stop and notifies the condition variable. Application signal handlers must not lock, allocate, format diagnostics or acquire sensors.
4. Give the worker a close handle for the signal iterator. A cleanup guard closes it on both normal return and unwinding, so fatal worker exit wakes the coordinator even without a signal.
5. Close the iterator after requesting shutdown; keep signal registration alive while joining so repeated SIGINT/SIGTERM are idempotent requests rather than a new forced-exit policy. Drop signal resources only after joining. Verify these exact semantics with isolated child tests against the selected dependency.
6. Release cache read views before joining. Stop/drop owner, join, classify outcome, then emit the final report. No final reporting failure may bypass required cleanup.

The signal iterator's close handle interrupts its blocking wait; it does **not** interrupt sysfs reads. Pending events may still be returned. Signals may coalesce, so never depend on counting them. Registration affects process-global signal behavior, and unregistering handlers is not a promise to restore an OS default disposition. Keep it confined to this command's process and isolate tests. [Signal iterator](https://docs.rs/signal-hook/latest/signal_hook/iterator/struct.SignalsInfo.html), [close handle](https://docs.rs/signal-hook/latest/signal_hook/iterator/backend/struct.Handle.html), [signal-hook limitations](https://docs.rs/signal-hook/latest/signal_hook/)

Proposed exit policy: `0` for requested SIGINT/SIGTERM shutdown after complete cleanup, even if recoverable acquisitions failed; `2` for invalid arguments; `1` for setup, fatal runtime, join or required-reporting failure. A concurrent signal never masks an already observed fatal error. Preserve fatal causes with bounded reporting; do not accumulate an error history. SIGKILL, process abort and loss of all reporting destinations have no graceful-cleanup guarantee.

There is no forced shutdown timeout or second-signal escalation in this scope. A blocked hardware read, read guard or output sink can delay shutdown indefinitely. A wait notification accelerates idle-worker shutdown only; it is not a hard real-time bound.

## 7. Locking, memory and performance considerations

Never hold the lifecycle mutex while scanning, acquiring a cache lock, writing output, waiting for a worker or joining. Condition-variable waits release that mutex and recheck the predicate after reacquiring it. Stop acknowledgement and admission share this predicate to avoid lost wakeups.

For any combined assessment, acquire the cache read view first, then briefly inspect lifecycle under the control mutex; prohibit the reverse order. Release both before output or waiting. Keep readers short-lived and non-recursive. A stopped lifecycle must override cache currentness even if the owner has not yet returned. This does not alter cache roles or allow concurrent mutation of snapshots.

Treat control-lock poisoning as fatal, not permission to continue with the poisoned inner state. Notify waiters even when a healthy lock cannot be obtained. Worker unwind cleanup must close the signal iterator and invalidate the owner without needing that mutex. Join the worker and report that authoritative lifecycle state is unavailable if poisoning prevents its update; never invent a clean `Stopped` record. Test these paths explicitly.

Retention remains one published observation plus one incoming replacement, bounded scalar lifecycle state and one worker stack. No scan queue, snapshot history, copied snapshots for progress reporting or unbounded diagnostic list. This is **not** a byte/RSS bound: snapshot/error sizes depend on providers and drivers. The service runs continuously, so total lifetime CPU work is not bounded by an attempt count.

Expected overhead consists of one worker, short control-lock operations per attempt, condition-variable wakeups, and the existing cache publication lock. No measurements have been made for this proposed design. Standard-library locks do not provide a writer-fairness or latency guarantee. Continuous operation, suspend, blocking readers and slow drivers remain validation concerns; previous benchmark outliers establish neither daemon performance nor their cause.

## 8. Deterministic acceptance tests and verification plan

Use private clock/wait/acquisition seams, channels and barriers; keep production calls to the existing cache. Do not generalize the scanner into a provider framework. Temporary filesystem fixtures and synthetic failures must be sufficient; CI must not inspect real `/sys/class/hwmon`.

| Area | Required evidence |
| --- | --- |
| Options/setup | Interval endpoints and rejection cases; positive freshness threshold; invalid/duplicate flags; signal-registration and spawn failures start no acquisitions |
| Startup | Empty cache; worker ready before its startup acknowledgement; stop before readiness prevents later admission; panic before readiness wakes coordinator without a readiness deadlock |
| Scheduling | Exact controlled dispatch times; remaining-time recalculation after spurious wake; late-scan reanchoring; checked arithmetic; wall-clock changes do not schedule scans |
| Admission/concurrency | Stop immediately before/after admission; no admission after acknowledgement; maximum one acquisition at a time; repeated stop requests do not lose wakeups |
| Failure/recovery | Missing fixture root and malformed rows; explicit failed publication; historical prior data; later recovery; empty snapshot preserved without fabricated sensors |
| Shutdown boundaries | Idle wait wakes; controlled blocked acquisition/publication remains `Stopping`; admitted attempt may publish after stop but never becomes service-current; release permits join and `Stopped` |
| Ownership/locking | One owner; concurrent short readers see whole snapshots; no control-to-cache lock inversion; release read guards before join; no false `Stopped` while worker alive |
| Fatal paths | Cache/control poison, fatal cache error, worker panic and invalid clock wake coordinator; cleanup still occurs; fatal error wins over concurrent graceful request |
| Output | Failed startup/final writes return nonzero with cleanup; stdout unused; one-shot broken-pipe success behavior remains unchanged |
| OS signals | Separate fixture child processes for SIGINT and SIGTERM, repeated requests and worker-initiated exit; registration/close races; no leaked or detached worker |
| Preservation | Raw readings, conversions, identity evidence, ordering, diagnostics, cache metadata/roles, snapshot and JSON Lines schemas unchanged |
| Retention | Many controlled attempts retain no history/backlog; only current cache payload/replacement and bounded control state survive |

Child tests must synchronize readiness with the worker startup acknowledgement or test-only channels, not assume readiness after a sleep. While waiting for readiness, also detect child exit and enforce a test deadline; a failed startup need not emit an acknowledgement. Controlled tests establish transition/timing behavior; subprocess deadlines are deadlock guards, not proof of a shutdown latency guarantee. Ensure timeout cleanup kills and reaps test children if a regression hangs. Do not install signal handlers in the shared unit-test process.

After implementation authorization, run formatting, configured Clippy, all Rust tests/doctests and fixture CLI regressions on Rust 1.85.0; preserve existing Python/DCO CI checks. Validate the signal dependency's selected features and lockfile on that toolchain. Any short, read-only hardware check must be separately described in the implementation report; no historical benchmark rerun is needed to establish lifecycle correctness.

## 9. Product constraints, exclusions and alternatives

The installed application remains entirely Rust. Python remains development/test/benchmark tooling only. Preserve raw observations unchanged; converted, calibrated and derived values must remain distinguishable from them. Celsius remains the default future display unit, with global/per-sensor Fahrenheit preferences confined to presentation. Do not implement display preferences or calibration in this cache/lifecycle work.

Decision 43's local API permissions remain **Pending**. Read-only acquisition runs without root by default; permission restrictions are reported through existing diagnostics. No writes to hardware, driver loading, elevated helpers, settings changes or claims to correct firmware readings.

Explicitly exclude HTTP/Unix-socket/D-Bus/network APIs, GUI, database/history, adaptive polling, alarms/notifications, hardware control, automatic protective actions, production deployment and systemd installation. Do not change accepted benchmark artifacts or begin another milestone.

Alternatives considered:

- Acquisition in the main thread: fewer threads, but leaves shutdown coordination blocked inside acquisition. Recommend a worker and coordinator instead.
- Atomic stop flag with ordinary sleep: simple, but cannot wake a long configured wait promptly; prefer the predicate/condition-variable approach.
- Separate signal thread or async runtime: unnecessary with the main-thread signal iterator and one worker.
- `ctrlc`, manual signal FFI, or a provider refactor: no concrete requirement justifies them here. The iterator approach gives an explicit coordinator wake/close mechanism without changing the scanner.
- Immediate cache invalidation/cancellation: would require a different cache ownership contract. Preserve the accepted cache and use the explicit `Stopping` semantics instead.

## 10. Decisions for Chris and recommended implementation order

Approve or revise these recommendations before implementation:

1. Foreground `daemon` command with explicit interval/freshness arguments and the limits above.
2. One acquisition worker plus main-thread coordination, with a narrowly reviewed `signal-hook` dependency subject to Rust 1.85.0 verification.
3. Cooperative shutdown: one admitted scan may complete/publish; no hard deadline, second-signal force policy or detached worker.
4. Lifecycle-aware currentness during stopping, without changing cache schemas or exposing a read API.
5. Constant-volume lifecycle messages, graceful-signal exit `0`, and nonzero fatal/reporting failures.

Implementation order, **only after approval**: first options/state-machine and controlled loop tests; then existing cache integration and ownership/lock tests; then signal/coordinator cleanup and isolated subprocess tests; finally minimal CLI wiring, full regression verification and a factual milestone report. If this requires changing cache semantics or an unexpectedly large dependency set, stop and request review rather than expanding scope.

Recommended next action is implementation review/authorization for this same Prototype 008 proposal. No Prototype 009 work is authorized.

## 11. Actual planning verification, preservation and Git status

[Alpha 1 CI run 44056335816](https://github.com/lgraak/Siliconwatch/actions/runs/44056335816) was inspected for exact contribution HEAD `d671b6684798b4aef83fe1f9fdae51531e2b3ed0`. Both configured jobs and their steps completed successfully:

- Rust 1.85.0 formatting, configured Clippy and tests: **76 executable Rust tests and 2 compile-fail doctests passed, 0 failed**.
- Release fixture build and strict Python suites: **153 passed, 0 failed, 0 skipped**.
- DCO checker tests: **6 passed, 0 failed**; contribution signoff verification succeeded.

These are observed hosted Prototype 007 results, not locally rerun tests or validation of an implemented daemon. The two previously documented supplemental all-target Clippy findings remain separate from configured CI. No new Cargo/Python runtime tests, application execution, hardware measurements or benchmark matrices were run for this documentation-only task. Local contribution DCO verification passed. Git ancestry, parent/tree equality, seven-file manifest, documentation links and whitespace were checked.

A separate `codex/alpha1-prototype008` worktree was created from verified `origin/main` at `28d7177cc334e357646023e5a2292b0cc434e0a7`. It has no upstream and no new commits. Its sole new file is this untracked proposal; the index is empty. No Prototype 008 files were pushed or submitted in a PR.

Both prior worktrees and intentional local modifications were preserved: the original checkout's 78-file baseline, including its Prototype 006 publication update, and Prototype 007's clean 75-file baseline. Historical audit found matching tracked benchmark artifacts, all 10 compressed result payloads, 21 recorded provider-source hashes and 78 recorded capture hashes. All 225 private evidence files remained excluded. Prototype 004 raw captures have only recorded-size checks, not per-capture historical hashes; do not overstate that audit as byte-identity proof. No original measurements, recovery records, or historical reports were edited.

## Revision history

| Date | Revision |
| --- | --- |
| 2026-11-17 | Initial proposal following verified regular merge of PR #6. Planning only; implementation awaits approval. |

## HANDOFF TO CHATGPT

Chris authorized the regular merge of SiliconWatch PR #6 and preparation of Prototype 008 planning only. PR [#6](https://github.com/lgraak/Siliconwatch/pull/6) is merged as **`28d7177cc334e357646023e5a2292b0cc434e0a7`**. Its parents are the expected old main `52f92916430d3e515fa39477802106fef4c3620c` and original DCO-signed contribution `d671b6684798b4aef83fe1f9fdae51531e2b3ed0`. The merge tree exactly matches the reviewed seven-file contribution; the contribution is reachable from fetched `origin/main`, which points to the merge. Feature branches were retained, and history and branch protection were preserved.

The exact contribution HEAD passed both configured GitHub Actions jobs in run **44056335816**: Rust 1.85.0 formatting, configured Clippy, 76 executable Rust tests and 2 doctests; 153 Python tests without skips; and 6 DCO checker tests plus contribution signoff verification. These hosted results were inspected, not inferred. Local DCO verification also passed. No daemon code, new runtime tests, hardware measurements or benchmark runs were executed during planning.

A separate worktree, `SiliconWatch-prototype008`, now holds branch **`codex/alpha1-prototype008`** at the merge. Its only change is the untracked `docs/milestones/008-headless-daemon-lifecycle-proposal.md`; the index is empty, with no new commit, push, upstream or PR. Both earlier worktrees and their unrelated modifications remain preserved, including the intentional Prototype 006 report update. Historical evidence hashes and exclusions were audited without changing the evidence; Prototype 004 raw-capture checks remain limited to recorded sizes where historical hashes do not exist.

The proposal recommends a foreground Rust `daemon` command with explicit polling/freshness settings, one acquisition worker owning the existing cache, main-thread signal coordination, and standard-library mutex/condition-variable control. A narrowly scoped `signal-hook` dependency is proposed, pending approval and exact lockfile/MSRV verification. Startup, running, stopping and stopped are separate from sensor health. Recoverable acquisition failures retain explicit historical data; fatal errors stop the service.

The central limitation is cooperative shutdown: an already admitted acquisition may finish and publish after stopping begins. Service-level assessment must treat observations as non-current during stopping. `Stopped` requires owner cleanup and a joined worker. Blocking drivers, held cache views and blocked output prevent a hard shutdown-time guarantee. Deterministic tests are specified for admission races, timing, failures, recovery, signals, locking and cleanup.

Chris should review the command, dependency, output/exit policy and shutdown contract before authorizing implementation. APIs, GUI, production deployment, systemd installation and hardware controls remain excluded; Decision 43 remains Pending. The next action is approval of this proposal, not automatic implementation or another milestone.
