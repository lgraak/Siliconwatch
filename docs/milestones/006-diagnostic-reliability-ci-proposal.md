# Alpha 1, Prototype 006: Diagnostic Harness Reliability and CI

**Date:** 2026-11-16 (America/Los_Angeles). **Status: PROPOSAL — planning complete; implementation not authorized.**

Chris authorized merging accepted Prototype 005 after final verification and preparing this proposal only. No Prototype 006 code, workflow or test changes have been made. Approving this document later would authorize only its narrowly scoped diagnostic reliability and CI work; hardware measurements and further milestones remain outside scope. Decision 43's local API permissions policy remains **Pending**.

## 1. Verified Prototype 005 merge

[PR #4](https://github.com/lgraak/Siliconwatch/pull/4) merged at **2026-11-17 02:01:00 UTC** (2026-11-16 local date).

| Item | Verified value |
| --- | --- |
| PR base / head | `main` / `codex/alpha1-prototype005` |
| Expected and actual contribution HEAD | `dacda42aa941df18c5a32ddf9da9958c4f5b863d` |
| Previous `origin/main` | `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e` |
| **Regular merge commit / updated `origin/main`** | **`9a190176fcaf2b697e0e2692a1847efd60ab3002`** |
| Contribution diff | Exactly the approved 20-file [publication manifest](005-publication-readiness.md#exact-intended-publication-manifest); one contribution commit |
| DCO | Matching contribution author/signoff; local checker passed; merge message also retains a signoff |
| GitHub merge eligibility | MERGEABLE / CLEAN; regular merge enabled, automatic branch deletion disabled |
| Configured Actions | [Alpha 1 CI run 43998394403](https://github.com/lgraak/Siliconwatch/actions/runs/43998394403): both Rust 1.85.0 and DCO jobs successful for the expected head |
| Post-merge ancestry | Two parents exactly previous main and contribution HEAD; contribution reachable from `origin/main` |
| Post-merge content | Merge tree equals the approved contribution tree |
| Branch retention | Local and remote `codex/alpha1-prototype005` still point to the contribution HEAD |

The merge used a HEAD guard and `merge`, without protection overrides, squash, rebase or force-push. Additional external Graphify checks reported Success and Neutral (Formal Verification); those are distinct from the two configured GitHub Actions jobs, both of which passed. No unexpected commits, local changes or remote divergence were found.

## 2. Objective and boundaries

Correct the confirmed progress-status persistence defect in the Python provider diagnostic and add the existing Python diagnostic suites to deterministic CI. Preserve acquisition records, raw evidence, cleanup ownership and accepted Prototype 005 provenance.

Relevant context: [accepted Prototype 005 report](005-provider-latency-characterization.md), [observations](../benchmarks/005-provider-latency-results.md), [Prototype 004 harness](004-overhead-characterization.md), and [design decisions](../DESIGN_DECISIONS.md). This is a diagnostic-tooling change, consistent with read-only monitoring, explicit failure states, privacy-first diagnostics and DCO provenance. No provider abstraction, general logging framework or new dependency is proposed.

Expected implementation files, subject to the smallest necessary patch:

- [Provider orchestrator](../../scripts/measure_provider_latency.py) and [its fixture tests](../../scripts/tests/test_measure_provider_latency.py).
- [Provider report renderer/tests](../../scripts/provider_results.py) and [renderer fixtures](../../scripts/tests/test_provider_results.py) only if needed to surface new diagnostic metadata while preserving old report output.
- [Provenance fixtures](../../scripts/tests/test_overhead_provenance.py) for narrow isolation of the remaining host-governor lookup; no change to production hardware provenance behavior.
- [Existing CI workflow](../../.github/workflows/alpha1.yml), plus a very small standard-library test gate only if inline unittest result checking becomes unclear.
- New Prototype 006 implementation/handoff documentation and narrowly relevant usage notes after implementation is authorized.

No Rust source, dependencies, sensor identity, ordering, acquisition, sampling, CLI snapshot schema or hardware configuration changes. No existing benchmark evidence or accepted historical reports will be rewritten. Any later documentation correction must be explicitly dated and limited to factual correction.

No daemon, adaptive polling, persistent cache, database, API, GUI, alarms, hardware control or protective shutdowns. Decision 43 is not resolved by this work.

## 3. Root-cause assessment

**Established:** in `execute()`, final cleanup is followed by `announce()` before the final `save()`. Announcements can append to `execution.log` and print with flushing. A `BrokenPipeError` or other output exception at this point leaves the final save unreachable, although the outer `finally` still restores signal handlers. During execution, an output error may enter the error handler and then encounter the same failing output sink again during finalization.

The previously executed fixture confirmed a final stdout failure leaving three complete mocked cases and 33 verified records in a saved `running` checkpoint. Raw captures survived; owned aliases were removed, but final cleanup diagnostics were absent. The accepted hardware completion archive explicitly contains `complete` and successful cleanup, so that measurement evidence remains valid.

**Related static concern:** `save()` currently renders Markdown before writing JSON; report rendering can therefore prevent an authoritative JSON update. It writes `results.pending` and replaces `results.json`, then writes `report.md`. A later Markdown write failure is a different failure stage from a failed JSON replacement and must be reported accurately.

The known output bug is not evidence explaining the historical exit 143 or the 503.365 ms outlier. Those remain unresolved. Execution-log and persistence-failure combinations beyond existing fixtures require new deterministic tests; they are not claimed verified now.

## 4. Proposed reliability contract

### Authoritative state and output

- `results.json` remains authoritative for actual observations, validity, lifecycle state and cleanup results. Raw captures and existing case/resource records remain untouched.
- Terminal progress and `execution.log` are advisory. Their failure must not prevent cleanup or the final JSON persistence attempt. Record bounded sanitized stage/sink/type/errno diagnostics; do not copy arbitrary exception strings, paths or raw telemetry. Disable repeated attempts to a known failed progress sink.
- Treat `report.md` as a derived, regenerable view. Separate authoritative JSON persistence from report rendering/writing sufficiently that either report failure still permits final JSON persistence. Do not introduce a general persistence framework.
- After a progress-output failure, launch no further measurement cases, preserve completed evidence and enter finalization. If failure occurs before a child launch, retain the case as unstarted only when that fact is established. Never infer unknown attempt counts or resource values.
- Return nonzero for progress, report, cleanup or final persistence failures. If cases finished but reporting failed, keep their successful acquisition records while reporting suite-level diagnostic failure. Do not silently turn that into a successful harness exit.

### Status precedence and cleanup

Retain the existing interruption/deadline contract. Secondary output errors must not overwrite an established incomplete-case, validation or other stop reason. Deadline expiry remains `suite_timeout`; an observed interrupt remains `interrupted` when the deadline has not expired. Keep cleanup outcome and secondary diagnostics independently visible. An otherwise successful or still-running suite with an output failure should use an existing explicit failure status such as `stopped_error`, rather than `complete` or stale `running`.

Keep handlers installed through child cancellation/reaping, owned-alias cleanup and final persistence, with unconditional restoration afterward. Cleanup remains ownership-aware and nonrecursive: unexpected local contents and hardware targets are never deleted. Preserve the existing child-process budgets and no-overlap behavior.

Recompute final state from observed facts after cleanup and check interruption/deadline changes after persistence. Retain at most a bounded corrective save for a newly observed finalization event; no retry loop, unlimited logging or new case launches. Preserve the original failure alongside any later timeout, interrupt or cleanup diagnostic.

### Persistence failure

Continue same-directory temporary-file replacement of the authoritative JSON, within existing metadata/disk limits. Failed serialization, temporary writing or replacement must leave the prior checkpoint parseable and unchanged, never truncate the committed JSON. Cleanup and handler restoration must still be attempted.

If JSON persistence fails, return nonzero and emit a concise privacy-safe diagnostic through a remaining usable error channel. Do not pretend the error was saved to the failing destination. If all writable/error channels fail, a nonzero process outcome may be the only remaining signal; document that boundary. Retain recoverable pending files/evidence without relabeling them as committed state.

Small additive diagnostic metadata may be needed in newly generated harness results; it must be optional for legacy readers/reports and documented. Rust snapshot/identity/sampling schemas do not change. Existing saved artifacts and their hashes remain identical, and legacy detail reports must still regenerate byte-for-byte.

Atomic replacement protects against partial JSON replacement; it is not a power-loss durability guarantee. SIGKILL, abrupt executor loss, blocking kernel I/O, or a signal arriving after the final observation boundary can prevent final recording. This milestone must not promise hard real-time cleanup or guaranteed persistence on an unavailable filesystem.

## 5. Minimal CI integration

Append fixture build and Python validation to the existing `Rust 1.85.0` job in `alpha1.yml`. Keep its existing formatting, Clippy and Rust test commands and the separate DCO job unchanged. Preserve `pull_request` targeting `main`, `contents: read`, `persist-credentials: false`, the existing immutable checkout SHA, and the 15-minute Rust / five-minute DCO job limits. Do not add services, credentials, upload steps, caching infrastructure or pip packages.

Use the existing non-root `ubuntu-24.04` runner and its installed `python3`; log the actual interpreter version and require the existing Python 3.11+ standard-library API baseline. A particular runner image or Python version is not a new compatibility claim. The current locally executed baseline used Python 3.14.10; hosted harness compatibility still needs to be demonstrated.

Proposed build/preflight commands, not executed for this proposal:

```bash
cargo +1.85.0 build --release --locked --target-dir target/provider-latency
test -x target/provider-latency/release/siliconwatch
python3 --version
```

Discover all `scripts/tests/test_*.py` using standard-library unittest, serially, only after the binary exists. A small strict result gate should fail on errors, failures, any skipped test, empty discovery or a count below the 139-test existing baseline. Require all eight existing modules and the four real-binary `CliAliasTests` to remain covered; new reliability tests increase the count. Do not freeze the exact final count at 139.

The fixture binary's expected path is currently hard-coded in the four CLI tests; their `skipUnless` condition is evaluated at import. Building first and treating skips as CI failure prevents a green job with those tests silently absent. No Rust CLI command may run against the default real hwmon root in CI.

Give Python test execution a proposed five-minute step timeout within the existing job limit. Tests use temporary sensor trees and short synthetic children; real process/signal/`/proc` lifecycle checks remain valid. Mock the remaining CPU-governor lookup in the provenance fixture so environment-dependent sysfs data cannot influence fixture assertions. No benchmark matrix, hardware-provider timing, performance threshold or real sensor access is required.

The first hosted run may expose scheduling-sensitive fixtures: existing short TERM/RSS windows are not performance requirements. If necessary, add bounded readiness handshakes or deterministic clocks in tests, without changing production timeouts, weakening lifecycle assertions, adding automatic retries or broad refactoring.

## 6. Acceptance criteria and deterministic tests

| Case | Required evidence |
| --- | --- |
| Progress output fails during execution | Inject stdout/flush and execution-log failures at START/END/smoke transition; no subsequent case launches, no fabricated record, final non-success state persisted where writable |
| Progress output fails during finalization | Reproduce final `BrokenPipeError`; completed case values/counts and raw hashes preserved, cleanup recorded, exit nonzero, saved status not stale `running` |
| Output failure plus existing stop cause | Timeout, interruption, incomplete acquisition and identity/validation failure stay explicit; secondary diagnostic does not erase the primary cause |
| Interruption during final writes | Inject SIGINT/SIGTERM before/during/after first replacement; handlers stay active, observed event receives bounded corrective persistence where possible, handlers restored |
| Timeout during cleanup/finalization | Controlled monotonic clock crosses budget; no further child launches, timeout and cleanup outcomes recorded, no false success |
| Cleanup failure plus output failure | Ownership rules preserved; unexpected contents remain, failure diagnostics survive and all child accounting already collected remains unchanged |
| Authoritative persistence fails | Inject serialization, temporary write and replacement errors; previous JSON remains readable/hash-identical, nonzero exit and explicit usable-channel diagnostic; failure of the corrective save is not hidden |
| Derived report fails | Inject render/write errors; final JSON persistence still attempted, report error explicit, stale/partial report is never claimed current or authoritative |
| Ordinary success and retained evidence | Existing successful contract preserved; raw/case data unchanged; old public archives and local evidence hashes unchanged; old reports regenerate in temporary destinations |
| Bounded error handling | Repeated broken sink is disabled; diagnostics/storage remain bounded; no retries or unbounded correction loop; finalization restores handlers |
| CI coverage | Existing 139 tests plus new regressions execute with zero failures/errors/skips, all eight existing modules and four binary cases covered; missing binary/unsupported runtime/discovery loss fails visibly |
| Rust/DCO preservation | Formatting, Clippy, all existing Rust tests and contribution DCO checks still pass; hosted success is observed for the reviewed implementation commit |

New lifecycle tests should inject clocks, sinks, serializers and write boundaries, using existing fixture helpers; use only a small number of bounded real-process tests to verify cleanup, exit and signal contracts. Count fixtures that intentionally exercise failures as successful tests only when their failure assertions pass. Do not require reproducing historical hardware outliers, interrupted-session causes or real hardware performance.

## 7. Verification performed for this merge/planning task

- Re-fetched remote state; verified clean local tree, expected base/head, exact manifest and contribution content.
- Executed the local contribution DCO checker and `git diff --check` on the accepted commit range: passed.
- Rechecked the existing hosted run and each configured job/step: formatting, Clippy, Rust tests and DCO all passed. Hosted logs already reviewed for this exact head show **53 Rust tests / zero failures or ignored tests**, **six DCO tests / zero failures**, and one valid contribution signoff.
- Verified the regular merge's parents, unchanged tree, contribution reachability and retained source branches after fetching.
- Reviewed current orchestration/save/cleanup code, existing fixtures and CI configuration. This source review confirms the known defect remains present; no fix or new regression has been executed.
- This proposal's Markdown links/anchors, whitespace and sole-file working-tree scope are checked before handoff.
- No Cargo/Python regression suite, release build, CI modification or hardware measurement was performed during this planning task. Prior publication results (139 Python tests; 53 Rust tests per approved toolchain) are a baseline, not new execution evidence. Proposed CI behavior and new tests remain unverified until implementation is authorized.

## 8. Risks, approval and Git status

The design cannot guarantee final persistence after SIGKILL, process-environment loss or failed storage, nor identify the historical termination cause. Filesystem replacement is not a transaction spanning JSON and Markdown. New secondary diagnostics must not hide the original failure or leak local paths. Keep changes local to the diagnostic lifecycle and tests.

Hosted Linux scheduling and Python-version differences may expose assumptions in existing short process fixtures. Rust dependencies/toolchain still require the existing normal CI build access; no new service is introduced. Reject skipped tests rather than representing missing coverage as success. Do not conflate correctness CI with a performance baseline.

**Chris's approval is required to implement** the conservative nonzero/stop-on-progress-failure policy, separation of authoritative JSON from derived reporting, bounded corrective persistence/diagnostics, and the proposed CI/no-skips integration. These are proposed choices, not implemented or newly accepted architectural decisions. No hardware authorization is requested or implied.

Branch: **`codex/alpha1-prototype006`**, based on **`9a190176fcaf2b697e0e2692a1847efd60ab3002`**. No upstream. The only added file is this untracked, unstaged proposal; index empty. Prototype 005 source/evidence and all tracked files remain unchanged. No Prototype 006 commit, push or PR exists.

Recommend implementing this small reliability-and-CI milestone only after Chris approves the proposal. Complete the finalization regression/fix first, verify existing evidence and tests, then add CI and observe its actual results when publication is separately authorized. Do not start Prototype 007 or any additional runtime work.

## HANDOFF TO CHATGPT

SiliconWatch **PR #4 is merged** into `main` using the authorized regular merge commit **`9a190176fcaf2b697e0e2692a1847efd60ab3002`**. The original Prototype 005 contribution, **`dacda42aa941df18c5a32ddf9da9958c4f5b863d`**, is preserved and reachable from `origin/main`. The previous main commit was `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`; the merge has exactly those two expected parents and the same tree as the approved contribution. Local and remote Prototype 005 branches remain intact. No squash, rebase, force-push or branch-protection change occurred.

Final verification confirmed the expected PR HEAD, `main` base, exact approved 20-file manifest, matching DCO author/signoff, clean working tree and no divergence. GitHub reported MERGEABLE/CLEAN. Both configured Actions jobs in run **43998394403** passed: Rust 1.85.0 formatting, Clippy and tests, plus DCO verification. Published logs record **53 Rust tests** and **six DCO-checker tests** passing. These existing hosted results were rechecked; the local Cargo/Python suites and hardware measurements were not rerun for this documentation-only planning task. The last publication baseline remains 139 Python tests and 53 Rust tests on each approved toolchain.

A new local branch, **`codex/alpha1-prototype006`**, starts at the verified merge. The only new working-tree file is **`docs/milestones/006-diagnostic-reliability-ci-proposal.md`**, untracked and unstaged. No Prototype 006 commit, push or PR exists; implementation has not begun.

The proposal targets the confirmed progress-status persistence bug and deterministic Python CI coverage. Current finalization calls `announce()` before saving final state; a progress-output exception can bypass persistence even after cleanup. The earlier fixture reproduced a stale `running` checkpoint with intact captures. The proposal makes progress output advisory, preserves authoritative JSON and per-case evidence, records bounded sanitized failures, stops further case launches after an output failure, and returns nonzero without inventing measurements. Interruption, timeout and cleanup outcomes must remain visible. Failed persistence must be reported explicitly where possible, with no promise of durability after SIGKILL, blocked I/O or loss of every output destination.

CI would append a Rust 1.85.0 fixture-binary build and strict Python unittest execution to the existing Rust job, preserving Rust/DCO checks, permissions and immutable action pinning. All existing 139 harness tests plus new regressions must run without skips. Tests use temporary sensor fixtures and short synthetic processes; no real sensor measurements or benchmark matrices enter CI.

Acceptance covers execution/finalization output failures, interrupted writes, cleanup timeouts, persistence failures, evidence hashes, no false completion, and observed green hosted checks. Chris must approve implementation of the proposed failure policy, narrow persistence changes and CI integration. Historical Prototype 005 results remain unchanged; exit 143 and the old latency outlier remain unexplained. Decision 43 stays Pending. Stop for Chris's review.
