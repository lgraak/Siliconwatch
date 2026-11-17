# Milestone 005 — Alpha 1, Prototype 005: Provider-Level Sensor Latency Characterization

**Review date:** 2026-11-16 (local date)

**Current status: Accepted. Accepted by: Chris. Scope: Alpha 1, Prototype 005. Acceptance recorded: 2026-11-16.** Chris formally accepted technical completion with documented limitations, based on successful deterministic validation, completed authorized hardware characterization, independent evidence verification and preservation of original observations. See the [formal acceptance and publication review](#formal-acceptance-and-publication-review--2026-11-16) and subsequent [publication preflight](#publication-authorization-and-final-precommit-verification--2026-11-16). Chris has now separately authorized publication; the contributor identity is verified. Decision 43's local API permissions remain Pending. The original interrupted experiment remains incomplete, with its evidence and missing accounting preserved.

**Earlier review status: Partial.** Implementation and deterministic verification passed. Six fresh smoke cases and 18 of 21 primary cases completed; execution was interrupted during the next case. Useful comparisons were retained, but that protocol did not finish. The following original review is preserved as historical context; the completion update supersedes its current-status and next-action statements.

## 1. Milestone summary

Chris authorized the [provider-latency proposal](005-provider-latency-proposal.md): a Python standard-library diagnostic around the unchanged release executable, with at most five registrations, three repetitions, 301 attempts at 100 ms, JSON Lines, bracketing controls, 21 primary cases plus six smokes, and 25-minute / 256 MiB limits. The harness implements these bounds and preserves incomplete evidence. After an initial two-smoke cleanup failure, Chris explicitly approved one fresh bounded run, allowing eight total smoke cases. The fresh run was subsequently interrupted; final protocol acceptance remains open.

## 2. Implementation details

- [Orchestrator](../../scripts/measure_provider_latency.py): plan/run/smoke-only/offline-summary commands, sequential cases, rotated blocks, full-scan controls, smoke identity validation, time/storage projections, inventory/binary guards, bounded capture and explicit failure states.
- [Selection](../../scripts/provider_selection.py): bounded enumeration, exact session tokens or unique-name selection, missing/duplicate/ambiguous registration handling, private source evidence, and owned alias roots with nonrecursive descriptor-based cleanup.
- [Evidence/reporting](../../scripts/provider_results.py): uses existing strict parser/statistics and process accounting, verifies ordered registration/channel coverage, separates unsupported and failed readings, checks identity context, preserves scalar tail/neighbor evidence, and renders deterministic reports.
- Three new fixture test modules: [orchestration](../../scripts/tests/test_measure_provider_latency.py), [selection and CLI preservation](../../scripts/tests/test_provider_selection.py), [evidence/reporting](../../scripts/tests/test_provider_results.py).
- [Reproduction instructions](../benchmarks/005-provider-latency-README.md), [observations](../benchmarks/005-provider-latency-results.md), [generated detail report](../benchmarks/005-provider-latency-detail.md), and three compact archives: preliminary results, fresh checkpoint, explicitly reviewed results including the interrupted prefix.
- No new dependencies. Python standard library only; tested with Python 3.14.10. Rust/Cargo, application tests, CLI, schemas, sensor ordering, raw/error precedence, timing/freshness, README, existing benchmark modules and CI remain unchanged.

Release build: `cargo +1.85.0 build --release --locked --offline --target-dir target/provider-latency`. Binary SHA-256: `130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8`. Seven measured harness source hashes match retained provenance. The original application revision is the existing merge checkpoint; uncommitted harness content is identified by hashes rather than falsely attributed to that commit.

Alias roots are not an OS read-only sandbox. Read-only behavior comes from reviewed code, the unchanged normal-user executable, protected source/output separation, and writes limited to owned local artifacts. No driver, kernel, hardware setting or sysfs attribute was modified. Cleanup never follows a symlink into hardware or recursively removes unexpected contents.

## 3. Actual verification results

| Executed check | Result |
| --- | --- |
| `cargo fmt --check` | Passed, Rust 1.100.0 |
| `cargo clippy -- -D warnings` | Passed, Rust 1.100.0 |
| `cargo test` | 53 passed, 0 failed, 0 ignored; no conditional permission skips |
| `cargo +1.85.0 fmt --check` | Passed |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings` | Passed |
| `cargo +1.85.0 test --locked --offline` | 53 passed, 0 failed, 0 ignored; no conditional permission skips |
| Rust 1.85.0 release build, locked/offline | Passed |
| `python3 -B -m unittest discover -s scripts/tests -v` | Final suite: 127 passed, 0 failed, 0 skipped; 67 existing + 60 new |
| Existing CLI regressions | One-shot and sampling suites passed in both Rust test runs |
| Additional actual-binary fixture regressions | Four alias/CLI tests passed with the Rust 1.85.0 release binary; fixture hardware only |
| Real SIGTERM fixture | Passed; TERM-resistant synthetic child escalated, reaped, aliases removed, interrupted accounting retained |
| Fresh hardware smoke | Six passed, 66 records; five compatible identity comparisons |
| Fresh primary hardware protocol | 18 complete, one interrupted, two unstarted; not a complete protocol |
| Independent raw-evidence audit | 5,484 completed-case records checked against raw data; timing, schema, freshness, statuses, counts, CPU arithmetic, quantiles, tails and caps passed |
| Interrupted prefix review | 277 valid records retained separately; final resource accounting and total attempts unavailable |
| Report regeneration | Original fresh checkpoint and reviewed detail report regenerate byte-for-byte; archive round trips verified |
| Final artifact hygiene | Python syntax, Markdown links/fences, all 14 new files' whitespace, `git diff --check`, empty index, archive privacy patterns and unchanged tracked files verified |

New deterministic tests cover enumeration/selection, duplicate identities and targets, missing/retargeted providers, invalid choices, alias creation/cleanup, stale directory streams, CLI/schema equivalence, timing arithmetic, percentile thresholds, tail preservation, incomplete output, timeout/interruption, output/storage/metadata limits and report generation. These tests do not establish physical sensor accuracy or hardware latency guarantees. Hosted CI was not run for this uncommitted work.

### Initial cleanup defect

The first suite stopped after two 11-record smokes because alias cleanup failed; no primary cases started. A workspace fixture reproduced held-directory enumeration omitting a newly inserted link, leaving a nonempty directory. Rewinding the descriptor before enumeration corrected that reproduction. Deterministic stale-stream and real-process fixture tests cover it. This does not establish a general filesystem/kernel defect. A separate bounds-display correction made the renderer read the actual saved configuration fields.

Both initial raw captures, results, original report and exact measured sources remain unchanged. Only the verified owned alias and empty directories were removed, with a separate cleanup note. Chris's explicit fresh-run approval is recorded in the proposal's dated authorization notes.

### Fresh execution interruption

After 24 complete cases, the execution session ended with code 143 during `025-primary-isolated-r001-b3`. The termination source is unknown. The original checkpoint still says `running` and records **909.687585 seconds before that case**; final suite elapsed time is unavailable. Exit 143 is not proof of which process received SIGTERM. A new real-signal fixture confirms ordinary handling works, but cannot explain this interruption or guarantee finalization after execution-environment termination.

Case 025 retained 277 valid records from 301 requested; final child exit, CPU, peak RSS, total attempts and elapsed accounting are unknown. Cases 026 and 027 were unstarted. No live harness/measurement executable remained in a host process check. Only the verified owned alias and empty directories were manually removed. The original checkpoint/report/raw files remain unchanged. A separate reviewed derivative contains the prefix, null accounting and explicit recovery provenance. No hardware measurements were restarted.

## 4. Hardware validation

CachyOS, Linux `7.3.4-1-cachyos`, x86-64, AMD Ryzen AI 9 HX 370 with Radeon 890M, 24 logical CPUs, 15,956,052 KiB RAM; observed powersave governor and Btrfs output filesystem. Interactive workload was not controlled and no system settings changed.

Selected registrations: acpitz (1), nvme (3), amdgpu (5), k10temp (1), iwlwifi_1 (1). hidpp_battery_0 remained visible with zero inputs. Eleven inputs comprise seven temperatures, two voltages, one frequency and one power input. Nine readings are supported; GPU frequency/power remain unsupported. IT8613/fan inputs present in older observations were absent. No permission failure was observed in retained hardware records.

Example first-control readings: acpitz 20.0 °C, NVMe 41.85/41.85/32.85 °C, amdgpu 40.0 °C, k10temp 43.75 °C, Wi-Fi 36.0 °C. GPU voltage inputs reported 0 V; neither these readings nor other values were independently calibrated or physically validated.

Completed primary cases produced 5,418 records: 20,468 OK readings, 4,816 unsupported observations, zero failed readings and zero failed attempts. Five full controls had a maximum scan window of **334.564 ms** and 16 windows above 100 ms. Two completed isolated NVMe cases had a maximum of **334.584 ms** and six such windows. Other completed isolated providers had no >100 ms windows. Blocks one and two were fully bracketed; block three lacks its final control and two complete isolated cases.

These observations associate longer windows with the selected NVMe registration. They do not establish pure driver latency or explain the historical 503.365 ms event. Both largest windows began with only about 62–63 µs start lateness, but descheduling or waits inside a scan remain unresolved. No sustained 300-second run occurred, and isolated windows were not treated as additive.

Raw output totals **25,430,839 bytes (24.253 MiB)** for the fresh suite including its prefix; reviewed local evidence is about 30.1 MiB. Final duration is unavailable. Completed primary child CPU totals 3.406999 seconds; full-control average CPU ranges 1.258–1.336% of one logical CPU. Kernel child-lifetime peak RSS ranges 23,732–34,780 KiB and includes inherited launch high-water accounting. Sampled RSS is intentionally unavailable. Per-case/window p95 is eligible at N ≥200; p99 is unavailable below N=1,000. No pooling was used to reach thresholds.

See the [observation report](../benchmarks/005-provider-latency-results.md) for distributions, provenance, missing data and interpretation.

## 5. Architecture and design review

The external diagnostic preserves `pub fn scan(root: &Path) -> io::Result<Snapshot>`, both production callers, existing schemas/identity/error ordering and sampling/freshness behavior. No general provider interface or Rust instrumentation was needed. The proposal's direct source check of Graphify's older analysis still applies because application source remains unchanged. Read-only monitoring, explicit ambiguity, privacy and safe fixture execution remain intact. Decision 43 remains pending.

Session registration tokens are not physical sensor identities. Symlinks preserve canonical hardware evidence while changing diagnostic source/root paths. Live smoke comparisons found no candidate/evidence or identity-status changes; fixtures cover expected collision-context removal. The protocol remains deliberately bounded and separate from runtime architecture. The resource limit and interruption caveats do not justify adding a daemon, cache or other unapproved infrastructure.

Deviations: the explicitly approved fresh run increased total smokes to eight across attempts; the approved 21-primary protocol remains incomplete. The implementation's 30 MiB result-document cap tightens the proposal's 31 MiB metadata estimate. No application functionality or dependency was added.

## 6. Known issues and risks

- **Confirmed and fixed:** initial alias cleanup defect; missing bounds display in generated report. Regressions pass.
- **Confirmed incomplete execution:** fresh session ended with code 143 before finalization; source of termination unknown. Three planned primary cases are not complete. No final suite or interrupted-child accounting was invented.
- **Measurement limits:** one machine, short windows, unbalanced third block, uncontrolled background/thermal conditions, pipe-capture effects, discovery/identity overhead, inherited RSS accounting and no sampled RSS. A driver wait and in-scan descheduling cannot be distinguished here.
- **Compatibility/safety limits:** Linux wait4/procfs, available Python standard-library APIs, provider-layout changes and races. Fingerprints reduce unnoticed retargeting but cannot freeze sysfs. Blocking I/O and abrupt process-environment termination prevent hard cleanup/deadline guarantees.
- **Privacy:** private manifests/raw telemetry remain ignored locally; shared archives omit source paths, candidates and raw readings. Operator metadata still needs review before later publication.
- **Not tested:** hardware reboot/renumbering during measurements, broader hardware cohorts, full three-block completion, sustained 300-second provider runs, or physical calibration. New Python tests have only local execution evidence, not hosted CI.

## 7. Git status

Branch **`codex/alpha1-prototype005`**, HEAD **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**, no upstream. All Prototype 005 deliverables are untracked and unstaged; the index is empty and tracked project files are unchanged. Fourteen intended new files comprise three Python modules, three test modules, two milestone documents, three benchmark Markdown files and three result archives. Raw evidence, source snapshots and release builds remain ignored under `target/`.

No Prototype 005 commit, push, PR, merge or remote operation was performed. DCO signoff remains required for a later authorized commit. No Prototype 006 implementation began.

## 8. Recommended next milestone

First, Chris should review the partial measurements and decide whether to accept their limited coverage or authorize a separate bounded completion plan. Missing cases must not be silently appended without contemporary controls or treated as an uninterrupted third block. No additional measurement is authorized by this recommendation.

Once Prototype 005 is accepted, recommend **Prototype 006: Diagnostic Harness CI Coverage**. Objective: prevent regression in the deterministic diagnostic tooling. Scope: run existing Python tests in CI using a Rust 1.85.0 release fixture binary; retain existing Rust/DCO checks, minimal permissions and immutable action pins. Exclude hardware runs, performance thresholds, new runtime features and dependencies.

Acceptance criteria: all 127 current Python tests run without fixture-binary skips; all 53 Rust tests and existing formatting/Clippy/DCO checks pass; CI uses only temporary fixtures, preserves application behavior and reports failures honestly. Expected checks include alias safety, parser/statistics, process timeout/signal cleanup and report regeneration. Dependencies: review/acceptance of this partial milestone and separate authorization for CI changes. Hosted scheduling differences may expose test portability issues; address those without introducing real hardware access.

## Revision history

| Date | Change |
| --- | --- |
| 2026-11-16 | Implemented and verified harness; preserved initial smoke cleanup failure and fix; executed explicitly approved fresh bounded run; documented partial measurements and externally interrupted accounting; added real-signal regression and retained sanitized evidence. |
| 2026-11-16 | Validated the authorized completion mode; executed six smokes and one seven-case block once; independently audited results and preserved prior evidence; recommended technical acceptance with Chris's acceptance still Pending. |
| 2026-11-16 | Recorded Chris's subsequent formal technical acceptance; reran deterministic checks and independent publication/evidence audits, documented a fixture-confirmed final-status persistence bug and historical-source reproducibility limits; prepared publication-readiness report without staging or publication. |
| 2026-11-16 | Recorded separate publication authorization, verified the configured public contributor identity, reran deterministic/evidence checks and prepared the single-commit publication preflight; implementation and historical evidence unchanged. |

## Historical handoff before completion

SiliconWatch **Alpha 1, Prototype 005 remains Partial**. Chris authorized a bounded provider-latency diagnostic using Python's standard library and the existing Rust 1.85.0 release binary. The implementation is complete and deterministic checks pass, but the approved hardware protocol did not finish. No application source, schemas, identity behavior, sampling semantics or dependencies changed.

Three new modules implement deterministic registration selection, temporary symlink scan roots, sequential bracketed measurements, resource limits, identity-context checks, strict capture validation, tail/neighbor retention and reproducible reports. Three test modules, reproduction instructions, observation/detail reports, milestone documentation and three sanitized result archives accompany them. Private raw evidence and measured source copies remain preserved under ignored `target/`; no raw telemetry or private manifests are proposed for Git.

Formatting, Clippy with warnings denied, and all **53 Rust tests passed on both Rust 1.100.0 and 1.85.0**, with zero failures/ignored tests and no conditional permission skips. The Rust 1.85.0 release build passed. The final Python suite passed **127 tests, zero failures or skips**, including four real-binary fixture regressions and a real SIGTERM test verifying child escalation/reaping and alias cleanup. An independent audit checked 5,484 completed-case records, raw hashes, timing/freshness, CPU arithmetic, counts, quantiles and tails. Reports regenerate from saved data.

The initial attempt stopped after two smoke cases on a cleanup defect, which was fixed and regression-tested. Chris approved one fresh bounded run. That run passed six smokes and completed **18 of 21 primary cases**, then its execution session ended with code **143** during case 025. The cause remains unknown. Its 277 valid emitted records are preserved separately; final child CPU, exit status, total attempts and final suite duration are unavailable. Two cases never started. No measurement was restarted. A host process check found no remaining benchmark process, and only the owned alias and empty directories were removed. Original results remain unchanged; a reviewed derivative explicitly records recovery and missing accounting.

Hardware exposed 11 inputs across acpitz, nvme, amdgpu, k10temp and iwlwifi_1, with a zero-input battery provider also recorded. Completed primary cases had zero failed attempts/readings and two unsupported GPU channels. Full scans reached **334.564 ms**; isolated NVMe reached **334.584 ms**. Other isolated providers had no >100 ms windows. This supports an association with NVMe-registration scan windows, not a causal driver explanation of the historical **503.365 ms** outlier. Two blocks are fully bracketed; the third is incomplete. In-scan scheduling, kernel waits, short observation windows, uncontrolled workload and inherited peak-RSS accounting limit inference.

Branch is **`codex/alpha1-prototype005`**, HEAD **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**. Intended additions are untracked/unstaged; tracked files and index are unchanged. Nothing was committed, pushed, merged or opened as a PR.

Chris should review the partial evidence and decide whether limited coverage is acceptable or separately authorize a bounded completion plan. After acceptance, recommend **Prototype 006: Diagnostic Harness CI Coverage**, with all 127 fixture tests, 53 Rust tests and existing quality/DCO checks passing without hardware access or new runtime scope. Decision 43 remains pending. No further milestone has begun.

## Completion update — 2026-11-16

### Scope and implementation

Chris authorized one fresh completion session under [Decision 43](../DESIGN_DECISIONS.md#prototype-005-completion-authorization--2026-11-16), independently of local API policy and milestone acceptance. The Python-only `--completion-block` mode performs six smokes and a fixed seven-case block. Default three-block ordering/counts and application interfaces remain unchanged; no dependency, Rust source, schema, identity or sampling change was introduced. The original Rust 1.85.0 release binary was reused without rebuilding.

Completion mode enforces 600 seconds / 128 MiB, preserves the 64 MiB metadata reserve, and requires the full per-case timeout plus ten seconds of cancellation/finalization allowance immediately before launch. Twelve new regressions cover its cohort/order/bounds, fail-stop behavior, cleanup, handler restoration and signals/deadlines during final output and evidence saving. Review caught and corrected deadline gaps around final writing and slow pre-launch preparation before any new hardware launch. Signal handlers now remain installed through cleanup and saving. A completion-only private execution log is bounded to 64 KiB. Blocking I/O and abrupt process-environment loss remain limitations, not claims of a hard deadline guarantee.

### Verification actually executed

| Check | Observed result |
| --- | --- |
| Formatting and Clippy, Rust 1.100.0 and 1.85.0 | All passed; Clippy warnings denied |
| Rust tests, each toolchain | 53 passed, 0 failed, 0 ignored; existing one-shot/sampling regressions included |
| All Python harness tests | **139 passed, 0 failed, 0 skipped**: original 127 plus 12 completion regressions |
| Release binary provenance | Existing SHA-256 unchanged; no release rebuild |
| Original evidence preservation | All 78 baseline files retain hash, size and modification time |
| Fresh metadata preflight | Expected provider/parent/channel fingerprint, kernel and CPU matched; no conflicting process; storage/load/projections passed |
| New hardware smokes | Six passed, 66 records; five compatible identity comparisons |
| New hardware primary block | Seven passed in approved order, 2,107 records; no retries |
| Independent new evidence audit | All 2,173 raw records checked; hashes, schemas, timing/freshness, identities, CPU, quantiles, tails, counts and bounds passed |
| Regeneration | New and original detail reports reproduce from their corresponding saved data |
| Process/alias cleanup | All 13 children reaped with exit 0; no alias directory or remaining host measurement process |

The final full Python suite was executed after the implementation fixes. Deterministic fixture checks include actual synthetic-process TERM/KILL/reaping and alias cleanup; those are not hardware tests. Hosted CI was not run because no publication is authorized.

### Hardware session and findings

Preflight completed in 0.074059 seconds. The foreground wrapper allowed the approved session and reserved cancellation time by sending TERM before 590 seconds from preflight start, with KILL escalation after five seconds. It exited **0**, without reaching its deadline. The harness recorded **398.407239 seconds**, including its discovery, smoke, settling, primary cases and finalization work. The separate timing scopes are retained; no exact missing duration from the earlier interrupted run was reconstructed. Offline auditing and report preparation followed the completed measurement session.

The unchanged CachyOS/kernel/CPU cohort still exposed 11 inputs across acpitz, nvme, amdgpu, k10temp and iwlwifi_1, plus zero-input hidpp_battery_0. Nine channels were supported and GPU frequency/power remained unsupported. No permission failure or failed reading occurred in the new records. The primary block contained 8,127 OK reading observations and 1,806 unsupported observations, with zero failed scan attempts. These are status observations, not independent physical calibration.

The full-before maximum was 231.904714 ms; isolated NVMe reached 286.210567 ms; the full-after maximum was **376.175665 ms**. Those cases contained seven, five and five >100 ms windows respectively. Other isolated providers had none. Full-after's largest window and all its >100 ms windows occurred before ten seconds, while one NVMe tail occurred after warmup. Both windows and their neighboring records are retained. The historical 503.365 ms maximum did not recur; recurrence was not required for completion.

New primary child CPU totaled 1.437297 seconds and capture-runner CPU 0.424681972 seconds. Full-control average child CPU was 1.483279% and 1.330953% of one logical CPU, using parent-observed elapsed time. Kernel child-lifetime peak RSS spanned 23,440–26,484 KiB; inherited-launch accounting prevents treating it as isolated application memory. Sampled RSS remained unavailable by design. Background workload and thermal state were uncontrolled.

Raw output occupied **10,053,219 bytes (9.587 MiB)**. Final completion evidence stayed below 12 MiB, and preserved local suites below 42 MiB; neither the 128 MiB new-evidence nor 256 MiB combined limit was approached. Source snapshots, raw data and manifests remain local and ignored. [Sanitized verification evidence](../benchmarks/results/005-provider-latency-completion-verification.json) records the observed checks and provenance.

### Interpretation and acceptance assessment

Original complete blocks one/two plus the fresh block yield **21 bracketed primary cases / 6,321 records** across explicitly separate sessions. Original block three's four completed cases and 277-record prefix remain additional incomplete evidence, not retroactively bracketed data. All old checkpoints, original reports, raw files, hashes, embedded timestamps and recovery records remain untouched. Unavailable old CPU/RSS/exit/reaping/attempt/duration accounting remains null.

The combined comparison set has six full controls with 26 >100 ms windows and three isolated NVMe cases with 11; its maxima are 376.175665 ms for full scans and 334.583664 ms for isolated NVMe. Per-case distributions stay separate; no pooling is used to qualify p95/p99. Details are in the [updated observations](../benchmarks/005-provider-latency-results.md#authorized-completion-review--2026-11-16) and [new generated report](../benchmarks/005-provider-latency-completion-detail.md).

These observations support NVMe-registration latency association. They do not prove a driver cause, isolate in-scan descheduling from kernel waits, make isolated costs additive, explain the historical 503 ms event, or establish performance guarantees. The earlier exit 143 remains unexplained; the successful new session does not resolve its cause.

**Assessment at completion review:** all approved technical completion criteria were satisfied; Chris's acceptance was Pending at that time. The subsequent [formal acceptance record](#formal-acceptance-and-publication-review--2026-11-16) supersedes that acceptance status only. Decision 43's local API permissions remain Pending, with execution authorization and technical acceptance recorded separately.

### Exact files and Git status

The resumed validation/measurement task changed only these three existing documentation files:

- [Reproduction instructions](../benchmarks/005-provider-latency-README.md)
- [Measurement observations](../benchmarks/005-provider-latency-results.md)
- [This milestone report](005-provider-latency-characterization.md)

It created these three shareable artifacts:

- [Completion detail report](../benchmarks/005-provider-latency-completion-detail.md)
- [Completion results archive](../benchmarks/results/005-provider-latency-completion.json.gz)
- [Completion verification record](../benchmarks/results/005-provider-latency-completion-verification.json)

The completion implementation and tests in `scripts/measure_provider_latency.py` and `scripts/tests/test_measure_provider_latency.py` were already present from the preceding authorized work and were validated without further edits in this resumption. No Rust, dependency or schema file changed. Other existing untracked work was preserved. The earlier Decision 43 documentation edit remains unstaged and unchanged by this resumption.

Branch: **`codex/alpha1-prototype005`**. HEAD: **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**. No upstream; index empty. The working tree contains the existing modified decision document and 18 untracked Prototype 005 deliverables, including the three new artifacts. There is no Prototype 005 commit, push, merge or PR. DCO signoff remains required for any later authorized commit.

### Next action

Stop for Chris's acceptance review. The previously recommended Prototype 006, deterministic diagnostic-harness CI coverage, would run the 139 Python fixture tests and existing Rust/DCO checks without hardware measurements or performance thresholds. Its objective and acceptance criteria remain small and testable, but it requires separate authorization; no Prototype 006 work has begun. No further hardware run is authorized.

## Historical handoff after technical completion, before acceptance

SiliconWatch Alpha 1, Prototype 005 has completed its authorized technical work. **All approved completion criteria are satisfied; Chris's milestone acceptance remains Pending.** Decision 43's local API permissions policy also remains Pending. Nothing was committed or published, and Prototype 006 has not begun.

The minimal Python-only completion mode was validated without changing Rust application code, dependencies, schemas, identity or sampling semantics. It preserves the default three-block protocol while offering the explicitly authorized six-smoke/seven-primary session. Signal handlers remain active through cleanup and evidence saving; deadline and interruption regressions cover pre-launch preparation, final output and final writes. The existing Rust 1.85.0 release binary was reused without rebuilding and still matches its preserved SHA-256.

Fresh checks passed: **139 Python tests, zero failures/skips**, including all original 127 plus 12 completion regressions; **53 Rust tests on each of Rust 1.100.0 and 1.85.0**, zero failures/ignored tests; formatting and Clippy passed on both toolchains. Preflight confirmed the original provider/parent/channel evidence, kernel/CPU, adequate storage, acceptable resource projections and no conflicting benchmark process.

Exactly one hardware session ran, with no retries. All six smoke cases passed, including five identity comparisons. All seven primary cases completed in order: full-before, amdgpu, k10temp, Wi-Fi, acpitz, NVMe, full-after. The harness recorded **398.407 seconds** and **10,053,219 raw bytes**, within the 600-second/128 MiB bounds; preflight separately took 0.074 seconds. The enclosing command exited zero, all 13 children were reaped, aliases were removed and no measurement process remained.

Independent auditing passed for **2,173 raw records**, including schemas, timestamps, freshness, identity/channel coverage, hashes, source/binary provenance, CPU arithmetic, quantiles, tails and completeness. Reports regenerate exactly. Primary observations comprised **8,127 OK readings, 1,806 unsupported readings, zero failed readings and zero failed attempts**. All 78 original evidence files retain their hashes, sizes and modification times, including the 277-record interrupted prefix and recovery records. Missing historical accounting remains unavailable.

The new full-scan maximum was **376.176 ms**; isolated NVMe reached **286.211 ms**. Other isolated providers had no >100 ms windows. Original complete blocks one/two plus the new block form 21 bracketed cases across separate sessions; the old incomplete block remains supplemental evidence. These results support NVMe-registration latency association, not driver causality or an explanation of the historical 503.365 ms event. Warmup, in-scan scheduling, uncontrolled workload and inherited peak-RSS accounting limit inference; the earlier exit 143 remains unexplained.

This resumption updated the reproduction instructions, observation report and milestone report, and added the completion detail report, compressed results and verification JSON. Original archives and private data remain preserved. Branch is **`codex/alpha1-prototype005`**, HEAD **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**; the index is empty, with existing unstaged documentation and untracked deliverables retained. No PR was created.

Recommend Chris review the evidence and explicitly accept Prototype 005. Any publication, further hardware run or Prototype 006 work needs separate authorization. Stop for that review.

## Formal acceptance and publication review — 2026-11-16

**Status: Accepted. Accepted by: Chris. Scope: Alpha 1, Prototype 005.** Chris's explicit acceptance follows the historical technical-completion recommendation above. Acceptance is based on successful deterministic validation, completed authorized hardware characterization, independent evidence verification and preservation of original observations. It does not imply production monitoring, protective-action readiness or authorization to publish.

| Acceptance evidence | Verified result |
| --- | --- |
| Python regression suite, rerun for this review | 139 passed, 0 failed, 0 skipped |
| Rust tests, rerun on each of Rust 1.100.0 and 1.85.0 | 53 passed, 0 failed, 0 ignored per toolchain |
| Formatting and Clippy with warnings denied | Passed on both toolchains |
| Previously authorized completion hardware session | Six smoke cases and seven primary cases completed; one completion session, no retries |
| Completion record audit | 2,173 records independently verified |
| Original evidence | 78 files unchanged in hash, size and modification time |
| Completion harness duration | 398.407238873 seconds; separate preflight timing retained |
| Raw completion evidence | 10,053,219 bytes |
| Offline report regeneration and archive/source audit | Passed; historical and final renderers distinguished |
| Additional final-status failure reproduction | Confirmed bug in one bounded fixture; three mocked cases / 33 records, no hardware |
| Hardware execution or hosted CI during acceptance review | Not performed |

The [publication-readiness report](005-publication-readiness.md) contains the exact 20-file intended manifest, exclusions, audit results, command details and Git state. This review edits documentation only; it preserves all code, generated reports, compressed results, verification JSON, private captures and incomplete recovery evidence. The verification JSON's earlier Pending assessment remains an immutable historical record, not the current acceptance status.

**Qualifications retained:** original block three remains incomplete; the new block is a separate cohort and cannot retroactively bracket old observations. Historical exit 143 remains unexplained. NVMe-registration latency association does not establish driver causality or explain the earlier 503.365 ms outlier. Scheduling, uncontrolled workload, warmup, pipe-capture and inherited peak-RSS accounting limitations remain. No unavailable accounting has been reconstructed.

**Additional review findings:** exact pre-fix harness source snapshots are preserved privately and hash-verified; public numerical reports regenerate, but hashes alone do not retrieve those historical execution revisions from Git. A fixture confirmed that a progress-output exception during final announcement can prevent the final status save. Captures and previous checkpoints survive, aliases can be removed, and no success is fabricated; the saved status can still say `running` without cleanup diagnostics. The completed hardware archive explicitly records `complete` and successful cleanup, so this bug does not invalidate its measurements. It remains unfixed and should receive a narrow deterministic regression/correction before further diagnostic execution.

**Git/publication:** branch and HEAD remain unchanged, index empty, one modified decision document and 19 untracked deliverables. Nothing was staged or published. Content is ready with documented limitations, but actual publication is **Blocked** until the intended DCO contributor identity is confirmed/configured: neither Git author name nor email is configured and author resolution fails. No identity was inferred or configuration changed. A future separately authorized cohesive milestone commit must use `git commit -s` and have its author/signoff verified.

**Next milestone recommendation:** retain Prototype 006, Diagnostic Harness CI Coverage, as a proposal only. Address the narrow final-status persistence regression first; then run all current Python fixture tests and existing Rust/format/Clippy/DCO checks in CI, with no real hardware access, performance thresholds, new dependencies or application behavior changes. Acceptance criteria include saved stopped/error status despite failed progress output, preservation of captures, restored handlers and cleanup evidence, followed by passing deterministic hosted checks. Separate approval is required; no Prototype 006 work has begun.

## Historical acceptance-review handoff

SiliconWatch **Alpha 1, Prototype 005 is Accepted by Chris**, with the documented limitations. Acceptance is recorded in the milestone report and Decision 43's history, separately from the earlier hardware execution authorization. **Decision 43's local API permissions policy remains Pending.** This task reviewed publication readiness only; no hardware measurements, staging, commits, pushes, merges or PR creation occurred.

The implementation remains a Python standard-library diagnostic around the unchanged Rust executable. Provider selection, bounded sequential capture, identity checks, explicit incomplete evidence, tail preservation and reproducible reports are implemented. Rust application source, dependencies, schemas, sensor identity and sampling behavior are unchanged. This review updated six existing documentation files and added the publication-readiness report. Its manifest lists all **20 intended publication files**: the decision document, three milestone records plus this readiness report, four benchmark Markdown files, four compressed result archives, one verification JSON, three Python modules and three test modules.

Fresh deterministic verification passed: **139 Python tests, zero failures or skips; 53 Rust tests on each of Rust 1.100.0 and 1.85.0, zero failures or ignored tests; formatting and Clippy with warnings denied on both toolchains**. Existing one-shot and sampling regressions are included. Offline audits verified report regeneration, archive contents, source hashes and evidence preservation. No hosted CI was run for this unpublished patch.

The previously authorized completion session passed six smokes and seven primary cases, yielding **2,173 independently verified records**, **398.407 seconds** of harness execution and **10,053,219 raw evidence bytes**. That completion session ran once without retries. All **78 original evidence files** retain their hashes, sizes and modification times. The original third block remains incomplete; the fresh block is a separate cohort. Original exit 143 and the historical 503.365 ms outlier remain unexplained. NVMe-registration latency association is supported, but driver causality is unproven. Scheduling, workload, warmup and inherited peak-RSS limitations remain.

The content/evidence audit found no integrity or privacy blocker. Public archives contain reviewed scalar evidence and necessary nonunique system provenance; private raw captures, manifests and historical source snapshots remain ignored. Exact historical harness revisions are privately preserved, while published numerical reports regenerate with the final renderer. A bounded fixture additionally confirmed an unfixed progress-output failure bug: final status can remain `running` after completed captures and cleanup. Existing completed-session evidence is unaffected; the bug should be corrected and regression-tested before further diagnostic execution.

**Publication readiness is Blocked by contributor identity configuration**, while the reviewed content is ready with documented limitations. Neither Git author name nor email is configured, and author-identity resolution fails. Chris should confirm the intended DCO identity when separately authorizing publication. Recommend one cohesive milestone commit using `git commit -s`, followed by inspection of its signoff and exact file list.

Branch remains **`codex/alpha1-prototype005`**, HEAD **`6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`**, with no upstream, an empty index, one modified tracked file and 19 untracked deliverables. No Prototype 005 commit or PR exists. Prototype 006 remains only a recommendation: deterministic diagnostic-harness CI coverage, with the final-status regression addressed first, existing Python/Rust checks passing and no hardware execution. Await Chris's review and separate authorization.

## Publication authorization and final precommit verification — 2026-11-16

Chris separately authorized one DCO-signed Prototype 005 commit, a normal push of `codex/alpha1-prototype005`, and one PR targeting `main`, subject to final verification. This supersedes the earlier publication-authorization and contributor-identity blockers only. Technical acceptance remains valid; Decision 43's local API permissions remain Pending. Merging, new hardware measurements, implementation changes and Prototype 006 remain unauthorized.

**Identity verified:** the configured name is `lgraak`; the configured email matches the existing public contribution email apart from casing. Git resolves the author identity and GitHub authentication succeeds for `lgraak`. This is an established public contribution identity suitable for this project's DCO signoff. Configuration and email casing were left unchanged; no credentials were extracted.

**Readiness: Ready with documented limitations.** Final deterministic checks were executed successfully: 139 Python harness tests (zero failures/skips), 53 Rust tests on each of Rust 1.100.0 and 1.85.0 (zero failures/ignored), formatting and Clippy with warnings denied on both toolchains, plus six DCO-checker unit tests. Existing CLI regressions are included; no release rebuild or hardware session was run. The offline audit reverified all 78 original and 118 private baseline files, four archive hashes/payloads, both published and three historical report regenerations, all 21 source hashes, the unchanged release binary and the interrupted prefix/null accounting.

The exact 20-file manifest is unchanged. Private evidence stays ignored. Only this readiness report and the milestone report receive current publication-status notes; accepted Python/Rust implementation, tests, schemas, dependencies and historical evidence remain untouched. All relative links and decision numbering pass. A fresh fetch found `origin/main`, local HEAD and their common ancestor all at `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`, with no remote Prototype 005 branch or unexpected history.

This section is deliberately a **precommit snapshot**. The authorized commit message is `feat: add provider-level latency characterization`. Staged allowlist, commit content/signoff, clean-tree, remote and hosted-CI verification must follow; commit SHA, PR URL and actual hosted results will be reported in the PR and final chat handoff. They are not predicted here, and no second documentation commit is implied.

The PR must retain the confirmed progress-status persistence bug and all measurement qualifications. A failed progress sink during final announcement can prevent saving final status while captures survive; the accepted completed session is unaffected. Recommend a separate narrow correction with regression coverage before further diagnostic execution. Historical exit 143, the old incomplete block, separate cohorts, unproven driver causality and the 503.365 ms outlier remain unresolved.

## HANDOFF TO CHATGPT

Chris has accepted **SiliconWatch Alpha 1, Prototype 005**, with documented limitations, and now separately authorizes publication of its exact 20-file manifest through one DCO-signed commit and a feature-branch PR targeting `main`. No merge, new hardware measurement, implementation change or Prototype 006 is authorized. Decision 43's local API permissions remain Pending.

The previous identity blocker is resolved. Git now supplies the established public contributor name `lgraak` and the existing contribution email, with only a casing difference from earlier commits. Author identity resolves, and GitHub authentication succeeds for `lgraak`. No configuration was changed and no identity was invented. The content is **Ready with documented limitations**.

Final local verification passed: **139 Python harness tests with zero failures/skips; 53 Rust tests on each of Rust 1.100.0 and 1.85.0 with zero failures/ignored tests; formatting and Clippy on both toolchains; six DCO-checker tests**. Existing one-shot, identity and sampling regressions are included. Independent offline checks verified all four public archive hashes and payloads, both published and three historical report regenerations, all 21 measured-source hashes and the unchanged release binary. All 78 original files and 118 private baseline files retain their hashes, sizes and modification times. Documentation links, decision consistency and whitespace checks pass.

The accepted hardware completion evidence remains six smoke cases and seven primary cases, 2,173 verified records, 398.407 seconds of harness execution and 10,053,219 raw bytes from one completion session without retries. No hardware was rerun during publication preparation. The original incomplete block and 277-record prefix remain separate, with unavailable accounting unchanged.

Limitations remain explicit: exit 143 and the historical 503.365 ms outlier are unexplained; NVMe-registration latency association is not driver causality; workload, scheduling, warmup and RSS accounting constrain conclusions. Exact older execution sources are privately preserved, while numerical reports regenerate publicly. The confirmed final progress-output failure bug remains unfixed and must be disclosed in the PR. Recommend a narrow correction with deterministic regression coverage before further diagnostic execution.

Only publication-status documentation was updated during this task. The branch is `codex/alpha1-prototype005`; this precommit snapshot has HEAD `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`, identical to freshly fetched `origin/main`. The index is empty at this snapshot, with the reviewed 20-file change set present. No unexpected remote history was found. Next, verify the staged allowlist, create the single authorized commit, verify its signoff, push normally, create one PR, and observe hosted CI. The resulting SHA, PR URL and actual checks belong in the PR and final publication handoff; they are not claimed here. Do not merge or begin further development.
