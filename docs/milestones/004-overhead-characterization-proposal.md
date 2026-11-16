# Alpha 1, Prototype 004: Reproducible Sensor Monitoring Overhead Measurements

**Date:** 2026-11-16

**Publication authorization — 2026-11-16:** Chris subsequently approved final review, a DCO-signed commit, normal feature-branch push, and one PR targeting `main`, conditional on verification. The earlier authorization/status statements below describe their historical stage. Merging and Prototype 005 implementation remain unauthorized. See the [current milestone report](004-overhead-characterization.md) for publication evidence.

**Implementation authorization — 2026-11-16:** Chris approved the measurement harness and staged verification/execution described in the subsequent task. The original proposal below remains a historical planning record. The implementation uses bounded pipe-to-file capture instead of direct regular-file stdout so it can enforce a hard combined output limit without a launcher. See [reproducibility instructions](../benchmarks/README.md) and the [implementation report](004-overhead-characterization.md) for actual behavior, budgets, deviations, and results. No commit or publication of Prototype 004 is authorized.

**Status: PROPOSAL — awaiting Chris's approval.** Merge and planning are complete; no Prototype 004 implementation, measurement script, benchmark run, or performance result is claimed.

## 1. Milestone summary and verified merge

Chris authorized merging [PR #2](https://github.com/lgraak/Siliconwatch/pull/2) after exact-head verification, then preparing this proposal on a new local branch. GitHub recorded the regular merge at **2026-11-16 21:01:46 UTC**.

| Item | Commit |
| --- | --- |
| Main before merge | `b918a2bc3618dbe77d6c6ce3656c9d6d19afa66d` |
| Prototype 003 implementation | `80d724df05cf2f622b333a1bacef1f26e96ee435` |
| Publication report / expected PR HEAD | `897edebfe23ff23ab6f694caa81243ec1f2bfc01` |
| **PR #2 regular merge** | **`3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7`** |

Final verification confirmed base `main`, the exact expected HEAD, two signed-off contribution commits, the reviewed eight-file diff, a clean working tree, no divergence, and GitHub `MERGEABLE`/`CLEAN` status. The configured Rust and DCO jobs passed for that exact HEAD. The merge has the former main and expected HEAD as its two parents, and its tree is identical to the reviewed tree. Both original commits remain reachable from `origin/main` with unchanged DCO signoffs; the merge also has a DCO signoff. No squash, rebase, force-push, branch deletion, or protection change occurred.

The existing Prototype 003 branch is retained. `codex/alpha1-prototype004` was created locally from updated `origin/main`, at the merge above, without an upstream.

## 2. Objective, proposed deliverables, and tooling

Establish a repeatable baseline for the **unchanged foreground CLI** before designing background monitoring. Measure process CPU use, peak and observed post-warmup memory, scan duration, scheduling deviation, and output effects. Results describe the recorded machine/workload; they are not performance guarantees, production budgets, or evidence that background monitoring is ready.

Proposed implementation deliverables, only after approval:

- A small Python standard-library runner and offline summarizer, with deterministic fixture tests.
- A documented command matrix, environment manifest, and machine-readable scalar results sufficient to regenerate a human-readable Markdown report.
- One reviewed benchmark report under `docs/benchmarks/`, clearly separating measured observations, incomplete cases, and recommendations. Raw telemetry stays local and is not committed automatically.

Only this proposal is created now. No application source, dependencies, tests, workflow, README, accepted decisions, or existing reports are changed.

### Tooling choice

| Option | Assessment |
| --- | --- |
| **External Python standard-library runner — recommended** | Measures the real release executable, including discovery, sleeping, formatting, and output. Can collect child accounting and read procfs without changing the application or adding packages. |
| Rust benchmark harness | Useful later for isolated parsing/identity algorithms, but a microbenchmark does not characterize the current sleeping CLI and output path. Cargo's built-in `#[bench]` requires nightly; stable alternatives add tooling/dependencies. Not needed for this baseline. |
| GNU time / pidstat / perf / strace | Potential optional diagnostics, not baseline dependencies. They are not installed in the inspected environment; do not install or require them automatically. |

Python **3.14.10** is available; the current default Rust/Cargo is **1.100.0**. The initial comparison should use a single prebuilt **Rust 1.85.0 release binary with locked dependencies**, matching the tested minimum. Record the exact toolchain, target, flags, binary hash, and Git revision. Do not add a compiler-comparison factor to this milestone. Cargo documents the [benchmark harness limitations](https://doc.rust-lang.org/cargo/commands/cargo-bench.html).

### Proposed commands, not executed in this task

Build outside the measured interval and then run the executable directly; do not include Cargo startup or compilation in application measurements:

```bash
CARGO_TARGET_DIR=target/overhead cargo +1.85.0 build --release --locked

# Short, fast-interval examples of existing application commands:
target/overhead/release/siliconwatch sample --count 301 --interval-ms 100
target/overhead/release/siliconwatch sample --count 301 --interval-ms 100 --json-lines

# Proposed future runner interface; this script does not exist yet:
python3 scripts/measure_overhead.py --binary target/overhead/release/siliconwatch \
  --matrix baseline --repeats 3 --output-dir target/overhead-results/run-001
python3 scripts/measure_overhead.py --summarize target/overhead-results/run-001
```

The runner would record the complete argv and settings for every child so an individual case can be reproduced without relying on the matrix preset. Use argument arrays, no shell interpolation. A fresh output directory must prevent overwriting prior evidence.

## 3. Proposed workload and measurement contract

### Primary matrix

Each row runs in **table and JSON Lines** format, with **three independent process repetitions** per format. All primary output goes to regular files on the same recorded filesystem; parsing happens after the child exits.

| Nominal sampling span | Interval | Attempt count | Repetitions per format |
| --- | ---: | ---: | ---: |
| Short: 30 s | 100 ms | 301 | 3 |
| Short: 30 s | 1,000 ms | 31 | 3 |
| Sustained: 300 s | 100 ms | 3,001 | 3 |
| Sustained: 300 s | 1,000 ms | 301 | 3 |

This is **24 primary runs**, approximately **66 minutes** before settling, setup, and additional scan/output delays. Count is `nominal_span_ms / interval_ms + 1`, because the first attempt is immediate. All counts fit Prototype 003's limits. Report actual elapsed duration: the nominal span is neither an exact duration nor a timeout.

Use a fixed, published balanced order across repetition blocks, reversing/rotating condition order to reduce systematic warmup/order bias. Run one child at a time. Allow at least 15 seconds between cases, record actual settling time and workload conditions, and retain all attempted runs. Do not discard slow or failed runs merely to improve a percentile.

Preserve whole-run/startup measurements. For post-warmup summaries, exclude the first **10 seconds**, using application monotonic offsets for scan records and runner monotonic offsets for RSS observations. Record both origins and actual included sample counts. This is an operational observation window, not proof of thermodynamic or allocator steady state. Report early/late portions of the sustained RSS window separately; visible growth or drift must not be hidden behind a single median.

### CPU and peak memory

Launch the binary as the runner's direct child, preferably with `os.posix_spawn` and explicit file descriptors. Use one child-reaping path through **`os.wait4`** to collect user/system CPU time, exit/signal status, and Linux peak RSS. Do not let another wait/poll API consume the status first, and do not subtract cumulative `RUSAGE_CHILDREN` high-water marks. Python documents [per-child wait accounting](https://docs.python.org/3/library/os.html#os.wait4).

Report per run:

- User CPU seconds, system CPU seconds, and their sum.
- `100 × (user + system) / monotonic_elapsed_seconds`, with **100% meaning one logical CPU**, not the whole machine.
- CPU milliseconds per attempt only for runs whose complete emitted sequence validates the attempt count, alongside requested/emitted/failed counts. For interrupted, truncated, or discarded-output runs, actual attempts and CPU per attempt are unknown/N/A: an extra scan may have begun without emitting a complete record. Keep total CPU and observable counts; do not divide only by successful readings and conceal failures.
- Peak RSS in KiB from Linux child accounting, clearly separated from virtual memory, sampled RSS, and runner memory.

CPU totals include process startup, discovery, formatting, and writes. They exclude the external runner, terminal-emulator process, and unrelated/asynchronous kernel work; they are not total machine overhead. Peak accounting can include launch/exec history. Validate launch attribution with known synthetic children before calling the metric an application-process peak; report any unresolved launch contribution instead of subtracting a guessed constant. Linux documents [RSS units and resource-accounting lifetime](https://man7.org/linux/man-pages/man2/getrusage.2.html).

### Observed steady-window memory and observer effects

Sample child RSS at **1 Hz**, storing actual monotonic observation times and read duration. Prefer `Rss` from readable `/proc/<pid>/smaps_rollup`; it costs more to read than approximate `/proc/<pid>/status` `VmRSS`. If only VmRSS is available, label it approximate and keep that method consistent across a comparison. Never silently mix methods or turn unavailable samples into zero. Procfs memory accounting and mapping summaries are described in the [kernel documentation](https://docs.kernel.org/filesystems/proc.html).

Use only the benchmark child's PID; confirm its identity/start time, stop on exit, and handle exit races. Avoid sampling the runner or a reused PID. Report post-warmup RSS median, minimum, maximum, eligible quantiles, sample count, missing observations, and early/late-window medians. A 1 Hz observation series can miss transient allocations; the lifetime peak is a separate metric. Sampling phase may interact with 1 Hz application sampling, so publish the cadence/phase and avoid claiming an unbiased per-scan allocation profile.

Measure the runner's own CPU and sampling-read durations separately. Add **three 30-second, 100 ms runs per format with RSS observation disabled**, using the same file sink, to compare with the matching primary cases. Quantify perturbation as a paired comparison with variability; do not subtract it as an exact correction. A noisy or materially perturbed result is provisional and requires a revised protocol, not an application optimization in this milestone.

### Scan duration and scheduling deviation

Parse existing fields without adding instrumentation to production code. JSON Lines is the primary structured evidence; table runs use a version-specific parser for the existing sample/timing headers. Validate the parser against fixtures, including escaped labels resembling headers. Reject missing, duplicate, ambiguous, or malformed records explicitly.

For each run, record scan duration, start lateness, actual start-to-start gap, actual gap minus requested interval, and counts of attempts/errors/window-exceeds-interval flags. Cross-check record numbering and timing arithmetic. Separate snapshot-returning and failed-attempt latency distributions; error paths must not make normal scans look faster. Per-sensor unsupported/error counts remain visible. Exit 0 alone is not a successful-telemetry criterion.

Scan duration brackets discovery and sequential reads; it excludes emission and does not measure each physical sensor's update time. Gap/lateness include effects of previous output, sleeping, and scheduling. The first record's lateness is startup-relative and should be reported separately from subsequent interval deviation.

### Output and terminal separation

The primary common-file sink compares the two **formats** under the same nonterminal conditions. Record filesystem, output bytes, stderr bytes, and flush/write behavior. File output measures buffered writes/page-cache effects, not guaranteed durable-storage completion.

Add **three 30-second, 100 ms runs per format to `/dev/null`, with RSS observation disabled**, matching the observer-disabled file controls. These are resource-only controls: because output is discarded, record completeness/latency cannot be independently parsed, so do not invent those measurements. Process CPU still includes formatting and writes to the sink. The two control groups add approximately **six minutes**, making the mandatory proposed matrix about **72 minutes before settling**.

Optional, separate visible-terminal trials: three 30-second, 100 ms runs per format in an identified terminal, with dimensions, wrapping, scrollback settings, and concurrent workload recorded. Require Chris's approval/cooperation for the visible terminal and avoid changing its settings silently. If only a PTY drain is available, label it **PTY transport only**, not terminal rendering. Do not pool terminal results with file/null results or attribute renderer CPU to SiliconWatch. Report rendering cost unavailable unless separately measured reliably. These comparisons characterize output effects; they cannot exactly subtract pure acquisition CPU from formatting, kernel I/O, or rendering.

### Provider-level latency, where practical

No production per-provider instrumentation is proposed. Optional diagnostic runs may use existing `--hwmon-root` with a temporary directory containing a symlink to one currently registered hwmon device. No real sysfs write, driver load, or provider filtering change is permitted.

Cap this exploratory phase at **three input-bearing providers selected and recorded before testing**, three repetitions each, 101 attempts at 100 ms, JSON Lines to file (about ten seconds per run), with a declared two-second exclusion for diagnostic summaries. Label results **isolated-provider scan latency**, which includes that root's discovery/identity/reading work. Changed collision context, warm driver caches, and fixed costs make these measurements non-additive: do not sum them as full-scan attribution or claim per-sensor physical latency. Validate alias behavior using fixtures first. If isolation changes evidence unexpectedly or cannot be established safely, omit the experiment and mark provider attribution unavailable. This optional phase adds roughly 90 seconds plus setup/settling and needs Chris's approval.

## 4. Statistical reporting and reproducibility

Report **per-run** distributions first, with units, included/excluded counts, error counts, and warmup policy. Use the ordinary median (average the middle pair for even N); define upper quantiles by nearest rank `sorted[ceil(p × N) - 1]`. Test this definition on fixed datasets and boundaries.

Conservative reporting rules for this proposal:

- Median and maximum whenever N is nonzero; otherwise N/A.
- p95 only for **N ≥ 200**; p99 only for **N ≥ 1,000**. Otherwise show N/A with the reason rather than a misleading tail estimate.
- These sample-count rules are heuristics, not confidence guarantees. Samples can be autocorrelated and conditions can drift.
- At 1,000 ms, a sustained run will normally have too few post-warmup samples for p99. This is expected; do not increase scope or pool runs just to fill that cell.
- Across three independent process repetitions, report the median and range of per-run CPU/RSS/latency summaries and N=3. Do not calculate a credible run-to-run p95/p99 from three values. Do not pool different providers, formats, intervals, failure outcomes, or conditions into one tail distribution.

Record OS/distribution, kernel, CPU model/architecture/logical CPU count, RAM capacity, Rust/Cargo/Python versions, binary hash, Git revision, release flags, provider inventory/input categories/status counts, interval/count, output sink, measurement method/cadence, run order, power source, observed governor/frequency policy, affinity/CPU quota if present, and relevant background activity. Use a small explicit metadata allowlist; omit hostnames, serials, MAC addresses, full environment dumps, and unnecessary source identity paths.

Record conditions without modifying drivers, governors, affinity, power policy, configuration, or dropping caches. Keep AC/battery state and workload comparable where practical, with operator agreement; disclose uncontrolled factors. Warm page caches are allowed and should be described. Detect inventory changes between runs and separate cohorts or mark comparisons invalid. In particular, do not combine the historical 24-input IT8613 cohort with the currently observed 11-input cohort.

Store sanitized numeric per-attempt/per-process observations and a manifest in a local result directory so the Markdown tables can be regenerated offline. Keep raw stdout/stderr private/local by default; never automatically upload or commit hardware paths. Record output size and a chosen storage budget before running; on budget exhaustion stop the suite and mark any interrupted case incomplete. Do not silently truncate successful records. No persistent monitoring database or historical service is introduced by static benchmark artifacts.

## 5. Acceptance criteria and required tests

Implementation acceptance, only after authorization, requires:

1. **Reproducible commands:** release build plus documented runner/individual case commands, exact Git/binary/toolchain provenance, and regeneration of the Markdown report from saved sanitized results.
2. **Repeated matrix:** all 24 primary cases and the 12 output/observer controls attempted in recorded order, with three process repetitions per condition; incomplete/invalid cases disclosed instead of fabricated or silently replaced.
3. **Defined conditions/bounds:** counts/durations/cadences/warmup fixed as above, actual duration and counts reported, environment/provider inventory captured, and no system or application changes made to improve results.
4. **Metrics:** per-child user/system CPU, explicitly normalized CPU percentage, peak RSS, sampled post-warmup memory with method/coverage, and per-run scan/deviation distributions. Missing metrics remain unavailable rather than zero.
5. **Noise and comparisons:** observer-enabled/disabled and common-file/null comparisons, per-run variation and quantile eligibility, and documented workload, thermal, clock, kernel, cache, allocator, filesystem, and output effects. Terminal/provider diagnostics are separately labeled optional or unavailable.
6. **Deterministic harness tests:** fixed JSON/table fixtures, missing/error/truncated/duplicate/malicious-looking headers, schema mismatch, quantile small-N/even-N/tail boundaries, unit/CPU normalization, memory-method labeling, timeout/interruption/output-budget handling, and known synthetic child accounting/exit races. A synthetic allocation child checks attribution without treating memory allocator behavior as an exact byte oracle.
7. **No application regression:** no changes to discovery, identity, sampling, timing, output formats, or exit semantics. All existing 53 Rust tests, formatting, and Clippy pass on Rust 1.85.0; run ordinary one-shot and short sample smoke checks. Harness tests must not write to real sysfs or depend on host-specific values.
8. **Human-readable results:** summary matrix, per-run tables with N, measured versus unavailable fields, uncertainty/noise, observed limitations, and one evidence-based recommendation. No invented CPU/memory/frame-time thresholds or universal performance claim.

Finite attempt counts do not prevent a blocked driver/output stream from hanging. A runner may stop launching further cases and record interruptions; any optional time limit must target only its own benchmark child and report incomplete evidence, never a successful shortened run. Do not promise that userspace cancellation can promptly terminate uninterruptible kernel I/O. The exact timeout/storage-budget values belong in the approved runner plan before execution.

## 6. Architecture, constraints, and risks

This is external characterization of the merged foreground prototype, not a monitoring redesign. It advances measurement evidence relevant to [Decision 23](../DESIGN_DECISIONS.md), without adding CI performance gates or promising approved budgets. The proposed harness uses no third-party Python packages or new Rust dependencies. **Decision 43 remains PENDING.**

Explicitly excluded: background daemon/service, adaptive polling, sensor cache, persistent database, GUI, API, alarm engine, hardware control, protective shutdowns, and any change to current discovery/identity/sampling/timing/output semantics. Optional provider roots use an existing input option rather than changing those semantics; their different dataset/context must remain visible.

Risks include blocking sysfs/output, thermal/frequency drift, competing workloads, timer/suspend behavior, driver caching, sysfs/provider availability, process-launch/accounting contamination, approximate or intrusive RSS observation, correlated samples, small-N tails, and terminal/file-system effects. Measurements on one machine cannot establish physical sensor accuracy, system-wide/gaming overhead, universal scaling, or background-daemon readiness. No benchmark result exists yet.

## 7. Actual verification and Git status for this task

- [CI run 43954579572](https://github.com/lgraak/Siliconwatch/actions/runs/43954579572) was verified against exact PR HEAD `897edebfe23ff23ab6f694caa81243ec1f2bfc01`. Formatting and Clippy passed; **53 Rust tests passed, zero failed or ignored**, with no conditional permission skips. **Six DCO checker tests passed**, and both contribution signoffs passed. These are observed prior hosted results, not newly rerun local Cargo tests.
- Local DCO verification passed for both contribution commits during merge preflight. Fresh fetch, exact history/diff checks, mergeability, successful merge, post-merge ancestry/tree equivalence, and branch retention were verified.
- Only tool availability/version inspection occurred for the measurement plan. No benchmark, new hardware scan, or performance validation was executed. Prior Prototype 003 hardware evidence remains historical; IT8613's current-boot absence was already investigated without a software regression finding.
- Proposal validation passed: local Markdown links resolve, fenced blocks are balanced, and whitespace checks pass, including the untracked file. A separate read-only methodology review confirmed matrix arithmetic and identified the incomplete-run CPU denominator limitation now documented above.
- Branch: `codex/alpha1-prototype004`, local only with no upstream; HEAD `3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7`.
- This proposal is the only working-tree addition, untracked and unstaged. No Prototype 004 commit, push, or PR exists. No application files or previous reports were modified.

## 8. Recommendation and approval decisions

Recommend approving the minimal external-runner approach after reviewing: **(a)** the 100/1,000 ms and 30/300 s matrix with three repetitions, approximately 72 minutes before settling; **(b)** the 1 Hz RSS method/fallback and observation window; **(c)** optional visible-terminal and isolated-provider trials; and **(d)** runtime interruption and result-storage budgets before execution. No new architecture or performance targets need approval as part of this proposal.

If approved, implement only the runner, tests, reproducibility instructions, and benchmark report, preserving the merged Rust binary's behavior. Use the evidence to propose the next small change afterward. Do not begin Prototype 004 implementation or publish this planning document without separate authorization.

## Revision history

| Date | Change | State |
| --- | --- | --- |
| 2026-11-16 | Verified and regularly merged PR #2, retained its history/branch, and prepared the overhead-characterization plan on a new local branch. | Proposal only; implementation and planning publication await approval |

## HANDOFF TO CHATGPT

Chris authorized merging SiliconWatch PR #2 after exact-head verification and preparing a Prototype 004 proposal only. PR #2 is now merged using regular merge commit **3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7**. The preserved implementation commit is **80d724df05cf2f622b333a1bacef1f26e96ee435** and the publication-report commit is **897edebfe23ff23ab6f694caa81243ec1f2bfc01**. Both retain DCO signoffs and are reachable from origin/main. The merge has the expected two parents and exactly the reviewed tree; it also contains a signoff. The Prototype 003 branch was retained. No squash, rebase, force-push, or protection change occurred.

Before merging, GitHub confirmed base main, the unchanged expected HEAD, the reviewed eight-file diff, no divergence, and MERGEABLE/CLEAN status. Exact-head CI run 43954579572 passed formatting, Clippy, all **53 Rust tests**, six DCO checker tests, and both contribution signoffs, with no Rust failures, ignored tests, or conditional permission skips. Local DCO verification passed again. These are observed CI results; no new local Cargo suite or benchmark was run for this planning task.

The new local branch is **codex/alpha1-prototype004**, with no upstream, based on the verified merge. Its only pending change is **docs/milestones/004-overhead-characterization-proposal.md**, untracked and unstaged. No Prototype 004 commit, push, PR, or implementation was created.

The proposed milestone is Reproducible Sensor Monitoring Overhead Measurements. It favors a Python standard-library runner around the unchanged Rust 1.85 release binary over Rust microbenchmarks or additional tools. Python is available; external time, pidstat, perf, and strace were not found. Build costs are excluded. The proposed primary matrix compares 100/1,000 ms intervals, table/JSON Lines, nominal 30/300-second spans, and three repetitions: 24 runs, roughly 66 minutes. Observer/output controls bring the planned total to about 72 minutes before settling; terminal/provider diagnostics are optional.

Measure per-child CPU and peak RSS, sample RSS at 1 Hz, and parse existing scan duration/lateness fields. Retain whole-run totals and use a declared ten-second exclusion for post-warmup summaries. Report per-run median, maximum, and sample counts; p95 requires at least 200 observations and p99 at least 1,000 as explicit heuristics, not guarantees. File/null and observer-off comparisons expose output and measurement effects without claiming exact acquisition/renderer CPU separation.

Acceptance requires reproducible commands/provenance, repeated controlled cases, deterministic parser/accounting/statistics tests, explicit unavailable results, unchanged application semantics and passing regressions, and a sanitized reproducible Markdown report. Blocking I/O, process-accounting/observer noise, workload/thermal drift, correlated samples, and current provider availability remain limitations. No performance result or universal budget is claimed. Decision 43 remains pending.

Chris should approve or revise the matrix/runtime, RSS method, optional diagnostics, and interruption/storage budgets before implementation. No daemon, cache, adaptive polling, database, API, GUI, alarms, controls, or shutdowns are proposed. Stop for review.
