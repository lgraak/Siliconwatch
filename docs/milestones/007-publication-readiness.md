# Alpha 1, Prototype 007: Acceptance and Publication Readiness

**Review date:** November 17, 2026. **Status: Complete — locally ready for publication approval.**

## 1. Acceptance and scope

**Accepted by Chris on November 17, 2026:** the implemented **library-only Shared In-Memory Sensor Snapshot Cache**, with its documented limitations. Acceptance does not resolve those limitations or authorize production deployment, daemon/API/GUI implementation, hardware controls, or another milestone. See the [accepted milestone](007-shared-snapshot-cache.md).

This review added acceptance documentation and verified the proposed publication set. No application, test, dependency, schema, sampling or identity behavior was changed. The [proposal](007-shared-snapshot-cache-proposal.md) remains the original historical planning record; its earlier planning-only authorization and Git state predate implementation and acceptance.

No accepted architectural decision changed. Existing conventions require a revision entry when changing a decision, not for every milestone acceptance. The Prototype 005 appendix was specifically requested and is not extended here. `docs/DESIGN_DECISIONS.md` is unchanged; **Decision 43 remains Pending**.

## 2. Exact proposed publication manifest

Only these **seven files** belong in a later authorized contribution. Nothing is staged now.

| Path | Change relative to base | Purpose |
| --- | --- | --- |
| `src/lib.rs` | Modified | One-line module export. |
| `src/snapshot_cache.rs` | New | Accepted cache library. |
| `src/snapshot_cache/tests.rs` | New | Sixteen controlled-clock, concurrency, lifecycle and retention tests. |
| `tests/snapshot_cache.rs` | New | Seven filesystem-fixture integration tests. |
| `docs/milestones/007-shared-snapshot-cache-proposal.md` | New | Preserved approved proposal and historical planning evidence. |
| `docs/milestones/007-shared-snapshot-cache.md` | New | Implementation, verification, limitations and dated acceptance. |
| `docs/milestones/007-publication-readiness.md` | New | This review, manifest and publication recommendation. |

All seven are ordinary UTF-8 text. No generated binaries, Python caches, raw captures, archives, private manifests, credentials or temporary verification artifacts are included. The four source/test files and proposal remain byte-identical to the accepted implementation reviewed at task start. Only the milestone report and this new readiness report changed during this task.

The original checkout's modified `AGENTS.md`, Prototype 006 report, `.agents/`, `docs/agents/` and `skills-lock.json` are explicitly excluded. Do not copy or stage them from either worktree. `README.md`, Cargo files, workflows, existing acquisition/identity/sampling modules, baseline tests and historical benchmark reports/data remain unchanged.

## 3. Implementation and disclosure review

Source review and independent fixture checks confirmed:

- One non-cloneable acquisition owner; cloneable read handles cannot acquire or publish.
- Scans outside the state lock, followed by atomic publication of a complete result. Displaced allocations are dropped after unlocking.
- Explicit startup, successful empty inventory, row diagnostics, whole-scan failure, historical data, recovery and terminal lifecycle. `Ok(Acquisition::Failed)` denotes publication of a failure, never successful acquisition.
- Immutable borrowed observations, checked monotonic observation-age bounds and separate wall-time correlation. Numeric freshness never overrides historical role or row/identity diagnostics.
- Raw readings, conversions, ordering, warning/error precedence and identity collisions preserved. Existing CLI, sampling and snapshot/identity/JSON-Lines schemas are unchanged.
- Rust-only installed runtime; standard-library synchronization and no new dependency. Python remains test/development tooling.

No confirmed cache defect or source correction was identified. No credentials, private home-directory paths, machine identifiers or sensitive hardware evidence were found in the publication set. Generic temporary-log references in the implementation report identify local validation records, not portable inputs. Reproduction uses the checked-in workflow below, not those temporary paths. This is a bounded source/disclosure review, not a claim that every possible sensitive value has been mechanically excluded.

Future remote clients must not receive raw paths/errors indiscriminately; this cache introduces no remote interface. Default Celsius and future display preferences remain separate from telemetry.

## 4. Fresh deterministic verification

Verification used an isolated copy of the exact source, tests, scripts, workflow and Cargo inputs. Both project worktrees and their historical build outputs were left intact. Toolchains: Rust **1.85.0** and current stable **1.100.0**; Python **3.14.10**. Commands used cached dependencies with `--locked --offline` where applicable.

| Executed check | Actual result |
| --- | --- |
| `cargo +1.85.0 fmt --check`; same with `+stable` | Both passed, exit 0. |
| `cargo +1.85.0 clippy --locked --offline -- -D warnings`; same with `+stable` | Both passed, exit 0. |
| `cargo +1.85.0 test --locked --offline`; same with `+stable` | Each passed **76 executable tests + 2 compile-fail doctests**, 0 failed, 0 ignored. |
| `cargo +1.85.0 clippy --locked --offline --lib --test snapshot_cache -- -D warnings`; same with `+stable` | Both passed, exit 0. |
| `cargo +1.85.0 test --locked --offline --test hwmon --test identity permission -- --nocapture` | Both permission assertions passed; no conditional skips. |
| `cargo +1.85.0 build --release --locked --offline --target-dir target/provider-latency` | Passed, exit 0; fixture executable only. |
| Exact strict Python block from [Alpha 1 CI](../../.github/workflows/alpha1.yml) | **153 passed, 0 failed, 0 skipped**, exit 0. Required modules/CLI fixtures were present. |
| `python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v` | **6 passed, 0 failed, 0 skipped**, exit 0. |
| Supplemental Rust 1.85 all-target Clippy | Exit **101**, the two unchanged baseline findings detailed below. |
| `git diff --check`, new-file whitespace, local links, manifest and preservation checks | Passed. |

All 53 pre-cache Rust regressions remain alongside 23 cache tests. Compile-fail doctests cover owner cloning and borrowed-view lifetime. The existing suites exercise one-shot table/JSON and foreground table/JSON-Lines contracts without real hardware. No hardware acquisition, matrix, suspend test, production contention measurement or report regeneration was performed. Hosted CI has not run for these uncommitted files.

### Separate baseline Clippy findings

`cargo +1.85.0 clippy --locked --offline --all-targets -- -D warnings` reports `clippy::non_octal_unix_permissions` at unchanged `tests/hwmon.rs:217` and `tests/identity.rs:368`. Both use `fs::Permissions::from_mode(0)`. Their contents match the base commit exactly; this is not introduced by the cache. The new cache target passes Clippy, and configured CI uses default-target Clippy, which passed locally.

These two issues do not block this scoped publication-readiness recommendation. They remain documented, not fixed or suppressed. Recommend a separate authorized test-only cleanup, followed by all-target Clippy and the relevant permission tests; it must not be silently included in the seven-file manifest. No hosted green-check claim is made.

### Reproduction from a reviewed checkout

Use Linux with Rust 1.85.0 plus rustfmt/Clippy installed and Python 3.11 or newer. A fresh dependency cache may require network access; the review's `--offline` option can be omitted for initial dependency retrieval. Do not execute benchmark commands.

```bash
cargo +1.85.0 fmt --check
cargo +1.85.0 clippy --locked -- -D warnings
cargo +1.85.0 test --locked
cargo +1.85.0 clippy --locked --lib --test snapshot_cache -- -D warnings
cargo +1.85.0 build --release --locked --target-dir target/provider-latency
python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v
```

Run the exact checked-in strict Python block after building its fixture binary; do not replace it with discovery that tolerates skipped tests:

```bash
python3 -B - <<'PY'
from pathlib import Path
from textwrap import dedent

workflow = Path('.github/workflows/alpha1.yml').read_text()
block = workflow.split("python3 -B - <<'PY'\n", 1)[1].split('\n          PY', 1)[0]
exec(compile(dedent(block), '<Alpha 1 CI Python fixtures>', 'exec'))
PY
```

The extraction matches this reviewed workflow layout; inspect it again if the workflow changes. Repeat formatting, default Clippy and Rust tests with `+stable` to check the current toolchain. Build outputs stay under ignored `target/`; no source copy of temporary runners/logs is needed for publication.

## 5. Preservation and evidence integrity

The original checkout remains on `codex/agent-skills-setup` at `fd5a25e17a5c7bb0a6d6dccc729e3ed172321ae0`, with its same intentional changes. All **78** original tracked/protected file fingerprints match the implementation baseline, including content, size, modification time and mode. Its Prototype 006 report retains SHA-256:

`327485f986260c8612b7fc27b58693546ab990763645c7948e7862fc327b5325`

Additional read-only evidence verification found:

- All **19** tracked benchmark artifacts match their committed versions in both checkouts.
- All **10** compressed archives, totaling **2,523,103 bytes**, decompress exactly to preserved local JSON.
- The separate historical **78-file Prototype 005 private manifest**, **21** historical provider-source hashes, **78** unique provider capture hashes, **2** preserved release binaries and **4** completion-verification artifacts match their recorded checks.
- **225** private evidence files remain ignored and untracked; all three private manifests retain restricted group/world access.
- For Prototype 004 raw captures without saved per-capture hashes, only **88 recorded-size checks** could be verified. This does **not** establish historical byte identity for those captures. No missing hashes or measurements were reconstructed.

No original measurements, hashes, recovery records, archive contents or reports were rewritten. Private audit details and local test logs remain outside the publication set. Evidence limitations are preserved rather than inferred away.

## 6. DCO, Git state and exclusions

Decision 41 and repository instructions require DCO signoffs for any future contribution. Configured author name/email are present and match the previous approved contribution identity; no identity configuration was changed or personal email copied into this report. A later authorized commit must use `git commit -s`, and its final trailer and contribution range must then be verified. Passing checker tests is not signoff verification of a nonexistent commit.

Branch: **`codex/alpha1-prototype007`**. HEAD/base: **`52f92916430d3e515fa39477802106fef4c3620c`**. Working tree: one modified tracked file (`src/lib.rs`) and six untracked files, exactly the manifest above. The index is empty. No new commit, push, PR, merge, rebase, history rewrite, upstream change or branch-protection change occurred. Remote state was not refreshed during this local documentation review; publication must fetch and verify it before proceeding.

Exclusions remain: original checkout/agent-skills changes, private evidence, generated outputs, hardware measurement, production deployment, daemon/API/GUI/database work, adaptive polling, alarms, controls and protective shutdowns. Prototype 008 has not begun.

## 7. Recommendation and remaining limits

**Recommend publication approval for the exact seven-file manifest. No scoped technical blocker was found.** The accepted cache limitations remain unresolved: read guards may block/starve publication; no hard byte/RSS, cancellation, real-time or suspend-detection guarantee exists; physical sensor age is unknown; poisoning fails closed. Live deployment and performance were not validated.

The next authorization should permit only staging this manifest, one DCO-signed contribution, a normal feature-branch push and one PR targeting `main`, after fresh remote/ancestry verification and followed by inspection of final-HEAD hosted CI. It must not imply merge permission or Prototype 008 authorization. Any source change, manifest expansion, unexpected divergence or failed required check should trigger a new review.

## Revision history

| Date | Change | Status |
| --- | --- | --- |
| 2026-11-17 | Recorded acceptance, verified the seven-file publication set, reran deterministic checks and audited preservation. | Ready for publication approval; no publication authorized |

## HANDOFF TO CHATGPT

Chris formally accepted **Prototype 007: Shared In-Memory Sensor Snapshot Cache** on **November 17, 2026**, with its documented limitations. The milestone now records **Accepted by Chris**. This review is complete and the seven-file change set is locally ready for publication approval. Acceptance does not authorize production deployment, daemon/API/GUI work, hardware controls, or Prototype 008. Decision 43 remains Pending.

This task changed only the acceptance/milestone documentation and added the publication-readiness report. Source and tests are unchanged from the accepted implementation. The exact proposed publication manifest is:

- `src/lib.rs`
- `src/snapshot_cache.rs`
- `src/snapshot_cache/tests.rs`
- `tests/snapshot_cache.rs`
- `docs/milestones/007-shared-snapshot-cache-proposal.md`
- `docs/milestones/007-shared-snapshot-cache.md`
- `docs/milestones/007-publication-readiness.md`

The cache preserves one acquisition owner, concurrent borrowed read handles, atomic complete publications, explicit startup/failure/historical/recovery states, raw readings, identities and diagnostics. Existing acquisition, CLI, sampling and JSON contracts remain intact. The installed runtime remains Rust-only. No dependency, lockfile, workflow or architectural-decision change was needed.

Fresh deterministic verification passed on **Rust 1.85.0 and 1.100.0**: formatting, requested Clippy, **76 executable tests and two compile-fail doctests per toolchain**, with zero failures or ignored tests. Cache-target Clippy and the Rust 1.85 release build also passed. The strict Python suite passed **153 tests without skips**, and all **six DCO-checker tests** passed. Permission assertions ran without conditional skips. No hardware measurements or benchmark matrices ran.

The supplemental all-target Clippy command still fails on two pre-existing permission literals in unchanged hwmon/identity tests. This is separate baseline cleanup, not a cache defect or a failure of the configured CI command. No fix or lint suppression was added. Hosted CI and any future contribution signoff remain unverified because nothing has been committed or published.

Both worktrees are preserved. All 78 original checkout fingerprints remain unchanged, including the intentional Prototype 006 report and agent-skills work. Historical benchmark archives and private evidence remain excluded from the publication manifest and unchanged within the preservation audit's stated coverage. No credentials, private home paths or sensitive hardware evidence were found in the seven reviewed text files.

The feature branch remains **`codex/alpha1-prototype007`**, HEAD **`52f92916430d3e515fa39477802106fef4c3620c`**. Its review state is one modified tracked file and six untracked files; nothing is staged. No commit, push, PR, merge or history rewrite occurred.

Accepted limitations remain unresolved: reader-held locks can block publication; retention is not a fixed RSS bound; physical sensor age, automatic suspend detection, cancellation and real-time guarantees are absent. The next authorization should cover only staging the seven reviewed files, a DCO-signed contribution, normal branch push and one PR to main, after fresh remote verification and followed by hosted CI inspection. No merge or next-milestone implementation is recommended without separate approval.
