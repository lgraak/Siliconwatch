# Prototype 005 — Hardware characterization completion proposal

**Date:** 2026-11-16 (America/Los_Angeles). **Status: Proposal — execution not authorized.** Prototype 005 remains **Partial**. This planning review changes no implementation or existing evidence. Decision 43 remains pending; Prototype 006 must not begin.

**Historical planning record — 2026-11-16:** The preceding status and proposal below describe the state before Chris's separate execution authorization. The bounded completion protocol was subsequently authorized under [Decision 43](../DESIGN_DECISIONS.md#43-local-api-permissions--pending), executed once without retries, independently verified and formally accepted by Chris. See the [current milestone acceptance record](005-provider-latency-characterization.md#formal-acceptance-and-publication-review--2026-11-16). Original evidence and this proposal's historical text remain preserved. Local API permissions remain Pending; publication, additional runs and Prototype 006 remain unauthorized.

## 1. Findings and unfinished work

Reviewed [milestone 005](005-provider-latency-characterization.md), [observations](../benchmarks/005-provider-latency-results.md), [reproduction instructions](../benchmarks/005-provider-latency-README.md), the diagnostic/process/selection/reporting code and tests, saved validation logs, and preserved local captures/checkpoints/recovery records.

| Original case | Established state | Unavailable information |
| --- | --- | --- |
| `025-primary-isolated-r001-b3` — acpitz | Interrupted; 277 valid emitted records from 301 requested | Final child exit/reaping, CPU, peak RSS, total completed attempts and final elapsed time |
| `026-primary-isolated-r002-b3` — nvme | Unstarted | All measurements |
| `027-primary-full_after-all-b3` — full scan | Unstarted | All measurements |

Original block three completed its full-before control and amdgpu, k10temp and iwlwifi_1 cases. It lacks a closing control and two complete isolated cases. Blocks one and two remain fully bracketed. The 24 completed fresh-suite cases contain **5,484 records: 66 smoke and 5,418 primary**. Case 025's prefix is separate, not an established completed-attempt count. The saved **909.687585 seconds** is a pre-case checkpoint, not final suite duration.

**Exit 143:** the execution session reported that value. It is compatible with conventional `128 + SIGTERM` encoding, but does not identify the recipient, sender, cause or whether Python handled a signal. No reviewed local log establishes a timeout, out-of-memory event, driver failure, or a 900-second execution limit. Those explanations remain unproven. Source review confirms SIGINT/SIGTERM handlers and bounded child cancellation; the saved real-SIGTERM fixture passed. Abrupt loss of the enclosing execution environment can prevent finalization, but that is a limitation, not an established explanation of this incident.

The original checkpoint SHA-256 remains `ce55ff87315da595c7fd31bd567c3861fdad4d850f3989b04696b866cff8d420`. Recovery records document a host process check finding no surviving benchmark process, followed by owned-link/empty-directory cleanup. These records do not recover unavailable child accounting.

## 2. Recommended scope and interpretation

**Recommend one fresh, complete seven-case comparison block**, in a separate session and output directory, following six fresh smoke checks. This preserves the original requirement for three complete, contemporary, all-provider comparison blocks without rerunning the full protocol.

Primary order retains the original third-block rotation:

1. Full scan before.
2. amdgpu.
3. k10temp.
4. iwlwifi_1.
5. acpitz.
6. nvme.
7. Full scan after.

Fresh smoke order: full scan, then each of the five unambiguously remapped registrations in deterministic inventory order. Names/tokens must be validated against private hardware evidence; old `rNNN` or `hwmonN` values are not durable identities. A changed, missing, duplicate or ambiguous cohort stops execution for review rather than silently substituting hardware.

New measurements can be independent valid comparisons without invalidating the old ones. They **cannot** finish the historical block retroactively. The final analysis would identify original blocks one/two plus the new complete block as a 21-case bracketed comparison set, with session boundaries explicit. All four completed original block-three cases and the interrupted prefix remain separately reported supplemental evidence. This inclusion rule is specified now, based on completeness, not measured latency. No observation is deleted or relabeled as contemporaneous with a later control.

**Smaller alternative, requiring changed acceptance criteria:** full-before → acpitz → nvme → full-after, after three smokes. This provides missing provider repetitions but leaves the other original third observations without an after-control and does not deliver three complete all-provider blocks. Its nominal duration is 213 seconds, with a proposed six-minute/128 MiB cap. It is not the recommended plan and is not included in the approval wording below.

## 3. Execution procedure and bounds

The existing CLI cannot request one block: `--run` always plans three primary blocks; selecting two providers still requests 12 primary cases plus three smokes. `--smoke-only` launches no primary cases. Do not use manual interruption, checkpoint edits, runtime monkeypatches or an unrestricted case selector as a workaround.

After separate authorization, add only a fixed **completion-block mode** to the Python harness with fixture tests. Preserve the default protocol, all application and benchmark JSON schemas, the shared measurement/parser/statistics methods, dependencies and Rust behavior. Record the completion session and predecessor checkpoint hash in existing descriptive configuration metadata. Keep normal signal handlers installed through final cleanup/save, restoring them afterward; test that finalization window. The current earlier restoration is a code-supported interruption risk, not proof of the observed cause. No such changes are made by this proposal.

| New-session bound | Limit |
| --- | --- |
| Hardware launches | Exactly six smokes followed by seven primary cases if every gate passes; no retries |
| Samples/output | Smokes 11 attempts; primary 301; all 100 ms, JSON Lines |
| Maximum emitted records | 2,173: 66 smoke + 2,107 primary |
| Settling/warmup | At least 15 seconds between cases; ten-second primary post-warmup subset, whole-run data retained |
| Per-case timeout | 5 seconds smoke; 40 seconds primary; cancellation/reaping allowance reserved |
| Per-case stdout + stderr | 1 MiB smoke; 8 MiB primary |
| New-session elapsed budget | **600 seconds**, including discovery, smoke, settling, acquisition and finalization |
| New evidence storage | **128 MiB**, including temporary/atomic-write files and bounded logs; 62 MiB maximum raw capture + 64 MiB reserve = 126 MiB |
| Combined local measurement evidence | Stay within 256 MiB across preserved and new suites; never delete old evidence to fit |
| Retention/parser | No more than 2,173 scalar records; existing 4 MiB line and 64 KiB capture-chunk limits; existing metadata caps |
| Memory observation | Periodic RSS disabled consistently; retain kernel child-lifetime peak RSS with its inherited-accounting caveat |

Nominal sampling plus settling is **396 seconds (6 minutes 36 seconds)**; startup, provenance and reporting add time. Case timeouts plus settling total 490 seconds, leaving 110 seconds within the budget for other work and cleanup. These are planning estimates and operational caps, not hard guarantees against blocking kernel/filesystem calls.

Procedure after approval:

1. Implement and fixture-test the fixed mode and finalization handling before touching hardware. Verify exact case order, counts, budget arithmetic, unchanged default mode, rejection of missing/ambiguous cohorts, signal/timeout/output-cap stopping, no overlap, evidence preservation and report regeneration. Confirm the 127 existing tests still pass without fixture skips; record any added-test count.
2. Hash-check old evidence, measured sources and the existing Rust 1.85.0 binary (`130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8`). A changed/missing binary or incompatible environment/cohort requires review before measurement, not an undocumented rebuild or substitution.
3. Establish a foreground execution session whose documented enclosing timeout permits the 600-second protocol and orderly cancellation. If lifecycle/cleanup ownership cannot be established, stop before hardware access. No detached job, persistent supervisor or service is proposed. Record configured runner limits and its terminal result only when observed; do not infer an undocumented limit from exit 143.
4. Use a new private output directory, for example `target/provider-latency-results/003-completion-block`, refusing an existing destination. Record fresh sanitized provenance and private mapping. Check disk availability, workload suitability and the full/selected inventory before each case. Make no performance, driver or hardware configuration changes.
5. Run all six smokes sequentially. Require usable coverage, complete output/accounting, safe cleanup, compatible identity evidence and time/storage projections. If any check fails, retain evidence and stop; no primary launch or automatic restart.
6. Run the seven primary cases sequentially under the same checks. Admit a case only if its settle time, full timeout and cleanup/final-save reserve fit the remaining budget. Stop launching before reserve is exhausted. Keep bounded private start/stop/received-signal records when observable; missing terminal evidence remains unknown.
7. Validate and hash captures offline, regenerate sanitized reports, verify original hashes/timestamps and confirm no owned child/alias remains. Report this as a new session, not resumed case 025 or a repaired original checkpoint. Stop for review.

## 4. Failure handling and evidence integrity

- On handled SIGINT/SIGTERM, timeout, output cap, capture error, changed inventory/binary, unusable output or failed cleanup: stop additional launches, retain the output prefix and return nonzero. Preserve actual received-signal information only when observed; no sender/cause inference.
- Reuse owned child-group termination: TERM, then KILL after 0.5 seconds if necessary, with bounded reaping (existing three-second wait). Budget at least four seconds for cancellation and additional final-save allowance. Never signal an unrelated PID/group. An unreaped child stops the protocol and requires inspection; do not claim process absence without checking.
- Remove only owned local symlinks and empty alias directories using descriptor-based checks; never follow them into sysfs or recursively remove unexpected contents. If abrupt session loss prevents cleanup, inspect and document recovery separately before any further measurement.
- Preserve suites 001/002, original/checkpoint/reviewed archives, source copies, raw captures, embedded timestamps, hashes and recovery notes unchanged. New derived analyses link to them by hash; no overwriting or in-place recovery. Keep raw/private manifests ignored and local. Publishable artifacts must exclude unnecessary identifiers and paths.
- A parsed prefix is evidence of emitted records, not proof of total completed attempts or child exit. Old case-025 CPU/RSS/exit/reaping/duration and total attempts remain unavailable permanently unless new contemporaneous evidence is actually found; never estimate them from neighboring cases, timestamps or exit 143.
- No repeat after interruption, no extra provider, no 300-second trial, no chase for a matching outlier. Any failed gate leaves completion Partial and requires Chris's review.

## 5. Acceptance criteria and limitations

Accept the proposed completion experiment only when all six smokes and seven primary cases pass their independent checks, all 2,173 expected records are present with valid schemas/timing/cohort, process accounting is actually observed, aliases/children are cleaned up, bounds are respected, reports regenerate and old evidence is unchanged. Keep reading failures and unsupported channels distinct; do not call an entirely unreadable cohort valid acquisition. Record all failed attempts explicitly; a failed comparison is not silently counted as a successful block.

Show whole-run and post-warmup statistics separately per case/session. Use median/max/N; p95 only at N ≥200 and p99 only at N ≥1,000. Do not pool providers or repetitions to reach thresholds. Primary cases have at most 301 observations, so per-case p99 remains unavailable. Preserve every >100 ms scan and neighboring records regardless of completion outcome.

The existing 334.584 ms isolated NVMe and 334.564 ms full-scan maxima suggest an association, not driver causality. New observations cannot establish the cause of the historical 503.365 ms event. No recurrence is required for success. Kernel waits, descheduling within scan windows, discovery/identity work, output effects, thermal/workload drift and changed session conditions remain confounders. Isolated timings are not additive full-scan components. Physical accuracy, universal identity stability and hard performance guarantees remain outside this task.

## 6. Verification, Git state and approval requested

This planning review inspected the saved logs recording 53 passing Rust tests on each of Rust 1.100.0/1.85.0, the successful Rust 1.85.0 release build, and 127 passing Python tests. Those suites were **not rerun** for a documentation-only change. The prior independent 5,484-record arithmetic/schema audit remains historical executed evidence.

Safe offline checks during planning reverified 48 completed raw-file hashes, all seven measured harness hashes, the release binary hash, archive/original equality, the 24 unchanged completed run entries, counts of 5,484 completed records and 277 prefix records, unknown-accounting fields, and byte-identical detail-report regeneration. Protocol time/storage arithmetic was checked without launching the application. No hardware discovery, sensor reads, test suite, or characterization was initiated in this review.

Branch remains `codex/alpha1-prototype005`, HEAD `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`, without upstream. Only this new proposal is added by this task. All 14 prior untracked deliverables remain unchanged; no tracked file or index change, commit, push, PR, merge or history operation occurred. A before/after hash, size and modification-time check covers the existing deliverables and preserved measurement files. No private evidence is staged.

**Exact authorization requested — not granted by this proposal:**

> Approve the narrowly scoped Python-only completion-block mode, its deterministic tests and signal-finalization handling, followed—only if validation and preflight pass—by one new foreground session containing six 11-attempt smokes and seven 301-attempt primary cases at 100 ms. Cap that session at 600 seconds and 128 MiB of new evidence, with all local measurement suites remaining within 256 MiB. Preserve all originals; allow no retries or application/schema/dependency changes. Report the new complete block separately alongside original blocks one/two and retain the incomplete original block as supplemental evidence. Do not commit, push, merge, open a PR or begin Prototype 006.

The requested approval would explicitly authorize additional cases beyond the earlier interrupted protocol; it would not authorize a fresh 21-primary-case run. If Chris prefers the smaller four-case alternative, approve its revised completion criterion separately instead. No subsequent milestone is proposed for execution while Prototype 005 remains Partial.

## HANDOFF TO CHATGPT

Prototype 005 remains **Partial**. This task prepared a completion proposal only; no hardware characterization, application changes, dependency changes or Git publication occurred. The proposal is saved as `docs/milestones/005-provider-latency-completion-proposal.md`.

The unfinished original cases are **025: acpitz repetition three**, with 277 valid emitted records but unknown final process accounting; **026: NVMe repetition three**, unstarted; and **027: the final full-scan control**, unstarted. Original block three completed its preceding control and amdgpu, k10temp and Wi-Fi cases. A new control cannot retroactively bracket those earlier observations. Original blocks one and two remain valid complete comparisons.

Exit **143** establishes the execution session's reported return value. Although compatible with conventional SIGTERM encoding, available evidence does not identify its cause, recipient or sender, prove Python handled it, or establish a 900-second execution limit. Existing signal handlers and the prior real-SIGTERM regression are present. The saved 909.687585 seconds is a pre-case checkpoint, not final duration. No unavailable CPU, exit status, total attempts, RSS or duration was reconstructed.

I recommend **one fresh seven-case block after six short smokes**, preserving the original comparison design. Primary order is full-before, amdgpu, k10temp, Wi-Fi, acpitz, NVMe, full-after. Use the unchanged Rust 1.85.0 binary, 301 primary attempts at 100 ms, JSON Lines, 15-second settling and ten-second post-warmup reporting. Proposed limits are **600 seconds and 128 MiB of new evidence**, with combined preserved/new suites below 256 MiB. Nominal sampling plus settling is 396 seconds; at most 2,173 records are expected. No retry is authorized. A four-primary-case supplement is smaller but requires explicitly accepting that three complete all-provider blocks would still not exist.

The current CLI cannot request one block. Approval must therefore cover a minimal Python-only completion mode, fixture tests and retaining handlers through final cleanup/save before any measurement. Default behavior, application and benchmark schemas, identity and sampling semantics remain unchanged. Foreground execution ownership, fresh cohort/identity checks, time/storage projections and safe cleanup are mandatory gates. Any failure stops the session and preserves incomplete evidence.

Saved validation evidence confirms 53 Rust tests passed on both toolchains, a successful Rust 1.85.0 release build, and 127 Python tests passed. These suites were reviewed, not rerun. Offline checks reverified completed capture hashes, source/binary provenance, archived originals, 5,484 saved completed records, the separate 277-record prefix and deterministic report regeneration. Existing files and recovery records are preserved.

Branch remains **`codex/alpha1-prototype005`**, HEAD **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**. This task adds only the untracked completion proposal; the previous 14 untracked deliverables, tracked files and index remain unchanged. Chris must approve the explicit implementation-and-single-run scope before work proceeds. Historical outlier causality remains unresolved; reproducing it is not an acceptance requirement. Decision 43 remains pending. Prototype 006 has not begun.
