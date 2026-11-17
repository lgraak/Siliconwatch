# Reproducing provider-level scan latency measurements

Prototype 005 compares individual hwmon registrations with contemporary full scans using the unchanged SiliconWatch release binary. The diagnostic uses Linux and Python's standard library, without root, added packages, driver changes, or hardware control. Results describe **userspace scan windows**, not pure driver-read latency or performance guarantees.

## Build and test outside measurements

Run from the repository root:

```bash
CARGO_TARGET_DIR=target/provider-latency cargo +1.85.0 build --release --locked
cargo +1.85.0 fmt --check
cargo +1.85.0 clippy --locked -- -D warnings
cargo +1.85.0 test --locked
python3 -B -m unittest discover -s scripts/tests -v
```

The four CLI/alias fixture tests use the binary under `target/provider-latency/release/`; they explicitly skip if it has not been built. They operate only on temporary fixtures. Record the actual build command; installed compiler versions alone do not prove binary provenance. Do not rebuild or edit harness sources during a measurement suite.

## Inspect and select registrations

```bash
python3 -B scripts/measure_provider_latency.py --plan
python3 -B scripts/measure_provider_latency.py --plan --select r001 --select r003
python3 -B scripts/measure_provider_latency.py --plan --select acpitz
```

`--plan` reads registration metadata and prints sanitized inventory and the proposed case order; it does not launch the executable. Selection accepts exact session tokens or a unique provider name. Provider names are not unique device identifiers: duplicate names require tokens, while duplicate requests, ambiguous names, unavailable selections and duplicate canonical registrations are rejected. With no `--select`, all eligible registrations are selected only if there are at most five; larger inventories require an explicit subset.

Tokens such as `r001` describe the current lexical class-entry inventory. They are **not persistent hardware identities** and must not be carried blindly across enumeration changes. Inspect the plan again if availability changes before execution. The private manifest records exact source paths, targets, inode evidence and channels; the exported inventory uses tokens, provider names, counts and availability codes. Zero-input and unavailable registrations remain visible. Within a suite, inventory and target changes stop comparison rather than silently selecting a replacement.

## Execute only the approved bounded experiment

The full command performs its own smoke cases and starts primary cases only if all smoke validity, identity-context and resource projections pass:

```bash
python3 -B scripts/measure_provider_latency.py --run \
  --output-dir target/provider-latency-results/reproduction-001 \
  --build-command 'CARGO_TARGET_DIR=target/provider-latency cargo +1.85.0 build --release --locked'
```

Append repeated `--select` options for an approved subset. Every output directory must be new; existing evidence is never overwritten. `--binary` may select another reviewed release executable; `--hwmon-root` supplies a custom fixture root. Neither option authorizes running an arbitrary hardware-control program or altering real sysfs.

For a separately planned **smoke-only** check:

```bash
python3 -B scripts/measure_provider_latency.py --plan --smoke-only
python3 -B scripts/measure_provider_latency.py --run --smoke-only \
  --output-dir target/provider-latency-results/smoke-only-001
```

Smoke-only mode does not resume into primary measurements. A later full `--run` repeats its own smoke cases and is a separate experiment. Do not treat both commands as one authorized six-smoke protocol, repeatedly restart failed suites, or enlarge the approved measurement budget without approval.

### Authorized single-block completion mode

`--completion-block` selects the separately approved completion protocol recorded under [Decision 43](../DESIGN_DECISIONS.md#prototype-005-completion-authorization--2026-11-16). It preserves the default three-block protocol. It requires exactly the five approved provider names, with unambiguous registration evidence; it cannot be combined with `--smoke-only` or `--summarize`.

```bash
python3 -B scripts/measure_provider_latency.py --plan --completion-block
```

The mode performs six 11-attempt smoke cases, then one 301-attempt-per-case primary block in fixed order: full-before, amdgpu, k10temp, iwlwifi_1, acpitz, nvme, full-after. All cases use 100 ms and JSON Lines; primary reporting preserves the whole run and a ten-second post-warmup subset. Fifteen-second settling intervals remain unchanged. There are at most **13 cases / 2,173 records**.

For an explicitly authorized execution only:

```bash
python3 -u -B scripts/measure_provider_latency.py --run --completion-block \
  --output-dir target/provider-latency-results/NEW-completion-session \
  --build-command 'Preserved existing Rust 1.85.0 release binary; verify its recorded SHA-256' \
  --notes 'Separately authorized completion session; identify the predecessor checkpoint hash'
```

The completion limits are **600 seconds / 128 MiB**, including a 64 MiB metadata reserve. Raw capture caps sum to 62 MiB, leaving a 126 MiB planned allowance; the total local preservation budget is 256 MiB including older suites. Keep old results immutable and check this combined budget before launch. The default protocol's 25-minute / 256 MiB bounds below do not apply to completion mode.

Admission requires the full case timeout plus ten seconds for cancellation/final preservation, rechecked after preparation. Signal handlers remain installed through alias cleanup and final writes; signals or deadline overruns during finalization persist failure. A private `execution.log` records bounded progress messages (64 KiB maximum). Application and benchmark JSON schemas are unchanged. Blocking I/O or loss of the enclosing execution environment still prevents hard cleanup guarantees; record actual enclosing timeout/cancellation policy and observed exit status separately.

Do not repeat a stopped completion session, append new observations into an old checkpoint, or treat later controls as retrospective bracketing. Any additional hardware run requires separate authorization. Execution authorization does not mark Prototype 005 accepted or resolve the local API permissions policy.

### Default three-block protocol

The default plan has at most five isolated registrations, three repetition blocks, and a full-scan control before and after each block. Registration order rotates deterministically. Every main case uses 301 attempts at 100 ms and JSON Lines, with a ten-second post-warmup subset in addition to whole-run results. Only one child runs at a time; at least 15 seconds separate cases. All cases use the same pipe-to-file capture and disable periodic RSS observations.

| Limit | Value |
| --- | --- |
| Main cases | At most 21, 40-second timeout and 8 MiB combined stdout/stderr each |
| Smoke cases | At most six, 11 attempts, 5-second timeout and 1 MiB each |
| Entire execution | 25 minutes including discovery, smoke, settling and report work |
| Storage | 256 MiB, including a 64 MiB metadata/atomic-write reserve |
| Retention | At most 27 cases / 6,387 scalar records; raw output streams to files |
| Parser/capture | 4 MiB maximum line, 64 KiB capture chunks |
| Result/report | 30 MiB compact result document, 1 MiB generated report, 1 MiB private manifest |

The maximum configured raw allowance is 174 MiB; adding the reserve gives 238 MiB. Smoke projections must fit both case and suite limits before primary sampling starts. Finite buffers and records bound retention structurally, not by a promised hard process-RSS ceiling. Python object expansion and the unchanged reader's per-snapshot inventory also consume memory. Blocking kernel/filesystem operations can defeat a hard timeout or prompt cleanup.

## Read-only aliases, failures and interpretation

Temporary class roots contain one symlink with the original registration basename and canonical target. Source files are never copied or modified. The unchanged executable only reads; the harness writes only owned local output and alias paths. Cleanup uses owned directory descriptors, explicitly rewinds directory enumeration after mutation, and unlinks only the owned symlink/empty directory. Unexpected contents remain for inspection, never recursive deletion.

Symlinks do not grant additional permissions, but they are **not an operating-system read-only sandbox**. This workflow's read-only property comes from its reviewed code paths and normal-user permissions; do not use aliases with unrelated writing tools.

Snapshot/source paths necessarily reflect the alias root. Smoke comparison checks canonical paths, channels, identity candidates/evidence and status differences. Expected changes from removing collision peers remain explicit; unexpected evidence or candidate changes stop the experiment. Isolation does not establish unique physical identity.

Timeout, handled Ctrl-C/SIGTERM, output limits, changed providers, malformed/incomplete output or failed cleanup stop the suite and retain evidence. Child-group cancellation is bounded; an unreaped child prevents further cases. Abrupt termination of the execution environment can prevent handlers, child accounting and cleanup from finishing: a stale `running` checkpoint is never proof of completion. Preserve it unchanged and document any offline recovery separately. Exit zero means `complete` or `smoke_complete`, not sensor calibration or proof that every reading is successful. Valid output prefixes do not establish all completed attempts after interruption. Missing measurements remain unavailable, not zero.

Existing scan duration includes the timestamp preamble, discovery, identity/collision processing, reads and possible descheduling. Serialization/output follows the window but can affect later scheduling. Record child CPU, runner CPU, output bytes and scheduling deviation separately. CPU percentage uses child user-plus-system CPU over parent-observed launch/capture/reap elapsed time; that denominator includes observation overhead rather than exact kernel process lifetime. Kernel child peak RSS includes inherited launch high-water accounting; it is not isolated application peak memory. Sampled RSS is intentionally unavailable in this protocol. Never subtract or sum isolated windows to explain a full scan.

Saved result JSON retains all scalar records and tail/neighbor sequence indexes for scans over 100 ms; generated Markdown summarizes those observations. Report median, maximum and N; p95 requires N ≥200 and p99 N ≥1,000, without pooling repetitions or providers to satisfy thresholds. These are reporting thresholds, not confidence guarantees. The initial 30-second cases do not reproduce the historical 300-second conditions; failure to observe 503 ms is inconclusive. Valid completed comparisons may yield inconclusive attribution; an incomplete suite is not a completed milestone.

## Evidence, privacy and offline regeneration

Each suite retains `results.json`, `report.md`, `private-manifest.json`, and per-case `stdout.raw`/`stderr.raw`. Directories are private (0700), files 0600. Raw output and manifests can contain paths, identity candidates and readings: keep them local under ignored `target/` and preserve original results.

```bash
python3 -B scripts/measure_provider_latency.py \
  --summarize target/provider-latency-results/reproduction-001 \
  > /tmp/provider-latency-regenerated.md
cmp target/provider-latency-results/reproduction-001/report.md \
  /tmp/provider-latency-regenerated.md
```

`--summarize` accepts a suite directory or its JSON result file. It reads saved scalar evidence only; it does not read current hardware or rerun cases. Identical result data with the same report implementation produces identical Markdown.

Review sanitized results and reports before sharing, especially operator-supplied `--notes` and `--build-command`. Publish only necessary environment/configuration, scalar timings/status counts and provenance hashes; omit hostnames, serials, network identifiers, source paths, full identity candidates and private manifests. Background activity, power/thermal drift, caches, identity context and provider availability limit generalization. No automatic upload or publication is performed.

## Saved Prototype 005 evidence

The [observation report](005-provider-latency-results.md) documents the interrupted protocol and the later authorized completion session separately. Three original deterministic gzip archives retain the initial stopped smoke results, the unmodified fresh-run checkpoint, and an explicitly reviewed derivative including the interrupted output prefix. They are approximately 5.3, 351.7 and 369.1 KiB respectively; no raw captures are included. Their original local counterparts remain preserved.

Regenerate the [detail report](005-provider-latency-detail.md) without hardware access:

```bash
python3 - <<'PY'
import gzip
from pathlib import Path
with gzip.open('docs/benchmarks/results/005-provider-latency-reviewed.json.gz', 'rb') as stream:
    data = stream.read(30 * 1024 * 1024 + 1)
assert len(data) <= 30 * 1024 * 1024
Path('/tmp/siliconwatch005-reviewed.json').write_bytes(data)
PY
python3 -B scripts/measure_provider_latency.py \
  --summarize /tmp/siliconwatch005-reviewed.json \
  > /tmp/siliconwatch005-detail.md
cmp docs/benchmarks/005-provider-latency-detail.md /tmp/siliconwatch005-detail.md
```

The reviewed derivative preserves the 24 completed entries exactly, appends only independently parsed case-025 evidence, marks final accounting unavailable, and distinguishes two unstarted cases. Its `offline_review` identifies the original checkpoint hash and cleanup review. The comparative narrative excludes the interrupted prefix. No completed third block or final suite duration is inferred.

The separately authorized completion session has its own [results archive](results/005-provider-latency-completion.json.gz), [detail report](005-provider-latency-completion-detail.md) and [verification record](results/005-provider-latency-completion-verification.json). The archive is 145,783 bytes, SHA-256 `faaf4b0e62d843ca28f75279b622b1e017df356f8df22620eb3042c9182adf33`. It contains all 13 cases / 2,173 scalar records, including warmup and tail neighbors. No private manifests or raw telemetry are published.

Regenerate the completion report independently of the original interrupted report:

```bash
python3 - <<'PY'
import gzip
from pathlib import Path
with gzip.open('docs/benchmarks/results/005-provider-latency-completion.json.gz', 'rb') as stream:
    data = stream.read(30 * 1024 * 1024 + 1)
assert len(data) <= 30 * 1024 * 1024
Path('/tmp/siliconwatch005-completion.json').write_bytes(data)
PY
python3 -B scripts/measure_provider_latency.py \
  --summarize /tmp/siliconwatch005-completion.json \
  > /tmp/siliconwatch005-completion.md
cmp docs/benchmarks/005-provider-latency-completion-detail.md /tmp/siliconwatch005-completion.md
```

The measurement session completed within its approved bounds. **Chris accepted Alpha 1, Prototype 005 on 2026-11-16 with documented limitations**; see the [formal acceptance record](../milestones/005-provider-latency-characterization.md#formal-acceptance-and-publication-review--2026-11-16). Publication and further hardware measurements require separate authorization. Reproducing a saved report is offline work. Decision 43's local API permissions remain Pending.

### Historical source reproducibility

All 21 measured-source hashes match their retained local source snapshots. Preliminary suite 001 predates changes to three provider-diagnostic modules; interrupted suite 002 predates the completion-mode orchestrator change. Their exact execution sources remain privately preserved and are not retrievable from public Git history by hash alone. The final harness corresponds to completion suite 003. Both published numerical detail reports regenerate from their saved archives using the final renderer; this does not imply that the old hardware sessions ran that same source revision. Original local reports also regenerate with their retained historical renderers.

The [publication-readiness review](../milestones/005-publication-readiness.md) records the exact shareable artifacts, privacy audit and remaining tooling limitations. The completion verification JSON is a preserved pre-acceptance audit, so its historical Pending assessment remains intact; current acceptance is recorded in the milestone.
