# Reproducing the foreground monitoring baseline

The Prototype 004 harness measures the existing SiliconWatch executable without changing its discovery, identity, sampling, timing, or output semantics. It uses **Linux and Python 3.11+ standard-library modules**; no Python packages, root access, performance-policy changes, or additional services are required. Measurements characterize the recorded workload and machine, not universal performance guarantees.

## Build and verify

Run from the repository root. Build once, outside all measured intervals, using the declared minimum Rust version:

```bash
CARGO_TARGET_DIR=target/overhead cargo +1.85.0 build --release --locked
cargo +1.85.0 fmt --check
cargo +1.85.0 clippy --locked -- -D warnings
cargo +1.85.0 test --locked
python3 -B -m unittest discover -s scripts/tests -v
```

Use the same binary for all compared cases. The harness records its SHA-256 and checks it before each case. Installed compiler strings alone do not prove how a binary was built: record the actual command with `--build-command`. Do not rebuild during a suite.

## Plan, smoke-test, and run individual cases

`--plan` prints configurations without launching a child. Each output directory must be new; previous evidence is never overwritten.

```bash
python3 -B scripts/measure_overhead.py --matrix baseline --plan

python3 -B scripts/measure_overhead.py --matrix smoke --repeats 1 \
  --settle-seconds 0 --output-dir target/overhead-results/smoke-001 \
  --build-command 'CARGO_TARGET_DIR=target/overhead cargo +1.85.0 build --release --locked'

python3 -B scripts/measure_overhead.py --count 11 --interval-ms 100 \
  --format json-lines --repeats 1 --warmup-seconds 0 --settle-seconds 0 \
  --output-dir target/overhead-results/individual-001
```

The smoke matrix exercises both formats at 100/1,000 ms, using 11/3 attempts respectively and no warmup exclusion. Individual runs accept application count/interval bounds, `--format table|json-lines`, `--sink file|null`, `--no-rss`, and `--hwmon-root` for isolated fixtures. These options configure individual cases; matrix presets retain their published configuration. `--binary` selects another release executable explicitly.

Inspect smoke results and resource estimates before proceeding. A missing or changed inventory, unexpectedly slow scans, resource pressure, or unstable workload requires investigation; do not repeatedly restart expensive suites to obtain favorable results.

## Baseline and controls

```bash
python3 -B scripts/measure_overhead.py --matrix baseline --repeats 3 \
  --preflight-smoke target/overhead-results/smoke-001 \
  --output-dir target/overhead-results/baseline-001 \
  --build-command 'CARGO_TARGET_DIR=target/overhead cargo +1.85.0 build --release --locked'

python3 -B scripts/measure_overhead.py --matrix controls --repeats 3 \
  --preflight-smoke target/overhead-results/smoke-001 \
  --output-dir target/overhead-results/controls-001
```

The baseline is **24 sequential runs**: two intervals (100/1,000 ms), two formats (table/JSON Lines), two nominal spans (30/300 seconds), and three repetitions. Attempt counts are 301/31 for short runs and 3,001/301 for sustained runs. The first attempt is immediate, so count is nominal milliseconds divided by interval, plus one. Nominal durations are not exact wall-clock deadlines.

The 12 controls use 301 attempts at 100 ms: both formats, three repetitions, with RSS observation disabled, comparing capture with direct `/dev/null` stdout. They help expose observer and output effects; they do not exactly subtract acquisition or terminal-rendering CPU. The order alternates/reverses and rotates across repetition blocks, recorded in the manifest. Only one benchmark child runs at a time; baseline/controls require at least 15 seconds between runs. Nominal sampling totals 66 minutes for baseline and six minutes for controls, plus approximately 8.5 minutes of mandatory settling across the two suites and other overhead.

Baseline/controls require complete smoke evidence for the exact binary and provider inventory. Smoke-based disk/runtime projections must fit configured budgets. There are no visible-terminal or isolated-provider trials in these presets; such measurements must be identified separately and are not inferred from table-format output.

## Bounds and failure handling

| Setting | Default and meaning |
| --- | --- |
| Per-case timeout | Nominal span + greater of 10 seconds or 20%; override with `--timeout-seconds` |
| Suite timeout | `--suite-timeout-seconds 5400` (90 minutes), including settling |
| Raw output cap | `--output-limit-mib 64`, combined captured stdout/stderr per case; configurable 1–128 MiB |
| Suite disk budget | `--suite-limit-mib 1024`, configurable 128–1,024 MiB; includes a 64 MiB metadata reserve |
| Free disk margin | Suite budget plus 128 MiB is required initially; checked between runs |
| RSS observations | 1 Hz when enabled; bounded sample retention, starting after the first interval |
| Parsing | Maximum 10,000 records and 4 MiB per line; malformed/truncated data remains explicit |

The harness drains bounded pipes into private files and never stores raw telemetry in an unbounded in-memory buffer. Its combined raw-byte cap is enforced before writes. Hitting the cap marks the run incomplete, retains its bounded prefix, and stops the suite; captured byte counts are not total generated volume after truncation. Compact result JSON is capped at 31 MiB and the report at 1 MiB; the 64 MiB reserve also covers an old/new atomic JSON replacement. Preflight estimates metadata as well as raw output. The whole result reader rejects files over 64 MiB.

The manifest records `configuration.timeout_override_seconds` (null means the default formula), and each retained run records `effective_timeout_seconds`, including any reduction to the remaining suite budget. These fields were added during publication review; older archived results omit them. Missing historical fields mean unrecorded metadata, not a zero timeout.

Ctrl-C/SIGTERM requests stop the active child and further cases. Timeout, interruption, and output-limit termination first signal the child's dedicated process group, then escalate to SIGKILL after a short grace period. Reaping is bounded; an uninterruptible kernel operation may prevent prompt termination. Such a run is reported unreaped and the suite stops. Cancellation applies only to the benchmark child/group, never system shutdown or hardware settings.

Failed/incomplete runs and unstarted cases remain in the results. A case-start marker is persisted before launching a child. SIGKILL or external session teardown cannot guarantee final accounting; an in-progress marker is unfinished evidence, never success. The suite also stops on inventory changes, detected excessive background load, budget exhaustion, parsing failures, and recoverable scan errors. The coarse load guard is a one-minute load average above 80% of the reported logical CPU count; it is not a CPU-utilization measurement or thermal safety guarantee. Exit 0 means the requested suite completed under its validation rules; exit 1 means an interrupted/incomplete suite; argument/setup errors return nonzero. Application exit 0 alone never establishes healthy readings or independently verified attempt counts.

## Metrics and interpretation

- Child user/system CPU comes from `wait4`, excluding the Python parent's CPU. Average CPU is `100 × child CPU seconds / observed elapsed seconds`; **100% means one logical CPU**. The runner's CPU during launch/capture/reaping is recorded separately; subsequent parsing/report-generation CPU is outside that window. Elapsed time includes small parent-observed launch/capture/reap overhead.
- **Kernel peak RSS is child-lifetime accounting, including inherited launch high-water memory on the tested platform.** Even direct `posix_spawn` does not establish an isolated application peak. Never subtract an assumed launcher constant. This metric is separate from sampled current-process RSS.
- RSS prefers `smaps_rollup` `Rss`; `status` `VmRSS` is an explicitly approximate fallback. Once selected, a run keeps its method; failed observations stay null. PID/start-time checks guard reuse. A 1 Hz series can miss peaks or align with the application's sampling phase. Do not combine different RSS methods silently.
- Scan duration, start lateness, start gaps, and scheduling deviation come from validated existing output. They measure userspace observations, not physical sensor freshness or a hard real-time deadline. Failure distributions and sensor-status counts remain separate.
- Complete, validated output plus normal exit establishes completed attempts. A valid prefix establishes only emitted records. After interruption, actual attempts may be greater than emitted records; attempts and CPU per attempt remain unavailable. `/dev/null` runs cannot establish stdout volume or attempt/failure counts from discarded output.
- Whole-run results are retained. Primary post-warmup timing/RSS summaries exclude the first ten seconds using their respective monotonic origins. RSS early/late window summaries expose drift; this operational window is not proof of steady state.
- Report median, maximum, and N. Ordinary medians average the middle pair; tails use nearest rank. **p95 requires N ≥ 200; p99 requires N ≥ 1,000.** Otherwise report N/A. These are reporting thresholds, not confidence guarantees. Across repetitions, use median/range of per-run summaries, not tail estimates from three values.

**Transport deviation from the proposal:** captured output is pipe-to-file, not a regular file descriptor in the child. This enforces a hard shared disk cap without adding a launcher process; it introduces draining/backpressure overhead that must remain visible in comparisons. Table format captured this way does not measure a terminal emulator. `/dev/null` controls bypass stdout capture, so they compare more than just filesystem cost.

## Results, privacy, and regeneration

Each suite creates `results.json`, a reproducible `report.md`, and private per-case `stdout.raw`/`stderr.raw` files. Numeric result records retain timing, counts, resource observations, configuration, binary/Git provenance, and allowlisted environment/provider information. Missing measurements are null/N/A, never zero. Provider inventories include zero-input devices and inaccessible entries and are compared before/after each run.

```bash
python3 -B scripts/measure_overhead.py \
  --summarize target/overhead-results/baseline-001 > /tmp/siliconwatch-baseline-report.md
cmp target/overhead-results/baseline-001/report.md /tmp/siliconwatch-baseline-report.md
```

Regeneration uses sanitized results only and does not run the executable or reread raw telemetry. Raw output contains diagnostic paths/labels and stays local under ignored `target/`; it must not be committed automatically. Review result/report contents and operator-supplied `--notes`/`--build-command` before publication. Avoid hostnames, serials, private paths, or credentials in those free-text fields.

Record background activity, power source, and observed governor policy without changing them. Thermal/frequency drift, interactive work, page caches, driver caching, filesystem behavior, clock/suspend events, provider availability, and correlated observations limit comparisons. Load averages are a coarse guard, not proof of an idle or thermally stable machine. No benchmark changes drivers, CPU affinity, governors, thermal limits, or hardware settings. See the [approved proposal](../milestones/004-overhead-characterization-proposal.md) for rationale; implementation results belong in the separate milestone report.

## Recorded Prototype 004 results

See the [performance review](004-overhead-results.md), [baseline details](004-baseline-details.md), [control details](004-controls-details.md), and [milestone handoff](../milestones/004-overhead-characterization.md). The actual completed baseline used an 80-minute suite limit; controls used ten minutes. The primary matrix and controls otherwise followed the presets above. Configured settling was at least 15 seconds between cases; exact per-wait durations were not separately logged. Case elapsed times and sample timestamps are retained.

The `results/004-*.json.gz` files contain the full sanitized result documents, with deterministic gzip timestamps. Baseline/control archives preserve all scalar per-attempt observations; the other four preserve initial/final smoke evidence, the interrupted preliminary matrix attempt, and the expected output-limit failure. Raw telemetry and executable builds remain local under ignored `target/`.

**Publication review, 2026-11-16:** all six archives are byte-identical to the original local JSON and total 1,633,848 compressed bytes. The primary matrix used the default timeout formula. Review subsequently added timeout-override/effective-timeout metadata and a regression test; it did not change process execution, accounting, parsing, reporting, the Rust binary, or saved measurements. The measured runner hash in the completed baseline/control manifests is `0317667ac8decde8bf54714ae8a987cf5a828555b94531604452d9fe17359fb3`; the published runner hash after that additive correction is `5c3b17b9926bc32f62d0135e0b16e549e7c85173d18f7bc095c7770164d7bd69`. The other three harness modules retain their measured hashes. Earlier smoke/interrupted archives correctly retain their own historical hashes. The original runner and all original data remain local; results were not rewritten to pretend they used the later metadata fields. No repeat of the full matrix was needed or performed during publication review.

To regenerate the two detailed reports from the review archives without collecting hardware data:

```bash
python3 -B - <<'PY'
import gzip
from pathlib import Path
out = Path('target/overhead-review')
out.mkdir(parents=True, exist_ok=False)
for name in ('baseline', 'controls'):
    with gzip.open(f'docs/benchmarks/results/004-{name}.json.gz', 'rb') as stream:
        data = stream.read(64 * 1024 * 1024 + 1)
    if len(data) > 64 * 1024 * 1024:
        raise ValueError('archive exceeds result-reader bound')
    (out / f'{name}.json').write_bytes(data)
PY
python3 -B scripts/measure_overhead.py --summarize target/overhead-review/baseline.json > target/overhead-review/baseline.md
python3 -B scripts/measure_overhead.py --summarize target/overhead-review/controls.json > target/overhead-review/controls.md
cmp docs/benchmarks/004-baseline-details.md target/overhead-review/baseline.md
cmp docs/benchmarks/004-controls-details.md target/overhead-review/controls.md
```
