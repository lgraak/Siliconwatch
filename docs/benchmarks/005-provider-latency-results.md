# Prototype 005 provider-latency observations

**Acceptance update — 2026-11-16:** **Chris accepted Alpha 1, Prototype 005 as technically complete with documented limitations.** The separately authorized six-smoke/seven-primary session completed and passed independent verification. The original partial experiment below remains historical evidence; see [the completion review](#authorized-completion-review--2026-11-16) for the separate new cohort and the [formal acceptance record](../milestones/005-provider-latency-characterization.md#formal-acceptance-and-publication-review--2026-11-16). Acceptance does not authorize Git publication or resolve Decision 43's pending local API permissions policy.

Reviewed 2026-11-16 (local date). **Partial experiment:** six fresh smoke cases and 18 of 21 primary cases completed. Two complete repetition blocks provide bracketed comparisons; block three lacks its final control. No performance guarantee or causal explanation of the historical 503.365 ms maximum is established.

## Protocol and provenance

The unchanged Rust 1.85.0 release executable sampled through the existing `--hwmon-root` interface. Each primary case requested 301 attempts at 100 ms with JSON Lines output. Three rotated blocks were planned, each with full-scan controls before and after five isolated registrations. Cases ran sequentially, with 15-second settling intervals. Whole-run data and a ten-second post-warmup subset remain saved. Capture used bounded pipes and local files, with periodic RSS observation disabled consistently. There were no 300-second trials or system configuration changes.

- Build: `cargo +1.85.0 build --release --locked --offline --target-dir target/provider-latency`.
- Binary SHA-256: `130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8`.
- Application revision: `6051a2e8622f7f95bcf51e614a7086eafb7dcf4e`; exact seven harness source hashes are in each archive. Their measured versions are also retained privately.
- CachyOS, Linux `7.3.4-1-cachyos`, x86-64, AMD Ryzen AI 9 HX 370 with Radeon 890M, 24 logical CPUs, 15,956,052 KiB RAM; observed governor `powersave`, Btrfs output filesystem, Python 3.14.10. No governor or other setting was changed.
- Interactive workstation with light documentation/review work. Background load, temperatures, frequency drift, driver caching and scheduling were not controlled. Pre-case one-minute load averages for retained cases were at most 1.499; load average does not attribute process interference.
- Six registrations were visible: acpitz (1 input), nvme (3), amdgpu (5), k10temp (1), iwlwifi_1 (1), and hidpp_battery_0 (zero inputs, recorded but not selected). The 11 inputs comprise seven temperature, two voltage, one frequency and one power input. Nine are supported; GPU frequency/power remain unsupported. Previously observed IT8613/fan inputs were absent.

All five smoke identity comparisons preserved candidates, evidence and canonical device paths. Diagnostic root/source paths changed as expected. No live collision status changed; fixtures cover that difference when isolation removes collision peers. Tokens are session-local registrations, not persistent physical IDs.

## Completion and interruptions

An initial attempt stopped after two 11-record smoke cases on an alias-cleanup defect. It ran no primary cases. Rewinding the held directory stream before cleanup fixed the reproduced problem; regression tests passed. Original captures, results and measured sources remain preserved. Chris explicitly approved one fresh bounded run, allowing eight total smoke cases across both attempts.

The fresh run completed 24 cases: six smokes (66 records) and 18 primary cases (5,418 records). Its execution session then ended with code **143** while case 025 was active. The original checkpoint remains unchanged with `status: running`, `in_progress: 025-primary-isolated-r001-b3`, and elapsed **909.687585 seconds at the pre-case checkpoint**. This is not final elapsed time. The termination source is unknown; that exit code does not prove the Python handler received or processed SIGTERM. Ordinary SIGINT/SIGTERM handling exists, but abrupt execution-environment termination can prevent finalization.

Case 025 contains 277 complete, valid JSON records from the requested 301, with no failed readings. Its final process status, CPU, peak RSS, final elapsed time and total completed attempts are unavailable. Cases 026 (nvme repetition three) and 027 (the final full-scan control) were unstarted. No retry or continuation was run. A host process check found no remaining harness or measurement executable. Only the verified owned alias and empty local directories were removed; no raw data or original result was deleted or overwritten.

The reviewed derivative labels the interruption explicitly and includes the parsed prefix with null accounting. It preserves all 24 completed run entries. Its `offline_review` records the original checkpoint hash and recovery evidence. Total retained raw output is **25,430,839 bytes (24.253 MiB)**, well below the 256 MiB storage limit. Reviewed local evidence, including source copies and derivative metadata, occupies about 30.1 MiB. Final suite elapsed time is unavailable; the checkpoint was within the 25-minute limit, but successful bounded finalization is not claimed.

## Completed primary comparisons

Ranges below are ranges of individual per-run statistics, not pooled quantiles. Every completed primary case has 301 observations; post-warmup windows have 201–203. p95 is reported only at N ≥200; no case/window reaches the N ≥1,000 threshold for p99. These thresholds do not guarantee statistical confidence. Maximum and tail counts include warmup. The interrupted prefix is excluded from this comparison.

| Cohort | Complete cases | Post-warmup median range ms | Post-warmup p95 range ms | Whole-run maximum ms | Scan windows >100 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Full scan | 5/6 | 14.849–17.668 | 15.817–18.555 | 334.564 | 16 |
| acpitz | 2/3 | 0.215–0.231 | 0.327–0.340 | 0.576 | 0 |
| nvme | 2/3 | 15.788–15.811 | 16.137–17.006 | 334.584 | 6 |
| amdgpu | 3/3 | 0.602–0.637 | 1.067–1.164 | 3.005 | 0 |
| k10temp | 3/3 | 0.175–0.190 | 0.259–0.275 | 0.488 | 0 |
| iwlwifi_1 | 3/3 | 1.171–1.230 | 1.231–1.514 | 2.087 | 0 |

Blocks one and two are fully bracketed. The third amdgpu/k10temp/iwlwifi observations have a preceding full control but no following control. Keep this imbalance visible when interpreting the ranges. Completed primary captures contain **20,468 OK reading observations, 4,816 unsupported observations, zero failed readings and zero failed scan attempts**. These counts do not establish calibration or physical accuracy.

| Cohort | Average child CPU % range across cases | Kernel child-lifetime peak KiB range |
| --- | ---: | ---: |
| Full scan | 1.258–1.336 | 23,732–34,424 |
| acpitz | 0.257–0.274 | 24,116–31,536 |
| nvme | 0.392–0.407 | 24,884–28,988 |
| amdgpu | 0.657–0.702 | 25,460–34,424 |
| k10temp | 0.220–0.228 | 25,844–34,424 |
| iwlwifi_1 | 0.241–0.311 | 26,612–34,780 |

CPU percentage is child user-plus-system CPU divided by parent-observed launch/capture/reap elapsed time, with 100% equal to one logical CPU. This denominator includes observation/cleanup overhead and is not exact kernel process lifetime; the polling loop can add roughly 50 ms of detection delay. Completed primary child CPU totals 3.406999 seconds; runner CPU during those captures totals 1.105168 seconds, excluding other orchestration/report work. Completed primary capture durations range from 30.020 to 30.444 seconds. Kernel peak RSS includes inherited launch high-water accounting and must not be interpreted as isolated application memory or growth. Sampled RSS is unavailable by design. These measurements do not isolate all observer/output overhead.

## Long-tail evidence and interpretation

The largest completed isolated observation is **334,583,664 ns**, case `009-primary-isolated-r002-b1`, sequence 176. Its initial start lateness was 61,923 ns; all three NVMe readings were OK. The largest full-scan observation is **334,563,647 ns**, case `014-primary-full_before-all-b2`, sequence 128, with initial start lateness 63,337 ns, nine OK and two unsupported readings. Their next attempts started approximately 234.614 and 234.597 ms late, respectively, then returned to approximately 100 ms start gaps. Raw hashes and neighboring scalar records corroborate these observations.

**Observed association:** selected NVMe-registration scan windows contain the only isolated >100 ms tails, while contemporary full scans also contain tails. Comparable tails were not distributed across the other selected registrations in retained observations. Initial wakeup lateness alone does not explain these two maxima because the delay lies within the recorded scan window.

**Unknown cause:** the window includes userspace discovery, identity processing, file reads, kernel waits and possible descheduling. This experiment cannot distinguish them or prove that a driver caused either these events or the historical **503.365 ms** event. External activity might contribute, but dominance by scheduling or any other mechanism is unproven. Do not sum isolated durations or subtract them from full scans as component costs. The historical maximum did not recur; shorter, incomplete experiments neither reproduce its conditions nor disprove it.

## Evidence and reproduction

- [Commands, bounds and interpretation](005-provider-latency-README.md)
- [Generated per-case details](005-provider-latency-detail.md)
- [Reviewed results including interrupted prefix](results/005-provider-latency-reviewed.json.gz)
- [Unmodified fresh-run checkpoint](results/005-provider-latency-checkpoint.json.gz)
- [Unmodified preliminary two-smoke results](results/005-provider-latency-preliminary.json.gz)
- [Milestone review and verification](../milestones/005-provider-latency-characterization.md)

Archives contain sanitized configuration, provenance, scalar records, counts, statistics and hashes; they exclude raw sensor values, source paths, full identity candidates and private manifests. Original private output remains under ignored `target/provider-latency-results/`. The detail report regenerates byte-for-byte from the reviewed archive using the current renderer. Original preliminary report rendering predates the bounds-display correction; its original local report remains untouched.

**Earlier recommendation:** review this partial characterization before authorizing any further hardware runs. A completed third block would require a separately approved continuation/repeat plan with contemporary controls; it cannot be silently appended as if uninterrupted. No sustained experiment or in-scan instrumentation is recommended without a separate, bounded proposal.

## Authorized completion review — 2026-11-16

Chris subsequently authorized the [completion proposal](../milestones/005-provider-latency-completion-proposal.md), recorded under [Decision 43](../DESIGN_DECISIONS.md#prototype-005-completion-authorization--2026-11-16). One new session ran six smokes followed by exactly seven primary cases, with **no retries or extra hardware runs**. This is a separate experiment, not a resumed or repaired original block three.

### Preflight, execution and preservation

All 139 Python tests and all 53 Rust tests on each approved toolchain passed before launch, together with formatting and Clippy. The Rust 1.85.0 release executable was not rebuilt; its preserved SHA-256 still matches. The full registration fingerprint matched the original private manifest, including provider, parent/device links and channel evidence. No conflicting measurement process was found. Kernel, CPU and the 11-input cohort were unchanged; free storage, background-load and resource projections passed. Live smoke identity checks then preserved candidate/evidence for all five selected registrations.

The harness recorded **398.407239 seconds**, with **0.074059 seconds separately recorded for metadata preflight**. Foreground GNU timeout was configured to send TERM before 590 seconds from preflight start and escalate after five seconds, within the 600-second authorization. Its enclosing command exited **0**, without reaching that deadline. These are observed timings and configured cancellation limits, not a guarantee against blocking kernel I/O or abrupt loss of the enclosing environment. Independent offline review and documentation occurred after the measurement session.

All 13 measurement children exited zero and were reaped. Alias cleanup completed; the alias directory was absent, and a host process check found no remaining measurement process. Raw capture totaled **10,053,219 bytes (9.587 MiB)**. Final local completion evidence, including measured-source copies and offline verification material, remained below 11 MiB; all preserved suites remained below 42 MiB, within the 128 MiB new / 256 MiB combined limits. The exact per-artifact sizes and hashes are independently checkable in the saved evidence.

All **78 original files** retained their SHA-256, size and modification time. This includes the 277-record interrupted prefix, unchanged checkpoint, original reports, source snapshots and recovery documentation. Its unavailable exit/CPU/RSS/attempt/duration accounting remains unavailable. The source and binary hashes in the new result identify the code actually measured.

### New seven-case results

Every primary case contains 301 records. Median and p95 below refer to its post-warmup window; maximum and tail counts refer to its whole run. No quantiles are pooled. Each post-warmup N exceeds 200, permitting p95; none reaches 1,000, so p99 remains unavailable.

| New primary case | Post-warmup N | Median ms | p95 ms | Whole-run max ms | >100 ms windows | Average child CPU % |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Full-before | 202 | 17.696243 | 19.186732 | 231.904714 | 7 | 1.483279 |
| amdgpu | 201 | 0.726064 | 1.579746 | 3.523832 | 0 | 0.804014 |
| k10temp | 201 | 0.182150 | 0.270805 | 1.139144 | 0 | 0.218885 |
| iwlwifi_1 | 201 | 1.164863 | 1.228911 | 1.338666 | 0 | 0.250625 |
| acpitz | 201 | 0.216524 | 0.362666 | 0.687602 | 0 | 0.262581 |
| nvme | 207 | 15.788920 | 16.166594 | 286.210567 | 5 | 0.384793 |
| Full-after | 206 | 17.732258 | 18.785577 | 376.175665 | 5 | 1.330953 |

Six smokes produced 66 records; seven primary cases produced 2,107, totaling **2,173 independently validated records**. Primary reading observations were **8,127 OK, 1,806 unsupported, zero failed readings and zero failed attempts**. Primary child CPU totaled 1.437297 seconds; runner CPU during capture totaled 0.424681972 seconds. Kernel child-lifetime peak RSS ranged 23,440–26,484 KiB, retaining the inherited-launch accounting limitation. Periodic RSS remained disabled. CPU percentages use parent-observed launch/capture/reap elapsed time, not exact kernel process lifetime.

The largest new full scan, 376.175665 ms, occurred at full-after sequence 90, starting 9.222576828 seconds into the case. Its initial lateness was 63,795 ns; the following start was 276.206652 ms late, followed by a 100.064317 ms gap. All five full-after tails occurred during warmup. NVMe's new maximum was its first observation; one of its five tails occurred after warmup. Both reporting windows and all neighboring evidence remain saved.

### Comparison set and inference

Following the inclusion rule specified before execution, original complete blocks one/two plus the new block form **21 bracketed primary cases / 6,321 records**. Original block three's four completed cases and interrupted prefix remain separately available supplemental evidence. Session labels and actual timestamps remain intact; no later control is represented as bracketing older observations.

This comparison set contains six full controls with 26 >100 ms windows and a maximum of 376.175665 ms; three isolated NVMe cases contain 11 such windows and a maximum of 334.583664 ms. Other isolated providers have no >100 ms windows in that set. These counts are descriptive summaries, not pooled quantiles or independent probabilities.

The new session continues the **observed NVMe-registration latency association**. It does not distinguish kernel/provider waits from descheduling inside scan windows, establish that isolated costs add up to full scans, or demonstrate the cause of the historical 503.365 ms event. The 503 ms maximum did not recur and recurrence was not an acceptance requirement. The workstation remained uncontrolled; the largest observed pre-case one-minute load average was 2.225 on 24 logical CPUs, versus 1.499 in the earlier retained session. This difference is context, not an explanation of any individual outlier.

### Evidence and recommendation

- [Completion machine-readable results](results/005-provider-latency-completion.json.gz)
- [Completion generated per-case report](005-provider-latency-completion-detail.md)
- [Preflight, execution and independent verification record](results/005-provider-latency-completion-verification.json)
- [Reproduction instructions](005-provider-latency-README.md#authorized-single-block-completion-mode)

The new detail report regenerates byte-for-byte from its corresponding archive. Original archives/detail reports remain unchanged. Independent checks covered all capture hashes, schemas, timestamps, monotonic timing, freshness bounds, identities and channel coverage, statuses, CPU arithmetic, quantile thresholds, tail neighbors, completeness, resource bounds and source/binary provenance.

**Accepted — 2026-11-16:** Chris subsequently accepted Prototype 005 based on successful deterministic validation, the authorized completed session, independent verification and preserved original evidence. The earlier recommendation for acceptance has been fulfilled; publication and further hardware runs remain unauthorized. Local API permissions remain **Pending**.

Acceptance retains every measurement qualification above: the original third block is incomplete, the fresh block is a separate cohort, exit 143 remains unexplained, NVMe association is not driver causality, and the historical 503.365 ms outlier is not causally explained. Scheduling, uncontrolled workload, warmup and RSS-accounting limitations remain applicable. The immutable completion verification JSON records the pre-acceptance assessment and is intentionally unchanged. See the [publication-readiness review](../milestones/005-publication-readiness.md) for privacy, source-reproducibility and tooling findings; no new measurements were performed for that review.
