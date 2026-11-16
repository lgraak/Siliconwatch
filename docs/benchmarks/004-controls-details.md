# SiliconWatch foreground monitoring overhead measurements

These are measurements of the recorded release binary and conditions, not performance guarantees.

Suite status: **complete**. Retained runs: 12; not started/unfinished: 0. In-progress case: N/A.

## Provenance

```json
{
  "background_workload": "uncontrolled; load averages are observations, not process attribution",
  "binary_sha256": "130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8",
  "captured_at_utc": "2026-11-16T22:38:16.099113+00:00",
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
  "git_revision": "3ccde79d9c95a7e212a0f3c2dd4424dfa23032a7",
  "installed_cargo_1_85": "cargo 1.85.0 (d73d2caf9 2024-12-31)",
  "installed_rust_1_85": "rustc 1.85.0 (4d91de4e4 2025-02-17)",
  "limitations": [
    "No physical sensor-read validation in inventory",
    "Current cgroup quota may omit stricter ancestor limits",
    "Installed compiler version does not prove binary build provenance",
    "No hostname, serial, hardware source path, or environment dump collected"
  ],
  "load_average_1_5_15_minutes": [
    0.8720703125,
    0.91650390625,
    0.9873046875
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
  "output_free_bytes": 1745914417152,
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

## Per-run resource measurements

CPU 100% represents one logical CPU. Kernel peak RSS and sampled RSS are different metrics. N/A means unavailable, not zero.

**Peak-RSS attribution warning:** kernel child-lifetime peak includes inherited launch high-water accounting on the tested platform, even with posix_spawn. It is not an isolated SiliconWatch application peak. Sampled RSS describes the running child and can miss transient peaks; no guessed launch constant is subtracted.

| Case | Interval ms | Format | Requested | Emitted | Failed attempts | State | Elapsed s | CPU s | CPU % | Peak RSS KiB | Stdout bytes |
| --- | ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | 100 | table | 301 | 301 | 0 | completed | 30.109 | 0.501815 | 1.66666 | 22572 | 1.04726e+06 |
| 002-json-lines-100ms-301n-file-r1 | 100 | json-lines | 301 | 301 | 0 | completed | 30.5081 | 0.454192 | 1.48876 | 23480 | 2.85162e+06 |
| 003-table-100ms-301n-null-r1 | 100 | table | 301 | N/A | N/A | completed | 30.7277 | 0.438264 | 1.42629 | 24056 | N/A |
| 004-json-lines-100ms-301n-null-r1 | 100 | json-lines | 301 | N/A | N/A | completed | 30.3349 | 0.433611 | 1.42941 | 24056 | N/A |
| 005-table-100ms-301n-null-r2 | 100 | table | 301 | N/A | N/A | completed | 30.2078 | 0.395251 | 1.30844 | 24056 | N/A |
| 006-json-lines-100ms-301n-file-r2 | 100 | json-lines | 301 | 301 | 0 | completed | 30.1779 | 0.365186 | 1.21011 | 24056 | 2.85158e+06 |
| 007-table-100ms-301n-file-r2 | 100 | table | 301 | 301 | 0 | completed | 30.7895 | 0.427195 | 1.38747 | 24632 | 1.04726e+06 |
| 008-json-lines-100ms-301n-null-r2 | 100 | json-lines | 301 | N/A | N/A | completed | 30.411 | 0.412617 | 1.3568 | 26360 | N/A |
| 009-table-100ms-301n-null-r3 | 100 | table | 301 | N/A | N/A | completed | 30.4068 | 0.49149 | 1.61638 | 26360 | N/A |
| 010-json-lines-100ms-301n-null-r3 | 100 | json-lines | 301 | N/A | N/A | completed | 30.6235 | 0.516481 | 1.68655 | 26552 | N/A |
| 011-table-100ms-301n-file-r3 | 100 | table | 301 | 301 | 0 | completed | 30.288 | 0.441728 | 1.45843 | 26552 | 1.04724e+06 |
| 012-json-lines-100ms-301n-file-r3 | 100 | json-lines | 301 | 301 | 0 | completed | 30.8663 | 0.54512 | 1.76607 | 27896 | 2.85161e+06 |

## CPU attribution and completion

CPU per attempt requires validated complete output, exit 0, and normal termination. Failed attempts remain in that denominator. Resource-only null-output runs cannot establish completion counts.

| Case | Exit | Output complete | User CPU s | System CPU s | CPU ms/attempt | Runner CPU s | Stderr bytes |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | 0 | true | 0.08166 | 0.420155 | 1.66716 | 0.0557761 | 0 |
| 002-json-lines-100ms-301n-file-r1 | 0 | true | 0.057104 | 0.397088 | 1.50894 | 0.0496723 | 0 |
| 003-table-100ms-301n-null-r1 | 0 | false | 0.093488 | 0.344776 | N/A | 0.0297007 | 0 |
| 004-json-lines-100ms-301n-null-r1 | 0 | false | 0.073887 | 0.359724 | N/A | 0.0275842 | 0 |
| 005-table-100ms-301n-null-r2 | 0 | false | 0.091745 | 0.303506 | N/A | 0.0245557 | 0 |
| 006-json-lines-100ms-301n-file-r2 | 0 | true | 0.059243 | 0.305943 | 1.21324 | 0.0424913 | 0 |
| 007-table-100ms-301n-file-r2 | 0 | true | 0.061373 | 0.365822 | 1.41925 | 0.0476758 | 0 |
| 008-json-lines-100ms-301n-null-r2 | 0 | false | 0.067624 | 0.344993 | N/A | 0.0269407 | 0 |
| 009-table-100ms-301n-null-r3 | 0 | false | 0.081614 | 0.409876 | N/A | 0.0321512 | 0 |
| 010-json-lines-100ms-301n-null-r3 | 0 | false | 0.057561 | 0.45892 | N/A | 0.0303797 | 0 |
| 011-table-100ms-301n-file-r3 | 0 | true | 0.061927 | 0.379801 | 1.46753 | 0.0456285 | 0 |
| 012-json-lines-100ms-301n-file-r3 | 0 | true | 0.057642 | 0.487478 | 1.81103 | 0.0550406 | 0 |

## Sampled process memory

Each series retains its RSS method; approximate VmRSS is not interchangeable with smaps_rollup. The post-warmup window is an operational observation window, not a guaranteed steady state. Early/late medians expose sustained drift.

| Case | Method | Window | N | Median KiB | p95 KiB | p99 KiB | Max KiB |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 007-table-100ms-301n-file-r2 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 007-table-100ms-301n-file-r2 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 007-table-100ms-301n-file-r2 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 007-table-100ms-301n-file-r2 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-100ms-301n-file-r3 | N/A | whole_run | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-100ms-301n-file-r3 | N/A | post_warmup | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-100ms-301n-file-r3 | N/A | early_post_warmup | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-100ms-301n-file-r3 | N/A | late_post_warmup | 0 | N/A | N/A | N/A | N/A |

## Per-run timing distributions

Times are milliseconds; N is the number of observations. First-start lateness is excluded from interval lateness. Snapshot and failed-attempt scan paths are separate cohorts.

p95 requires N ≥ 200; p99 requires N ≥ 1,000. These reporting thresholds are not statistical confidence guarantees. N/A tails remain unavailable; runs are not pooled.

| Case | Window | Metric | N | Median ms | p95 ms | p99 ms | Max ms |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | whole_run | snapshot_scan_duration_ns | 301 | 17.8911 | 21.6085 | N/A | 155.149 |
| 001-table-100ms-301n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | whole_run | lateness_ns | 300 | 0.061286 | 0.127244 | N/A | 55.2227 |
| 001-table-100ms-301n-file-r1 | whole_run | interval_deviation_ns | 300 | 0.061286 | 0.127244 | N/A | 55.2227 |
| 001-table-100ms-301n-file-r1 | post_warmup | snapshot_scan_duration_ns | 201 | 18.0781 | 22.3498 | N/A | 111.871 |
| 001-table-100ms-301n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | post_warmup | lateness_ns | 201 | 0.061301 | 0.129409 | N/A | 12 |
| 001-table-100ms-301n-file-r1 | post_warmup | interval_deviation_ns | 201 | 0.061301 | 0.129409 | N/A | 12 |
| 002-json-lines-100ms-301n-file-r1 | whole_run | snapshot_scan_duration_ns | 301 | 17.7993 | 21.1892 | N/A | 306.777 |
| 002-json-lines-100ms-301n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | whole_run | lateness_ns | 300 | 0.060651 | 0.112738 | N/A | 206.854 |
| 002-json-lines-100ms-301n-file-r1 | whole_run | interval_deviation_ns | 300 | 0.060651 | 0.112738 | N/A | 206.854 |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | snapshot_scan_duration_ns | 203 | 17.8964 | 21.1928 | N/A | 206.708 |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | lateness_ns | 203 | 0.060951 | 0.121695 | N/A | 206.854 |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | interval_deviation_ns | 203 | 0.060951 | 0.121695 | N/A | 206.854 |
| 003-table-100ms-301n-null-r1 | whole_run | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | whole_run | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | whole_run | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | post_warmup | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | post_warmup | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-100ms-301n-null-r1 | post_warmup | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | whole_run | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | whole_run | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | whole_run | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | post_warmup | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | post_warmup | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-100ms-301n-null-r1 | post_warmup | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | whole_run | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | whole_run | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | whole_run | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | post_warmup | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | post_warmup | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-301n-null-r2 | post_warmup | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | whole_run | snapshot_scan_duration_ns | 301 | 17.6 | 18.6545 | N/A | 208.222 |
| 006-json-lines-100ms-301n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | whole_run | lateness_ns | 300 | 0.060677 | 0.077463 | N/A | 108.257 |
| 006-json-lines-100ms-301n-file-r2 | whole_run | interval_deviation_ns | 300 | 0.060677 | 0.077463 | N/A | 108.257 |
| 006-json-lines-100ms-301n-file-r2 | post_warmup | snapshot_scan_duration_ns | 202 | 17.6106 | 18.815 | N/A | 117.908 |
| 006-json-lines-100ms-301n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-301n-file-r2 | post_warmup | lateness_ns | 202 | 0.061594 | 0.078175 | N/A | 17.9483 |
| 006-json-lines-100ms-301n-file-r2 | post_warmup | interval_deviation_ns | 202 | 0.061594 | 0.078175 | N/A | 17.9483 |
| 007-table-100ms-301n-file-r2 | whole_run | snapshot_scan_duration_ns | 301 | 17.6349 | 20.3448 | N/A | 314.461 |
| 007-table-100ms-301n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-table-100ms-301n-file-r2 | whole_run | lateness_ns | 300 | 0.060262 | 0.078806 | N/A | 214.528 |
| 007-table-100ms-301n-file-r2 | whole_run | interval_deviation_ns | 300 | 0.060262 | 0.078806 | N/A | 214.528 |
| 007-table-100ms-301n-file-r2 | post_warmup | snapshot_scan_duration_ns | 201 | 17.594 | 21.473 | N/A | 314.461 |
| 007-table-100ms-301n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-table-100ms-301n-file-r2 | post_warmup | lateness_ns | 201 | 0.060623 | 0.097992 | N/A | 214.528 |
| 007-table-100ms-301n-file-r2 | post_warmup | interval_deviation_ns | 201 | 0.060623 | 0.097992 | N/A | 214.528 |
| 008-json-lines-100ms-301n-null-r2 | whole_run | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | whole_run | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | whole_run | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | post_warmup | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | post_warmup | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-100ms-301n-null-r2 | post_warmup | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | whole_run | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | whole_run | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | whole_run | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | post_warmup | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | post_warmup | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-100ms-301n-null-r3 | post_warmup | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | whole_run | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | whole_run | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | whole_run | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | post_warmup | snapshot_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | post_warmup | lateness_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-301n-null-r3 | post_warmup | interval_deviation_ns | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | whole_run | snapshot_scan_duration_ns | 301 | 17.7258 | 21.296 | N/A | 336.726 |
| 011-table-100ms-301n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | whole_run | lateness_ns | 300 | 0.060323 | 0.079008 | N/A | 236.791 |
| 011-table-100ms-301n-file-r3 | whole_run | interval_deviation_ns | 300 | 0.060323 | 0.079008 | N/A | 236.791 |
| 011-table-100ms-301n-file-r3 | post_warmup | snapshot_scan_duration_ns | 203 | 17.6394 | 19.73 | N/A | 114.197 |
| 011-table-100ms-301n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-301n-file-r3 | post_warmup | lateness_ns | 203 | 0.060343 | 0.076914 | N/A | 14.2629 |
| 011-table-100ms-301n-file-r3 | post_warmup | interval_deviation_ns | 203 | 0.060343 | 0.076914 | N/A | 14.2629 |
| 012-json-lines-100ms-301n-file-r3 | whole_run | snapshot_scan_duration_ns | 301 | 18.5373 | 22.1895 | N/A | 412.057 |
| 012-json-lines-100ms-301n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-100ms-301n-file-r3 | whole_run | lateness_ns | 300 | 0.060253 | 0.120565 | N/A | 312.088 |
| 012-json-lines-100ms-301n-file-r3 | whole_run | interval_deviation_ns | 300 | 0.060253 | 0.120565 | N/A | 312.088 |
| 012-json-lines-100ms-301n-file-r3 | post_warmup | snapshot_scan_duration_ns | 202 | 18.4643 | 21.8796 | N/A | 412.057 |
| 012-json-lines-100ms-301n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-100ms-301n-file-r3 | post_warmup | lateness_ns | 202 | 0.059962 | 0.110046 | N/A | 312.088 |
| 012-json-lines-100ms-301n-file-r3 | post_warmup | interval_deviation_ns | 202 | 0.059962 | 0.110046 | N/A | 312.088 |

## Repetition comparisons

Each row summarizes per-process values within one condition and RSS method, including only runs explicitly marked comparable. Incomplete, failed-attempt, or changed-inventory runs remain visible above but are excluded here. Null-output controls qualify for resource comparisons only; output-derived metrics remain unavailable. N varies when metrics are unavailable. Reported ranges expose run-to-run variability; no p95/p99 is inferred from three repetitions and unlike conditions are not pooled.

| Interval ms | Format | Duration s | Sink | RSS observer | RSS method | Metric | N | Median | Min | Max |
| ---: | --- | ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| 100 | json-lines | 30 | file | false | N/A | cpu_percent | 3 | 1.48876 | 1.21011 | 1.76607 |
| 100 | json-lines | 30 | file | false | N/A | peak_rss_kib | 3 | 24056 | 23480 | 27896 |
| 100 | json-lines | 30 | file | false | N/A | post_warmup_scan_median_ms | 3 | 17.8964 | 17.6106 | 18.4643 |
| 100 | json-lines | 30 | null | false | N/A | cpu_percent | 3 | 1.42941 | 1.3568 | 1.68655 |
| 100 | json-lines | 30 | null | false | N/A | peak_rss_kib | 3 | 26360 | 24056 | 26552 |
| 100 | json-lines | 30 | null | false | N/A | post_warmup_scan_median_ms | 0 | N/A | N/A | N/A |
| 100 | table | 30 | file | false | N/A | cpu_percent | 3 | 1.45843 | 1.38747 | 1.66666 |
| 100 | table | 30 | file | false | N/A | peak_rss_kib | 3 | 24632 | 22572 | 26552 |
| 100 | table | 30 | file | false | N/A | post_warmup_scan_median_ms | 3 | 17.6394 | 17.594 | 18.0781 |
| 100 | table | 30 | null | false | N/A | cpu_percent | 3 | 1.42629 | 1.30844 | 1.61638 |
| 100 | table | 30 | null | false | N/A | peak_rss_kib | 3 | 24056 | 24056 | 26360 |
| 100 | table | 30 | null | false | N/A | post_warmup_scan_median_ms | 0 | N/A | N/A | N/A |

## Integrity and limitations

Requested counts and exit status never substitute for independently validated emitted records. A valid prefix does not establish all attempted scans after interruption. CPU per attempt is meaningful only for complete validated output and successful process completion.

CPU includes startup, discovery, formatting, and output writes, excluding the Python parent where child accounting permits. Kernel peak RSS may include launch/exec history; sampled RSS can miss transient peaks. The result manifest identifies RSS method and observer cadence.

Scan duration covers sequential userspace observation, not physical sensor freshness. File/null comparisons cannot exactly isolate acquisition CPU or terminal rendering. Thermal/frequency drift, background workload, page caches, provider availability, and correlated samples limit generalization.

| Case | Comparable | Inventory unchanged | RSS read failures | Output complete |
| --- | --- | --- | ---: | --- |
| 001-table-100ms-301n-file-r1 | true | true | 0 | true |
| 002-json-lines-100ms-301n-file-r1 | true | true | 0 | true |
| 003-table-100ms-301n-null-r1 | true | true | 0 | false |
| 004-json-lines-100ms-301n-null-r1 | true | true | 0 | false |
| 005-table-100ms-301n-null-r2 | true | true | 0 | false |
| 006-json-lines-100ms-301n-file-r2 | true | true | 0 | true |
| 007-table-100ms-301n-file-r2 | true | true | 0 | true |
| 008-json-lines-100ms-301n-null-r2 | true | true | 0 | false |
| 009-table-100ms-301n-null-r3 | true | true | 0 | false |
| 010-json-lines-100ms-301n-null-r3 | true | true | 0 | false |
| 011-table-100ms-301n-file-r3 | true | true | 0 | true |
| 012-json-lines-100ms-301n-file-r3 | true | true | 0 | true |
