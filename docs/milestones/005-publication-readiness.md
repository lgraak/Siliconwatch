# Prototype 005 — Acceptance and publication-readiness review

**Review date:** 2026-11-16 (America/Los_Angeles). **Milestone:** Alpha 1, Prototype 005. **Accepted by:** Chris.

**Current precommit readiness: Ready with documented limitations.** Chris has separately authorized publication, and the configured public contributor identity is now verified. See the [final publication preflight](#publication-authorization-and-final-precommit-verification--2026-11-16). The earlier audit below, including its identity blocker and uncommitted Git state, is preserved as historical review evidence.

**Historical readiness finding: Blocked — commit/DCO identity prerequisite only.** At that review, the content and evidence were ready with documented limitations, but Git author name and email were not configured and author identity resolution failed. The host-derived fallback is deliberately omitted. No configuration was changed during that review, and publication was outside its authorization. The subsequent publication preflight below resolves this finding.

The [formal acceptance record](005-provider-latency-characterization.md#formal-acceptance-and-publication-review--2026-11-16) records technical completion based on successful deterministic tests, authorized completed hardware characterization, independent verification and preserved original evidence. [Decision 43](../DESIGN_DECISIONS.md#43-local-api-permissions--pending) retains its **Pending local API permissions policy**, separate execution authorization and subsequent technical acceptance. Earlier proposal/status text and the immutable verification JSON remain historical records.

## Exact intended publication manifest

This is a review manifest, not a staging instruction. All paths below are relative to the repository. Every listed file was read or, for compressed results, fully decompressed and inspected. All 20 files belong to Prototype 005; no other file is proposed.

| Path | Git state | Purpose |
| --- | --- | --- |
| `docs/DESIGN_DECISIONS.md` | Modified | Historical execution authorization and separate acceptance; local API policy Pending |
| `docs/milestones/005-provider-latency-proposal.md` | Untracked | Original design and authorization history, current-status pointer |
| `docs/milestones/005-provider-latency-completion-proposal.md` | Untracked | Bounded fresh-block plan, preserved historical status and current pointer |
| `docs/milestones/005-provider-latency-characterization.md` | Untracked | Historical reviews, completion evidence, formal acceptance and handoff |
| `docs/milestones/005-publication-readiness.md` | Untracked | This complete publication audit and manifest |
| `docs/benchmarks/005-provider-latency-README.md` | Untracked | Reproduction commands, privacy and historical-source limitations |
| `docs/benchmarks/005-provider-latency-results.md` | Untracked | Observations, separate cohorts, interpretation and acceptance |
| `docs/benchmarks/005-provider-latency-detail.md` | Untracked | Deterministically regenerated reviewed interrupted-session detail |
| `docs/benchmarks/005-provider-latency-completion-detail.md` | Untracked | Deterministically regenerated completion-session detail |
| `docs/benchmarks/results/005-provider-latency-preliminary.json.gz` | Untracked | Initial stopped two-smoke evidence |
| `docs/benchmarks/results/005-provider-latency-checkpoint.json.gz` | Untracked | Unmodified checkpoint from interrupted session |
| `docs/benchmarks/results/005-provider-latency-reviewed.json.gz` | Untracked | Explicit reviewed derivative, including prefix and null accounting |
| `docs/benchmarks/results/005-provider-latency-completion.json.gz` | Untracked | Separate successful completion session |
| `docs/benchmarks/results/005-provider-latency-completion-verification.json` | Untracked | Preserved pre-acceptance verification/provenance record |
| `scripts/measure_provider_latency.py` | Untracked | Bounded diagnostic orchestration and completion mode |
| `scripts/provider_selection.py` | Untracked | Explicit registration selection and owned alias cleanup |
| `scripts/provider_results.py` | Untracked | Capture validation, evidence and deterministic reporting |
| `scripts/tests/test_measure_provider_latency.py` | Untracked | Orchestration, limit, signal and completion fixture tests |
| `scripts/tests/test_provider_selection.py` | Untracked | Selection, identity, cleanup and CLI preservation tests |
| `scripts/tests/test_provider_results.py` | Untracked | Parsing, timing, statistics, incomplete evidence and report tests |

**Changed by this acceptance review:** the decision document, both proposals, milestone characterization, benchmark README and observation report; this readiness report is new. The other 13 proposed files are byte-for-byte preserved from the review baseline, including all code/tests, generated reports and result artifacts. No application or benchmark implementation was edited.

**Excluded and preserved:** all of `target/provider-latency-results/`, including private manifests, source snapshots, raw stdout/stderr, original results/reports, logs, recovery notes, audit scripts and preflight records; release/build outputs under `target/`; Python caches under `__pycache__/`; temporary validation logs and preservation inventories outside the repository. These are not publication candidates. Existing ignore rules cover target/build/evidence and Python caches; all other untracked repository files are accounted for by the manifest. Nothing was deleted or staged.

## Deterministic verification executed

| Command or audit | Actual result |
| --- | --- |
| `cargo fmt --check` | Passed, Rust 1.100.0 |
| `cargo clippy --locked --offline -- -D warnings` | Passed, Rust 1.100.0 |
| `cargo test --locked --offline` | 53 passed, 0 failed, 0 ignored |
| `cargo +1.85.0 fmt --check` | Passed |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings` | Passed |
| `cargo +1.85.0 test --locked --offline` | 53 passed, 0 failed, 0 ignored |
| `python3 -B -m unittest discover -s scripts/tests -v` | 139 passed, 0 failed, 0 skipped |
| Existing one-shot/sampling/identity regressions | Included in both Rust runs; fixture-only binary checks also passed in Python suite |
| Additional progress-output failure reproduction | One bounded fixture confirmed the final-save bug; three mocked cases, 33 records, no hardware |
| Offline independent capture audits | 7,679 completed-case records across all three suites verified; interrupted 277-record prefix separately reproduced |
| Source provenance | All 21 measured-source hashes match retained snapshots; final source matches completion suite |
| Evidence preservation | 78 original evidence files unchanged in SHA-256, byte size and modification time |
| Archive comparison | Four public gzip archives match corresponding preserved local results |
| Report regeneration | Three original local reports reproduce with their historical renderers; both published detail reports reproduce with final renderer |
| Documentation / Git hygiene | 69 relative links/heading anchors passed; Markdown fences and text whitespace passed; Decisions 1–43 occur once, 1–42 unchanged and API policy 43 Pending; `git diff --check` passed |
| Scope comparison | Rust source/tests, Cargo/lockfile, existing benchmark helpers, README, LICENSE, AGENTS and CI unchanged from HEAD |
| Git contributor identity | Failed prerequisite: name/email unconfigured; author identity cannot resolve |

No additional hardware measurement or release rebuild was performed. The prior successful Rust 1.85.0 release build and unchanged binary hash remain historical provenance, not a newly executed build. Hosted CI was not run for these uncommitted additions. Local passing tests are not hosted CI evidence. The extra reproduction demonstrates an unfixed defect; it is not counted as an additional passing regression test.

## Privacy, licensing and artifact review

All four gzip streams and the verification JSON were inspected structurally and recursively. The archives contain intended configuration, nonunique environment provenance, scalar timing/status evidence, counts, tails and hashes. They contain no raw capture payloads, private manifest contents, full identity candidates, sysfs hardware paths, serials, network identifiers, credentials, hostnames, usernames or home-directory paths. `raw_sha256` entries and `stdout.raw`/`stderr.raw` names identify hashes, not embedded raw output.

CPU model/count, OS/kernel, RAM, governor, quota, load/free-space observations, power status, provider names/counts and timestamps intentionally remain. This is reviewed benchmark provenance; it is not a claim of zero host-specific telemetry. Historical illustrative temperature/voltage readings are limited, nonidentifying observations. Test paths/readings are synthetic; reproduction paths are generic local destinations.

| Artifact | Compressed/stored bytes | Uncompressed bytes |
| --- | ---: | ---: |
| Preliminary JSON gzip | 5,387 | 29,969 |
| Checkpoint JSON gzip | 360,156 | 2,905,819 |
| Reviewed JSON gzip | 377,929 | 3,049,155 |
| Completion JSON gzip | 145,783 | 1,167,353 |
| Completion verification JSON | 5,863 | 5,863 |

Only the four intended gzip archives are binary; all other proposed files are UTF-8 text without NUL bytes. Gzip headers have zero timestamps and no original filenames. The largest archive is below 370 KiB. No generated executable, raw trace, temporary file or Python cache is proposed.

Artifact SHA-256 values:

| Artifact | SHA-256 |
| --- | --- |
| Preliminary | `0179e4137530b63852b53c5788b815b496ffac1d5d1adb4f1bd2551c19331b96` |
| Checkpoint | `b35ef41678159827f96ceb3cb0868bc5f2541668caaea4686e1291ba13b3f3cb` |
| Reviewed | `83361512c6f01e3afd00bd3dbb539ed71eb3d265e007a52fe390224c4eeb943a` |
| Completion | `faaf4b0e62d843ca28f75279b622b1e017df356f8df22620eb3042c9182adf33` |
| Verification JSON | `6d60e4b5230ae4dd304bbac2b5caeb48b80f705e68b5ddd7a43b3b6093c11623` |

New modules use Python's standard library and existing repository helpers. No dependency, schema or application behavior changed. The root GPLv3 license and Cargo `GPL-3.0-only` declaration agree; no copied third-party implementation or conflicting license notice was identified. Future contribution commits must meet Decision 41's DCO requirements; no signoff exists for this uncommitted work.

## Provenance, completed observations and limitations

The approved completion session ran **once, without retries**: six smoke cases, seven primary cases, 2,173 independently verified records, 398.407238873 seconds of harness execution and 10,053,219 raw evidence bytes. Separate preflight took 0.074059 seconds. The original release binary hash remains `130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8`. The new session stayed within its 600-second/128 MiB bounds. No accounting for missing historical data was reconstructed.

All 78 original evidence files remain unchanged, and this review additionally preserves all private files in its 118-file starting inventory. Original block three is incomplete and cannot be retroactively bracketed. Complete original blocks one/two plus the fresh block give 21 comparison cases / 6,321 records across distinct sessions; supplemental old cases and the 277-record prefix remain separate. The completion verification JSON intentionally retains its pre-acceptance Pending assessment, superseded only by the dated milestone acceptance record.

Original exit 143 remains unexplained. Full-scan and isolated NVMe long-tail windows support a registration-level association, not driver causality, additive isolated costs or a causal explanation of the earlier 503.365 ms outlier. New full scans reached 376.175665 ms and isolated NVMe 286.210567 ms. Scheduling/descheduling, workload, warmup, thermal drift, pipe capture and inherited kernel peak-RSS accounting limit conclusions. Periodic RSS is unavailable by design. Missing IT8613/fan inputs, two unsupported GPU channels and the zero-input battery provider remain visible. These measurements are observations, not performance guarantees.

**Historical source limitation:** preliminary suite 001 predates changes in three diagnostic modules, and suite 002 predates the completion orchestrator change. Exact measured sources are privately retained and hash-verified but are not retrievable from public Git history by those hashes alone. Completion suite 003 matches the final harness. Published numerical reports regenerate from saved data using the final renderer; exact historical execution-source reproducibility requires access to the retained snapshots. Do not claim all suites ran the final code.

**Confirmed unfixed tooling bug:** the final `announce()` call in `scripts/measure_provider_latency.py` precedes final saving. A fixture-injected `BrokenPipeError` left a saved `running` checkpoint with three completed cases/33 valid records, no in-progress case and preserved raw files. Aliases were removed and handlers restored, but final cleanup/status diagnostics were not saved. An execution-log write error at that point is also a static risk, not separately reproduced. The completed historical archive explicitly records `complete`, cleanup `complete` and all 13 runs, so this does not invalidate its measurements or conceal a fabricated success. Recommend a narrow deterministic fix before any further diagnostic execution; none is authorized or attempted here. Existing signal/timeout tests do not cover that exception path.

## Git summary and publication prerequisites

- Branch: `codex/alpha1-prototype005`; HEAD: `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`; no upstream.
- Index empty. One modified tracked file (`docs/DESIGN_DECISIONS.md`) and 19 untracked intended deliverables; no unrelated work changed.
- The complete prospective change set contains six new Python files, 12 new documentation/data files from implementation, this new readiness report, and the modified decision document: **20 files total**. Ordinary `git diff` omits untracked files; this review inspected all of them separately.
- Tracked diff: 26 inserted lines, zero deletions in the decision document. The 19 new files comprise 15 UTF-8 text files and four gzip archives totaling 889,255 compressed bytes; the entire prospective content is approximately 1.2 MiB.
- Tracked changes only append authorization/acceptance history and clarify historical wording under Decision 43; Decisions 1–42 and their text remain unchanged.
- No staging, commit, push, PR, merge, rebase, remote operation or branch-protection change occurred. No Prototype 006 implementation began.

**Blocking prerequisite:** confirm/configure the intended contribution author/signoff name and email after separate authorization. Current author identity resolution fails. No secret/identity was guessed, and no Git configuration was changed.

**Recommended grouping:** one cohesive future milestone commit, for example `perf: characterize hwmon provider latency`, containing exactly the manifest above. Use `git commit -s`, inspect the resulting author and `Signed-off-by` trailer, and recheck the staged allowlist/whitespace before committing. Do not include ignored private evidence. Publication still requires Chris's explicit authorization and then the usual remote-state and hosted-CI checks. No source change or new hardware session is required to validate the accepted measurements.

The accepted measurement limitations and documented historical-source/final-status issues do not block accuracy or integrity of the reviewed content. They do constrain future claims and diagnostic use. A separately authorized Prototype 006 could add deterministic harness CI coverage, with the final-status regression/correction first, existing Python/Rust quality checks passing, and no real hardware access or runtime expansion.

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
