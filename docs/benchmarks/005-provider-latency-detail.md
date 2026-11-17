# SiliconWatch provider-level scan latency

Measured scan windows are observations, not performance guarantees or pure driver-read latency.

Suite status: **externally_interrupted_reviewed**; retained cases: 25; unstarted: 2.

Actual suite elapsed seconds: N/A. Selected session tokens: r001, r002, r003, r004, r005.

Configured bounds:

```json
{
  "metadata_reserve_bytes": 67108864,
  "observe_rss": false,
  "settle_seconds": 15,
  "suite_limit_bytes": 268435456,
  "suite_timeout_seconds": 1500
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
| 007-primary-full_before-all-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 008-primary-isolated-r001-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 009-primary-isolated-r002-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 010-primary-isolated-r003-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 011-primary-isolated-r004-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 012-primary-isolated-r005-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 013-primary-full_after-all-b1 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 014-primary-full_before-all-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 015-primary-isolated-r002-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 016-primary-isolated-r003-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 017-primary-isolated-r004-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 018-primary-isolated-r005-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 019-primary-isolated-r001-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 020-primary-full_after-all-b2 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 021-primary-full_before-all-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 022-primary-isolated-r003-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 023-primary-isolated-r004-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 024-primary-isolated-r005-b3 | 301 | 100 | 10 | 40 | 40 | 8388608 |
| 025-primary-isolated-r001-b3 | 301 | 100 | 10 | 40 | N/A | 8388608 |
| 026-primary-isolated-r002-b3 | 301 | 100 | 10 | 40 | N/A | 8388608 |
| 027-primary-full_after-all-b3 | 301 | 100 | 10 | 40 | N/A | 8388608 |

## Environment and registration inventory

```json
{
  "background_workload": "uncontrolled; load averages are observations, not process attribution",
  "binary_sha256": "130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8",
  "captured_at_utc": "2026-11-16T23:47:40.919710+00:00",
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
    0.65869140625,
    1.03271484375,
    1.09716796875
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
  "output_free_bytes": 1745872216064,
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
| 001-smoke-full_smoke-all-b0 | smoke / full_smoke | 0 | N/A | 11 | 100 | 11 | 0 | completed | 1.01905 | 0.015532 | 1.52416 | 0.00292072 | 23732 | 104106 |
| 002-smoke-isolated_smoke-r001-b0 | smoke / isolated_smoke | 0 | r001 | 11 | 100 | 11 | 0 | completed | 1.00241 | 0.004109 | 0.409911 | 0.00254404 | 23732 | 17210 |
| 003-smoke-isolated_smoke-r002-b0 | smoke / isolated_smoke | 0 | r002 | 11 | 100 | 11 | 0 | completed | 1.01766 | 0.005326 | 0.523357 | 0.00278793 | 23732 | 39500 |
| 004-smoke-isolated_smoke-r003-b0 | smoke / isolated_smoke | 0 | r003 | 11 | 100 | 11 | 0 | completed | 1.00251 | 0.0081 | 0.807975 | 0.00265593 | 23732 | 55078 |
| 005-smoke-isolated_smoke-r004-b0 | smoke / isolated_smoke | 0 | r004 | 11 | 100 | 11 | 0 | completed | 1.00219 | 0.003756 | 0.374781 | 0.00263909 | 23732 | 17015 |
| 006-smoke-isolated_smoke-r005-b0 | smoke / isolated_smoke | 0 | r005 | 11 | 100 | 11 | 0 | completed | 1.00289 | 0.00344 | 0.34301 | 0.00236749 | 23732 | 17333 |
| 007-primary-full_before-all-b1 | primary / full_before | 1 | N/A | 301 | 100 | 301 | 0 | completed | 30.3567 | 0.392751 | 1.29379 | 0.0646861 | 23732 | 2.85178e+06 |
| 008-primary-isolated-r001-b1 | primary / isolated | 1 | r001 | 301 | 100 | 301 | 0 | completed | 30.0213 | 0.08223 | 0.273905 | 0.0548642 | 24116 | 474109 |
| 009-primary-isolated-r002-b1 | primary / isolated | 1 | r002 | 301 | 100 | 301 | 0 | completed | 30.2847 | 0.123136 | 0.406595 | 0.0614702 | 24884 | 1.08396e+06 |
| 010-primary-isolated-r003-b1 | primary / isolated | 1 | r003 | 301 | 100 | 301 | 0 | completed | 30.0211 | 0.208056 | 0.693033 | 0.0603893 | 25460 | 1.51024e+06 |
| 011-primary-isolated-r004-b1 | primary / isolated | 1 | r004 | 301 | 100 | 301 | 0 | completed | 30.0207 | 0.065965 | 0.219732 | 0.0578414 | 25844 | 468735 |
| 012-primary-isolated-r005-b1 | primary / isolated | 1 | r005 | 301 | 100 | 301 | 0 | completed | 30.0213 | 0.07241 | 0.241195 | 0.059124 | 26612 | 477370 |
| 013-primary-full_after-all-b1 | primary / full_after | 1 | N/A | 301 | 100 | 301 | 0 | completed | 30.4442 | 0.384117 | 1.26171 | 0.0645929 | 27264 | 2.85179e+06 |
| 014-primary-full_before-all-b2 | primary / full_before | 2 | N/A | 301 | 100 | 301 | 0 | completed | 30.4022 | 0.382532 | 1.25824 | 0.0652413 | 28976 | 2.85183e+06 |
| 015-primary-isolated-r002-b2 | primary / isolated | 2 | r002 | 301 | 100 | 301 | 0 | completed | 30.2268 | 0.118498 | 0.392029 | 0.0632904 | 28988 | 1.08396e+06 |
| 016-primary-isolated-r003-b2 | primary / isolated | 2 | r003 | 301 | 100 | 301 | 0 | completed | 30.0228 | 0.197346 | 0.657321 | 0.059563 | 29616 | 1.5102e+06 |
| 017-primary-isolated-r004-b2 | primary / isolated | 2 | r004 | 301 | 100 | 301 | 0 | completed | 30.0206 | 0.065901 | 0.21952 | 0.0570705 | 30200 | 468773 |
| 018-primary-isolated-r005-b2 | primary / isolated | 2 | r005 | 301 | 100 | 301 | 0 | completed | 30.0243 | 0.076923 | 0.256202 | 0.0594556 | 30680 | 477428 |
| 019-primary-isolated-r001-b2 | primary / isolated | 2 | r001 | 301 | 100 | 301 | 0 | completed | 30.0204 | 0.077102 | 0.256832 | 0.0579099 | 31536 | 474108 |
| 020-primary-full_after-all-b2 | primary / full_after | 2 | N/A | 301 | 100 | 301 | 0 | completed | 30.1755 | 0.40311 | 1.33588 | 0.0658891 | 32140 | 2.85178e+06 |
| 021-primary-full_before-all-b3 | primary / full_before | 3 | N/A | 301 | 100 | 301 | 0 | completed | 30.1786 | 0.384181 | 1.27302 | 0.0630788 | 34424 | 2.8518e+06 |
| 022-primary-isolated-r003-b3 | primary / isolated | 3 | r003 | 301 | 100 | 301 | 0 | completed | 30.0218 | 0.210757 | 0.702014 | 0.0633195 | 34424 | 1.51023e+06 |
| 023-primary-isolated-r004-b3 | primary / isolated | 3 | r004 | 301 | 100 | 301 | 0 | completed | 30.0207 | 0.06857 | 0.228409 | 0.0590835 | 34424 | 468773 |
| 024-primary-isolated-r005-b3 | primary / isolated | 3 | r005 | 301 | 100 | 301 | 0 | completed | 30.0343 | 0.093414 | 0.311024 | 0.0682983 | 34780 | 477459 |
| 025-primary-isolated-r001-b3 | primary / isolated | 3 | r001 | 301 | 100 | N/A | 0 | external_interruption_accounting_unavailable | N/A | N/A | N/A | N/A | N/A | 436261 |

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
| 007-primary-full_before-all-b1 | 11 | 301 | 2709 | 0 | 602 | True | True |
| 008-primary-isolated-r001-b1 | 1 | 301 | 301 | 0 | 0 | True | True |
| 009-primary-isolated-r002-b1 | 3 | 301 | 903 | 0 | 0 | True | True |
| 010-primary-isolated-r003-b1 | 5 | 301 | 903 | 0 | 602 | True | True |
| 011-primary-isolated-r004-b1 | 1 | 301 | 301 | 0 | 0 | True | True |
| 012-primary-isolated-r005-b1 | 1 | 301 | 301 | 0 | 0 | True | True |
| 013-primary-full_after-all-b1 | 11 | 301 | 2709 | 0 | 602 | True | True |
| 014-primary-full_before-all-b2 | 11 | 301 | 2709 | 0 | 602 | True | True |
| 015-primary-isolated-r002-b2 | 3 | 301 | 903 | 0 | 0 | True | True |
| 016-primary-isolated-r003-b2 | 5 | 301 | 903 | 0 | 602 | True | True |
| 017-primary-isolated-r004-b2 | 1 | 301 | 301 | 0 | 0 | True | True |
| 018-primary-isolated-r005-b2 | 1 | 301 | 301 | 0 | 0 | True | True |
| 019-primary-isolated-r001-b2 | 1 | 301 | 301 | 0 | 0 | True | True |
| 020-primary-full_after-all-b2 | 11 | 301 | 2709 | 0 | 602 | True | True |
| 021-primary-full_before-all-b3 | 11 | 301 | 2709 | 0 | 602 | True | True |
| 022-primary-isolated-r003-b3 | 5 | 301 | 903 | 0 | 602 | True | True |
| 023-primary-isolated-r004-b3 | 1 | 301 | 301 | 0 | 0 | True | True |
| 024-primary-isolated-r005-b3 | 1 | 301 | 301 | 0 | 0 | True | True |
| 025-primary-isolated-r001-b3 | 1 | 277 | 277 | 0 | 0 | True | False |

## Timing distributions

Milliseconds. Each case/window is separate: no provider/repetition pooling. p95 requires N ≥200; p99 N ≥1,000. These reporting thresholds do not guarantee statistical confidence. First-start lateness is retained in scalar data but excluded from interval-lateness statistics.

| Case | Window | Metric | N | Median ms | p95 ms | p99 ms | Max ms |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 001-smoke-full_smoke-all-b0 | whole_run | snapshot_scan_duration_ns | 11 | 17.5412 | N/A | N/A | 44.0704 |
| 001-smoke-full_smoke-all-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-smoke-full_smoke-all-b0 | whole_run | lateness_ns | 10 | 0.0702585 | N/A | N/A | 0.090827 |
| 001-smoke-full_smoke-all-b0 | whole_run | interval_deviation_ns | 10 | 0.0702585 | N/A | N/A | 0.090827 |
| 001-smoke-full_smoke-all-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 17.5412 | N/A | N/A | 44.0704 |
| 001-smoke-full_smoke-all-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-smoke-full_smoke-all-b0 | post_warmup | lateness_ns | 10 | 0.0702585 | N/A | N/A | 0.090827 |
| 001-smoke-full_smoke-all-b0 | post_warmup | interval_deviation_ns | 10 | 0.0702585 | N/A | N/A | 0.090827 |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | snapshot_scan_duration_ns | 11 | 0.260064 | N/A | N/A | 0.28453 |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | lateness_ns | 10 | 0.079551 | N/A | N/A | 0.083333 |
| 002-smoke-isolated_smoke-r001-b0 | whole_run | interval_deviation_ns | 10 | 0.079551 | N/A | N/A | 0.083333 |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 0.260064 | N/A | N/A | 0.28453 |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | lateness_ns | 10 | 0.079551 | N/A | N/A | 0.083333 |
| 002-smoke-isolated_smoke-r001-b0 | post_warmup | interval_deviation_ns | 10 | 0.079551 | N/A | N/A | 0.083333 |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | snapshot_scan_duration_ns | 11 | 15.7444 | N/A | N/A | 16.0814 |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | lateness_ns | 10 | 0.066076 | N/A | N/A | 0.088122 |
| 003-smoke-isolated_smoke-r002-b0 | whole_run | interval_deviation_ns | 10 | 0.066076 | N/A | N/A | 0.088122 |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 15.7444 | N/A | N/A | 16.0814 |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | lateness_ns | 10 | 0.066076 | N/A | N/A | 0.088122 |
| 003-smoke-isolated_smoke-r002-b0 | post_warmup | interval_deviation_ns | 10 | 0.066076 | N/A | N/A | 0.088122 |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | snapshot_scan_duration_ns | 11 | 0.476227 | N/A | N/A | 1.02218 |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | lateness_ns | 10 | 0.0761755 | N/A | N/A | 0.078915 |
| 004-smoke-isolated_smoke-r003-b0 | whole_run | interval_deviation_ns | 10 | 0.0761755 | N/A | N/A | 0.078915 |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 0.476227 | N/A | N/A | 1.02218 |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | lateness_ns | 10 | 0.0761755 | N/A | N/A | 0.078915 |
| 004-smoke-isolated_smoke-r003-b0 | post_warmup | interval_deviation_ns | 10 | 0.0761755 | N/A | N/A | 0.078915 |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | snapshot_scan_duration_ns | 11 | 0.218918 | N/A | N/A | 0.333402 |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | lateness_ns | 10 | 0.0723875 | N/A | N/A | 0.0812 |
| 005-smoke-isolated_smoke-r004-b0 | whole_run | interval_deviation_ns | 10 | 0.0723875 | N/A | N/A | 0.0812 |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 0.218918 | N/A | N/A | 0.333402 |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | lateness_ns | 10 | 0.0723875 | N/A | N/A | 0.0812 |
| 005-smoke-isolated_smoke-r004-b0 | post_warmup | interval_deviation_ns | 10 | 0.0723875 | N/A | N/A | 0.0812 |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | snapshot_scan_duration_ns | 11 | 1.16845 | N/A | N/A | 1.29736 |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | lateness_ns | 10 | 0.0662415 | N/A | N/A | 0.07113 |
| 006-smoke-isolated_smoke-r005-b0 | whole_run | interval_deviation_ns | 10 | 0.0662415 | N/A | N/A | 0.07113 |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | snapshot_scan_duration_ns | 11 | 1.16845 | N/A | N/A | 1.29736 |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | lateness_ns | 10 | 0.0662415 | N/A | N/A | 0.07113 |
| 006-smoke-isolated_smoke-r005-b0 | post_warmup | interval_deviation_ns | 10 | 0.0662415 | N/A | N/A | 0.07113 |
| 007-primary-full_before-all-b1 | whole_run | snapshot_scan_duration_ns | 301 | 17.6151 | 18.4571 | N/A | 317.265 |
| 007-primary-full_before-all-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-primary-full_before-all-b1 | whole_run | lateness_ns | 300 | 0.063055 | 0.080948 | N/A | 217.297 |
| 007-primary-full_before-all-b1 | whole_run | interval_deviation_ns | 300 | 0.063055 | 0.080948 | N/A | 217.297 |
| 007-primary-full_before-all-b1 | post_warmup | snapshot_scan_duration_ns | 203 | 17.6194 | 18.5546 | N/A | 175.23 |
| 007-primary-full_before-all-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-primary-full_before-all-b1 | post_warmup | lateness_ns | 203 | 0.062995 | 0.080948 | N/A | 75.2632 |
| 007-primary-full_before-all-b1 | post_warmup | interval_deviation_ns | 203 | 0.062995 | 0.080948 | N/A | 75.2632 |
| 008-primary-isolated-r001-b1 | whole_run | snapshot_scan_duration_ns | 301 | 0.2305 | 0.333883 | N/A | 0.576415 |
| 008-primary-isolated-r001-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-primary-isolated-r001-b1 | whole_run | lateness_ns | 300 | 0.0636565 | 0.079917 | N/A | 0.125733 |
| 008-primary-isolated-r001-b1 | whole_run | interval_deviation_ns | 300 | 0.0636565 | 0.079917 | N/A | 0.125733 |
| 008-primary-isolated-r001-b1 | post_warmup | snapshot_scan_duration_ns | 201 | 0.2305 | 0.339552 | N/A | 0.576415 |
| 008-primary-isolated-r001-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-primary-isolated-r001-b1 | post_warmup | lateness_ns | 201 | 0.063857 | 0.080037 | N/A | 0.098381 |
| 008-primary-isolated-r001-b1 | post_warmup | interval_deviation_ns | 201 | 0.063857 | 0.080037 | N/A | 0.098381 |
| 009-primary-isolated-r002-b1 | whole_run | snapshot_scan_duration_ns | 301 | 15.7919 | 16.1942 | N/A | 334.584 |
| 009-primary-isolated-r002-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-primary-isolated-r002-b1 | whole_run | lateness_ns | 300 | 0.062805 | 0.078955 | N/A | 234.614 |
| 009-primary-isolated-r002-b1 | whole_run | interval_deviation_ns | 300 | 0.062805 | 0.078955 | N/A | 234.614 |
| 009-primary-isolated-r002-b1 | post_warmup | snapshot_scan_duration_ns | 201 | 15.7881 | 17.0063 | N/A | 334.584 |
| 009-primary-isolated-r002-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-primary-isolated-r002-b1 | post_warmup | lateness_ns | 201 | 0.062604 | 0.070419 | N/A | 234.614 |
| 009-primary-isolated-r002-b1 | post_warmup | interval_deviation_ns | 201 | 0.062604 | 0.070419 | N/A | 234.614 |
| 010-primary-isolated-r003-b1 | whole_run | snapshot_scan_duration_ns | 301 | 0.631627 | 1.15244 | N/A | 2.09204 |
| 010-primary-isolated-r003-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-primary-isolated-r003-b1 | whole_run | lateness_ns | 300 | 0.063111 | 0.077222 | N/A | 0.133246 |
| 010-primary-isolated-r003-b1 | whole_run | interval_deviation_ns | 300 | 0.063111 | 0.077222 | N/A | 0.133246 |
| 010-primary-isolated-r003-b1 | post_warmup | snapshot_scan_duration_ns | 201 | 0.636676 | 1.1643 | N/A | 2.09204 |
| 010-primary-isolated-r003-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-primary-isolated-r003-b1 | post_warmup | lateness_ns | 201 | 0.062996 | 0.07603 | N/A | 0.082011 |
| 010-primary-isolated-r003-b1 | post_warmup | interval_deviation_ns | 201 | 0.062996 | 0.07603 | N/A | 0.082011 |
| 011-primary-isolated-r004-b1 | whole_run | snapshot_scan_duration_ns | 301 | 0.179375 | 0.261878 | N/A | 0.4884 |
| 011-primary-isolated-r004-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-primary-isolated-r004-b1 | whole_run | lateness_ns | 300 | 0.0630205 | 0.077542 | N/A | 0.115383 |
| 011-primary-isolated-r004-b1 | whole_run | interval_deviation_ns | 300 | 0.0630205 | 0.077542 | N/A | 0.115383 |
| 011-primary-isolated-r004-b1 | post_warmup | snapshot_scan_duration_ns | 201 | 0.189743 | 0.26309 | N/A | 0.4884 |
| 011-primary-isolated-r004-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-primary-isolated-r004-b1 | post_warmup | lateness_ns | 201 | 0.063547 | 0.077953 | N/A | 0.093252 |
| 011-primary-isolated-r004-b1 | post_warmup | interval_deviation_ns | 201 | 0.063547 | 0.077953 | N/A | 0.093252 |
| 012-primary-isolated-r005-b1 | whole_run | snapshot_scan_duration_ns | 301 | 1.1681 | 1.23511 | N/A | 1.51016 |
| 012-primary-isolated-r005-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-primary-isolated-r005-b1 | whole_run | lateness_ns | 300 | 0.064293 | 0.078635 | N/A | 0.090106 |
| 012-primary-isolated-r005-b1 | whole_run | interval_deviation_ns | 300 | 0.064293 | 0.078635 | N/A | 0.090106 |
| 012-primary-isolated-r005-b1 | post_warmup | snapshot_scan_duration_ns | 201 | 1.17112 | 1.23107 | N/A | 1.39172 |
| 012-primary-isolated-r005-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-primary-isolated-r005-b1 | post_warmup | lateness_ns | 201 | 0.064548 | 0.079196 | N/A | 0.090106 |
| 012-primary-isolated-r005-b1 | post_warmup | interval_deviation_ns | 201 | 0.064548 | 0.079196 | N/A | 0.090106 |
| 013-primary-full_after-all-b1 | whole_run | snapshot_scan_duration_ns | 301 | 17.6397 | 18.4939 | N/A | 259.994 |
| 013-primary-full_after-all-b1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 013-primary-full_after-all-b1 | whole_run | lateness_ns | 300 | 0.062901 | 0.079055 | N/A | 160.034 |
| 013-primary-full_after-all-b1 | whole_run | interval_deviation_ns | 300 | 0.062901 | 0.079055 | N/A | 160.034 |
| 013-primary-full_after-all-b1 | post_warmup | snapshot_scan_duration_ns | 202 | 17.5775 | 18.3357 | N/A | 259.994 |
| 013-primary-full_after-all-b1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 013-primary-full_after-all-b1 | post_warmup | lateness_ns | 202 | 0.063091 | 0.078845 | N/A | 160.034 |
| 013-primary-full_after-all-b1 | post_warmup | interval_deviation_ns | 202 | 0.063091 | 0.078845 | N/A | 160.034 |
| 014-primary-full_before-all-b2 | whole_run | snapshot_scan_duration_ns | 301 | 17.6366 | 18.4789 | N/A | 334.564 |
| 014-primary-full_before-all-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 014-primary-full_before-all-b2 | whole_run | lateness_ns | 300 | 0.0639175 | 0.080428 | N/A | 234.597 |
| 014-primary-full_before-all-b2 | whole_run | interval_deviation_ns | 300 | 0.0639175 | 0.080428 | N/A | 234.597 |
| 014-primary-full_before-all-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 17.6676 | 18.2549 | N/A | 334.564 |
| 014-primary-full_before-all-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 014-primary-full_before-all-b2 | post_warmup | lateness_ns | 201 | 0.063707 | 0.078474 | N/A | 234.597 |
| 014-primary-full_before-all-b2 | post_warmup | interval_deviation_ns | 201 | 0.063707 | 0.078474 | N/A | 234.597 |
| 015-primary-isolated-r002-b2 | whole_run | snapshot_scan_duration_ns | 301 | 15.8162 | 16.1972 | N/A | 276.758 |
| 015-primary-isolated-r002-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 015-primary-isolated-r002-b2 | whole_run | lateness_ns | 300 | 0.0630505 | 0.082803 | N/A | 176.777 |
| 015-primary-isolated-r002-b2 | whole_run | interval_deviation_ns | 300 | 0.0630505 | 0.082803 | N/A | 176.777 |
| 015-primary-isolated-r002-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 15.8106 | 16.1373 | N/A | 276.758 |
| 015-primary-isolated-r002-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 015-primary-isolated-r002-b2 | post_warmup | lateness_ns | 201 | 0.062665 | 0.077864 | N/A | 176.777 |
| 015-primary-isolated-r002-b2 | post_warmup | interval_deviation_ns | 201 | 0.062665 | 0.077864 | N/A | 176.777 |
| 016-primary-isolated-r003-b2 | whole_run | snapshot_scan_duration_ns | 301 | 0.582676 | 1.05299 | N/A | 2.04884 |
| 016-primary-isolated-r003-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 016-primary-isolated-r003-b2 | whole_run | lateness_ns | 300 | 0.06299 | 0.072523 | N/A | 0.09205 |
| 016-primary-isolated-r003-b2 | whole_run | interval_deviation_ns | 300 | 0.06299 | 0.072523 | N/A | 0.09205 |
| 016-primary-isolated-r003-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 0.602292 | 1.06711 | N/A | 2.04884 |
| 016-primary-isolated-r003-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 016-primary-isolated-r003-b2 | post_warmup | lateness_ns | 201 | 0.062905 | 0.072453 | N/A | 0.09205 |
| 016-primary-isolated-r003-b2 | post_warmup | interval_deviation_ns | 201 | 0.062905 | 0.072453 | N/A | 0.09205 |
| 017-primary-isolated-r004-b2 | whole_run | snapshot_scan_duration_ns | 301 | 0.174165 | 0.260656 | N/A | 0.351665 |
| 017-primary-isolated-r004-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 017-primary-isolated-r004-b2 | whole_run | lateness_ns | 300 | 0.0630505 | 0.078615 | N/A | 0.089786 |
| 017-primary-isolated-r004-b2 | whole_run | interval_deviation_ns | 300 | 0.0630505 | 0.078615 | N/A | 0.089786 |
| 017-primary-isolated-r004-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 0.175256 | 0.258983 | N/A | 0.351665 |
| 017-primary-isolated-r004-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 017-primary-isolated-r004-b2 | post_warmup | lateness_ns | 201 | 0.063056 | 0.079055 | N/A | 0.089786 |
| 017-primary-isolated-r004-b2 | post_warmup | interval_deviation_ns | 201 | 0.063056 | 0.079055 | N/A | 0.089786 |
| 018-primary-isolated-r005-b2 | whole_run | snapshot_scan_duration_ns | 301 | 1.19566 | 1.27629 | N/A | 1.33161 |
| 018-primary-isolated-r005-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 018-primary-isolated-r005-b2 | whole_run | lateness_ns | 300 | 0.066712 | 0.081831 | N/A | 0.093833 |
| 018-primary-isolated-r005-b2 | whole_run | interval_deviation_ns | 300 | 0.066712 | 0.081831 | N/A | 0.093833 |
| 018-primary-isolated-r005-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 1.20018 | 1.27116 | N/A | 1.33157 |
| 018-primary-isolated-r005-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 018-primary-isolated-r005-b2 | post_warmup | lateness_ns | 201 | 0.07088 | 0.081831 | N/A | 0.093833 |
| 018-primary-isolated-r005-b2 | post_warmup | interval_deviation_ns | 201 | 0.07088 | 0.081831 | N/A | 0.093833 |
| 019-primary-isolated-r001-b2 | whole_run | snapshot_scan_duration_ns | 301 | 0.213658 | 0.317342 | N/A | 0.497397 |
| 019-primary-isolated-r001-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 019-primary-isolated-r001-b2 | whole_run | lateness_ns | 300 | 0.0628405 | 0.07603 | N/A | 0.084776 |
| 019-primary-isolated-r001-b2 | whole_run | interval_deviation_ns | 300 | 0.0628405 | 0.07603 | N/A | 0.084776 |
| 019-primary-isolated-r001-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 0.214751 | 0.32719 | N/A | 0.497397 |
| 019-primary-isolated-r001-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 019-primary-isolated-r001-b2 | post_warmup | lateness_ns | 201 | 0.063086 | 0.078394 | N/A | 0.084776 |
| 019-primary-isolated-r001-b2 | post_warmup | interval_deviation_ns | 201 | 0.063086 | 0.078394 | N/A | 0.084776 |
| 020-primary-full_after-all-b2 | whole_run | snapshot_scan_duration_ns | 301 | 14.8198 | 15.6749 | N/A | 200.393 |
| 020-primary-full_after-all-b2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 020-primary-full_after-all-b2 | whole_run | lateness_ns | 300 | 0.064599 | 0.08127 | N/A | 100.425 |
| 020-primary-full_after-all-b2 | whole_run | interval_deviation_ns | 300 | 0.064599 | 0.08127 | N/A | 100.425 |
| 020-primary-full_after-all-b2 | post_warmup | snapshot_scan_duration_ns | 201 | 14.8491 | 15.8174 | N/A | 200.393 |
| 020-primary-full_after-all-b2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 020-primary-full_after-all-b2 | post_warmup | lateness_ns | 201 | 0.064238 | 0.081279 | N/A | 100.425 |
| 020-primary-full_after-all-b2 | post_warmup | interval_deviation_ns | 201 | 0.064238 | 0.081279 | N/A | 100.425 |
| 021-primary-full_before-all-b3 | whole_run | snapshot_scan_duration_ns | 301 | 17.5622 | 18.2882 | N/A | 192.56 |
| 021-primary-full_before-all-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 021-primary-full_before-all-b3 | whole_run | lateness_ns | 300 | 0.0627355 | 0.078093 | N/A | 92.591 |
| 021-primary-full_before-all-b3 | whole_run | interval_deviation_ns | 300 | 0.0627355 | 0.078093 | N/A | 92.591 |
| 021-primary-full_before-all-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 17.6211 | 18.3618 | N/A | 192.56 |
| 021-primary-full_before-all-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 021-primary-full_before-all-b3 | post_warmup | lateness_ns | 201 | 0.062865 | 0.078485 | N/A | 92.591 |
| 021-primary-full_before-all-b3 | post_warmup | interval_deviation_ns | 201 | 0.062865 | 0.078485 | N/A | 92.591 |
| 022-primary-isolated-r003-b3 | whole_run | snapshot_scan_duration_ns | 301 | 0.606951 | 1.14832 | N/A | 3.0051 |
| 022-primary-isolated-r003-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 022-primary-isolated-r003-b3 | whole_run | lateness_ns | 300 | 0.0635815 | 0.078655 | N/A | 0.324493 |
| 022-primary-isolated-r003-b3 | whole_run | interval_deviation_ns | 300 | 0.0635815 | 0.078655 | N/A | 0.324493 |
| 022-primary-isolated-r003-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 0.606521 | 1.14832 | N/A | 2.20356 |
| 022-primary-isolated-r003-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 022-primary-isolated-r003-b3 | post_warmup | lateness_ns | 201 | 0.064088 | 0.079396 | N/A | 0.324493 |
| 022-primary-isolated-r003-b3 | post_warmup | interval_deviation_ns | 201 | 0.064088 | 0.079396 | N/A | 0.324493 |
| 023-primary-isolated-r004-b3 | whole_run | snapshot_scan_duration_ns | 301 | 0.176089 | 0.280422 | N/A | 0.349532 |
| 023-primary-isolated-r004-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 023-primary-isolated-r004-b3 | whole_run | lateness_ns | 300 | 0.0630355 | 0.079527 | N/A | 0.103802 |
| 023-primary-isolated-r004-b3 | whole_run | interval_deviation_ns | 300 | 0.0630355 | 0.079527 | N/A | 0.103802 |
| 023-primary-isolated-r004-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 0.174886 | 0.275393 | N/A | 0.349532 |
| 023-primary-isolated-r004-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 023-primary-isolated-r004-b3 | post_warmup | lateness_ns | 201 | 0.062996 | 0.080468 | N/A | 0.103802 |
| 023-primary-isolated-r004-b3 | post_warmup | interval_deviation_ns | 201 | 0.062996 | 0.080468 | N/A | 0.103802 |
| 024-primary-isolated-r005-b3 | whole_run | snapshot_scan_duration_ns | 301 | 1.23233 | 1.52652 | N/A | 2.0867 |
| 024-primary-isolated-r005-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 024-primary-isolated-r005-b3 | whole_run | lateness_ns | 300 | 0.078575 | 0.388833 | N/A | 0.614233 |
| 024-primary-isolated-r005-b3 | whole_run | interval_deviation_ns | 300 | 0.078575 | 0.388833 | N/A | 0.614233 |
| 024-primary-isolated-r005-b3 | post_warmup | snapshot_scan_duration_ns | 201 | 1.22956 | 1.51414 | N/A | 2.0867 |
| 024-primary-isolated-r005-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 024-primary-isolated-r005-b3 | post_warmup | lateness_ns | 201 | 0.07649 | 0.385116 | N/A | 0.614233 |
| 024-primary-isolated-r005-b3 | post_warmup | interval_deviation_ns | 201 | 0.07649 | 0.385116 | N/A | 0.614233 |
| 025-primary-isolated-r001-b3 | whole_run | snapshot_scan_duration_ns | 277 | 0.214019 | 0.342919 | N/A | 0.492989 |
| 025-primary-isolated-r001-b3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 025-primary-isolated-r001-b3 | whole_run | lateness_ns | 276 | 0.0627605 | 0.078094 | N/A | 0.083244 |
| 025-primary-isolated-r001-b3 | whole_run | interval_deviation_ns | 276 | 0.0627605 | 0.078094 | N/A | 0.083244 |
| 025-primary-isolated-r001-b3 | post_warmup | snapshot_scan_duration_ns | 177 | 0.215712 | N/A | N/A | 0.492989 |
| 025-primary-isolated-r001-b3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 025-primary-isolated-r001-b3 | post_warmup | lateness_ns | 177 | 0.062966 | N/A | N/A | 0.083244 |
| 025-primary-isolated-r001-b3 | post_warmup | interval_deviation_ns | 177 | 0.062966 | N/A | N/A | 0.083244 |

## Long-tail evidence

A scan window >100 ms is a descriptive flag, not an alarm. All scalar records remain saved; tail/neighbor sequence indexes reference them without duplicating raw telemetry. Failed attempts are separately identified in each record.

| Case | Tail count | Maximum sequence | Maximum ms | Neighbor records retained |
| --- | ---: | ---: | ---: | ---: |
| 001-smoke-full_smoke-all-b0 | 0 | 2 | 44.0704 | 0 |
| 002-smoke-isolated_smoke-r001-b0 | 0 | 6 | 0.28453 | 0 |
| 003-smoke-isolated_smoke-r002-b0 | 0 | 8 | 16.0814 | 0 |
| 004-smoke-isolated_smoke-r003-b0 | 0 | 1 | 1.02218 | 0 |
| 005-smoke-isolated_smoke-r004-b0 | 0 | 9 | 0.333402 | 0 |
| 006-smoke-isolated_smoke-r005-b0 | 0 | 9 | 1.29736 | 0 |
| 007-primary-full_before-all-b1 | 4 | 11 | 317.265 | 11 |
| 008-primary-isolated-r001-b1 | 0 | 164 | 0.576415 | 0 |
| 009-primary-isolated-r002-b1 | 3 | 176 | 334.584 | 9 |
| 010-primary-isolated-r003-b1 | 0 | 213 | 2.09204 | 0 |
| 011-primary-isolated-r004-b1 | 0 | 130 | 0.4884 | 0 |
| 012-primary-isolated-r005-b1 | 0 | 20 | 1.51016 | 0 |
| 013-primary-full_after-all-b1 | 4 | 242 | 259.994 | 11 |
| 014-primary-full_before-all-b2 | 4 | 128 | 334.564 | 11 |
| 015-primary-isolated-r002-b2 | 3 | 286 | 276.758 | 8 |
| 016-primary-isolated-r003-b2 | 0 | 301 | 2.04884 | 0 |
| 017-primary-isolated-r004-b2 | 0 | 117 | 0.351665 | 0 |
| 018-primary-isolated-r005-b2 | 0 | 19 | 1.33161 | 0 |
| 019-primary-isolated-r001-b2 | 0 | 102 | 0.497397 | 0 |
| 020-primary-full_after-all-b2 | 2 | 293 | 200.393 | 6 |
| 021-primary-full_before-all-b3 | 2 | 148 | 192.56 | 6 |
| 022-primary-isolated-r003-b3 | 0 | 90 | 3.0051 | 0 |
| 023-primary-isolated-r004-b3 | 0 | 106 | 0.349532 | 0 |
| 024-primary-isolated-r005-b3 | 0 | 114 | 2.0867 | 0 |
| 025-primary-isolated-r001-b3 | 0 | 229 | 0.492989 | 0 |

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

- 025-primary-isolated-r001-b3: fewer complete records than requested
