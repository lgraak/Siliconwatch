# Alpha 1, Prototype 009: Local Read-Only API

Date: **2026-11-17**. Implementation status: **Complete**.
Acceptance status: **Accepted by Chris — November 17, 2026**.

### Formal acceptance and publication-readiness review — 2026-11-17

Chris formally accepts the implemented **local read-only Unix-socket API** as
technically complete, with the documented security, lifecycle and freshness
limitations. Acceptance covers the current-user, peer-verified informational and
snapshot reads; it does not certify production readiness or authorize deployment.
Multiuser/remote access, write operations and hardware controls remain excluded.
Decisions 5 and 43 retain their already approved narrow Alpha 1 policy.

Chris separately authorizes acceptance documentation, fresh deterministic
verification, preservation checks and the [publication-readiness review](009-publication-readiness.md).
This does **not** authorize staging, committing, pushing, creating or merging a PR,
deployment, GUI implementation or Prototype 010.

The approved proposal is preserved unchanged. Sections 1–9 below retain the
original implementation-closeout record, including its then-pending acceptance,
22-file manifest and original handoff/recommendation. This dated acceptance entry
supersedes those earlier status statements; the readiness report provides the
current 23-file intended publication manifest and fresh verification results.
The local home-directory prefix in section 7 was removed during privacy review;
no implementation evidence, test result or historical recommendation was rewritten.
The exact local worktree path remains in the external continuation.

## 1. Milestone Summary

Implemented the approved current-user pathname Unix socket and two versioned
read-only JSON methods, integrated with Prototype 008's foreground daemon and
Prototype 007's unique acquisition owner/cache. The API is disabled by default.
Existing sensors, sampling and no-API daemon behavior retain their contracts.

The [approved proposal](009-local-readonly-api-proposal.md) remains byte-for-byte
unchanged as the historical planning record. Its earlier “awaiting authorization”
statements describe that planning stage, superseded by Chris's explicit
implementation authorization and the dated Decisions 5/43 updates. Proposal
SHA-256: `bf6eff85ec7309cc43b3d277a97ff280782ccdacbc54321ecc5f16b2d8d35975`.

Implementation acceptance criteria are satisfied within the approved test scope,
with verification limitations recorded below. This is not production/security
certification or Chris's formal acceptance.

## 2. Implementation Details

### Runtime, protocol and concurrency

Enable with:

```sh
cargo run -- daemon --interval-ms 1000 --stale-after-ms 3000 --local-api
```

The existing `$XDG_RUNTIME_DIR` must be owned by the non-root current user with
mode 0700. The daemon verifies every ancestor, creates a new private
`siliconwatch` child and mode-0600 `api.sock`, and refuses existing namespaces.
Only matching nonzero real/effective UID execution is allowed. Every accepted
connection must pass a kernel `SO_PEERCRED` UID check before parsing. No TCP,
D-Bus, cross-user sharing or configuration/control operation exists. This assumes
trusted local filesystem semantics and excludes hostile same-UID/root actors.

`get_info` reports API v1, snapshot v2, identity v1, methods and fixed limits.
`get_snapshot` returns lifecycle, assessment time, producer state, conservative
currentness, latest attempt and an optional observation with its own attempt,
role, age bounds and unchanged sensor snapshot. Acquisition failure is retrieved
as data, not confused with API failure. Partial rows, unsupported inputs, raw
text/exact i64 values, canonical values, identity evidence and diagnostics survive.
Celsius remains the temperature unit; no calibration or Fahrenheit acquisition
transformation was introduced.

The [public API contract](../api/local-v1.md) contains exact field tables, enum
spellings/nullability, both requests, complete successful/partial and historical
failure responses, errors, timestamp arithmetic and a diagnostic client command.
The [JSON Schema](../api/local-v1.schema.json) describes complete message shapes.
The example Python client is development tooling; the installed application is
entirely Rust. Clients must parse existing numeric raw values losslessly and
handle unknown future values conservatively.

There is one acquisition owner, one API worker and one main signal coordinator.
The API holds only a cache reader and lifecycle view; it cannot initiate scans.
Cache→lifecycle try-lock order distinguishes contention (`cache_busy`) from fatal
poison. Lifecycle is released after assessment; cache preflight and capped memory
encoding finish before releasing the view and performing any socket I/O.

Limits include one active connection, 4096 request bytes and 1048576 response
bytes including LF, one-second absolute request and two-second absolute response
budgets, 4096 rows, 1024 warnings, 64 identity issues per row, 16384 bytes per
text/path and 262144 aggregate text/path bytes. No response truncation, snapshot
clone, application queue, thread per client, general async runtime or web
framework is introduced. The kernel backlog and existing acquisition/cache
payloads are outside API memory limits. An instance token uses 128 bits of bounded
nonblocking kernel entropy with no weak fallback.

On stop, acquisition admission closes; active API I/O is cancelled, the API is
joined and its endpoint cleaned before waiting for an admitted acquisition.
All created workers are joined before final Stopped. Normal/fatal/panic API exits
wake the signal coordinator. Routine malformed requests, slow/disconnected
clients and oversize responses terminate only the request. Acquisition failures
remain recoverable. Already admitted kernel/hardware reads cannot be cancelled;
no hard shutdown deadline is promised. Final Stopped over the socket is not
promised because admission closes during shutdown.

### Dependencies and important files

Added Linux-only direct `libc = "0.2.191"`, already locked transitively. The
lockfile adds only `libc` to SiliconWatch's direct dependency list: no package,
version or checksum changed. The cached crate declares Rust 1.65 and license
`MIT OR Apache-2.0`; actual Rust 1.85 builds passed. Production unsafe calls are
confined to the small OS boundary for UID, peer credentials and entropy, with
buffer/length/result checks and bounded interrupted retries. Existing serde,
serde_json and signal-hook are reused. No package installation occurred.

Important source boundaries are `src/local_api.rs` (transport/cancellation),
`src/local_api/os.rs` (Linux boundary/namespace), `src/local_api/wire.rs` (borrowed
DTOs/preflight/encoding), `src/daemon/api.rs` (supervision/service view), and the
narrow `CacheReader::try_read` addition. Corresponding unit and process fixtures
are separate files, listed in the exact review manifest in section 7.

## 3. Verification Results

All final commands ran from the Prototype 009 worktree on the host as the normal
non-root user. No tests depended on real hardware. Host execution was required
for real Unix sockets and ordinary filesystem ownership: an earlier sandbox
integration attempt passed two tests and failed eleven because the sandbox
represented root-owned ancestors as UID 65534 and denied socket binding. The
same fixtures passed on the host; this was recorded separately, not hidden as
skips or attributed to a product defect.

Exact toolchain/environment versions:

- Rust 1.85.0 (`4d91de4e4`, 2025-02-17), Cargo 1.85.0 (`d73d2caf9`, 2024-12-31), LLVM 19.1.7.
- Installed stable Rust 1.100.0 (`0ef1bc386`, 2026-11-09), Cargo 1.100.0 (`3acc79c0c`, 2026-11-05), LLVM 23.1.5.
- Both target `x86_64-unknown-linux-gnu`; Python 3.14.10; CachyOS Linux kernel `7.3.4-1-cachyos`.

Each row below ran for **both** `T=1.85.0` and `T=stable`:

| Exact command | Rust 1.85 result | Stable result |
| --- | --- | --- |
| `cargo +T fmt --check` | exit 0 | exit 0 |
| `cargo +T clippy --locked --offline -- -D warnings` | exit 0 | exit 0 |
| `RUST_TEST_NOCAPTURE=1 cargo +T test --locked --offline` | exit 0; 166 tests + 2 doctests | exit 0; 166 tests + 2 doctests |
| `cargo +T build --release --locked --offline --target-dir target/provider-latency` | exit 0 | exit 0 |
| `bash /tmp/siliconwatch008-readiness/checks/strict-python-ci.sh` after that release build | exit 0; 153 tests | exit 0; 153 tests |
| `python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v` | exit 0; 6 tests | exit 0; 6 tests |

Every passing suite had **zero failures, ignored tests or skips**. The two
compile-fail doctests intentionally emit compiler errors and pass by proving
ownership/borrowing restrictions. `RUST_TEST_NOCAPTURE=1` confirmed that historical
permission fixtures did not silently return early. The strict Python step is an
exact copy of `.github/workflows/alpha1.yml`'s fixture step: it checks discovery,
required modules/CLI tests, fixture executable availability and rejects all skips.
It ran against each freshly built release binary, not merely once against an
unidentified executable. DCO-checker tests do not establish a new contribution
signoff; there is no contribution commit in this task.

Rust executable totals on each toolchain: library 100, main 2, CLI 7, daemon CLI 7,
hwmon 9, identity 12, local API CLI 14, sampling CLI 8, snapshot-cache integration 7.
This adds 61 tests over the 105-test Prototype 008 baseline, plus its unchanged two
doctests. The 14 API process tests passed with no ignored/conditional cases.

Coverage includes:

| Acceptance area | Executed evidence |
| --- | --- |
| Opt-in/default and regressions | Existing CLI/daemon/sampling suites; duplicate/opt-in flag checks |
| Permissions/credentials | Exact 0700/0600, actual same-UID credentials, mismatching expected UID, injected malformed/failed/interrupted credential lookup, root/setid policy |
| Namespace/cleanup | Unsafe roots/ancestors, symlinks, owner/mode policy, overlong path, stale/existing namespace, second instance, directory/socket replacement left intact |
| Protocol/values | Strict duplicate/unknown-field/type grammar, both methods, version errors, first-frame-only, exact frame boundary, missing LF, raw i64 extrema, metadata/diagnostics and schemas |
| Failure/history/freshness | Empty vs initial failure, partial scans, historical success after failure, recovery, threshold equality/staleness, stopped-role precedence, lifecycle override, full-width times and wall-time errors |
| Bounds/deadlines | Collection/text/aggregate limits, JSON escape expansion beyond 1 MiB, exact encoded limit, nonfinite/non-UTF-8 failure, partial reads/interruption and late/cancelled read rejection, actual blocked socket timeout |
| Cache/concurrency | Zero API-triggered attempts, cache/control busy vs poison, serial clients, real socket backpressure while a second publication completes, guards released before output |
| Lifecycle | SIGINT/SIGTERM during active API with FIFO-blocked acquisition; API cleanup before acquisition release; startup reporting/spawn failure; isolated normal/fatal/panic API exit waking real Signals; all workers joined |

A real differently credentialed client process was **not provisioned or executed**;
actual same-UID kernel credentials, a deliberately different expected UID and
injected lookup failures cover the safe local test scope. No such unexecuted
scenario is counted as a passing test. OS resource-exhaustion/thread-spawn failure
is injected at the supervisor result seam; the host was not deliberately exhausted.
No alternate architecture, Linux distribution or non-Linux runtime was tested.

### Supplemental all-target Clippy

For each toolchain, `cargo +T clippy --locked --offline --all-targets -- -D warnings`
exited **101** solely for two existing `clippy::non_octal_unix_permissions`
findings: `tests/hwmon.rs:217` and `tests/identity.rs:368`, both
`fs::Permissions::from_mode(0)`. These files remain unchanged. They are outside
configured Clippy and this implementation scope; no unrelated cleanup was made.

### Evidence location

Exact commands, exit codes, logs and release SHA-256 values are external in
`/tmp/siliconwatch009-implementation/checks/` (`results.json`, `versions.log`,
per-command logs and `hardware-smoke.json`). The strict Python script remains
at its exact command path above. Supplemental fixture logs and the sandbox
observation are in `/tmp/siliconwatch009-api-cli-verification/`. Final manifest,
document checks and preservation audit remain external as well. JSON syntax, 70
local schema references, field sets against 15 source structs, both status enums,
seven public examples, 21 invalid message mutations and 10000 deterministic
decimal range comparisons passed a dependency-free audit. Formal Draft 2020-12
metaschema validation was not available or claimed. These temporary
artifacts are local review evidence, not promised permanent repository assets.
No hosted CI or publication verification is claimed for uncommitted Prototype 009.

## 4. Hardware Validation

One short read-only functional smoke ran the stable release daemon against its
default existing hwmon root with `--interval-ms 60000 --stale-after-ms 3000
--local-api`, a test-owned private runtime directory, then requested SIGTERM.
Only acquisition sequence **1** was published. This was not a benchmark, matrix,
calibration or investigation of the historical timing outlier.

On CachyOS x86_64 / kernel `7.3.4-1-cachyos`, both API methods succeeded. The
snapshot exposed **11 sensors**, **9 ok and 2 unsupported**, with zero warnings
and zero permission-denied rows. Providers were `acpitz` (1), `amdgpu` (5),
`iwlwifi_1` (1), `k10temp` (1) and `nvme` (3). Categories were seven Celsius
rows, two voltage rows and two unsupported rows without canonical units;
identity status was nine best-effort and two ambiguous-channel. No fan row was
exposed in this sample; complete physical hardware coverage was not established.

One example preserved raw text `"20000"`, integer `20000`, and converted `20.0 °C`.
This verifies transport/conversion preservation, not sensor accuracy. Lifecycle
was running, producer active and service-current true at assessment, while
physical-age-known remained false and unsupported rows remained explicit.
Sensitive paths and device identifiers were not published in the report.

Directory/socket modes were 0700/0600, stdout remained empty, exit was 0 with
graceful reporting, the owned endpoint/child were removed, and the fixture daemon
was reaped. No hardware or system-service configuration was modified.

## 5. Architecture and Design Review

[Decision 5](../DESIGN_DECISIONS.md) now records Chris's approved Alpha 1 pathname
Unix socket/versioned JSON direction, retaining the original D-Bus rationale as
dated history. D-Bus is deferred. Decision 43 approves only current-user,
peer-verified, read-only access; multiuser/remote access, configuration writes and
privileged hardware controls remain pending. Decision 26's future shared service
is not silently claimed to be implemented. Prior Prototype 005 decision history
and earlier reports remain unchanged.

No functional departure from the approved proposal was necessary. The proposed
module was split into OS, wire and daemon-supervision files because the concrete
responsibilities/tests warranted it; the added JSON Schema is a documentation
artifact, not a runtime dependency or abstraction. The existing cache and unique
owner are retained. Short bounded read guards still impose encoding cost on
publication, and Rust RwLock fairness is not guaranteed. The test establishes
publication progress during actual socket backpressure, not a hardware-latency
or general fairness bound.

## 6. Known Issues and Risks

- No unresolved implementation defect was confirmed by the executed checks and
  review. This is limited experimental validation, not proof against every race.
- Same-UID hostile code, root/privileged namespaces, descriptor passing after
  authorization and untrusted filesystem semantics are outside isolation scope.
  Path identity cleanup is not atomic protection against hostile same-UID races.
- Existing endpoints require explicit inspected manual recovery. Crash/SIGKILL
  may leave artifacts; cleanup uncertainty leaves replacements untouched.
- One client can delay others; there is no fairness, kernel-backlog memory bound,
  rate limiting or authentication service. Clients need bounded retries.
- Entire snapshots above limits or with unrepresentable paths/values are rejected;
  pagination, truncation and lossy recovery are absent.
- Deadlines are cooperative; scheduling, kernel I/O, acquisition, filesystem or
  stderr stalls can delay shutdown. No forced second-signal exit is implemented.
- Age bounds apply only at assessment. Physical sensor age and freshness at receipt
  remain unknown. Known suspend requires restarting/recreating the cache;
  automatic suspend detection remains excluded. Identity is best-effort evidence,
  and acquisition success does not certify each sensor row or hardware health.
- Real cross-UID process isolation and other platforms were not executed. JSON
  Schema cannot detect duplicate keys after a parser discards them, guarantee
  byte/frame budgets, or prove cross-field timing/currentness semantics; use the
  public contract and the Rust parser/fixture evidence for those properties.
- The two supplemental historical Clippy findings remain. Hosted CI awaits future
  separately authorized publication.

No GUI, database, remote or D-Bus service, multiuser sharing, controls, calibration,
adaptive polling, alarms, protective actions, production systemd installation or
Prototype 010 work was performed.

## 7. Git Status and Exact Review Manifest

Worktree directory: `SiliconWatch-prototype009` (full local path retained in the external continuation).
Branch: `codex/alpha1-prototype009`.
HEAD/base: **231034f2822b9c91e8781c20579729374328280d** (unchanged).
No upstream; no staged entries/unmerged paths, new commit, push or Prototype 009 PR.
The worktree intentionally contains eight modified tracked files and fourteen
untracked review files. No historical report was edited.

Exact 22-file review manifest (all unstaged; the proposal predates implementation):

1. `Cargo.toml` — modified
2. `Cargo.lock` — modified
3. `README.md` — modified
4. `docs/DESIGN_DECISIONS.md` — modified
5. `src/lib.rs` — modified
6. `src/main.rs` — modified
7. `src/daemon.rs` — modified
8. `src/snapshot_cache.rs` — modified
9. `src/daemon/api.rs` — new
10. `src/daemon/api/tests.rs` — new
11. `src/daemon/api/output_tests.rs` — new
12. `src/local_api.rs` — new
13. `src/local_api/os.rs` — new
14. `src/local_api/tests.rs` — new
15. `src/local_api/wire.rs` — new
16. `src/local_api/wire/tests.rs` — new
17. `src/snapshot_cache/api_tests.rs` — new
18. `tests/local_api_cli.rs` — new
19. `docs/api/local-v1.md` — new
20. `docs/api/local-v1.schema.json` — new
21. `docs/milestones/009-local-readonly-api.md` — new, this report
22. `docs/milestones/009-local-readonly-api-proposal.md` — pre-existing untracked, unchanged

Preservation audit compares original/007/008 worktrees against saved baselines
(78/75/81 files), their branch/status state, ten prior branch refs, and all **225**
private evidence files. Original agent-skills setup, intentional Prototype 006
report changes, historical benchmark archives and existing branches are preserved.
No captures, binaries, temporary verification logs or skill setup were copied into
the review manifest. Ignored build outputs reside only in Prototype 009's target
directory. The unchanged proposal hash was rechecked independently.

## 8. Recommended Next Milestone

First obtain **Chris's formal technical review and acceptance of Prototype 009**.
If accepted, recommend one small **Prototype 009 publication-readiness review**
under separate authorization, not Prototype 010 implementation. Its objective is
to freeze an accepted exact manifest, audit dependency and policy text, reverify
preservation and applicable checks, and prepare a concrete publication proposal.

Acceptance criteria: resolve review findings; record accepted limitations and final
hashes; verify the index/scope/ancestry; preserve all excluded material; prepare
DCO/publication steps only under explicit later authorization. Expected tests are
the configured dual-toolchain/strict fixture checks as warranted by changes;
no new hardware benchmarks are needed. Blockers are review decisions and separate
publication authorization, not a known failing configured check. No next milestone
has been started.

## 9. HANDOFF TO CHATGPT

Chris authorized local implementation, tests, documentation and verification of **Prototype 009: Local Read-Only API**, following approval of its architecture on November 17, 2026. Implementation is complete and ready for formal review; technical acceptance and publication remain Chris's separate decisions.

The foreground Rust daemon now supports opt-in `--local-api` at `$XDG_RUNTIME_DIR/siliconwatch/api.sock`. The two API v1 methods, `get_info` and `get_snapshot`, consume the existing cache without scheduling hardware acquisition. Private runtime validation, 0700 directories, a 0600 socket, non-root matching real/effective UIDs and Linux peer credentials enforce the approved current-user boundary. Existing or stale endpoints are refused. Cleanup checks the identities of resources created by this process.

Responses preserve snapshot schema 2, identity schema 1, exact raw integers/text, canonical values, source evidence and diagnostics. Acquisition failure remains distinct from protocol failure; historical observations retain their own timestamps. Freshness applies at assessment, not receipt, and physical measurement age remains unknown. One API worker enforces request, response and deadline limits. Encoding completes before socket I/O; a regression proves cache publication progresses under real socket backpressure. API exit, fatal errors and panic wake the signal coordinator, and all created workers are joined before final Stopped.

Rust **1.85.0** and installed stable **1.100.0** both passed formatting, configured warnings-denied Clippy, **166 executable Rust tests plus two compile-fail doctests**, and release builds. Each release passed **153 strict Python tests with zero skips** and **six DCO-checker tests**. Supplemental all-target Clippy still reports only two pre-existing non-octal permission literals in historical tests. A brief read-only hardware smoke retrieved 11 rows, preserved two unsupported rows, and exited gracefully with its endpoint removed. No extended benchmarks or hardware settings changes occurred.

Decisions 5 and 43 now record the approved Unix-socket JSON/current-user read-only policy while preserving history; broader access and controls remain pending. The only dependency change is direct Linux `libc` 0.2.191, already present in the lockfile. Public protocol documentation, machine-readable schemas and this report are available locally.

Work remains on **codex/alpha1-prototype009** at unchanged HEAD **231034f2822b9c91e8781c20579729374328280d**. Nothing was staged, committed, pushed or published; there is no Prototype 009 PR or upstream. The exact 22-file review manifest includes the unchanged approved proposal. Original/007/008 worktrees, agent skills, intentional Prototype 006 edits, existing branches, archives and all 225 private evidence files are preserved.

Known limits include same-UID/root trust, unexecuted real cross-UID isolation testing, no fairness or hard shutdown guarantee, manual stale recovery and no freshness guarantee at receipt. No unresolved implementation blocker is known. The repository's **/handoff** skill was invoked and the continuation saved outside all worktrees. Recommended next authorization: **Chris's formal review and technical acceptance, followed by a separately authorized publication-readiness review**. No GUI, deployment, controls or Prototype 010 work has begun.
