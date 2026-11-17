# Milestone 006 — Alpha 1, Prototype 006: Diagnostic Harness Reliability and CI

**Date:** 2026-11-16 (America/Los_Angeles).

**Status: Complete — implementation and final local verification passed; publication authorized by Chris.** Hosted CI remains unverified until the authorized feature-branch push and PR. Merge and Prototype 007 implementation are not authorized.

**Publication review update — 2026-11-16:** Re-reviewed the complete change set, reran deterministic checks, and verified unchanged historical evidence. No correction to application or diagnostic code was needed during final review. The original proposal remains historical planning context.

## 1. Milestone summary

Implemented the [approved proposal](006-diagnostic-reliability-ci-proposal.md) to correct progress-output failures that prevented final-state persistence, and to run existing diagnostic fixture suites in CI. The proposal retains its original planning status as historical context; Chris's subsequent implementation authorization is recorded here.

No hardware measurements, historical benchmark reruns, outlier investigation or application-feature work occurred. The [accepted Prototype 005 results](../benchmarks/005-provider-latency-results.md) and their limitations remain unchanged. Decision 43 remains Pending.

## 2. Implementation details

### Root cause and correction

Previously, final `announce()` ran before `save()`. A broken progress stream could therefore prevent saving final status after cleanup, leaving a stale `running` checkpoint. Derived report rendering could also prevent JSON persistence.

Progress emission now catches expected I/O/encoding/closed-stream failures, records the first error per fixed sink, and avoids further attempts to that failed progress sink. Execution stops before launching another case. Final cleanup and persistence do not depend on progress output succeeding.

Authoritative `results.json` is saved before derived report work. Both JSON and Markdown use same-directory temporary replacement; a report error is recorded with a bounded JSON correction and the failed report sink is not retried. An unavailable authoritative destination leaves the prior committed JSON intact. The harness returns nonzero and attempts a sanitized diagnostic through stderr, then remaining stdout or the completion execution log. It never claims an unpersisted error was saved.

Required reporting errors cause `stopped_error` when no stronger stop condition already applies. Completed case validity, raw captures and accounting are retained. Existing interruption/deadline priority remains; earlier non-success status is preserved as `prior_stop_reason`. Cleanup outcome stays separate. A checkpoint error before child launch clears the known-unlaunched marker, leaving the case unstarted without fabricated accounting.

Signal handlers remain installed through cleanup and final saving and are always restored. A newly observed signal, deadline or secondary reporting failure can trigger one corrective final save. Handling remains finite; there is no retry loop or new diagnostic framework.

### Files and metadata

| File | Change |
| --- | --- |
| [Provider orchestrator](../../scripts/measure_provider_latency.py) | Advisory sinks, independent JSON/report persistence, finalization/fallback handling and original stop-reason preservation |
| [Provider renderer](../../scripts/provider_results.py) | Optional earlier-stop and reporting-failure display; legacy documents retain identical output |
| [Protocol fixtures](../../scripts/tests/test_measure_provider_latency.py) | 13 new regression methods; existing final-save case strengthened to require nonzero return and preserved checkpoint |
| [Renderer fixtures](../../scripts/tests/test_provider_results.py) | One new regression for optional diagnostics and unchanged legacy report content |
| [Provenance fixtures](../../scripts/tests/test_overhead_provenance.py) | Mock the remaining CPU-governor lookup; no real sysfs input needed |
| [CI workflow](../../.github/workflows/alpha1.yml) | Release fixture build and strict Python unittest step |
| [Historical proposal](006-diagnostic-reliability-ci-proposal.md) | Original approved planning record, unchanged |
| [This milestone report](006-diagnostic-reliability-ci.md) | Implementation, verification, limits and handoff |

The pre-existing proposal is included in the eight-file publication manifest unchanged. No Cargo/Rust file, accepted evidence archive or historical measurement report was edited. No dependencies were added or removed.

New optional fields apply only to freshly generated provider diagnostic result documents:

- `reporting_errors`: a bounded dictionary keyed by the fixed sinks `stdout`, `execution_log`, `stderr`, `report`, and `results`. Each retains its first failure as fixed lifecycle stage (`setup`, `execution` or `finalization`), exception type and errno; no arbitrary exception messages or private paths.
- `prior_stop_reason`: the original non-success status when a later finalization event takes precedence.

The diagnostic schema version remains 1; these are additive optional metadata. Rust snapshot, identity and sampling schemas are unchanged. Older saved documents do not acquire these fields or change their regenerated report output.

## 3. Actual verification results

| Executed check | Result |
| --- | --- |
| `cargo fmt --check` | Passed on Rust 1.100.0 |
| `cargo clippy --locked --offline -- -D warnings` | Passed on Rust 1.100.0 |
| `cargo test --locked --offline` | **53 passed, 0 failed, 0 ignored** |
| `cargo +1.85.0 fmt --check` | Passed |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings` | Passed |
| `cargo +1.85.0 test --locked --offline` | **53 passed, 0 failed, 0 ignored** |
| `python3 -B -m unittest discover -s scripts/tests -v` | **153 passed, 0 failed/errors/skips**, Python 3.14.10 |
| Existing test-method preservation | All 139 baseline methods retained; 14 added |
| `python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v` | **6 passed, 0 failed/errors/skips** |
| Rust 1.85.0 release fixture build | Passed locked/offline in an isolated source copy; historical binaries untouched |
| Exact strict CI Python step in isolated copy | **153 passed, 0 failed/errors/skips**, using the newly built fixture binary |
| Workflow configuration | YAML parsed; original jobs/settings compared; action SHA format, shell syntax and embedded Python checked |
| Strict gate negative/positive scenarios | Eight synthetic scenarios passed: valid run, failing/error result, skips, empty discovery, insufficient count, missing module, missing CLI case, unsupported Python |
| Change/document hygiene | `git diff --check`, proposal/report links, Markdown formatting, scope and empty index checked |
| Independent preservation audit | **256 files unchanged** in SHA-256, size and modification time; both historical release binaries unchanged |
| Archive/report audit | **10 archives** match local originals; **four legacy generated reports** regenerate byte-for-byte |

Final publication verification repeated all six Cargo commands, the 153-test Python suite, six DCO-checker tests, an isolated Rust 1.85.0 release build and the exact strict CI Python step (153 tests, no skips). Independent review repeated workflow parsing/syntax/preservation checks, all 22 relative documentation links/anchors, the 256-file preservation audit, ten archive comparisons and four report regenerations. No hardware measurements were performed.

The final suite covers output failures at START/END/smoke transition and finalization, stream-flush failure, execution-log denial, disabled failed sinks, nonzero exit, raw hashes/case preservation, cleanup failure, timeout/interruption priority, original stop reasons, and known-unlaunched checkpoints. It injects JSON serialization, temporary-write and replacement errors; derived-report failures; failed corrective writes; and unavailable primary/secondary/all output channels. Existing real synthetic-process SIGTERM/KILL, reaping and cleanup tests still pass.

Existing one-shot and bounded foreground-sampling regressions passed in both Rust suites and in four real-binary Python fixture cases. No command read live hwmon sensors. The Rust fixture was built in a disposable source copy, rather than overwriting either historical release executable.

The implementation was reviewed independently; two additional finalization cases found during review were corrected and regression-tested before the final 153-test runs. Historical artifact regeneration writes only temporary results.

## 4. Hardware validation

**Not performed or required for this milestone.** All sensor data came from temporary fixtures. Short synthetic child processes exercised signal, exit, pipe, cleanup and process-resource contracts; these are correctness tests, not hardware-performance measurements. No drivers, settings, hardware registers or real sysfs sensor inputs were accessed or changed.

SiliconWatch must preserve and accurately report available measurements and their limitations. This correction addresses a software reporting defect; it does not repair or validate third-party sensor accuracy or firmware behavior. The historical 503.365 ms observation and exit 143 were not investigated further.

## 5. Architecture and CI review

No architectural expansion was necessary. The correction stays in the existing Python diagnostic lifecycle, preserving read-only Rust acquisition, evidence provenance, privacy boundaries and [accepted design constraints](../DESIGN_DECISIONS.md).

Two steps were appended to the existing Rust CI job:

1. Build Rust 1.85.0 release fixtures at `target/provider-latency/release/siliconwatch`.
2. Require an executable before importing tests, log Python version, require Python 3.11+, discover all eight existing modules serially, require the four CLI fixture cases and at least the 139-test baseline, then reject failures, errors or any skips.

The Python step has a five-minute timeout inside the existing 15-minute job. Existing Rust formatting/Clippy/tests, DCO job, pull-request-to-main trigger, `contents: read`, credential persistence setting and immutable checkout pin are preserved. No additional action, package installation, service, secret, upload or benchmark matrix was introduced.

Only preinstalled local tooling was used to parse YAML during review; it is not a project or CI dependency. No `actionlint` executable was available. Local structural/syntax validation and actual isolated execution passed, but these are not GitHub-hosted results.

## 6. Known issues, limitations and risks

- No remaining confirmed defect was found within the reviewed correction scope. This is not a claim that every possible I/O or platform failure has been tested.
- **Hosted CI is not yet observed at this pre-publication checkpoint.** Chris has authorized one signed commit, push and PR; final readiness requires actual hosted results. Only Python 3.14.10 has been executed locally; other Python versions remain unverified.
- Atomic replacement is not power-loss durability. SIGKILL, complete storage failure, blocked kernel I/O, execution-environment loss or an event after the final observation boundary can prevent final recording.
- If final persistence fails, the prior checkpoint may remain stale, including a pre-interruption status. Nonzero exit and remaining-channel diagnostics identify the failure; unavailable accounting is never reconstructed.
- JSON and derived Markdown are not a cross-file transaction. On a report failure, JSON identifies the problem; an older report must not be treated as current. On authoritative storage failure, even that diagnostic may be unavailable.
- Linux process/procfs APIs are required. Shared-runner scheduling can expose timing-sensitive synthetic-process fixtures; no automatic retries or relaxed success assertions conceal failures.
- The strict CI baseline names must be maintained deliberately if tests are renamed. They prevent silent loss of the four binary fixture cases and eight existing modules.
- All accepted historical measurement limitations remain unchanged. No new claim about provider latency, sensor accuracy or production readiness is made.

## 7. Git status

Final-review branch: **`codex/alpha1-prototype006`**. Base and current pre-commit HEAD: **`9a190176fcaf2b697e0e2692a1847efd60ab3002`**. Fetch confirmed `origin/main` still matches that base, with no remote Prototype 006 branch or existing PR. GitHub CLI authentication is configured for `lgraak`; the existing public Git author/signoff identity is configured. It was not changed.

Exact reviewed publication manifest (eight files):

- `.github/workflows/alpha1.yml`
- `docs/milestones/006-diagnostic-reliability-ci-proposal.md`
- `docs/milestones/006-diagnostic-reliability-ci.md`
- `scripts/measure_provider_latency.py`
- `scripts/provider_results.py`
- `scripts/tests/test_measure_provider_latency.py`
- `scripts/tests/test_overhead_provenance.py`
- `scripts/tests/test_provider_results.py`

At this checkpoint six tracked modifications and two untracked documents are reviewed; index empty. All eight are text files. No private evidence, source snapshots, local manifests, build outputs, credentials or unrelated files are included. Chris authorizes one DCO-signed commit and normal publication, with no merge. The final commit/PR/hosted result will be recorded after observation; this report does not pre-claim those outcomes. Updating this local report afterward does not authorize a second commit or history rewrite.

## 8. Recommended next milestone

**Prototype 007: Shared In-Memory Sensor Snapshot Cache** — proposed only, not implemented or authorized by publication.

Objective: establish one shared latest-snapshot state as a small step toward the headless monitoring architecture.

Proposed scope and acceptance criteria:

- One acquisition owner publishes immutable latest-attempt state to safe concurrent readers; readers never trigger duplicate hardware scans.
- Preserve raw observations, statuses, source paths and identities exactly. Never calibrate, deduplicate, fabricate missing readings or silently reuse an earlier success after failure.
- Record explicit observation/publication timestamps and monotonic age. Distinguish empty, available, stale and failed states, including partial sensor errors. Specify any retained earlier snapshot as historical/stale, never current success.
- Retain a bounded number of snapshots with an explicit policy for reader-held references; do not claim bounded total memory merely because the shared slot holds one value.
- Add deterministic fixtures for first publication, replacement, failures/recovery, freshness boundaries, wall-clock changes, concurrent readers and the retention bound. Preserve existing one-shot/sampling regressions and all Python/DCO CI coverage.

Dependencies and risks: Chris must approve a small cache contract and ownership/freshness policy before implementation. Avoid holding locks across acquisition or output, and keep process-local state separate from unresolved API permissions. Shared readers and retained references require explicit lifetime limits; wall-clock correlation must not govern freshness. No new dependency is presumed necessary.

Exclude daemon/service lifecycle, a new scheduler, adaptive polling, persistent history/database, API, GUI, alarms and hardware/protective actions. Decision 43 remains Pending. Planning targets after review are Prototype 008 headless daemon, 009 versioned local read-only API and 010 native GUI; none are implementation authorizations.

## 9. HANDOFF TO CHATGPT

SiliconWatch **Alpha 1, Prototype 006: Diagnostic Harness Reliability and CI is complete and has passed final local publication review**. Chris authorized one DCO-signed commit, normal push and a PR targeting `main`, subject to observed hosted CI. Publication is the next authorized action; no merge or Prototype 007 implementation is authorized. Decision 43 remains Pending.

The provider diagnostic now treats progress output as advisory. A failed sink is recorded and disabled, additional cases stop launching, and cleanup plus authoritative JSON persistence remain reachable. Required reporting failures produce a nonzero result while preserving completed captures and accounting. Bounded sanitized `reporting_errors` identify sink, stage, exception type and errno; `prior_stop_reason` preserves an earlier failure when interruption or timeout takes precedence. A failed prelaunch checkpoint correctly leaves its case unstarted.

JSON is saved before derived Markdown through same-directory replacement. A report failure permits a bounded JSON correction; final write failures retain the prior checkpoint and attempt diagnostics through remaining channels. Signal handlers remain installed through finalization and are restored. This cannot guarantee persistence after SIGKILL, unavailable storage or loss of every reporting destination; an unpersisted late event can leave an older checkpoint visible.

The reviewed eight-file manifest contains the workflow, orchestrator, renderer, three test modules, original proposal and milestone report. No dependency, Rust source, acquisition, identity, sampling or snapshot-schema change is included. Independent review found no publication-blocking correctness or privacy issue, and no code changes were needed during this final review.

Verification was executed again: formatting and Clippy with warnings denied passed on Rust 1.100.0 and 1.85.0; **all 53 Rust tests passed on each**. **All 153 Python tests passed without failures/errors/skips**, preserving 139 baseline tests plus 14 regressions. **Six DCO-checker tests passed**. A fresh isolated Rust 1.85.0 release build and exact strict CI Python step passed. Workflow configuration/syntax and documentation links passed. Hosted Actions are not yet observed at this pre-publication checkpoint.

The evidence audit reconfirmed **256 unchanged files**, ten archives matching local originals and four legacy reports regenerating byte-for-byte. Both historical binaries remain unchanged. No hardware measurement, historical benchmark rerun or outlier investigation occurred.

Branch `codex/alpha1-prototype006` is based on `9a190176fcaf2b697e0e2692a1847efd60ab3002`; fetched `origin/main` matches, with no existing branch PR. Six tracked modifications and two documents await the single authorized commit. Final commit, PR and hosted evidence will be recorded only after publication.

Recommend **Prototype 007: Shared In-Memory Sensor Snapshot Cache**, pending separate approval: one acquisition owner, safe readers, immutable raw observations/identities, explicit timestamps and freshness/failure states, bounded retention, and deterministic transitions/concurrency tests. Failed acquisition must never silently present an earlier success as current. Daemon, API and GUI remain later planning targets, not authorized work. Stop after publication for Chris's review.
