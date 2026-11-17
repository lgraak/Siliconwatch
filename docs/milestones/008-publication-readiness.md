# Prototype 008: Acceptance and Publication Readiness

**Milestone status: Accepted by Chris — November 17, 2026.**

**Readiness review: Complete; suitable for separately authorized publication with the documented limitations.** No staging, commit, push, PR, merge, deployment or Prototype 009 work was performed or authorized by this review.

## 1. Formal acceptance and scope

Chris formally accepts **Alpha 1, Prototype 008: Headless Daemon Lifecycle** as technically complete. Acceptance covers the implemented **Rust foreground daemon** and approved **Option 2: cooperative shutdown**. All limitations in the [milestone report](008-headless-daemon-lifecycle.md) remain accepted, not resolved.

Acceptance does not establish production readiness or authorize deployment, systemd installation, API/GUI development, sensor correction, hardware controls or protective actions. The installed application remains Rust-only; Python is development/test tooling. Decision 43's local API permissions policy remains **Pending**.

The [proposal](008-headless-daemon-lifecycle-proposal.md) remains byte-for-byte historical planning, including its earlier approval language. Its SHA-256 is `c37dfcd9b076a07b03150503dd01932d4030223e75fa0e2c6c08830dd843135c`. The milestone report preserves its implementation evidence and original handoff, adds the dated acceptance record, and points to this fresh readiness review.

## 2. Exact intended publication manifest

The user-supplied ten-file implementation manifest was present at preflight and all ten hashes matched the previous implementation handoff. This documentation review changes only the milestone report and adds this readiness report. Accepted source, tests, Cargo files, README and proposal are unchanged by this review.

The revised intended publication manifest is **exactly eleven files**:

| # | Repository-relative path | Purpose |
| --- | --- | --- |
| 1 | `Cargo.toml` | Narrow signal dependency |
| 2 | `Cargo.lock` | Exact resolution |
| 3 | `README.md` | Concise prototype status and daemon usage |
| 4 | `src/lib.rs` | Daemon module export |
| 5 | `src/main.rs` | Compatible daemon command parsing/dispatch |
| 6 | `src/daemon.rs` | Acquisition worker, lifecycle, scheduling, signals and cleanup |
| 7 | `src/daemon/tests.rs` | 21 deterministic unit/fixture tests |
| 8 | `tests/daemon_cli.rs` | Seven isolated-process CLI/signal tests |
| 9 | `docs/milestones/008-headless-daemon-lifecycle-proposal.md` | Preserved approved proposal |
| 10 | `docs/milestones/008-headless-daemon-lifecycle.md` | Implementation evidence and formal acceptance |
| 11 | `docs/milestones/008-publication-readiness.md` | This separately added readiness report |

Only these paths differ from the base or are untracked in this worktree. The index remains empty. Build outputs, temporary verification scripts/logs, handoff documents, private benchmark captures/manifests, agent-skills setup and other worktrees are excluded. No new binary, archive or generated measurement artifact is proposed for publication.

## 3. Correctness and compatibility review

Read-only review found **no confirmed correctness blocker** and required no implementation change:

- Exactly one worker owns the cache acquisition handle. Stop acknowledgement and admission share the control mutex. An admitted scan may still begin or publish after `Stopping`; subsequent admission is closed.
- Service currentness requires the running lifecycle, active producer, successful latest publication, non-historical role and age within budget. Stopping/stopped data remains non-current even when the cache temporarily still reports active/latest data.
- Recoverable failures remain explicit, preserve only clearly historical prior success, and permit normal recovery. Fatal control/cache/worker errors close admission and complete cleanup/join before final reporting.
- Worker exit/panic wakes the signal coordinator. The guarded signal wait converts unexpected iterator unwinding into a fatal result and the same stop/close/join path. Poisoned lifecycle state is reported unavailable rather than fabricated as healthy.
- Sequential scheduling uses checked monotonic dispatch times and predicate-based waits. There is no overlap, backlog or adaptive polling. The service does not replace cache observation timing with scheduler offsets or wall time.
- Startup acknowledgement precedes first admission; main has no competing readiness wait. Registration remains alive during join so repeated signals remain idempotent. Closing delivery does not cancel a hardware read.
- Daemon errors do not enter the legacy one-shot broken-pipe-success branch. Required-output failure is nonzero, invalid arguments are exit 2 when reportable, and completed cooperative shutdown is exit 0 even with recoverable acquisition failures.
- Scanner, source identity, foreground sampling, cache implementation and existing tests match the base. Existing CLI/schema regressions pass. No external read API, service installer or hardware control was added.

Raw values, conversions, identity evidence, sensor ordering and diagnostic precedence remain unchanged. Celsius remains the future default display unit; Fahrenheit preferences remain presentation-only. The private service inspection is not a public cache API.

## 4. Fresh deterministic verification — November 17, 2026

These are newly executed checks against the accepted implementation, not results copied from the earlier report. All 37 verification-input hashes were unchanged before/after testing. `RUST_TEST_NOCAPTURE=1` exposed conditional test-skip diagnostics; none occurred.

Versions observed:

- `rustc 1.85.0 (4d91de4e4 2025-02-17)`; `cargo 1.85.0 (d73d2caf9 2024-12-31)`.
- Installed stable: `rustc 1.100.0 (0ef1bc386 2026-11-09)`; `cargo 1.100.0 (3acc79c0c 2026-11-05)`.
- Python `3.14.10`.

| Executed command | Actual result |
| --- | --- |
| `cargo +1.85.0 fmt --check` | Exit 0 |
| `cargo +stable fmt --check` | Exit 0 |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings` | Exit 0 |
| `cargo +stable clippy --locked --offline -- -D warnings` | Exit 0 |
| `cargo +1.85.0 test --locked --offline` | **105 executable tests + 2 doctests passed; 0 failed/ignored** |
| `cargo +stable test --locked --offline` | **105 executable tests + 2 doctests passed; 0 failed/ignored** |
| `cargo +1.85.0 build --release --locked --offline --target-dir target/provider-latency` | Exit 0 |
| `cargo +stable build --release --locked --offline` | Exit 0 |
| `bash /tmp/siliconwatch008-readiness/checks/strict-python-ci.sh` | **153 Python tests passed; 0 failed/skipped**, exit 0 |
| `python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v` | **6 DCO-checker tests passed; 0 failed/skipped**, exit 0 |
| `cargo +1.85.0 clippy --locked --offline --all-targets -- -D warnings` | Exit **101**, only the two pre-existing errors below |

Test counts match the prior report exactly. Both toolchains executed all seven daemon CLI tests, including actual SIGINT/SIGTERM on isolated fixture child processes, repeated signals while a FIFO holds an admitted read, missing roots, invalid arguments and stderr failures. Worker/coordinator panic, lock poisoning, stop/admission races, recovery, currentness and timing arithmetic are also tested. Timeout cleanup is confined to fixture children/helpers; tests never signal the developer shell or unrelated processes.

The strict Python runner is an exact extraction of the existing [CI workflow](../../.github/workflows/alpha1.yml), including required discovery/module checks and rejection of skips. It uses the freshly verified Rust 1.85.0 release fixture binary. To reproduce from the repository after the release build, run that workflow's `Python diagnostic fixtures` shell step. The temporary script and logs are verification evidence, not publication files. `--offline` assumes dependencies are already cached; an initial dependency fetch may require network access without changing the lockfile.

Full fresh logs, exact commands, versions, test counts and input hashes are retained outside Git under `/tmp/siliconwatch008-readiness/checks/`. The aggregate evidence is `summary.json` in that directory.

### Separate supplemental all-target Clippy findings

The final supplemental command reports only `clippy::non_octal_unix_permissions` for `fs::Permissions::from_mode(0)` at:

- `tests/hwmon.rs:217`.
- `tests/identity.rs:368`.

Both files match the base exactly. No fix or suppression was introduced. These are existing all-target test errors, distinct from the passing configured/default Clippy checks; all-target success is not claimed. They remain a separately scoped future cleanup, not a newly introduced implementation blocker.

### Unexecuted work and hardware evidence

No new hardware measurements, benchmark matrices, sustained stress runs or historical-outlier investigation occurred during this readiness review. The earlier short non-root hardware smoke remains clearly dated implementation evidence in the milestone report. No Windows/other-architecture validation or hosted CI was run. No performance, physical-sensor-age or bounded shutdown guarantee is inferred from fixture success.

## 5. Dependency and DCO audit

The declared direct dependency remains:

```toml
signal-hook = { version = "0.4.5", default-features = false, features = ["iterator"] }
```

The lockfile resolves **signal-hook 0.4.6**, enabling only `iterator` and its required `channel` feature. Extended signal information and `cc` are not enabled. There is no implicit iterator worker thread; application coordination remains one main thread and one acquisition worker.

| New locked package | Version | Declared minimum Rust | Linux dependency |
| --- | --- | --- | --- |
| signal-hook | 0.4.6 | 1.66 | Yes |
| signal-hook-registry | 1.4.8 | 1.26 | Yes |
| libc | 0.2.191 | 1.65 | Yes |
| errno | 0.3.15 | 1.56 | Yes |
| windows-sys | 0.61.2 | 1.71 | No, target-specific transitive entry |
| windows-link | 0.2.1 | 1.71 | No, target-specific transitive entry |

All six declare `MIT OR Apache-2.0` and contain both license texts. Six cached crate archives match the lockfile checksums, and 42 inspected source/license/manifest files match those locked archives. All 11 pre-existing external lockfile records remain unchanged. Exact-version source review confirms that closing signal delivery wakes its wait without unregistering actions; registration ownership persists through shared handles. Unregistration does not guarantee restoration of OS dispositions. Rust 1.85.0 compatibility is demonstrated by the fresh Linux tests/builds, not merely inferred from declared MSRVs. This was a manifest/source/license/feature audit, not a comprehensive upstream vulnerability audit.

The configured Git author/committer identity is present and consistent with the existing DCO-signed contribution identity. No identity configuration was changed, and no credentials were accessed. The DCO-checker suite passes. A Prototype 008 signoff cannot yet be verified because no contribution commit exists: separate publication authorization must include `git commit -s`, followed by verification of author, parent, manifest and trailer. Passing checker tests is not a claim that an uncreated commit is signed.

## 6. Privacy and preservation

The intended files were reviewed for credentials, private absolute home paths, sensitive identifiers and accidental artifacts. No such publication finding was identified. Synthetic fixture paths and sanitized provider/count examples are intentional. Pattern checks support this review but are not a universal proof that arbitrary text is non-sensitive.

Preservation checks established:

- The original checkout's **78-file** baseline, intentional modifications, branch and HEAD remain unchanged, including AGENTS.md, the Prototype 006 report, agent-skills setup and related untracked files.
- The Prototype 007 worktree remains clean with its **75-file** baseline, branch and HEAD unchanged.
- Prototype 008 retains the original proposal hash and protected scanner, identity, sampler/cache, existing-test, workflow, harness and historical-report bytes. This review changes only acceptance/reporting documentation.
- Historical checks passed for 19 published artifacts across the three worktrees, 10 compressed archives matching local JSON, 21 recorded provider-source hashes, 78 capture hashes, two preserved binaries and four completion artifacts. All **225 private evidence files** remain ignored and untracked.
- No captured data, recovery record, historical evidence manifest, agent-skills file or earlier worktree file was edited or added to the proposed manifest.

The audit retains the known Prototype 004 limitation: raw captures with recorded sizes but no historical SHA-256 can establish size agreement, not historical byte identity. Neither absent measurements nor hashes were reconstructed. Temporary verification logs, binaries and continuation documents remain outside the intended manifest.

## 7. Known limitations, blockers and Git state

**No unresolved local technical publication blocker was found.** The existing supplemental Clippy findings and accepted runtime limitations remain explicit:

- Cooperative shutdown cannot interrupt a blocked read. A previously admitted attempt may publish after stopping starts; service-level data remains non-current during stopping/stopped.
- Hardware reads, held cache guards and output streams can delay shutdown indefinitely. No hard deadline, writer-fairness guarantee or second-signal forced exit is provided.
- Memory retention is bounded by generations, not bytes/RSS; userspace freshness does not establish physical sensor age or automatic suspend-aware validity.
- Signal registration is process-global. SIGKILL, abort, allocation failure and loss of reporting destinations have no graceful-cleanup guarantee.
- No production readiness, deployment, external API, GUI, database/history, adaptive polling, calibration, alarm engine, hardware controls or protective actions are accepted or implemented by this review.

The branch remains **`codex/alpha1-prototype008`**, HEAD/base **`28d7177cc334e357646023e5a2292b0cc434e0a7`**. It has five modified tracked files and six untracked files: exactly the eleven-file manifest above. The index is empty, no upstream is set, and no staging/commit/push/PR/merge/history rewrite occurred.

## 8. Recommended next authorization

Chris may separately authorize publication of the exact **eleven-file manifest**: one DCO-signed contribution, normal push to the existing feature branch, one PR targeting `main`, and inspection of hosted CI for the final PR HEAD. A publication task must freshly check remote ancestry, authentication, exact staged manifest and signoff. Remote publication and hosted checks were not attempted here. Merge requires separate approval; Prototype 009 must not start automatically.

## Revision history

| Date | Change |
| --- | --- |
| 2026-11-17 | Recorded Chris's formal technical acceptance, fresh deterministic verification, unchanged implementation/dependencies, preservation audit and revised eleven-file publication manifest. Publication remains unauthorized. |

## HANDOFF TO CHATGPT

Chris formally accepted **SiliconWatch Alpha 1, Prototype 008: Headless Daemon Lifecycle** on **November 17, 2026**, with all documented limitations. Acceptance covers the implemented Rust foreground daemon and cooperative shutdown. It does not establish production readiness or authorize deployment, API/GUI development, hardware controls or Prototype 009.

Acceptance documentation and publication-readiness review are complete. The milestone report now records the acceptance and preserves the original implementation evidence/handoff; the approved proposal remains byte-for-byte unchanged. A separate readiness report adds one documentation file to the original ten-file implementation manifest. **No unresolved local technical publication blocker was found, and no accepted application behavior was changed.**

The exact intended publication manifest is:

1. `Cargo.toml`
2. `Cargo.lock`
3. `README.md`
4. `src/lib.rs`
5. `src/main.rs`
6. `src/daemon.rs`
7. `src/daemon/tests.rs`
8. `tests/daemon_cli.rs`
9. `docs/milestones/008-headless-daemon-lifecycle-proposal.md`
10. `docs/milestones/008-headless-daemon-lifecycle.md`
11. `docs/milestones/008-publication-readiness.md`

Fresh checks passed on **Rust 1.85.0 and installed Rust 1.100.0**: formatting, configured Clippy, release builds, and **105 executable Rust tests plus two doctests per toolchain**, with zero failures, ignored tests or conditional skips. Actual SIGINT/SIGTERM and repeated-signal behavior passed in isolated fixture processes. The strict Python suite passed **153 tests without skips**; all **six DCO-checker tests** passed. Supplemental all-target Clippy still fails only on the two pre-existing permission-literal findings; neither was changed or suppressed. No new hardware measurements, benchmark matrices or historical-outlier investigation occurred.

The dependency audit confirmed locked `signal-hook 0.4.6`, iterator/channel-only features, declared licenses/MSRVs, locked archive checksums and unchanged prior packages. Configured author/committer identity matches existing published DCO attribution. No new commit exists, so no new contribution signoff or hosted CI result is claimed.

Branch **`codex/alpha1-prototype008`** remains at **`28d7177cc334e357646023e5a2292b0cc434e0a7`**, with five modified tracked and six untracked files, an empty index and no upstream. All eleven intended files are the only worktree changes. Both earlier worktrees, agent-skills setup, the proposal hash and historical evidence remain preserved; private captures and generated outputs remain excluded.

Cooperative shutdown can wait indefinitely for blocked hardware reads, cache guards or output. An admitted scan may publish after stopping begins, while service data remains non-current. Physical sensor age, suspend-aware freshness and production performance are not guaranteed. Decision 43 remains Pending. Recommended next action: Chris's separate authorization to publish the exact eleven-file manifest through a DCO-signed commit, normal push and PR with hosted verification. Nothing has been staged, committed, pushed or submitted; stop for publication approval.
