# Prototype 004 — Foreground monitoring overhead measurements

**Date:** 2026-11-16. **Scope:** the unchanged Prototype 003 release binary on one interactive Linux workstation. These observations are a reproducible baseline, not performance guarantees or approval for background monitoring.

## Evidence and conditions

The primary matrix completed **24 runs, 21,804 independently parsed attempts, zero failed attempts**, and zero truncated outputs. It took **72.318 minutes** including 15-second settling gaps, retained **134.779 MiB** of raw output locally, and used **31.740 child CPU seconds** across all runs. All runs retained the same provider inventory. The primary matrix uses three separate processes per condition, a ten-second post-warmup exclusion, and 1 Hz sampled RSS.

- CachyOS, Linux **7.3.4-1-cachyos**, x86-64; AMD Ryzen AI 9 HX 370 / Radeon 890M; 24 logical CPUs; 15,956,052 KiB RAM.
- Rust **1.85.0**, release build with locked dependencies; Python **3.14.10**. Application revision: `3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7`. Binary SHA-256: `130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8`. Result manifests preserve the harness hash and explicit case configuration.
- Observed governor: `powersave`; output filesystem: Btrfs. No drivers, hardware settings, affinity, governor, thermal policy, or caches were changed. The workstation remained interactive; primary before/after one-minute load averages ranged **0.575–2.008**. These load observations do not attribute interference to particular processes.
- Providers: `acpitz`, `nvme`, `amdgpu`, `k10temp`, `iwlwifi_1`, and `hidpp_battery_0` (zero input channels). Each snapshot contained **11 inputs: seven temperatures, two voltages, and two unsupported GPU inputs**. Nine inputs were `ok`, two `unsupported`; unsupported readings remain explicit and are not failed sampling attempts. Historical IT8613/fan inputs remained absent. This baseline is not comparable to the earlier 24-input cohort without controlling that difference.

The harness launches one child at a time and excludes compilation from timing. It captures stdout/stderr through bounded pipes into local files so output and disk budgets can be enforced. Therefore the primary sink is **pipe-to-file capture**, not a direct-file-descriptor benchmark or visible-terminal rendering. Child CPU excludes Python CPU accounting but can be affected by capture backpressure. Null controls write directly to `/dev/null`.

Whole-run/post-warmup distribution summaries, RSS coverage, CPU attribution, and per-run integrity checks are in [baseline details](004-baseline-details.md) and [control details](004-controls-details.md). Full scalar observations, including first-start lateness, are preserved in the sanitized [baseline results](results/004-baseline.json.gz) and [control results](results/004-controls.json.gz). See [reproduction commands and limitations](README.md).

## CPU and observed memory

Every row contains **three independent runs**. CPU is the median and range of per-process average utilization; **100% means one logical CPU**. RSS is the median and range of each run's **post-warmup median** in KiB, measured with `smaps_rollup`. These are run-to-run comparisons, not pooled distributions.

| Nominal span | Interval | Format | CPU %, median [range] | Sampled RSS KiB, median [range] |
| ---: | ---: | --- | ---: | ---: |
| 30 s | 100 ms | Table | 1.478 [1.381–1.724] | 2,944 [2,940–2,952] |
| 30 s | 100 ms | JSON Lines | 1.318 [1.151–1.398] | 2,972 [2,944–2,980] |
| 30 s | 1,000 ms | Table | 0.144 [0.143–0.186] | 2,916 [2,912–2,952] |
| 30 s | 1,000 ms | JSON Lines | 0.152 [0.148–0.158] | 2,932 [2,900–2,948] |
| 300 s | 100 ms | Table | 1.445 [1.313–1.662] | 2,968 [2,940–3,008] |
| 300 s | 100 ms | JSON Lines | 1.434 [1.263–1.524] | 2,956 [2,948–3,000] |
| 300 s | 1,000 ms | Table | 0.142 [0.140–0.144] | 2,944 [2,936–2,952] |
| 300 s | 1,000 ms | JSON Lines | 0.141 [0.136–0.154] | 2,952 [2,928–2,964] |

Across all primary RSS observations, values ranged **2,868–3,012 KiB**. No RSS reads failed. Post-warmup windows contained **20–21 observations** in short cases and **290–296** in sustained cases. Sustained early-to-late median changes ranged **0–20 KiB**; this small observed drift is reported rather than called proof of a flat allocation profile or absence of leaks. A 1 Hz observer can miss transient allocations and can interact with 1,000 ms sampling phase.

**Kernel peak RSS is not an isolated application peak.** Kernel-reported child-lifetime values ranged **23,644–82,140 KiB**, substantially above sampled child RSS. A separate synthetic-process check observed inherited launch high-water accounting on this platform even with `posix_spawn`; automated tests also cover accounting and allocation observations. The raw kernel measurement is preserved with this limitation; it is not corrected by subtracting a guessed constant. The parent runner used **7.426 CPU seconds** during primary launch/capture/reap windows, reported separately from child CPU; subsequent parsing/report CPU is outside those windows.

## Post-warmup scan duration and scheduling

Each cell below is the **range of three per-run statistics**, in milliseconds. N is the per-run observation count/range. No observations from different runs, intervals, or formats are pooled. p95 is reported only at **N ≥ 200**, p99 only at **N ≥ 1,000**; those reporting thresholds do not establish statistical confidence or independence. N/A means insufficient observations.

| Span | Interval | Format | N per run | Median range, ms | p95 range, ms | p99 range, ms | Maximum range, ms |
| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 30 s | 100 ms | Table | 203–204 | 14.771–18.510 | 15.665–40.253 | N/A | 208.124–368.478 |
| 30 s | 100 ms | JSON Lines | 201–202 | 14.761–17.938 | 15.403–20.507 | N/A | 299.976–381.172 |
| 30 s | 1,000 ms | Table | 21 | 17.511–18.138 | N/A | N/A | 18.578–251.043 |
| 30 s | 1,000 ms | JSON Lines | 21 | 17.415–18.020 | N/A | N/A | 19.479–195.604 |
| 300 s | 100 ms | Table | 2,902–2,903 | 17.417–17.791 | 18.722–22.069 | 115.138–128.500 | 449.747–503.365 |
| 300 s | 100 ms | JSON Lines | 2,901–2,907 | 17.512–18.152 | 18.393–22.133 | 83.181–118.626 | 382.942–438.808 |
| 300 s | 1,000 ms | Table | 291 | 17.714–17.749 | 19.203–20.617 | N/A | 103.829–308.769 |
| 300 s | 1,000 ms | JSON Lines | 291 | 17.567–17.898 | 19.195–21.201 | N/A | 120.065–361.939 |

Post-warmup start-lateness medians ranged **0.0598–0.0641 ms** across primary runs. The largest whole-run scan was **503.365 ms** and largest start lateness was **403.454 ms**. At 100 ms, long scans delayed the following attempt; the CLI's existing reanchoring behavior remains unchanged. At 1,000 ms, post-warmup maximum lateness stayed below **0.953 ms** in these cases. Actual child elapsed time exceeded nominal span by **0.020–5.886 seconds**, rather than being silently treated as the configured duration.

There were **296 observation-window-exceeds-interval flags** across the primary matrix. These are userspace observation advisories, not failed attempts or proof of physically stale hardware data. Scan timing includes discovery, identity processing, sequential reads, and any driver waits or descheduling during that window; the report does not attribute its tail to a specific provider or cause. Detailed reports preserve whole-run summaries and separate failed-attempt distributions (N = 0 here); saved scalar results preserve startup lateness.

Publication review independently matched the **503,365,198 ns** maximum to captured table case `019-table-100ms-3001n-file-r3`, sequence **2751**. The following start was **403,453,512 ns** late; the next start gap was **100,061,602 ns**, consistent with reanchoring rather than a catch-up burst. Both retained raw output and the sanitized scalar archive agree. No provider-level trace or continuous scheduler evidence was collected, so the cause of this observation remains **unknown**. Sustained 100 ms scan tails above the interval are observations, not failed attempts, performance guarantees, or evidence that any particular provider is responsible.

## Output and observer controls

All **12 controls completed normally with exit 0**, taking **8.845 minutes** including settling and retaining **11.155 MiB** of captured output. The six captured runs independently verified **301 attempts each: 1,806 total, zero failed attempts**. The six null-output runs provide resource measurements only: actual attempts, failed attempts, scan distributions, and stdout volume are **unknown**, not inferred from the requested count or exit code. RSS observation was disabled in all controls; missing RSS samples are intentional, not zero memory use.

The table compares matched **30-second, 100 ms** configurations. Each cell is the median and range of three per-run measurements. The first two rows reuse the corresponding primary results; they are not additional runs.

| Format | Output / RSS observation | Child CPU %, median [range] | Runner CPU s, median [range] | Independently parsed attempts per run |
| --- | --- | ---: | ---: | ---: |
| Table | Capture / on | 1.478 [1.381–1.724] | 0.068 [0.060–0.076] | 301 |
| JSON Lines | Capture / on | 1.318 [1.151–1.398] | 0.060 [0.053–0.073] | 301 |
| Table | Capture / off | 1.458 [1.387–1.667] | 0.048 [0.046–0.056] | 301 |
| JSON Lines | Capture / off | 1.489 [1.210–1.766] | 0.050 [0.042–0.055] | 301 |
| Table | Null / off | 1.426 [1.308–1.616] | 0.030 [0.025–0.032] | Unknown |
| JSON Lines | Null / off | 1.429 [1.357–1.687] | 0.028 [0.027–0.030] | Unknown |

Captured control output was approximately **1.047 MB per table run** and **2.852 MB per JSON Lines run** (decimal MB). Different serialized volume does not, by itself, establish a CPU ranking: the observed CPU ranges overlap.

Matching repetition numbers, child CPU differences for observer-on minus observer-off capture ranged **−0.286 to +0.337 percentage points** for table and **−0.615 to +0.187** for JSON Lines. Capture-minus-null differences ranged **−0.158 to +0.240** for table and **−0.147 to +0.080** for JSON Lines. These are descriptive matched-condition comparisons; controls ran later, not simultaneously or under an experimentally fixed background workload. The mixed signs and variability do **not** establish a precise observer or output penalty, and no measured difference is subtracted as an exact correction. Runner CPU is separately visible and higher with capture/observation, but its effect on child scheduling is not fully isolated.

Primary and control suite elapsed times sum to **81.163 minutes**, including their settling gaps; this excludes the inter-suite pause and preliminary/smoke validation. No visible terminal-rendering CPU cost was measured.

## Integrity, limits, and recommendation

An earlier preliminary `baseline-001` suite was intentionally interrupted after review identified a risk that accumulated metadata could exceed its initial cap. The cap was not actually reached. Compact encoding, explicit metadata projection, and reserve accounting corrected that risk before the completed matrix. Its [partial evidence](results/004-preliminary-interrupted.json.gz) is retained separately and excluded from baseline comparisons; unavailable final accounting remains unknown. The [output-limit fixture](results/004-output-limit-fixture.json.gz) intentionally hit its configured cap and is a safety test, not a successful performance measurement. Full primary/control runs did not require changing application behavior or system configuration.

The workstation's thermal state and background activity were not controlled experimentally; no separate thermal-drift measurement or renderer CPU attribution is claimed. Warm caches, kernel scheduling, drivers, power management, pipe draining, filesystem/page-cache effects, and the memory observer can all affect results. Three repetitions describe observed variability, not confidence bounds. CPU percentages are child process CPU, not whole-system or gaming/frame-time overhead. RSS samples do not establish instantaneous peak memory.

Visible-terminal rendering and isolated-provider experiments were not executed. The results cannot identify which provider causes the occasional long scans, promise performance on other hardware, or justify background monitoring, adaptive polling, caching, or protective actions. Decision 43 remains pending.

**Recommended next milestone:** [Prototype 005: Provider-Level Sensor Latency Characterization](../milestones/004-overhead-characterization.md#8-recommended-next-milestone), using the existing fixture-root capability, explicitly labeled isolated-provider scans, bounded repeated trials, and tests confirming source evidence is preserved. Acceptance requires reproducible comparative observations; causal attribution may remain unestablished. Isolated timings must not be added together to explain full-scan duration. Preserve current application semantics and do not implement this follow-up automatically.
