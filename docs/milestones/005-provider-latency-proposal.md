# Alpha 1, Prototype 005: Provider-Level Sensor Latency Characterization

**Date:** 2026-11-16

**Status:** PROPOSAL — merge and planning complete; implementation and measurements await Chris's separate approval.

**Historical planning record — 2026-11-16:** The status above describes the original proposal. Implementation and bounded measurement execution were subsequently authorized as recorded below and in Decision 43. Chris has now accepted Prototype 005 with documented limitations; the [milestone report](005-provider-latency-characterization.md#formal-acceptance-and-publication-review--2026-11-16) is the current acceptance record. This proposal's original requirements and authorization history remain intact. Acceptance does not authorize publication, additional runs or Prototype 006.

**Implementation authorization — 2026-11-16:** Chris subsequently approved the external Python diagnostic and the initial protocol below: at most five registrations, three repetitions, 301 attempts at 100 ms, 21 primary cases plus six smoke cases, and global 25-minute / 256 MiB limits. The historical proposal text is preserved. No 300-second extension, in-scan instrumentation, provider abstraction, application behavior change, publication, or Prototype 006 work is authorized. Actual implementation and measurement outcomes will be recorded in the separate Prototype 005 milestone report.

**Fresh-run authorization — 2026-11-16:** The first attempt stopped after two smoke cases because temporary alias cleanup failed; no primary cases started. After a fixture-reproduced cleanup correction and 126 passing harness tests, Chris explicitly approved a fresh six-smoke / 21-primary run with its own unchanged 25-minute / 256 MiB limits. This permits eight smoke cases in total across the retained preliminary attempt and fresh run. Original evidence remains separate and unchanged.

## 1. Milestone summary and verified merge

Chris authorized final verification and a regular merge of [PR #3](https://github.com/lgraak/Siliconwatch/pull/3), followed by this local proposal. GitHub recorded the merge at **2026-11-16 23:21:41 UTC**.

| Item | Verified commit |
| --- | --- |
| Previous `origin/main` | `3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7` |
| Prototype 004 implementation | `d8de603227c7fb9b88c784ad67e6dbf1b43665ba` |
| Prototype 004 publication report / expected PR HEAD | `565e1d58eed05c76be93abd779b01848f1d3604b` |
| **PR #3 regular merge / updated `origin/main`** | **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`** |

Final verification confirmed the exact expected HEAD, base `main`, only the two reviewed contribution commits, the intended 22-file diff, a clean local tree, and GitHub `MERGEABLE` / `CLEAN` status. Both contribution commits retain matching author DCO signoffs. All configured Actions checks passed for that HEAD. The merge was guarded by the expected HEAD and used `merge`, without squash, rebase, force-push, or protection changes.

After fetching, the merge's two parents were verified as the previous main and expected PR HEAD. Its tree is identical to the reviewed HEAD, and both contribution commits are reachable from `origin/main`. The merge also has a DCO signoff. Local and remote `codex/alpha1-prototype004` are retained. New local branch `codex/alpha1-prototype005` starts at the verified merge, with no upstream.

## 2. Objective and proposed scope

Investigate scan-time variability by comparing **individual hwmon registrations** with contemporary full scans. The earlier 503.365 ms maximum motivates this experiment; no evidence currently identifies its cause. A registration is one exposed hwmon device directory, not necessarily one physical chip, driver, or uniquely identified source. Provider names can repeat.

Recommend extending only the existing Python standard-library measurement tooling. Run the unchanged Rust 1.85.0 release binary through its existing `sample --json-lines` and `--hwmon-root` interfaces. Temporary local class directories contain symlinks to selected original registrations; no sensor files are copied and nothing is written in sysfs. No new runtime dependencies or Rust instrumentation are proposed for this first comparison.

Proposed deliverables are a small provider-selection/orchestration extension, deterministic fixtures, bounded private evidence, sanitized scalar results, reproducible Markdown, and an implementation milestone report. Exact wrapper filenames/flags remain implementation details; they do not exist yet. Reuse the process runner, parser, statistics, and provenance mechanisms from [Prototype 004](004-overhead-characterization.md).

Preserve without alteration:

- `pub fn scan(root: &Path) -> io::Result<Snapshot>` and all existing CLI/exit behavior.
- Snapshot schema 2, identity schema 1, and sampling envelope schema 1.
- Sensor and warning ordering, raw values, conversions, status semantics, and input-error precedence over optional flag errors.
- Sequential scheduling, reanchoring, observation/freshness fields, failed-attempt accounting, and output semantics.

No daemon, provider framework, plugin interface, cache, adaptive polling, database, API, GUI, alarms, hardware control, or protective actions are included. Decision 43 remains **PENDING**. Decision 8's eventual provider architecture does not require a general abstraction for this experiment.

## 3. Graphify findings and current source review

Graphify's query and strict-caller results still identify their indexed commit as `3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7`. They are not a newly indexed analysis of the merge. Direct source inspection and `git diff` against that commit confirm **no changes in `src/`, `tests/`, Cargo.toml, Cargo.lock, or CI** through the verified merge. The supplied findings therefore remain applicable.

| Finding | Current source evidence and implication |
| --- | --- |
| Direct one-shot production caller | [main.rs](../../src/main.rs#L315), `run()` at line 315, `hwmon::scan` call at line 345. |
| Sampling production caller | [sampling.rs](../../src/sampling.rs#L135), `run()` at line 135 passes a closure calling `hwmon::scan` at line 145; the CLI enters sampling at main.rs line 332. |
| Concentrated discovery and reading | [hwmon.rs](../../src/hwmon.rs#L64), public `scan` at line 64, registration/attribute traversal at lines 72–127, reading/conversion/flag logic at lines 136–239. |
| Identity and collision dependencies | `identity::for_device` at hwmon.rs line 91, channel identity at line 125, snapshot-wide `mark_collisions` at line 129. [identity.rs](../../src/identity.rs#L118) uses root-sensitive canonical evidence and examines retained sensor rows and their registrations for collisions; zero-input registrations do not participate. |
| Existing narrow selection seam | [main.rs](../../src/main.rs#L84) already parses `--hwmon-root`; [hwmon.rs](../../src/hwmon.rs#L248) sorts directory entries. No provider trait is needed to select a different class root. |

**Recommendation:** use the existing alternate-root seam. This measures a **selected-registration scan window**, including existing discovery and identity work, without modifying production call paths. No provider-interface refactor is justified by current evidence.

This approach cannot divide a single full scan into individual read costs. If later evidence requires timing registrations within the *same* full scan, propose a separate narrowly scoped observational seam with a separate diagnostic sink and equivalence tests. That would require approval; it is neither implemented nor part of this recommended initial scope. A general provider abstraction would require an additional demonstrated need beyond timing.

### Selection and identity safeguards

Create a private session manifest before measuring: map distinct sanitized run tokens to exact class entries, canonical targets, original registration names, available identity evidence, and ordered channel inventories. Names, labels, candidate strings, and volatile `hwmonN` numbers alone must never be treated as unique physical selectors. Ambiguous selection fails explicitly; do not choose the first name match. A session token is a diagnostic reference, not a persistent identity.

Keep alias basenames consistent with original class entries and preserve canonical targets/device links. Fixtures must place hardware directories outside both full and selected class roots, matching the supported identity topology. Renumbering or retargeting during a session invalidates the selection; do not silently rebind it. Read-only pre/post checks reduce but cannot eliminate replacement races without broader application changes.

Removing peers can change collision/ambiguity results. Compare and record full-root versus selected-root identity context; never interpret isolation as resolving physical identity. Alias root/source paths necessarily differ; canonical device paths and comparable sensor fields should remain equivalent in static fixtures. Do not normalize or overwrite actual emitted data to hide differences.

Record all providers, including zero-input, unavailable, unsupported, and unreadable entries. A missing/dangling registration can produce warnings and a successful empty snapshot in the existing reader. The harness must classify it as unavailable or incomplete comparison evidence, not a fast successful acquisition. Unexpected target/count changes or unreadable registration enumeration stop the affected comparison block. Stable individual sensor errors remain visible and separate from scan failures; an all-unreadable selection cannot establish usable acquisition timing. Do not load drivers, elevate privileges, or change hardware settings.

## 4. Proposed measurement protocol

All values below are proposed limits requiring approval. No hardware experiments were run while preparing this document.

1. Build once, outside measurement, with `CARGO_TARGET_DIR=target/overhead cargo +1.85.0 build --release --locked`. Record source/toolchain/binary hashes and the build command. Use one binary throughout; stop if it changes.
2. Capture the full inventory and choose at most five input-bearing registrations **before seeing latency results**. Keep unavailable/zero-input exclusions visible. Record OS/kernel/CPU, compiler/Python, counts, workload notes, governor/power observations and capture mode without changing settings. Do not infer host power state from peripheral batteries.
3. Prove alias selection with deterministic fixtures. Then run at most six hardware smoke cases: selected registrations plus the real full root, 11 attempts at 100 ms each. Validate presence, channel counts, output schemas, inventory stability, and projected resource use before measured cases.
4. Run three repetition blocks. Each block contains a full-scan reference, each selected registration once, then another full-scan reference. Rotate registration order deterministically and record actual order. Use the real full class root for controls. All cases use 301 attempts, 100 ms, JSON Lines, the same bounded pipe-to-file capture, and RSS observation disabled. Run only one child at a time, with at least 15 seconds settling between cases; record actual settling elapsed time.
5. Validate every retained record independently. Keep whole-run results and a ten-second post-warmup subset. Preserve requested/emitted/verified counts, failures, status/warning totals, scan windows, start gaps/lateness, freshness flags, actual elapsed time, child CPU, runner CPU and output bytes. Do not infer complete attempts from exit zero or requested count.
6. Retain incomplete runs/prefixes, stop on invalid selection, changed inventory, budget exhaustion or unsuitable conditions, and record why subsequent cases were not started. Do not replace unfavorable runs. Regenerate the report offline from sanitized results.

An individual child command remains the existing command, with a selected alias root or the full class root:

```text
target/overhead/release/siliconwatch sample --count 301 --interval-ms 100 --json-lines --hwmon-root <class-root>
```

### Execution, storage, and memory bounds

| Bound | Proposed value |
| --- | --- |
| Main cases | At most 21: 15 selected-registration cases plus six full-scan controls |
| Smoke cases | At most six, each with a 5-second timeout and 1 MiB combined output cap |
| Main case timeout/output | 40 seconds / 8 MiB combined stdout and stderr |
| Global execution budget | 25 minutes including smoke, preflight, settling and normal cleanup; do not launch work that cannot fit |
| Global disk budget | 256 MiB; worst capture allowance is `21 × 8 + 6 × 1 = 174 MiB`, plus a 64 MiB metadata/atomic-write reserve = 238 MiB |
| Record/case retention | At most 27 cases / 6,387 scalar records, including smoke; no accumulated raw telemetry in memory |
| Existing parser/capture bounds | 64 KiB capture chunks, one parsed line at most 4 MiB, 31 MiB compact result cap, 1 MiB report cap; metadata reserve includes atomic replacement |

The main nominal schedule is 15.5 minutes including twenty 15-second settling gaps, before smoke, delays and processing. Even maximum planned timeouts plus ordinary settling fit the proposed 25-minute budget; smoke projections must confirm headroom on the actual cohort. Metadata/private-manifest storage must fit the reserve, and every output path must be new. Original Prototype 004 data is preserved.

Memory is bounded by finite cases/records, streamed capture, line limits and scalar retention, **not a promised hard RSS ceiling**. Python object/JSON expansion differs from encoded size, and the unchanged reader's memory depends on one snapshot's hardware inventory. Do not claim an application memory limit or measured peak from those caps. Retain the existing bounded child-group termination/reaping strategy; blocking kernel/filesystem I/O can defeat a hard wall-time or cleanup guarantee. An unreaped child stops the suite; do not start another case or alter system configuration.

### Timing and measurement overhead

[sampling.rs](../../src/sampling.rs#L160) records monotonic start, converts timing offsets and reads wall time, invokes `scan`, then records finish. Consequently `scan_duration_ns = finished_ns - started_ns` includes the timing preamble, discovery, identity/collision work, sensor reads, possible driver waits, and descheduling. It is not pure sysfs-read or physical-sensor latency. Existing freshness bounds and wall-clock behavior remain unchanged.

Serialization/emission occurs after the measured window but before the next scan, so output/backpressure can influence later scheduling. Keep capture mode fixed, record differing byte volumes and runner CPU separately, and disable RSS observation consistently. Report child CPU versus elapsed window as separate evidence; neither is exact driver time. Do not subtract empty-root, full-minus-selected, or Prototype 004 control timings as an exact overhead correction. Do not sum isolated results into a full-scan estimate. Fixed costs, ordering, caches, collision context, output size and workstation interference differ.

### Long-tail reproduction protocol and evidence

Predeclare **scan window >100 ms** as a descriptive tail flag, not an alarm or performance budget. Preserve every record, per-run maximum/sequence, monotonic offsets, nullable wall timestamp, neighboring starts/gaps, counts/statuses/warnings and configuration. Keep bounded raw JSON Lines locally with content hashes; published reports use sanitized scalar records and run tokens. Do not retain only outliers or remove first attempts. Report median, maximum and N; p95 requires N ≥200, p99 requires N ≥1,000. Do not pool repetitions/providers to manufacture percentile eligibility.

Keep raw output and the path-to-token manifest in private ignored storage (directories mode 0700, files 0600); never automatically publish them. Share only allowlisted OS/kernel/CPU/toolchain information, configuration, counts, scalar timing/status evidence, and binary/source/data hashes. Omit hostnames, serials, network identifiers, absolute local/device paths and full identity candidates. Review operator notes and build-command text before export. Sanitized data must reproduce the numerical report without access to private paths.

Compare repeated selected windows with their contemporaneous bracketing full controls. Report recurrence, variability, non-recurrence and missing evidence explicitly. A slow selected window motivates investigation but does not prove it caused a full-scan tail or the historical maximum. Snapshot identity changes are diagnostic context, not permission to merge sources.

**The initial 30-second trials do not recreate the original 300-second run.** The original 503,365,198 ns observation occurred at sequence 2751; this experiment may miss duration-dependent effects. Success does not require reproducing it. If Chris wants matched sustained reproduction, prepare a separately approved extension with 3,001 attempts and corresponding repeated full/selected controls, runtime and larger capture budgets; a full JSON run can exceed the proposed 8 MiB cap. Do not automatically lengthen cases, rerun until an outlier appears, or claim non-reproduction disproves the earlier observation.

## 5. Acceptance criteria and required deterministic tests

| Area | Required evidence before hardware execution |
| --- | --- |
| Provider selection | Exact registration selection; duplicate names/labels and identity candidates; disjoint ambiguous registrations; enumeration/renumbering; session-token matching without arbitrary first-match behavior. |
| Missing/invalid selection | Missing, dangling, retargeted and unreadable registrations; empty/unknown/duplicate selections; zero-input providers; partial sensor permission failures. Preserve explicit unavailable evidence; empty exit-zero output is not success. |
| Reader equivalence | Full and selected fixture roots retain ordered channels, raw values, conversions, statuses and diagnostic precedence. Canonical targets remain unchanged; expected root/source-path and collision-context differences are asserted separately. No schema or public `scan` signature change. |
| Timing | Synthetic scalar fixtures verify durations, lateness, previous-actual-start arithmetic, observation bounds, wall-clock jumps and percentile eligibility. No precise real-time assertions or new production clock semantics. |
| Incomplete measurements | Timeout, interruption, malformed/truncated records, scan failure, missing provider mid-run and output errors preserve prefixes and unknown counts; no fabricated values or silent retry. |
| Bounds and cleanup | Capture cap across both streams, line/record/case/metadata limits, suite-budget exhaustion, atomic-write reserve, child termination/reaping, and no subsequent launch after an unreaped child. |
| Reproducibility | Fixed block order, consistent full controls, preflight cohort checks, sanitized manifests, preserved raw evidence, and byte-identical report regeneration from saved scalar data. |
| Regression | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, Rust 1.85.0 compatibility, all existing 67 harness tests plus new selection/orchestration tests, and existing one-shot/sampling regressions. |

Hardware acceptance requires valid comparable measurements for the available approved cohort and completion of its bounded smoke/comparison plan. Preflight exclusions must be recorded before finalizing that plan. Every attempted and unstarted case must be accounted for. An incomplete suite or no viable full/selected comparison means **Partial** (or **Blocked** if an external prerequisite prevents progress), not Complete. Inconclusive attribution after valid completed comparisons can still be **Complete**. Findings must distinguish observations, hypotheses and unknowns; no provider attribution or reproduction of 503 ms is required. No sysfs writes, driver changes, application optimizations or performance guarantees are permitted.

## 6. Actual verification and hardware status in this planning task

- Re-read project instructions, architecture and relevant milestone reports; reviewed source, call paths and Graphify's indexed-commit provenance.
- Re-fetched GitHub; verified expected base/head, reviewed commit/file set, clean pre-merge tree, signoffs, mergeability and ancestry. Local DCO checker passed for both contribution commits.
- Re-inspected [Actions run 43976173422](https://github.com/lgraak/Siliconwatch/actions/runs/43976173422) for exact HEAD `565e1d5…`: formatting and Clippy passed; **53 Rust tests passed, 0 failed, 0 ignored**, no conditional permission skips; **six DCO-checker tests passed**, and both contribution signoffs passed. These are observed hosted results for the reviewed HEAD, not newly executed tests on this proposal.
- Verified merge parents, identical merge/reviewed trees, preserved contribution ancestry and retained feature branch. Current source still matches Graphify's analyzed source revision.
- Documentation checks passed: relative links and source-line anchors resolve, Markdown fences/whitespace are valid, and only the intended proposal is untracked. `git diff --check` passed; the new file was checked separately because untracked files are excluded by that command. Post-merge DCO verification also passed for all three new commits, including the merge.
- No local Cargo suite, new fixture tests, hardware scan, or benchmark was run in this planning task. No runtime code changed; additional regression runs would not validate the proposed, unimplemented diagnostic. The prior 67 local harness tests are recorded in Prototype 004 and were not rerun here.
- The last verified hardware cohort in Prototype 004 was 11 inputs across five input-bearing registrations plus a zero-input provider; IT8613/fan inputs were absent. This is historical context, not a fresh availability claim. Future preflight must rediscover the cohort.

## 7. Risks, approval decisions, and recommendation

Known limitations are the root-sensitive identity context, non-additive costs, timestamp/discovery overhead within scan windows, no hard deadline or RSS ceiling, possible blocking reads, and one-machine coverage. Background workload, thermal/frequency drift, driver caches and observer/output effects may obscure differences. Registration retargeting races and privacy-sensitive raw paths require explicit handling. No confirmed provider cause for the historical outlier is known.

Recommend approving the external alias-root diagnostic first, with the proposed **21 measured + six smoke cases, 25-minute / 256 MiB limits**, and treating inconclusive attribution as a valid result. Chris must approve or revise these bounds and accept the short-window limitation before implementation/measurement. In-scan instrumentation, any general provider abstraction, or a matched 300-second extension requires a separate proposal and approval. No Prototype 006 is proposed or started.

Only this proposal file is added on `codex/alpha1-prototype005`. HEAD remains merge `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`; there is no upstream. The proposal is untracked/unstaged and has not been committed or pushed. Existing code, data, reports, dependencies, workflow and local benchmark evidence remain unchanged. No Prototype 005 PR exists.

## Revision history

| Date | Change |
| --- | --- |
| 2026-11-16 | Verified and regularly merged PR #3, retained history/branch, created the local Prototype 005 branch, checked Graphify against current source, and prepared this uncommitted proposal. |

## HANDOFF TO CHATGPT

SiliconWatch PR #3 was verified and merged using a regular merge commit: **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**. The original implementation commit `d8de603227c7fb9b88c784ad67e6dbf1b43665ba` and publication commit `565e1d58eed05c76be93abd779b01848f1d3604b` remain unchanged and reachable from updated `origin/main`, with their DCO signoffs. The merge also has a signoff. Both local and remote Prototype 004 branches were retained; no squash, rebase, force-push or protection change occurred.

Final verification confirmed the expected PR HEAD, base `main`, reviewed 22-file diff, two contribution commits, no divergence and GitHub mergeability. Actions run 43976173422 passed formatting, Clippy, all 53 Rust tests, six DCO-checker tests and both contribution signoffs for that exact HEAD. No tests failed or were ignored, and no conditional permission skips were observed. The fetched merge tree exactly matches the reviewed tree. Local Cargo tests and hardware measurements were not rerun for this documentation-only planning task.

New local branch `codex/alpha1-prototype005` starts at the merge, without an upstream. Its only change is the untracked, unstaged proposal `docs/milestones/005-provider-latency-proposal.md`. Nothing from Prototype 005 has been committed, pushed, implemented or measured. Existing application behavior and original benchmark evidence remain unchanged.

The proposal recommends an external Python standard-library diagnostic using the unchanged release executable and existing `--hwmon-root` option. Temporary local class roots select exact hwmon registrations through symlinks. No general provider-interface refactor or Rust instrumentation is needed for the initial comparison. Graphify still reports its analyzed commit as `3ccde79`; direct source comparison confirms the relevant source is unchanged. Its conclusions remain valid: `main::run` and `sampling::run` call `hwmon::scan`, with discovery/read logic in hwmon.rs and downstream identity/collision processing.

Proposed measurements cover at most five registrations, three repetitions, and full-scan controls before and after each block: 21 measured cases plus at most six smoke cases, within 25 minutes and 256 MiB. Main cases use 301 attempts at 100 ms and JSON Lines. Deterministic tests must cover selection/ambiguity, missing or invalid providers, timing arithmetic, incomplete records, output/resource bounds, cleanup, report regeneration and unchanged reader/CLI semantics.

Existing timing includes timestamp overhead, discovery, identity work, reads and descheduling. Results are selected-registration scan windows, not pure driver latency; isolated timings must not be summed or used as causal attribution. Preserve long-tail records and adjacent context, report eligible statistics, and never retry to chase 503 ms. Thirty-second trials do not reproduce the original 300-second conditions; failure to reproduce the maximum is inconclusive and does not fail acceptance. Blocking I/O, identity-context changes, workload drift and memory-accounting limits remain explicit.

Recommend Chris approve or revise the initial protocol and resource bounds before implementation. A sustained extension or in-scan instrumentation needs separate approval. Decision 43 remains pending. Prototype 005 implementation and Prototype 006 have not begun; stop for review.
