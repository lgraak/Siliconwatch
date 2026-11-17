# SiliconWatch provider-level scan latency

Measured scan windows are observations, not performance guarantees or pure driver-read latency.

Suite status: **complete**; retained cases: 13; unstarted: 0.

Actual suite elapsed seconds: 398.407. Selected session tokens: r001, r002, r003, r004, r005.

Configured bounds:

```json
{
  "metadata_reserve_bytes": 67108864,
  "observe_rss": false,
  "settle_seconds": 15,
  "suite_limit_bytes": 134217728,
  "suite_timeout_seconds": 600
}
```

## Case bounds

Configured limits describe every planned case, including unstarted cases. Effective timeout is the saved value passed to the child; N/A means unrecorded or unstarted, not zero.

| Case | Requested | Interval ms | Warmup s | Configured timeout s | Effective timeout s | Combined output cap bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 001-smoke-full_smoke-all-b0 | 11 | 100 | 0 | 5 | 5 | 1048576 |
| 002-smoke-isolated_smoke-r001-b0 | 11 | 100 | 0 | 5 | 5 | 1048576 |
| 003-smoke-isolated_smoke-r002-b0 | 11 | 100 | 0 | 5 | 5 | 1048576 |
| 004-smoke-isolated_smoke-r003-b0 | 11 | 100 | 0 | 5 | 5 | 1048576 |
| 005-smoke-isolated_smoke-r004-b0 | 11 | 100 | 0 | 5 | 5 | 1048576 |
| 006-smoke-isolated_smoke-r005-b0 | 11 | 100 | 0 | 5 | 5 | 1048576 |
| 007-primary-full_before-all-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 008-primary-isolated-r003-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 009-primary-isolated-r004-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 010-primary-isolated-r005-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 011-primary-isolated-r001-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 012-primary-isolated-r002-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 013-primary-full_after-all-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |

## Environment and registration inventory

```json
{
  "background_workload": "uncontrolled; load averages are observations, not process attribution",
  "binary_sha256": "130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8",
  "captured_at_utc": "2026-11-17T00:43:12.809313+00:00",
  "cpu": {
    "affinity_cpu_count": 24,
    "cgroup_quota": {
      "period_us": 100000,
      "quota_us": null,
      "status": "unlimited_at_current_cgroup"
    },
    "logical_count": 24,
    "model": "AMD Ryzen AI 9 HX 370 w/ Radeon 890M",
    "observed_governors": [
      "powersave"
    ]
  },
  "git_revision": "6051a2e8622f7f95bcf51e614a7086eafb7dcf4e",
  "installed_cargo_1_85": "cargo 1.85.0 (d73d2caf9 2024-12-31)",
  "installed_rust_1_85": "rustc 1.85.0 (4d91de4e4 2025-02-17)",
  "limitations": [
    "No physical sensor-read validation in inventory",
    "Current cgroup quota may omit stricter ancestor limits",
    "Installed compiler version does not prove binary build provenance",
    "No hostname, serial, hardware source path, or environment dump collected"
  ],
  "load_average_1_5_15_minutes": [
    2.0830078125,
    1.69873046875,
    1.50830078125
  ],
  "memory_total_kib": 15956052,
  "os": {
    "architecture": "x86_64",
    "id": "cachyos",
    "kernel_release": "7.3.4-1-cachyos",
    "pretty_name": "CachyOS",
    "version_id": null
  },
  "output_filesystem": "btrfs",
  "output_free_bytes": 1745853161472,
  "power_supplies": [
    {
      "online": 1,
      "status": "Full",
      "type": "Battery"
    }
  ],
  "provenance_schema_version": 1,
  "provider_inventory": [
    {
      "discovery_status": "ok",
      "exposed_input_count": 0,
      "input_categories": {},
      "name_status": "ok",
      "provider": "hidpp_battery_0"
    },
    {
      "discovery_status": "ok",
      "exposed_input_count": 1,
      "input_categories": {
        "temperature": 1
      },
      "name_status": "ok",
      "provider": "acpitz"
    },
    {
      "discovery_status": "ok",
      "exposed_input_count": 1,
      "input_categories": {
        "temperature": 1
      },
      "name_status": "ok",
      "provider": "iwlwifi_1"
    },
    {
      "discovery_status": "ok",
      "exposed_input_count": 1,
      "input_categories": {
        "temperature": 1
      },
      "name_status": "ok",
      "provider": "k10temp"
    },
    {
      "discovery_status": "ok",
      "exposed_input_count": 3,
      "input_categories": {
        "temperature": 3
      },
      "name_status": "ok",
      "provider": "nvme"
    },
    {
      "discovery_status": "ok",
      "exposed_input_count": 5,
      "input_categories": {
        "frequency": 1,
        "power": 1,
        "temperature": 1,
        "voltage": 2
      },
      "name_status": "ok",
      "provider": "amdgpu"
    }
  ],
  "python_version": "3.14.10"
}
```

| Token | Provider | Channels | Availability |
| --- | --- | ---: | --- |
| r001 | acpitz | 1 | ok |
| r002 | nvme | 3 | ok |
| r003 | amdgpu | 5 | ok |
| r004 | k10temp | 1 | ok |
| r005 | iwlwifi_1 | 1 | ok |
| r006 | hidpp_battery_0 | 0 | zero_inputs |

## Cases and resource accounting

CPU 100% is one logical CPU. Runner CPU is separate from child CPU. Kernel child-lifetime peak RSS includes inherited launch high-water accounting; it is not an isolated application peak. Periodic RSS observation is disabled consistently; absent sampled RSS is unavailable, not zero.

| Case | Phase / role | Block | Token | Requested | Interval ms | Verified attempts | Failed attempts | State | Elapsed s | Child CPU s | CPU % | Runner CPU s | Kernel peak KiB | Output bytes |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 001-smoke-full_smoke-all-b0 | smoke / full_smoke | 0 | N/A | 11 | 100 | 11 | 0 | completed | 1.04854 | 0.015076 | 1.43781 | 0.00314394 | 23440 | 104111 |
| 002-smoke-isolated_smoke-r001-b0 | smoke / isolated_smoke | 0 | r001 | 11 | 100 | 11 | 0 | completed | 1.00221 | 0.003981 | 0.397221 | 0.00251641 | 23440 | 17584 |
| 003-smoke-isolated_smoke-r002-b0 | smoke / isolated_smoke | 0 | r002 | 11 | 100 | 11 | 0 | completed | 1.01799 | 0.005278 | 0.518474 | 0.00266538 | 23440 | 40249 |
| 004-smoke-isolated_smoke-r003-b0 | smoke / isolated_smoke | 0 | r003 | 11 | 100 | 11 | 0 | completed | 1.0032 | 0.00766 | 0.763554 | 0.00232053 | 23440 | 56196 |
| 005-smoke-isolated_smoke-r004-b0 | smoke / isolated_smoke | 0 | r004 | 11 | 100 | 11 | 0 | completed | 1.00217 | 0.003371 | 0.33637 | 0.00232328 | 23440 | 17385 |
| 006-smoke-isolated_smoke-r005-b0 | smoke / isolated_smoke | 0 | r005 | 11 | 100 | 11 | 0 | completed | 1.00343 | 0.003835 | 0.38219 | 0.00246379 | 23440 | 17707 |
| 007-primary-full_before-all-b3 | primary / full_before | 3 | N/A | 301 | 100 | 301 | 0 | completed | 30.3314 | 0.449899 | 1.48328 | 0.0634108 | 23440 | 2.85183e+06 |
| 008-primary-isolated-r003-b3 | primary / isolated | 3 | r003 | 301 | 100 | 301 | 0 | completed | 30.021 | 0.241373 | 0.804014 | 0.0591054 | 23572 | 1.54098e+06 |
| 009-primary-isolated-r004-b3 | primary / isolated | 3 | r004 | 301 | 100 | 301 | 0 | completed | 30.0207 | 0.065711 | 0.218885 | 0.0573514 | 23956 | 479006 |
| 010-primary-isolated-r005-b3 | primary / isolated | 3 | r005 | 301 | 100 | 301 | 0 | completed | 30.0226 | 0.075244 | 0.250625 | 0.0586032 | 24532 | 487509 |
| 011-primary-isolated-r001-b3 | primary / isolated | 3 | r001 | 301 | 100 | 301 | 0 | completed | 30.0205 | 0.078828 | 0.262581 | 0.0571926 | 25108 | 484347 |
| 012-primary-isolated-r002-b3 | primary / isolated | 3 | r002 | 301 | 100 | 301 | 0 | completed | 30.8067 | 0.118542 | 0.384793 | 0.061754 | 25876 | 1.10448e+06 |
| 013-primary-full_after-all-b3 | primary / full_after | 3 | N/A | 301 | 100 | 301 | 0 | completed | 30.6322 | 0.4077 | 1.33095 | 0.0672645 | 26484 | 2.85184e+06 |

## Reading coverage

Counts describe emitted observations, not physical accuracy. Failed readings exclude unsupported channels, which remain separate. Valid output prefixes do not establish completed attempts after interruption.

| Case | Expected channels | Matched snapshots | OK readings | Failed readings | Unsupported | Usable cohort | Complete |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| 001-smoke-full_smoke-all-b0 | 11 | 11 | 99 | 0 | 22 | True | True |
| 002-smoke-isolated_smoke-r001-b0 | 1 | 11 | 11 | 0 | 0 | True | True |
| 003-smoke-isolated_smoke-r002-b0 | 3 | 11 | 33 | 0 | 0 | True | True |
| 004-smoke-isolated_smoke-r003-b0 | 5 | 11 | 33 | 0 | 22 | True | True |
| 005-smoke-isolated_smoke-r004-b0 | 1 | 11 | 11 | 0 | 0 | True | True |
| 006-smoke-isolated_smoke-r005-b0 | 1 | 11 | 11 | 0 | 0 | True | True |
| 007-primary-full_before-all-b3 | 11 | 301 | 2709 | 0 | 602 | True | True |
| 008-primary-isolated-r003-b3 | 5 | 301 | 903 | 0 | 602 | True | True |
| 009-primary-isolated-r004-b3 | 1 | 301 | 301 | 0 | 0 | True | True |
| 010-primary-isolated-r005-b3 | 1 | 301 | 301 | 0 | 0 | True | True |
| 011-primary-isolated-r001-b3 | 1 | 301 | 301 | 0 | 0 | True | True |
| 012-primary-isolated-r002-b3 | 3 | 301 | 903 | 0 | 0 | True | True |
| 013-primary-full_after-all-b3 | 11 | 301 | 2709 | 0 | 602 | True | True |

## Timing distributions

Milliseconds. Each case/window is separate: no provider/repetition pooling. p95 requires N ≥200; p99 N ≥1,000. These reporting thresholds do not guarantee statistical confidence. First-start lateness is retained in scalar data but excluded from interval-lateness statistics.

| Case | Window | Metric | N | Median ms | p95 ms | p99 ms | Max ms |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 001-smoke-full_smoke-all-b0 | whole_run | snapshot_scan_duration_ns | 11 | 17.5811 | N/A | N/A | 133.136 |
| 001-smoke-full_smoke-all-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-smoke-full_smoke-all-b0 | whole_run | lateness_ns | 10 | 0.063195 | N/A | N/A | 33.1658 |
| 001-smoke-full_smoke-all-b0 | whole_run | interval_deviation_ns | 10 | 0.063195 | N/A | N/A | 33.1658 |
| 001-smoke-full_smoke-all-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 17.5811 | N/A | N/A | 133.136 |
| 001-smoke-full_smoke-all-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-smoke-full_smoke-all-b0 | post_warmup | lateness_ns | 10 | 0.063195 | N/A | N/A | 33.1658 |
| 001-smoke-full_smoke-all-b0 | post_warmup | interval_deviation_ns | 10 | 0.063195 | N/A | N/A | 33.1658 |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | snapshot_scan_duration_ns | 11 | 0.250837 | N/A | N/A | 0.280122 |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | lateness_ns | 10 | 0.06604 | N/A | N/A | 0.081108 |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | interval_deviation_ns | 10 | 0.06604 | N/A | N/A | 0.081108 |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 0.250837 | N/A | N/A | 0.280122 |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | lateness_ns | 10 | 0.06604 | N/A | N/A | 0.081108 |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | interval_deviation_ns | 10 | 0.06604 | N/A | N/A | 0.081108 |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | snapshot_scan_duration_ns | 11 | 15.7677 | N/A | N/A | 16.064 |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | lateness_ns | 10 | 0.066028 | N/A | N/A | 0.078557 |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | interval_deviation_ns | 10 | 0.066028 | N/A | N/A | 0.078557 |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 15.7677 | N/A | N/A | 16.064 |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | lateness_ns | 10 | 0.066028 | N/A | N/A | 0.078557 |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | interval_deviation_ns | 10 | 0.066028 | N/A | N/A | 0.078557 |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | snapshot_scan_duration_ns | 11 | 0.493761 | N/A | N/A | 1.16427 |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | lateness_ns | 10 | 0.077096 | N/A | N/A | 0.081278 |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | interval_deviation_ns | 10 | 0.077096 | N/A | N/A | 0.081278 |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 0.493761 | N/A | N/A | 1.16427 |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | lateness_ns | 10 | 0.077096 | N/A | N/A | 0.081278 |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | interval_deviation_ns | 10 | 0.077096 | N/A | N/A | 0.081278 |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | snapshot_scan_duration_ns | 11 | 0.177782 | N/A | N/A | 0.258792 |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | lateness_ns | 10 | 0.0672135 | N/A | N/A | 0.086305 |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | interval_deviation_ns | 10 | 0.0672135 | N/A | N/A | 0.086305 |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 0.177782 | N/A | N/A | 0.258792 |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | lateness_ns | 10 | 0.0672135 | N/A | N/A | 0.086305 |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | interval_deviation_ns | 10 | 0.0672135 | N/A | N/A | 0.086305 |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | snapshot_scan_duration_ns | 11 | 1.19068 | N/A | N/A | 1.2076 |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | lateness_ns | 10 | 0.076468 | N/A | N/A | 0.081953 |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | interval_deviation_ns | 10 | 0.076468 | N/A | N/A | 0.081953 |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 1.19068 | N/A | N/A | 1.2076 |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | lateness_ns | 10 | 0.076468 | N/A | N/A | 0.081953 |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | interval_deviation_ns | 10 | 0.076468 | N/A | N/A | 0.081953 |
| 007-primary-full_before-all-b3 | whole_run | snapshot_scan_duration_ns | 301 | 17.7771 | 20.7079 | N/A | 231.905 |
| 007-primary-full_before-all-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-primary-full_before-all-b3 | whole_run | lateness_ns | 300 | 0.062351 | 0.090001 | N/A | 131.953 |
| 007-primary-full_before-all-b3 | whole_run | interval_deviation_ns | 300 | 0.062351 | 0.090001 | N/A | 131.953 |
| 007-primary-full_before-all-b3 | post_warmup | snapshot_scan_duration_ns | 202 | 17.6962 | 19.1867 | N/A | 185.879 |
| 007-primary-full_before-all-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-primary-full_before-all-b3 | post_warmup | lateness_ns | 202 | 0.0623605 | 0.081064 | N/A | 85.9119 |
| 007-primary-full_before-all-b3 | post_warmup | interval_deviation_ns | 202 | 0.0623605 | 0.081064 | N/A | 85.9119 |
| 008-primary-isolated-r003-b3 | whole_run | snapshot_scan_duration_ns | 301 | 0.692492 | 1.57975 | N/A | 3.52383 |
| 008-primary-isolated-r003-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-primary-isolated-r003-b3 | whole_run | lateness_ns | 300 | 0.062227 | 0.069144 | N/A | 0.115063 |
| 008-primary-isolated-r003-b3 | whole_run | interval_deviation_ns | 300 | 0.062227 | 0.069144 | N/A | 0.115063 |
| 008-primary-isolated-r003-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 0.726064 | 1.57975 | N/A | 3.52383 |
| 008-primary-isolated-r003-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-primary-isolated-r003-b3 | post_warmup | lateness_ns | 201 | 0.062262 | 0.070285 | N/A | 0.081787 |
| 008-primary-isolated-r003-b3 | post_warmup | interval_deviation_ns | 201 | 0.062262 | 0.070285 | N/A | 0.081787 |
| 009-primary-isolated-r004-b3 | whole_run | snapshot_scan_duration_ns | 301 | 0.173433 | 0.257951 | N/A | 1.13914 |
| 009-primary-isolated-r004-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-primary-isolated-r004-b3 | whole_run | lateness_ns | 300 | 0.0626555 | 0.076343 | N/A | 0.11344 |
| 009-primary-isolated-r004-b3 | whole_run | interval_deviation_ns | 300 | 0.0626555 | 0.076343 | N/A | 0.11344 |
| 009-primary-isolated-r004-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 0.18215 | 0.270805 | N/A | 1.13914 |
| 009-primary-isolated-r004-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-primary-isolated-r004-b3 | post_warmup | lateness_ns | 201 | 0.062816 | 0.074248 | N/A | 0.11344 |
| 009-primary-isolated-r004-b3 | post_warmup | interval_deviation_ns | 201 | 0.062816 | 0.074248 | N/A | 0.11344 |
| 010-primary-isolated-r005-b3 | whole_run | snapshot_scan_duration_ns | 301 | 1.1644 | 1.2457 | N/A | 1.33867 |
| 010-primary-isolated-r005-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-primary-isolated-r005-b3 | whole_run | lateness_ns | 300 | 0.0653105 | 0.079629 | N/A | 0.091875 |
| 010-primary-isolated-r005-b3 | whole_run | interval_deviation_ns | 300 | 0.0653105 | 0.079629 | N/A | 0.091875 |
| 010-primary-isolated-r005-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 1.16486 | 1.22891 | N/A | 1.28538 |
| 010-primary-isolated-r005-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-primary-isolated-r005-b3 | post_warmup | lateness_ns | 201 | 0.065635 | 0.07993 | N/A | 0.086563 |
| 010-primary-isolated-r005-b3 | post_warmup | interval_deviation_ns | 201 | 0.065635 | 0.07993 | N/A | 0.086563 |
| 011-primary-isolated-r001-b3 | whole_run | snapshot_scan_duration_ns | 301 | 0.215302 | 0.33237 | N/A | 0.687602 |
| 011-primary-isolated-r001-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-primary-isolated-r001-b3 | whole_run | lateness_ns | 300 | 0.0627635 | 0.070005 | N/A | 0.10841 |
| 011-primary-isolated-r001-b3 | whole_run | interval_deviation_ns | 300 | 0.0627635 | 0.070005 | N/A | 0.10841 |
| 011-primary-isolated-r001-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 0.216524 | 0.362666 | N/A | 0.687602 |
| 011-primary-isolated-r001-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-primary-isolated-r001-b3 | post_warmup | lateness_ns | 201 | 0.062634 | 0.070005 | N/A | 0.092878 |
| 011-primary-isolated-r001-b3 | post_warmup | interval_deviation_ns | 201 | 0.062634 | 0.070005 | N/A | 0.092878 |
| 012-primary-isolated-r002-b3 | whole_run | snapshot_scan_duration_ns | 301 | 15.779 | 16.269 | N/A | 286.211 |
| 012-primary-isolated-r002-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-primary-isolated-r002-b3 | whole_run | lateness_ns | 300 | 0.0626685 | 0.079559 | N/A | 186.256 |
| 012-primary-isolated-r002-b3 | whole_run | interval_deviation_ns | 300 | 0.0626685 | 0.079559 | N/A | 186.256 |
| 012-primary-isolated-r002-b3 | post_warmup | snapshot_scan_duration_ns | 207 | 15.7889 | 16.1666 | N/A | 256.697 |
| 012-primary-isolated-r002-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-primary-isolated-r002-b3 | post_warmup | lateness_ns | 207 | 0.062728 | 0.079591 | N/A | 156.739 |
| 012-primary-isolated-r002-b3 | post_warmup | interval_deviation_ns | 207 | 0.062728 | 0.079591 | N/A | 156.739 |
| 013-primary-full_after-all-b3 | whole_run | snapshot_scan_duration_ns | 301 | 17.6764 | 18.8808 | N/A | 376.176 |
| 013-primary-full_after-all-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 013-primary-full_after-all-b3 | whole_run | lateness_ns | 300 | 0.06314 | 0.078843 | N/A | 276.207 |
| 013-primary-full_after-all-b3 | whole_run | interval_deviation_ns | 300 | 0.06314 | 0.078843 | N/A | 276.207 |
| 013-primary-full_after-all-b3 | post_warmup | snapshot_scan_duration_ns | 206 | 17.7323 | 18.7856 | N/A | 99.9519 |
| 013-primary-full_after-all-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 013-primary-full_after-all-b3 | post_warmup | lateness_ns | 206 | 0.062837 | 0.077901 | N/A | 0.096837 |
| 013-primary-full_after-all-b3 | post_warmup | interval_deviation_ns | 206 | 0.062837 | 0.077901 | N/A | 0.096837 |

## Long-tail evidence

A scan window >100 ms is a descriptive flag, not an alarm. All scalar records remain saved; tail/neighbor sequence indexes reference them without duplicating raw telemetry. Failed attempts are separately identified in each record.

| Case | Tail count | Maximum sequence | Maximum ms | Neighbor records retained |
| --- | ---: | ---: | ---: | ---: |
| 001-smoke-full_smoke-all-b0 | 1 | 6 | 133.136 | 3 |
| 002-smoke-isolated_smoke-r001-b0 | 0 | 2 | 0.280122 | 0 |
| 003-smoke-isolated_smoke-r002-b0 | 0 | 9 | 16.064 | 0 |
| 004-smoke-isolated_smoke-r003-b0 | 0 | 11 | 1.16427 | 0 |
| 005-smoke-isolated_smoke-r004-b0 | 0 | 10 | 0.258792 | 0 |
| 006-smoke-isolated_smoke-r005-b0 | 0 | 11 | 1.2076 | 0 |
| 007-primary-full_before-all-b3 | 7 | 43 | 231.905 | 19 |
| 008-primary-isolated-r003-b3 | 0 | 120 | 3.52383 | 0 |
| 009-primary-isolated-r004-b3 | 0 | 127 | 1.13914 | 0 |
| 010-primary-isolated-r005-b3 | 0 | 48 | 1.33867 | 0 |
| 011-primary-isolated-r001-b3 | 0 | 196 | 0.687602 | 0 |
| 012-primary-isolated-r002-b3 | 5 | 1 | 286.211 | 12 |
| 013-primary-full_after-all-b3 | 5 | 90 | 376.176 | 10 |

## Identity context

Temporary roots change diagnostic source paths. Removing peers may change collision status; this never establishes unique physical identity. Candidate/evidence changes invalidate the smoke comparison.

```json
[
  {
    "candidate_equal": true,
    "canonical_device_equal": true,
    "changes": [],
    "channel_count": 1,
    "compatible": true,
    "errors": [],
    "evidence_equal": true,
    "root_changed": true,
    "source_path_changes": 1,
    "token": "r001"
  },
  {
    "candidate_equal": true,
    "canonical_device_equal": true,
    "changes": [],
    "channel_count": 3,
    "compatible": true,
    "errors": [],
    "evidence_equal": true,
    "root_changed": true,
    "source_path_changes": 3,
    "token": "r002"
  },
  {
    "candidate_equal": true,
    "canonical_device_equal": true,
    "changes": [],
    "channel_count": 5,
    "compatible": true,
    "errors": [],
    "evidence_equal": true,
    "root_changed": true,
    "source_path_changes": 5,
    "token": "r003"
  },
  {
    "candidate_equal": true,
    "canonical_device_equal": true,
    "changes": [],
    "channel_count": 1,
    "compatible": true,
    "errors": [],
    "evidence_equal": true,
    "root_changed": true,
    "source_path_changes": 1,
    "token": "r004"
  },
  {
    "candidate_equal": true,
    "canonical_device_equal": true,
    "changes": [],
    "channel_count": 1,
    "compatible": true,
    "errors": [],
    "evidence_equal": true,
    "root_changed": true,
    "source_path_changes": 1,
    "token": "r005"
  }
]
```

## Interpretation and limitations

Full-scan controls bracket isolated cases within each repetition block. Compare contemporary windows, never sum isolated durations or subtract them as exact full-scan contributions. Snapshot timing includes timestamp preamble, discovery, identity/collision processing, reads and descheduling. Serialization/capture happens afterward but may delay subsequent starts. Child/runner CPU and output bytes expose some overhead, not exact kernel/provider attribution.

Thirty-second cases do not recreate the historical 300-second run containing the 503.365 ms maximum. Non-recurrence is inconclusive. No driver is established as its cause by these measurements. Background work, thermal/frequency drift, driver caches and identity-context changes limit inference; blocking I/O prevents a hard completion guarantee.

Incomplete cases and missing measurements remain explicit. An entirely failed or incomplete comparison is not successful characterization; valid comparisons may still yield inconclusive attribution.
