# SiliconWatch foreground monitoring overhead measurements

These are measurements of the recorded release binary and conditions, not performance guarantees.

Suite status: **complete**. Retained runs: 24; not started/unfinished: 0. In-progress case: N/A.

## Provenance

```json
{
  "background_workload": "uncontrolled; load averages are observations, not process attribution",
  "binary_sha256": "130ea26f6cc27aae1378c63b373e2f88c22d3faadce87d547fbec836949d98d8",
  "captured_at_utc": "2026-11-16T21:24:56.627281+00:00",
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
    1.78564453125,
    1.9228515625,
    1.80029296875
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
  "output_free_bytes": 1745932222464,
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
| 001-table-100ms-301n-file-r1 | 100 | table | 301 | 301 | 0 | completed | 30.7747 | 0.424982 | 1.38095 | 23644 | 1.04726e+06 |
| 002-json-lines-100ms-301n-file-r1 | 100 | json-lines | 301 | 301 | 0 | completed | 30.7934 | 0.405712 | 1.31753 | 24220 | 2.85183e+06 |
| 003-table-1000ms-31n-file-r1 | 1000 | table | 31 | 31 | 0 | completed | 30.0215 | 0.043109 | 0.143594 | 24796 | 107815 |
| 004-json-lines-1000ms-31n-file-r1 | 1000 | json-lines | 31 | 31 | 0 | completed | 30.0206 | 0.044497 | 0.148222 | 24988 | 293675 |
| 005-table-100ms-3001n-file-r1 | 100 | table | 3001 | 3001 | 0 | completed | 305.787 | 4.41737 | 1.44459 | 24988 | 1.04594e+07 |
| 006-json-lines-100ms-3001n-file-r1 | 100 | json-lines | 3001 | 3001 | 0 | completed | 305.811 | 4.65975 | 1.52373 | 37276 | 2.84432e+07 |
| 007-table-1000ms-301n-file-r1 | 1000 | table | 301 | 301 | 0 | completed | 300.042 | 0.431579 | 0.14384 | 45648 | 1.04875e+06 |
| 008-json-lines-1000ms-301n-file-r1 | 1000 | json-lines | 301 | 301 | 0 | completed | 300.039 | 0.409022 | 0.136323 | 45648 | 2.85331e+06 |
| 009-table-1000ms-301n-file-r2 | 1000 | table | 301 | 301 | 0 | completed | 300.039 | 0.425992 | 0.141979 | 45648 | 1.04873e+06 |
| 010-json-lines-100ms-3001n-file-r2 | 100 | json-lines | 3001 | 3001 | 0 | completed | 302.076 | 4.33167 | 1.43397 | 47064 | 2.84461e+07 |
| 011-table-100ms-3001n-file-r2 | 100 | table | 3001 | 3001 | 0 | completed | 304.12 | 5.05358 | 1.66171 | 56084 | 1.04595e+07 |
| 012-json-lines-1000ms-31n-file-r2 | 1000 | json-lines | 31 | 31 | 0 | completed | 30.0208 | 0.045524 | 0.151642 | 61496 | 293669 |
| 013-table-1000ms-31n-file-r2 | 1000 | table | 31 | 31 | 0 | completed | 30.0203 | 0.042968 | 0.14313 | 63244 | 107809 |
| 014-json-lines-100ms-301n-file-r2 | 100 | json-lines | 301 | 301 | 0 | completed | 30.3311 | 0.423887 | 1.39753 | 63648 | 2.85166e+06 |
| 015-table-100ms-301n-file-r2 | 100 | table | 301 | 301 | 0 | completed | 30.6291 | 0.528041 | 1.72399 | 64152 | 1.04727e+06 |
| 016-json-lines-1000ms-301n-file-r2 | 1000 | json-lines | 301 | 301 | 0 | completed | 300.046 | 0.463346 | 0.154425 | 64808 | 2.85314e+06 |
| 017-table-1000ms-31n-file-r3 | 1000 | table | 31 | 31 | 0 | completed | 30.0208 | 0.055835 | 0.185988 | 65608 | 107817 |
| 018-json-lines-1000ms-31n-file-r3 | 1000 | json-lines | 31 | 31 | 0 | completed | 30.0224 | 0.047342 | 0.157689 | 65700 | 293651 |
| 019-table-100ms-3001n-file-r3 | 100 | table | 3001 | 3001 | 0 | completed | 305.886 | 4.0165 | 1.31307 | 65860 | 1.04593e+07 |
| 020-json-lines-100ms-3001n-file-r3 | 100 | json-lines | 3001 | 3001 | 0 | completed | 303.029 | 3.82582 | 1.26252 | 73308 | 2.84509e+07 |
| 021-table-1000ms-301n-file-r3 | 1000 | table | 301 | 301 | 0 | completed | 300.039 | 0.419284 | 0.139743 | 80652 | 1.04873e+06 |
| 022-json-lines-1000ms-301n-file-r3 | 1000 | json-lines | 301 | 301 | 0 | completed | 300.042 | 0.42189 | 0.140611 | 81244 | 2.85322e+06 |
| 023-table-100ms-301n-file-r3 | 100 | table | 301 | 301 | 0 | completed | 30.3939 | 0.449231 | 1.47803 | 82092 | 1.04725e+06 |
| 024-json-lines-100ms-301n-file-r3 | 100 | json-lines | 301 | 301 | 0 | completed | 30.6922 | 0.353386 | 1.15139 | 82140 | 2.85164e+06 |

## CPU attribution and completion

CPU per attempt requires validated complete output, exit 0, and normal termination. Failed attempts remain in that denominator. Resource-only null-output runs cannot establish completion counts.

| Case | Exit | Output complete | User CPU s | System CPU s | CPU ms/attempt | Runner CPU s | Stderr bytes |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | 0 | true | 0.08528 | 0.339702 | 1.4119 | 0.0759189 | 0 |
| 002-json-lines-100ms-301n-file-r1 | 0 | true | 0.08174 | 0.323972 | 1.34788 | 0.0726135 | 0 |
| 003-table-1000ms-31n-file-r1 | 0 | true | 0.012054 | 0.031055 | 1.39061 | 0.0474574 | 0 |
| 004-json-lines-1000ms-31n-file-r1 | 0 | true | 0.015028 | 0.029469 | 1.43539 | 0.0543498 | 0 |
| 005-table-100ms-3001n-file-r1 | 0 | true | 0.857066 | 3.5603 | 1.47197 | 0.703738 | 0 |
| 006-json-lines-100ms-3001n-file-r1 | 0 | true | 0.72155 | 3.9382 | 1.55273 | 0.678994 | 0 |
| 007-table-1000ms-301n-file-r1 | 0 | true | 0.090251 | 0.341328 | 1.43382 | 0.489731 | 0 |
| 008-json-lines-1000ms-301n-file-r1 | 0 | true | 0.069814 | 0.339208 | 1.35888 | 0.450559 | 0 |
| 009-table-1000ms-301n-file-r2 | 0 | true | 0.094277 | 0.331715 | 1.41526 | 0.483174 | 0 |
| 010-json-lines-100ms-3001n-file-r2 | 0 | true | 0.692733 | 3.63893 | 1.44341 | 0.654625 | 0 |
| 011-table-100ms-3001n-file-r2 | 0 | true | 0.793049 | 4.26053 | 1.68397 | 0.705598 | 0 |
| 012-json-lines-1000ms-31n-file-r2 | 0 | true | 0.010512 | 0.035012 | 1.46852 | 0.0378479 | 0 |
| 013-table-1000ms-31n-file-r2 | 0 | true | 0.009359 | 0.033609 | 1.38606 | 0.037452 | 0 |
| 014-json-lines-100ms-301n-file-r2 | 0 | true | 0.066232 | 0.357655 | 1.40826 | 0.0598285 | 0 |
| 015-table-100ms-301n-file-r2 | 0 | true | 0.08501 | 0.443031 | 1.75429 | 0.0679147 | 0 |
| 016-json-lines-1000ms-301n-file-r2 | 0 | true | 0.070215 | 0.393131 | 1.53936 | 0.410284 | 0 |
| 017-table-1000ms-31n-file-r3 | 0 | true | 0.011403 | 0.044432 | 1.80113 | 0.0403979 | 0 |
| 018-json-lines-1000ms-31n-file-r3 | 0 | true | 0.008733 | 0.038609 | 1.52716 | 0.0423335 | 0 |
| 019-table-100ms-3001n-file-r3 | 0 | true | 0.801727 | 3.21477 | 1.33839 | 0.671492 | 0 |
| 020-json-lines-100ms-3001n-file-r3 | 0 | true | 0.629982 | 3.19583 | 1.27485 | 0.663108 | 0 |
| 021-table-1000ms-301n-file-r3 | 0 | true | 0.088394 | 0.33089 | 1.39297 | 0.448198 | 0 |
| 022-json-lines-1000ms-301n-file-r3 | 0 | true | 0.086263 | 0.335627 | 1.40163 | 0.417227 | 0 |
| 023-table-100ms-301n-file-r3 | 0 | true | 0.067328 | 0.381903 | 1.49246 | 0.0598529 | 0 |
| 024-json-lines-100ms-301n-file-r3 | 0 | true | 0.05147 | 0.301916 | 1.17404 | 0.0528222 | 0 |

## Sampled process memory

Each series retains its RSS method; approximate VmRSS is not interchangeable with smaps_rollup. The post-warmup window is an operational observation window, not a guaranteed steady state. Early/late medians expose sustained drift.

| Case | Method | Window | N | Median KiB | p95 KiB | p99 KiB | Max KiB |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | smaps_rollup | whole_run | 30 | 2940 | N/A | N/A | 2944 |
| 001-table-100ms-301n-file-r1 | smaps_rollup | post_warmup | 21 | 2940 | N/A | N/A | 2944 |
| 001-table-100ms-301n-file-r1 | smaps_rollup | early_post_warmup | 11 | 2940 | N/A | N/A | 2940 |
| 001-table-100ms-301n-file-r1 | smaps_rollup | late_post_warmup | 10 | 2944 | N/A | N/A | 2944 |
| 002-json-lines-100ms-301n-file-r1 | smaps_rollup | whole_run | 30 | 2972 | N/A | N/A | 2984 |
| 002-json-lines-100ms-301n-file-r1 | smaps_rollup | post_warmup | 21 | 2972 | N/A | N/A | 2984 |
| 002-json-lines-100ms-301n-file-r1 | smaps_rollup | early_post_warmup | 11 | 2972 | N/A | N/A | 2972 |
| 002-json-lines-100ms-301n-file-r1 | smaps_rollup | late_post_warmup | 10 | 2984 | N/A | N/A | 2984 |
| 003-table-1000ms-31n-file-r1 | smaps_rollup | whole_run | 29 | 2952 | N/A | N/A | 2952 |
| 003-table-1000ms-31n-file-r1 | smaps_rollup | post_warmup | 20 | 2952 | N/A | N/A | 2952 |
| 003-table-1000ms-31n-file-r1 | smaps_rollup | early_post_warmup | 10 | 2952 | N/A | N/A | 2952 |
| 003-table-1000ms-31n-file-r1 | smaps_rollup | late_post_warmup | 10 | 2952 | N/A | N/A | 2952 |
| 004-json-lines-1000ms-31n-file-r1 | smaps_rollup | whole_run | 29 | 2932 | N/A | N/A | 2932 |
| 004-json-lines-1000ms-31n-file-r1 | smaps_rollup | post_warmup | 20 | 2932 | N/A | N/A | 2932 |
| 004-json-lines-1000ms-31n-file-r1 | smaps_rollup | early_post_warmup | 10 | 2932 | N/A | N/A | 2932 |
| 004-json-lines-1000ms-31n-file-r1 | smaps_rollup | late_post_warmup | 10 | 2932 | N/A | N/A | 2932 |
| 005-table-100ms-3001n-file-r1 | smaps_rollup | whole_run | 305 | 3008 | 3012 | N/A | 3012 |
| 005-table-100ms-3001n-file-r1 | smaps_rollup | post_warmup | 296 | 3008 | 3012 | N/A | 3012 |
| 005-table-100ms-3001n-file-r1 | smaps_rollup | early_post_warmup | 148 | 2992 | N/A | N/A | 3008 |
| 005-table-100ms-3001n-file-r1 | smaps_rollup | late_post_warmup | 148 | 3012 | N/A | N/A | 3012 |
| 006-json-lines-100ms-3001n-file-r1 | smaps_rollup | whole_run | 305 | 2956 | 2956 | N/A | 2956 |
| 006-json-lines-100ms-3001n-file-r1 | smaps_rollup | post_warmup | 296 | 2956 | 2956 | N/A | 2956 |
| 006-json-lines-100ms-3001n-file-r1 | smaps_rollup | early_post_warmup | 148 | 2956 | N/A | N/A | 2956 |
| 006-json-lines-100ms-3001n-file-r1 | smaps_rollup | late_post_warmup | 148 | 2956 | N/A | N/A | 2956 |
| 007-table-1000ms-301n-file-r1 | smaps_rollup | whole_run | 299 | 2936 | 2940 | N/A | 2940 |
| 007-table-1000ms-301n-file-r1 | smaps_rollup | post_warmup | 290 | 2936 | 2940 | N/A | 2940 |
| 007-table-1000ms-301n-file-r1 | smaps_rollup | early_post_warmup | 145 | 2928 | N/A | N/A | 2936 |
| 007-table-1000ms-301n-file-r1 | smaps_rollup | late_post_warmup | 145 | 2940 | N/A | N/A | 2940 |
| 008-json-lines-1000ms-301n-file-r1 | smaps_rollup | whole_run | 299 | 2964 | 2976 | N/A | 2976 |
| 008-json-lines-1000ms-301n-file-r1 | smaps_rollup | post_warmup | 290 | 2964 | 2976 | N/A | 2976 |
| 008-json-lines-1000ms-301n-file-r1 | smaps_rollup | early_post_warmup | 145 | 2964 | N/A | N/A | 2964 |
| 008-json-lines-1000ms-301n-file-r1 | smaps_rollup | late_post_warmup | 145 | 2976 | N/A | N/A | 2976 |
| 009-table-1000ms-301n-file-r2 | smaps_rollup | whole_run | 299 | 2944 | 2948 | N/A | 2948 |
| 009-table-1000ms-301n-file-r2 | smaps_rollup | post_warmup | 290 | 2944 | 2948 | N/A | 2948 |
| 009-table-1000ms-301n-file-r2 | smaps_rollup | early_post_warmup | 145 | 2936 | N/A | N/A | 2944 |
| 009-table-1000ms-301n-file-r2 | smaps_rollup | late_post_warmup | 145 | 2948 | N/A | N/A | 2948 |
| 010-json-lines-100ms-3001n-file-r2 | smaps_rollup | whole_run | 301 | 2948 | 2948 | N/A | 2948 |
| 010-json-lines-100ms-3001n-file-r2 | smaps_rollup | post_warmup | 292 | 2948 | 2948 | N/A | 2948 |
| 010-json-lines-100ms-3001n-file-r2 | smaps_rollup | early_post_warmup | 146 | 2948 | N/A | N/A | 2948 |
| 010-json-lines-100ms-3001n-file-r2 | smaps_rollup | late_post_warmup | 146 | 2948 | N/A | N/A | 2948 |
| 011-table-100ms-3001n-file-r2 | smaps_rollup | whole_run | 303 | 2968 | 2972 | N/A | 2972 |
| 011-table-100ms-3001n-file-r2 | smaps_rollup | post_warmup | 294 | 2968 | 2972 | N/A | 2972 |
| 011-table-100ms-3001n-file-r2 | smaps_rollup | early_post_warmup | 147 | 2952 | N/A | N/A | 2968 |
| 011-table-100ms-3001n-file-r2 | smaps_rollup | late_post_warmup | 147 | 2972 | N/A | N/A | 2972 |
| 012-json-lines-1000ms-31n-file-r2 | smaps_rollup | whole_run | 29 | 2900 | N/A | N/A | 2900 |
| 012-json-lines-1000ms-31n-file-r2 | smaps_rollup | post_warmup | 20 | 2900 | N/A | N/A | 2900 |
| 012-json-lines-1000ms-31n-file-r2 | smaps_rollup | early_post_warmup | 10 | 2900 | N/A | N/A | 2900 |
| 012-json-lines-1000ms-31n-file-r2 | smaps_rollup | late_post_warmup | 10 | 2900 | N/A | N/A | 2900 |
| 013-table-1000ms-31n-file-r2 | smaps_rollup | whole_run | 29 | 2916 | N/A | N/A | 2916 |
| 013-table-1000ms-31n-file-r2 | smaps_rollup | post_warmup | 20 | 2916 | N/A | N/A | 2916 |
| 013-table-1000ms-31n-file-r2 | smaps_rollup | early_post_warmup | 10 | 2916 | N/A | N/A | 2916 |
| 013-table-1000ms-31n-file-r2 | smaps_rollup | late_post_warmup | 10 | 2916 | N/A | N/A | 2916 |
| 014-json-lines-100ms-301n-file-r2 | smaps_rollup | whole_run | 30 | 2944 | N/A | N/A | 2956 |
| 014-json-lines-100ms-301n-file-r2 | smaps_rollup | post_warmup | 21 | 2944 | N/A | N/A | 2956 |
| 014-json-lines-100ms-301n-file-r2 | smaps_rollup | early_post_warmup | 11 | 2944 | N/A | N/A | 2944 |
| 014-json-lines-100ms-301n-file-r2 | smaps_rollup | late_post_warmup | 10 | 2956 | N/A | N/A | 2956 |
| 015-table-100ms-301n-file-r2 | smaps_rollup | whole_run | 30 | 2944 | N/A | N/A | 2948 |
| 015-table-100ms-301n-file-r2 | smaps_rollup | post_warmup | 21 | 2944 | N/A | N/A | 2948 |
| 015-table-100ms-301n-file-r2 | smaps_rollup | early_post_warmup | 11 | 2944 | N/A | N/A | 2944 |
| 015-table-100ms-301n-file-r2 | smaps_rollup | late_post_warmup | 10 | 2948 | N/A | N/A | 2948 |
| 016-json-lines-1000ms-301n-file-r2 | smaps_rollup | whole_run | 299 | 2928 | 2940 | N/A | 2940 |
| 016-json-lines-1000ms-301n-file-r2 | smaps_rollup | post_warmup | 290 | 2928 | 2940 | N/A | 2940 |
| 016-json-lines-1000ms-301n-file-r2 | smaps_rollup | early_post_warmup | 145 | 2928 | N/A | N/A | 2928 |
| 016-json-lines-1000ms-301n-file-r2 | smaps_rollup | late_post_warmup | 145 | 2940 | N/A | N/A | 2940 |
| 017-table-1000ms-31n-file-r3 | smaps_rollup | whole_run | 29 | 2912 | N/A | N/A | 2912 |
| 017-table-1000ms-31n-file-r3 | smaps_rollup | post_warmup | 20 | 2912 | N/A | N/A | 2912 |
| 017-table-1000ms-31n-file-r3 | smaps_rollup | early_post_warmup | 10 | 2912 | N/A | N/A | 2912 |
| 017-table-1000ms-31n-file-r3 | smaps_rollup | late_post_warmup | 10 | 2912 | N/A | N/A | 2912 |
| 018-json-lines-1000ms-31n-file-r3 | smaps_rollup | whole_run | 29 | 2948 | N/A | N/A | 2948 |
| 018-json-lines-1000ms-31n-file-r3 | smaps_rollup | post_warmup | 20 | 2948 | N/A | N/A | 2948 |
| 018-json-lines-1000ms-31n-file-r3 | smaps_rollup | early_post_warmup | 10 | 2948 | N/A | N/A | 2948 |
| 018-json-lines-1000ms-31n-file-r3 | smaps_rollup | late_post_warmup | 10 | 2948 | N/A | N/A | 2948 |
| 019-table-100ms-3001n-file-r3 | smaps_rollup | whole_run | 305 | 2940 | 2944 | N/A | 2944 |
| 019-table-100ms-3001n-file-r3 | smaps_rollup | post_warmup | 296 | 2940 | 2944 | N/A | 2944 |
| 019-table-100ms-3001n-file-r3 | smaps_rollup | early_post_warmup | 148 | 2924 | N/A | N/A | 2940 |
| 019-table-100ms-3001n-file-r3 | smaps_rollup | late_post_warmup | 148 | 2944 | N/A | N/A | 2944 |
| 020-json-lines-100ms-3001n-file-r3 | smaps_rollup | whole_run | 302 | 3000 | 3000 | N/A | 3000 |
| 020-json-lines-100ms-3001n-file-r3 | smaps_rollup | post_warmup | 293 | 3000 | 3000 | N/A | 3000 |
| 020-json-lines-100ms-3001n-file-r3 | smaps_rollup | early_post_warmup | 147 | 3000 | N/A | N/A | 3000 |
| 020-json-lines-100ms-3001n-file-r3 | smaps_rollup | late_post_warmup | 146 | 3000 | N/A | N/A | 3000 |
| 021-table-1000ms-301n-file-r3 | smaps_rollup | whole_run | 299 | 2952 | 2956 | N/A | 2956 |
| 021-table-1000ms-301n-file-r3 | smaps_rollup | post_warmup | 290 | 2952 | 2956 | N/A | 2956 |
| 021-table-1000ms-301n-file-r3 | smaps_rollup | early_post_warmup | 145 | 2944 | N/A | N/A | 2952 |
| 021-table-1000ms-301n-file-r3 | smaps_rollup | late_post_warmup | 145 | 2956 | N/A | N/A | 2956 |
| 022-json-lines-1000ms-301n-file-r3 | smaps_rollup | whole_run | 299 | 2952 | 2964 | N/A | 2964 |
| 022-json-lines-1000ms-301n-file-r3 | smaps_rollup | post_warmup | 290 | 2952 | 2964 | N/A | 2964 |
| 022-json-lines-1000ms-301n-file-r3 | smaps_rollup | early_post_warmup | 145 | 2952 | N/A | N/A | 2952 |
| 022-json-lines-1000ms-301n-file-r3 | smaps_rollup | late_post_warmup | 145 | 2964 | N/A | N/A | 2964 |
| 023-table-100ms-301n-file-r3 | smaps_rollup | whole_run | 30 | 2952 | N/A | N/A | 2956 |
| 023-table-100ms-301n-file-r3 | smaps_rollup | post_warmup | 21 | 2952 | N/A | N/A | 2956 |
| 023-table-100ms-301n-file-r3 | smaps_rollup | early_post_warmup | 11 | 2952 | N/A | N/A | 2952 |
| 023-table-100ms-301n-file-r3 | smaps_rollup | late_post_warmup | 10 | 2956 | N/A | N/A | 2956 |
| 024-json-lines-100ms-301n-file-r3 | smaps_rollup | whole_run | 30 | 2980 | N/A | N/A | 2992 |
| 024-json-lines-100ms-301n-file-r3 | smaps_rollup | post_warmup | 21 | 2980 | N/A | N/A | 2992 |
| 024-json-lines-100ms-301n-file-r3 | smaps_rollup | early_post_warmup | 11 | 2980 | N/A | N/A | 2980 |
| 024-json-lines-100ms-301n-file-r3 | smaps_rollup | late_post_warmup | 10 | 2992 | N/A | N/A | 2992 |

## Per-run timing distributions

Times are milliseconds; N is the number of observations. First-start lateness is excluded from interval lateness. Snapshot and failed-attempt scan paths are separate cohorts.

p95 requires N ≥ 200; p99 requires N ≥ 1,000. These reporting thresholds are not statistical confidence guarantees. N/A tails remain unavailable; runs are not pooled.

| Case | Window | Metric | N | Median ms | p95 ms | p99 ms | Max ms |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 001-table-100ms-301n-file-r1 | whole_run | snapshot_scan_duration_ns | 301 | 17.7519 | 40.2687 | N/A | 412.378 |
| 001-table-100ms-301n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | whole_run | lateness_ns | 300 | 0.062887 | 0.080975 | N/A | 312.482 |
| 001-table-100ms-301n-file-r1 | whole_run | interval_deviation_ns | 300 | 0.062887 | 0.080975 | N/A | 312.482 |
| 001-table-100ms-301n-file-r1 | post_warmup | snapshot_scan_duration_ns | 204 | 17.7712 | 40.2526 | N/A | 368.478 |
| 001-table-100ms-301n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 001-table-100ms-301n-file-r1 | post_warmup | lateness_ns | 204 | 0.062516 | 0.079332 | N/A | 268.567 |
| 001-table-100ms-301n-file-r1 | post_warmup | interval_deviation_ns | 204 | 0.062516 | 0.079332 | N/A | 268.567 |
| 002-json-lines-100ms-301n-file-r1 | whole_run | snapshot_scan_duration_ns | 301 | 17.8247 | 20.5069 | N/A | 336.488 |
| 002-json-lines-100ms-301n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | whole_run | lateness_ns | 300 | 0.0624765 | 0.082639 | N/A | 236.548 |
| 002-json-lines-100ms-301n-file-r1 | whole_run | interval_deviation_ns | 300 | 0.0624765 | 0.082639 | N/A | 236.548 |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | snapshot_scan_duration_ns | 202 | 17.9377 | 20.5069 | N/A | 336.488 |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | lateness_ns | 202 | 0.062266 | 0.082668 | N/A | 236.548 |
| 002-json-lines-100ms-301n-file-r1 | post_warmup | interval_deviation_ns | 202 | 0.062266 | 0.082668 | N/A | 236.548 |
| 003-table-1000ms-31n-file-r1 | whole_run | snapshot_scan_duration_ns | 31 | 17.7016 | N/A | N/A | 251.043 |
| 003-table-1000ms-31n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-1000ms-31n-file-r1 | whole_run | lateness_ns | 30 | 0.0621775 | N/A | N/A | 0.084709 |
| 003-table-1000ms-31n-file-r1 | whole_run | interval_deviation_ns | 30 | 0.0621775 | N/A | N/A | 0.084709 |
| 003-table-1000ms-31n-file-r1 | post_warmup | snapshot_scan_duration_ns | 21 | 17.5617 | N/A | N/A | 251.043 |
| 003-table-1000ms-31n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 003-table-1000ms-31n-file-r1 | post_warmup | lateness_ns | 21 | 0.062197 | N/A | N/A | 0.084709 |
| 003-table-1000ms-31n-file-r1 | post_warmup | interval_deviation_ns | 21 | 0.062197 | N/A | N/A | 0.084709 |
| 004-json-lines-1000ms-31n-file-r1 | whole_run | snapshot_scan_duration_ns | 31 | 17.5077 | N/A | N/A | 195.604 |
| 004-json-lines-1000ms-31n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-1000ms-31n-file-r1 | whole_run | lateness_ns | 30 | 0.0636975 | N/A | N/A | 0.082377 |
| 004-json-lines-1000ms-31n-file-r1 | whole_run | interval_deviation_ns | 30 | 0.0636975 | N/A | N/A | 0.082377 |
| 004-json-lines-1000ms-31n-file-r1 | post_warmup | snapshot_scan_duration_ns | 21 | 17.4146 | N/A | N/A | 195.604 |
| 004-json-lines-1000ms-31n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 004-json-lines-1000ms-31n-file-r1 | post_warmup | lateness_ns | 21 | 0.062731 | N/A | N/A | 0.082377 |
| 004-json-lines-1000ms-31n-file-r1 | post_warmup | interval_deviation_ns | 21 | 0.062731 | N/A | N/A | 0.082377 |
| 005-table-100ms-3001n-file-r1 | whole_run | snapshot_scan_duration_ns | 3001 | 17.4357 | 21.5103 | 128.5 | 465.028 |
| 005-table-100ms-3001n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-3001n-file-r1 | whole_run | lateness_ns | 3000 | 0.061972 | 0.091958 | 28.5668 | 365.093 |
| 005-table-100ms-3001n-file-r1 | whole_run | interval_deviation_ns | 3000 | 0.061972 | 0.091958 | 28.5668 | 365.093 |
| 005-table-100ms-3001n-file-r1 | post_warmup | snapshot_scan_duration_ns | 2903 | 17.4173 | 21.244 | 128.5 | 465.028 |
| 005-table-100ms-3001n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 005-table-100ms-3001n-file-r1 | post_warmup | lateness_ns | 2903 | 0.061971 | 0.089883 | 28.5668 | 365.093 |
| 005-table-100ms-3001n-file-r1 | post_warmup | interval_deviation_ns | 2903 | 0.061971 | 0.089883 | 28.5668 | 365.093 |
| 006-json-lines-100ms-3001n-file-r1 | whole_run | snapshot_scan_duration_ns | 3001 | 18.11 | 22.1374 | 132.419 | 438.808 |
| 006-json-lines-100ms-3001n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-3001n-file-r1 | whole_run | lateness_ns | 3000 | 0.0614265 | 0.383522 | 32.4503 | 338.862 |
| 006-json-lines-100ms-3001n-file-r1 | whole_run | interval_deviation_ns | 3000 | 0.0614265 | 0.383522 | 32.4503 | 338.862 |
| 006-json-lines-100ms-3001n-file-r1 | post_warmup | snapshot_scan_duration_ns | 2907 | 18.1515 | 22.1332 | 118.626 | 438.808 |
| 006-json-lines-100ms-3001n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 006-json-lines-100ms-3001n-file-r1 | post_warmup | lateness_ns | 2907 | 0.061431 | 0.383422 | 18.6555 | 338.862 |
| 006-json-lines-100ms-3001n-file-r1 | post_warmup | interval_deviation_ns | 2907 | 0.061431 | 0.383422 | 18.6555 | 338.862 |
| 007-table-1000ms-301n-file-r1 | whole_run | snapshot_scan_duration_ns | 301 | 17.7491 | 20.6168 | N/A | 294.846 |
| 007-table-1000ms-301n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-table-1000ms-301n-file-r1 | whole_run | lateness_ns | 300 | 0.063006 | 0.082839 | N/A | 0.938883 |
| 007-table-1000ms-301n-file-r1 | whole_run | interval_deviation_ns | 300 | 0.063006 | 0.082839 | N/A | 0.938883 |
| 007-table-1000ms-301n-file-r1 | post_warmup | snapshot_scan_duration_ns | 291 | 17.7491 | 20.6168 | N/A | 294.846 |
| 007-table-1000ms-301n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 007-table-1000ms-301n-file-r1 | post_warmup | lateness_ns | 291 | 0.063 | 0.08362 | N/A | 0.938883 |
| 007-table-1000ms-301n-file-r1 | post_warmup | interval_deviation_ns | 291 | 0.063 | 0.08362 | N/A | 0.938883 |
| 008-json-lines-1000ms-301n-file-r1 | whole_run | snapshot_scan_duration_ns | 301 | 17.5899 | 19.4077 | N/A | 219.651 |
| 008-json-lines-1000ms-301n-file-r1 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-1000ms-301n-file-r1 | whole_run | lateness_ns | 300 | 0.06341 | 0.078332 | N/A | 0.091347 |
| 008-json-lines-1000ms-301n-file-r1 | whole_run | interval_deviation_ns | 300 | 0.06341 | 0.078332 | N/A | 0.091347 |
| 008-json-lines-1000ms-301n-file-r1 | post_warmup | snapshot_scan_duration_ns | 291 | 17.5667 | 19.1952 | N/A | 219.651 |
| 008-json-lines-1000ms-301n-file-r1 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 008-json-lines-1000ms-301n-file-r1 | post_warmup | lateness_ns | 291 | 0.063346 | 0.078332 | N/A | 0.091347 |
| 008-json-lines-1000ms-301n-file-r1 | post_warmup | interval_deviation_ns | 291 | 0.063346 | 0.078332 | N/A | 0.091347 |
| 009-table-1000ms-301n-file-r2 | whole_run | snapshot_scan_duration_ns | 301 | 17.704 | 19.4855 | N/A | 140.455 |
| 009-table-1000ms-301n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-1000ms-301n-file-r2 | whole_run | lateness_ns | 300 | 0.0629975 | 0.080889 | N/A | 0.659329 |
| 009-table-1000ms-301n-file-r2 | whole_run | interval_deviation_ns | 300 | 0.0629975 | 0.080889 | N/A | 0.659329 |
| 009-table-1000ms-301n-file-r2 | post_warmup | snapshot_scan_duration_ns | 291 | 17.7144 | 19.4855 | N/A | 103.829 |
| 009-table-1000ms-301n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 009-table-1000ms-301n-file-r2 | post_warmup | lateness_ns | 291 | 0.063012 | 0.080889 | N/A | 0.659329 |
| 009-table-1000ms-301n-file-r2 | post_warmup | interval_deviation_ns | 291 | 0.063012 | 0.080889 | N/A | 0.659329 |
| 010-json-lines-100ms-3001n-file-r2 | whole_run | snapshot_scan_duration_ns | 3001 | 17.819 | 21.1922 | 91.5947 | 382.942 |
| 010-json-lines-100ms-3001n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-3001n-file-r2 | whole_run | lateness_ns | 3000 | 0.0612755 | 0.09415 | 0.715059 | 282.984 |
| 010-json-lines-100ms-3001n-file-r2 | whole_run | interval_deviation_ns | 3000 | 0.0612755 | 0.09415 | 0.715059 | 282.984 |
| 010-json-lines-100ms-3001n-file-r2 | post_warmup | snapshot_scan_duration_ns | 2901 | 17.8289 | 21.2163 | 91.5947 | 382.942 |
| 010-json-lines-100ms-3001n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 010-json-lines-100ms-3001n-file-r2 | post_warmup | lateness_ns | 2901 | 0.061253 | 0.094486 | 0.715059 | 282.984 |
| 010-json-lines-100ms-3001n-file-r2 | post_warmup | interval_deviation_ns | 2901 | 0.061253 | 0.094486 | 0.715059 | 282.984 |
| 011-table-100ms-3001n-file-r2 | whole_run | snapshot_scan_duration_ns | 3001 | 17.8078 | 22.0448 | 115.169 | 449.747 |
| 011-table-100ms-3001n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-3001n-file-r2 | whole_run | lateness_ns | 3000 | 0.061538 | 0.386638 | 15.2815 | 349.839 |
| 011-table-100ms-3001n-file-r2 | whole_run | interval_deviation_ns | 3000 | 0.061538 | 0.386638 | 15.2815 | 349.839 |
| 011-table-100ms-3001n-file-r2 | post_warmup | snapshot_scan_duration_ns | 2902 | 17.7915 | 22.0693 | 115.138 | 449.747 |
| 011-table-100ms-3001n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 011-table-100ms-3001n-file-r2 | post_warmup | lateness_ns | 2902 | 0.061532 | 0.386801 | 15.231 | 349.839 |
| 011-table-100ms-3001n-file-r2 | post_warmup | interval_deviation_ns | 2902 | 0.061532 | 0.386801 | 15.231 | 349.839 |
| 012-json-lines-1000ms-31n-file-r2 | whole_run | snapshot_scan_duration_ns | 31 | 17.8889 | N/A | N/A | 124.073 |
| 012-json-lines-1000ms-31n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-1000ms-31n-file-r2 | whole_run | lateness_ns | 30 | 0.0647755 | N/A | N/A | 0.077189 |
| 012-json-lines-1000ms-31n-file-r2 | whole_run | interval_deviation_ns | 30 | 0.0647755 | N/A | N/A | 0.077189 |
| 012-json-lines-1000ms-31n-file-r2 | post_warmup | snapshot_scan_duration_ns | 21 | 18.0203 | N/A | N/A | 124.073 |
| 012-json-lines-1000ms-31n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 012-json-lines-1000ms-31n-file-r2 | post_warmup | lateness_ns | 21 | 0.062829 | N/A | N/A | 0.076171 |
| 012-json-lines-1000ms-31n-file-r2 | post_warmup | interval_deviation_ns | 21 | 0.062829 | N/A | N/A | 0.076171 |
| 013-table-1000ms-31n-file-r2 | whole_run | snapshot_scan_duration_ns | 31 | 17.5911 | N/A | N/A | 19.1142 |
| 013-table-1000ms-31n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 013-table-1000ms-31n-file-r2 | whole_run | lateness_ns | 30 | 0.0641915 | N/A | N/A | 0.078386 |
| 013-table-1000ms-31n-file-r2 | whole_run | interval_deviation_ns | 30 | 0.0641915 | N/A | N/A | 0.078386 |
| 013-table-1000ms-31n-file-r2 | post_warmup | snapshot_scan_duration_ns | 21 | 17.5109 | N/A | N/A | 18.5784 |
| 013-table-1000ms-31n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 013-table-1000ms-31n-file-r2 | post_warmup | lateness_ns | 21 | 0.064093 | N/A | N/A | 0.078386 |
| 013-table-1000ms-31n-file-r2 | post_warmup | interval_deviation_ns | 21 | 0.064093 | N/A | N/A | 0.078386 |
| 014-json-lines-100ms-301n-file-r2 | whole_run | snapshot_scan_duration_ns | 301 | 17.784 | 21.0887 | N/A | 381.172 |
| 014-json-lines-100ms-301n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 014-json-lines-100ms-301n-file-r2 | whole_run | lateness_ns | 300 | 0.059971 | 0.078126 | N/A | 281.259 |
| 014-json-lines-100ms-301n-file-r2 | whole_run | interval_deviation_ns | 300 | 0.059971 | 0.078126 | N/A | 281.259 |
| 014-json-lines-100ms-301n-file-r2 | post_warmup | snapshot_scan_duration_ns | 201 | 17.5853 | 18.3866 | N/A | 381.172 |
| 014-json-lines-100ms-301n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 014-json-lines-100ms-301n-file-r2 | post_warmup | lateness_ns | 201 | 0.059791 | 0.077384 | N/A | 281.259 |
| 014-json-lines-100ms-301n-file-r2 | post_warmup | interval_deviation_ns | 201 | 0.059791 | 0.077384 | N/A | 281.259 |
| 015-table-100ms-301n-file-r2 | whole_run | snapshot_scan_duration_ns | 301 | 18.3124 | 22.2358 | N/A | 368.797 |
| 015-table-100ms-301n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 015-table-100ms-301n-file-r2 | whole_run | lateness_ns | 300 | 0.0608005 | 0.115769 | N/A | 268.891 |
| 015-table-100ms-301n-file-r2 | whole_run | interval_deviation_ns | 300 | 0.0608005 | 0.115769 | N/A | 268.891 |
| 015-table-100ms-301n-file-r2 | post_warmup | snapshot_scan_duration_ns | 203 | 18.5099 | 22.2358 | N/A | 262.249 |
| 015-table-100ms-301n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 015-table-100ms-301n-file-r2 | post_warmup | lateness_ns | 203 | 0.060826 | 0.152417 | N/A | 162.421 |
| 015-table-100ms-301n-file-r2 | post_warmup | interval_deviation_ns | 203 | 0.060826 | 0.152417 | N/A | 162.421 |
| 016-json-lines-1000ms-301n-file-r2 | whole_run | snapshot_scan_duration_ns | 301 | 17.8782 | 21.2402 | N/A | 120.065 |
| 016-json-lines-1000ms-301n-file-r2 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 016-json-lines-1000ms-301n-file-r2 | whole_run | lateness_ns | 300 | 0.062803 | 0.082343 | N/A | 0.952055 |
| 016-json-lines-1000ms-301n-file-r2 | whole_run | interval_deviation_ns | 300 | 0.062803 | 0.082343 | N/A | 0.952055 |
| 016-json-lines-1000ms-301n-file-r2 | post_warmup | snapshot_scan_duration_ns | 291 | 17.8978 | 21.2015 | N/A | 120.065 |
| 016-json-lines-1000ms-301n-file-r2 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 016-json-lines-1000ms-301n-file-r2 | post_warmup | lateness_ns | 291 | 0.062727 | 0.081449 | N/A | 0.952055 |
| 016-json-lines-1000ms-301n-file-r2 | post_warmup | interval_deviation_ns | 291 | 0.062727 | 0.081449 | N/A | 0.952055 |
| 017-table-1000ms-31n-file-r3 | whole_run | snapshot_scan_duration_ns | 31 | 17.9557 | N/A | N/A | 100.855 |
| 017-table-1000ms-31n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 017-table-1000ms-31n-file-r3 | whole_run | lateness_ns | 30 | 0.0620645 | N/A | N/A | 0.08221 |
| 017-table-1000ms-31n-file-r3 | whole_run | interval_deviation_ns | 30 | 0.0620645 | N/A | N/A | 0.08221 |
| 017-table-1000ms-31n-file-r3 | post_warmup | snapshot_scan_duration_ns | 21 | 18.1375 | N/A | N/A | 30.217 |
| 017-table-1000ms-31n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 017-table-1000ms-31n-file-r3 | post_warmup | lateness_ns | 21 | 0.061737 | N/A | N/A | 0.07067 |
| 017-table-1000ms-31n-file-r3 | post_warmup | interval_deviation_ns | 21 | 0.061737 | N/A | N/A | 0.07067 |
| 018-json-lines-1000ms-31n-file-r3 | whole_run | snapshot_scan_duration_ns | 31 | 17.9316 | N/A | N/A | 112.057 |
| 018-json-lines-1000ms-31n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 018-json-lines-1000ms-31n-file-r3 | whole_run | lateness_ns | 30 | 0.0621565 | N/A | N/A | 0.079356 |
| 018-json-lines-1000ms-31n-file-r3 | whole_run | interval_deviation_ns | 30 | 0.0621565 | N/A | N/A | 0.079356 |
| 018-json-lines-1000ms-31n-file-r3 | post_warmup | snapshot_scan_duration_ns | 21 | 17.5869 | N/A | N/A | 19.4791 |
| 018-json-lines-1000ms-31n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 018-json-lines-1000ms-31n-file-r3 | post_warmup | lateness_ns | 21 | 0.061706 | N/A | N/A | 0.064564 |
| 018-json-lines-1000ms-31n-file-r3 | post_warmup | interval_deviation_ns | 21 | 0.061706 | N/A | N/A | 0.064564 |
| 019-table-100ms-3001n-file-r3 | whole_run | snapshot_scan_duration_ns | 3001 | 17.62 | 18.8439 | 117.897 | 503.365 |
| 019-table-100ms-3001n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 019-table-100ms-3001n-file-r3 | whole_run | lateness_ns | 3000 | 0.0616685 | 0.078049 | 17.9888 | 403.454 |
| 019-table-100ms-3001n-file-r3 | whole_run | interval_deviation_ns | 3000 | 0.0616685 | 0.078049 | 17.9888 | 403.454 |
| 019-table-100ms-3001n-file-r3 | post_warmup | snapshot_scan_duration_ns | 2902 | 17.6143 | 18.7216 | 117.897 | 503.365 |
| 019-table-100ms-3001n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 019-table-100ms-3001n-file-r3 | post_warmup | lateness_ns | 2902 | 0.0616835 | 0.078062 | 17.9888 | 403.454 |
| 019-table-100ms-3001n-file-r3 | post_warmup | interval_deviation_ns | 2902 | 0.0616835 | 0.078062 | 17.9888 | 403.454 |
| 020-json-lines-100ms-3001n-file-r3 | whole_run | snapshot_scan_duration_ns | 3001 | 17.4897 | 18.3889 | 86.7574 | 387.622 |
| 020-json-lines-100ms-3001n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 020-json-lines-100ms-3001n-file-r3 | whole_run | lateness_ns | 3000 | 0.0617445 | 0.078064 | 0.091692 | 287.654 |
| 020-json-lines-100ms-3001n-file-r3 | whole_run | interval_deviation_ns | 3000 | 0.0617445 | 0.078064 | 0.091692 | 287.654 |
| 020-json-lines-100ms-3001n-file-r3 | post_warmup | snapshot_scan_duration_ns | 2905 | 17.5117 | 18.3933 | 83.1812 | 387.622 |
| 020-json-lines-100ms-3001n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 020-json-lines-100ms-3001n-file-r3 | post_warmup | lateness_ns | 2905 | 0.061745 | 0.078104 | 0.091318 | 287.654 |
| 020-json-lines-100ms-3001n-file-r3 | post_warmup | interval_deviation_ns | 2905 | 0.061745 | 0.078104 | 0.091318 | 287.654 |
| 021-table-1000ms-301n-file-r3 | whole_run | snapshot_scan_duration_ns | 301 | 17.6988 | 19.2029 | N/A | 308.769 |
| 021-table-1000ms-301n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 021-table-1000ms-301n-file-r3 | whole_run | lateness_ns | 300 | 0.0631175 | 0.081086 | N/A | 0.130569 |
| 021-table-1000ms-301n-file-r3 | whole_run | interval_deviation_ns | 300 | 0.0631175 | 0.081086 | N/A | 0.130569 |
| 021-table-1000ms-301n-file-r3 | post_warmup | snapshot_scan_duration_ns | 291 | 17.7188 | 19.2029 | N/A | 308.769 |
| 021-table-1000ms-301n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 021-table-1000ms-301n-file-r3 | post_warmup | lateness_ns | 291 | 0.063162 | 0.082488 | N/A | 0.130569 |
| 021-table-1000ms-301n-file-r3 | post_warmup | interval_deviation_ns | 291 | 0.063162 | 0.082488 | N/A | 0.130569 |
| 022-json-lines-1000ms-301n-file-r3 | whole_run | snapshot_scan_duration_ns | 301 | 17.7845 | 19.9254 | N/A | 361.939 |
| 022-json-lines-1000ms-301n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 022-json-lines-1000ms-301n-file-r3 | whole_run | lateness_ns | 300 | 0.0624215 | 0.07926 | N/A | 0.420991 |
| 022-json-lines-1000ms-301n-file-r3 | whole_run | interval_deviation_ns | 300 | 0.0624215 | 0.07926 | N/A | 0.420991 |
| 022-json-lines-1000ms-301n-file-r3 | post_warmup | snapshot_scan_duration_ns | 291 | 17.7752 | 19.9254 | N/A | 361.939 |
| 022-json-lines-1000ms-301n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 022-json-lines-1000ms-301n-file-r3 | post_warmup | lateness_ns | 291 | 0.062375 | 0.079529 | N/A | 0.420991 |
| 022-json-lines-1000ms-301n-file-r3 | post_warmup | interval_deviation_ns | 291 | 0.062375 | 0.079529 | N/A | 0.420991 |
| 023-table-100ms-301n-file-r3 | whole_run | snapshot_scan_duration_ns | 301 | 15.0738 | 20.184 | N/A | 316.097 |
| 023-table-100ms-301n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 023-table-100ms-301n-file-r3 | whole_run | lateness_ns | 300 | 0.060188 | 0.103619 | N/A | 216.185 |
| 023-table-100ms-301n-file-r3 | whole_run | interval_deviation_ns | 300 | 0.060188 | 0.103619 | N/A | 216.185 |
| 023-table-100ms-301n-file-r3 | post_warmup | snapshot_scan_duration_ns | 203 | 14.7714 | 15.6654 | N/A | 208.124 |
| 023-table-100ms-301n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 023-table-100ms-301n-file-r3 | post_warmup | lateness_ns | 203 | 0.060719 | 0.078542 | N/A | 108.216 |
| 023-table-100ms-301n-file-r3 | post_warmup | interval_deviation_ns | 203 | 0.060719 | 0.078542 | N/A | 108.216 |
| 024-json-lines-100ms-301n-file-r3 | whole_run | snapshot_scan_duration_ns | 301 | 14.7501 | 15.364 | N/A | 299.976 |
| 024-json-lines-100ms-301n-file-r3 | whole_run | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 024-json-lines-100ms-301n-file-r3 | whole_run | lateness_ns | 300 | 0.0605185 | 0.07705 | N/A | 200.019 |
| 024-json-lines-100ms-301n-file-r3 | whole_run | interval_deviation_ns | 300 | 0.0605185 | 0.07705 | N/A | 200.019 |
| 024-json-lines-100ms-301n-file-r3 | post_warmup | snapshot_scan_duration_ns | 201 | 14.7613 | 15.4027 | N/A | 299.976 |
| 024-json-lines-100ms-301n-file-r3 | post_warmup | failed_scan_duration_ns | 0 | N/A | N/A | N/A | N/A |
| 024-json-lines-100ms-301n-file-r3 | post_warmup | lateness_ns | 201 | 0.06074 | 0.078904 | N/A | 200.019 |
| 024-json-lines-100ms-301n-file-r3 | post_warmup | interval_deviation_ns | 201 | 0.06074 | 0.078904 | N/A | 200.019 |

## Repetition comparisons

Each row summarizes per-process values within one condition and RSS method, including only runs explicitly marked comparable. Incomplete, failed-attempt, or changed-inventory runs remain visible above but are excluded here. Null-output controls qualify for resource comparisons only; output-derived metrics remain unavailable. N varies when metrics are unavailable. Reported ranges expose run-to-run variability; no p95/p99 is inferred from three repetitions and unlike conditions are not pooled.

| Interval ms | Format | Duration s | Sink | RSS observer | RSS method | Metric | N | Median | Min | Max |
| ---: | --- | ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| 100 | json-lines | 30 | file | true | smaps_rollup | cpu_percent | 3 | 1.31753 | 1.15139 | 1.39753 |
| 100 | json-lines | 30 | file | true | smaps_rollup | peak_rss_kib | 3 | 63648 | 24220 | 82140 |
| 100 | json-lines | 30 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.5853 | 14.7613 | 17.9377 |
| 100 | json-lines | 300 | file | true | smaps_rollup | cpu_percent | 3 | 1.43397 | 1.26252 | 1.52373 |
| 100 | json-lines | 300 | file | true | smaps_rollup | peak_rss_kib | 3 | 47064 | 37276 | 73308 |
| 100 | json-lines | 300 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.8289 | 17.5117 | 18.1515 |
| 100 | table | 30 | file | true | smaps_rollup | cpu_percent | 3 | 1.47803 | 1.38095 | 1.72399 |
| 100 | table | 30 | file | true | smaps_rollup | peak_rss_kib | 3 | 64152 | 23644 | 82092 |
| 100 | table | 30 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.7712 | 14.7714 | 18.5099 |
| 100 | table | 300 | file | true | smaps_rollup | cpu_percent | 3 | 1.44459 | 1.31307 | 1.66171 |
| 100 | table | 300 | file | true | smaps_rollup | peak_rss_kib | 3 | 56084 | 24988 | 65860 |
| 100 | table | 300 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.6143 | 17.4173 | 17.7915 |
| 1000 | json-lines | 30 | file | true | smaps_rollup | cpu_percent | 3 | 0.151642 | 0.148222 | 0.157689 |
| 1000 | json-lines | 30 | file | true | smaps_rollup | peak_rss_kib | 3 | 61496 | 24988 | 65700 |
| 1000 | json-lines | 30 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.5869 | 17.4146 | 18.0203 |
| 1000 | json-lines | 300 | file | true | smaps_rollup | cpu_percent | 3 | 0.140611 | 0.136323 | 0.154425 |
| 1000 | json-lines | 300 | file | true | smaps_rollup | peak_rss_kib | 3 | 64808 | 45648 | 81244 |
| 1000 | json-lines | 300 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.7752 | 17.5667 | 17.8978 |
| 1000 | table | 30 | file | true | smaps_rollup | cpu_percent | 3 | 0.143594 | 0.14313 | 0.185988 |
| 1000 | table | 30 | file | true | smaps_rollup | peak_rss_kib | 3 | 63244 | 24796 | 65608 |
| 1000 | table | 30 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.5617 | 17.5109 | 18.1375 |
| 1000 | table | 300 | file | true | smaps_rollup | cpu_percent | 3 | 0.141979 | 0.139743 | 0.14384 |
| 1000 | table | 300 | file | true | smaps_rollup | peak_rss_kib | 3 | 45648 | 45648 | 80652 |
| 1000 | table | 300 | file | true | smaps_rollup | post_warmup_scan_median_ms | 3 | 17.7188 | 17.7144 | 17.7491 |

## Integrity and limitations

Requested counts and exit status never substitute for independently validated emitted records. A valid prefix does not establish all attempted scans after interruption. CPU per attempt is meaningful only for complete validated output and successful process completion.

CPU includes startup, discovery, formatting, and output writes, excluding the Python parent where child accounting permits. Kernel peak RSS may include launch/exec history; sampled RSS can miss transient peaks. The result manifest identifies RSS method and observer cadence.

Scan duration covers sequential userspace observation, not physical sensor freshness. File/null comparisons cannot exactly isolate acquisition CPU or terminal rendering. Thermal/frequency drift, background workload, page caches, provider availability, and correlated samples limit generalization.

| Case | Comparable | Inventory unchanged | RSS read failures | Output complete |
| --- | --- | --- | ---: | --- |
| 001-table-100ms-301n-file-r1 | true | true | 0 | true |
| 002-json-lines-100ms-301n-file-r1 | true | true | 0 | true |
| 003-table-1000ms-31n-file-r1 | true | true | 0 | true |
| 004-json-lines-1000ms-31n-file-r1 | true | true | 0 | true |
| 005-table-100ms-3001n-file-r1 | true | true | 0 | true |
| 006-json-lines-100ms-3001n-file-r1 | true | true | 0 | true |
| 007-table-1000ms-301n-file-r1 | true | true | 0 | true |
| 008-json-lines-1000ms-301n-file-r1 | true | true | 0 | true |
| 009-table-1000ms-301n-file-r2 | true | true | 0 | true |
| 010-json-lines-100ms-3001n-file-r2 | true | true | 0 | true |
| 011-table-100ms-3001n-file-r2 | true | true | 0 | true |
| 012-json-lines-1000ms-31n-file-r2 | true | true | 0 | true |
| 013-table-1000ms-31n-file-r2 | true | true | 0 | true |
| 014-json-lines-100ms-301n-file-r2 | true | true | 0 | true |
| 015-table-100ms-301n-file-r2 | true | true | 0 | true |
| 016-json-lines-1000ms-301n-file-r2 | true | true | 0 | true |
| 017-table-1000ms-31n-file-r3 | true | true | 0 | true |
| 018-json-lines-1000ms-31n-file-r3 | true | true | 0 | true |
| 019-table-100ms-3001n-file-r3 | true | true | 0 | true |
| 020-json-lines-100ms-3001n-file-r3 | true | true | 0 | true |
| 021-table-1000ms-301n-file-r3 | true | true | 0 | true |
| 022-json-lines-1000ms-301n-file-r3 | true | true | 0 | true |
| 023-table-100ms-301n-file-r3 | true | true | 0 | true |
| 024-json-lines-100ms-301n-file-r3 | true | true | 0 | true |
