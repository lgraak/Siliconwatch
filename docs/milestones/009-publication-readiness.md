# Alpha 1, Prototype 009: Publication Readiness

Review date: **November 17, 2026**.
Milestone status: **Accepted by Chris — November 17, 2026**.
Readiness status: **Complete; ready for separate publication approval**, with the
limitations below. No confirmed security or correctness publication blocker was
identified within this review's scope. This is not production/security certification.

## 1. Acceptance and authorized work

Chris formally accepts the implemented **local read-only Unix-socket API** as
technically complete, including its documented security, lifecycle and freshness
limitations. This applies only to the current-user, peer-verified, opt-in API with
`get_info` and `get_snapshot` over the existing acquisition cache. Acceptance does
not authorize production deployment, multiuser/remote access, writes, privileged
execution or hardware controls.

This review authorized only acceptance documentation, deterministic verification,
preservation and publication-readiness work. Nothing was staged, committed,
pushed, opened as a PR, merged or deployed. GUI and Prototype 010 work were not
started. No hardware measurements, benchmark matrices, extended stress tests or
historical-outlier investigation were performed.

The [milestone report](009-local-readonly-api.md) now has a dated acceptance entry;
its original implementation-closeout sections, results and handoff are retained
as historical context. README and the [public API contract](../api/local-v1.md)
status now reflect acceptance. One private home-directory prefix was removed
from the public milestone's worktree locator; its exact path remains in the
external continuation. No other original evidence or recommendation was rewritten.

The [approved proposal](009-local-readonly-api-proposal.md) remains byte-for-byte
unchanged as historical planning. Its earlier Pending/awaiting-approval statements
are historical, not a reversal of Chris's subsequent approval. Proposal SHA-256:
`bf6eff85ec7309cc43b3d277a97ff280782ccdacbc54321ecc5f16b2d8d35975`.
[Decisions 5 and 43](../DESIGN_DECISIONS.md) retain the approved narrow Alpha 1
policy unchanged from implementation completion; broader policy remains pending.

## 2. Repository and exact intended publication manifest

Worktree directory: `SiliconWatch-prototype009`.
Branch: **codex/alpha1-prototype009**.
HEAD/base: **231034f2822b9c91e8781c20579729374328280d**.
Its parents are **28d7177cc334e357646023e5a2292b0cc434e0a7** and
**0df9ac1e342b617748bf65ca8ac38e0d67636461**. No ancestry or history was changed.
The index is empty; there are no unresolved conflicts or upstream. No new
contribution commit or Prototype 009 PR was created.

At entry, all 22 implementation files matched the completion hash manifest, with
no unexpected changes. Since that check, only README, the public API status and
the milestone acceptance/privacy text changed, and this report was added. The
other **19 original manifest entries remain byte-identical**, including all Rust
source/tests, Cargo files, schema, Decisions and proposal. There is no functional
change during readiness review.

The revised intended publication set is exactly **23 files** (eight modified
tracked files and fifteen untracked review files; none staged):

1. `Cargo.toml`
2. `Cargo.lock`
3. `README.md`
4. `docs/DESIGN_DECISIONS.md`
5. `src/lib.rs`
6. `src/main.rs`
7. `src/daemon.rs`
8. `src/snapshot_cache.rs`
9. `src/daemon/api.rs`
10. `src/daemon/api/tests.rs`
11. `src/daemon/api/output_tests.rs`
12. `src/local_api.rs`
13. `src/local_api/os.rs`
14. `src/local_api/tests.rs`
15. `src/local_api/wire.rs`
16. `src/local_api/wire/tests.rs`
17. `src/snapshot_cache/api_tests.rs`
18. `tests/local_api_cli.rs`
19. `docs/api/local-v1.md`
20. `docs/api/local-v1.schema.json`
21. `docs/milestones/009-local-readonly-api.md`
22. `docs/milestones/009-local-readonly-api-proposal.md`
23. `docs/milestones/009-publication-readiness.md`

File 22 is the pre-existing historical proposal; file 23 is the new readiness
report. No private captures, benchmark evidence, binaries, temporary verification
files, generated output or agent-skills setup are included. Ignored local build
outputs remain outside the manifest. All 73 tracked base files outside the eight
intended tracked changes remain unchanged. Final scope includes both tracked and
untracked files, not merely `git diff` output.

The complete final SHA-256 manifest is external at
`/tmp/siliconwatch009-readiness/final-files.json`; the final audit is
`/tmp/siliconwatch009-readiness/final-audit.json`. Keeping hashes external avoids a
self-referential report hash. These are the concrete artifacts to reverify before
any later authorized publication. Remote branch/PR state and hosted results were
not refreshed or claimed by this local review; publication must fetch and verify
current remote state separately.

## 3. Fresh deterministic verification

All commands ran from the Prototype 009 worktree on the host as the normal
non-root user. Fixture socket paths were test-owned and temporary. No production
service, privileged account configuration or hardware read was needed for these
checks. No source changed between the accepted implementation and fresh checks.

Exact versions:

- Rust **1.85.0** (`4d91de4e4`, 2025-02-17), Cargo **1.85.0** (`d73d2caf9`, 2024-12-31), LLVM 19.1.7.
- Installed stable Rust **1.100.0** (`0ef1bc386`, 2026-11-09), Cargo **1.100.0** (`3acc79c0c`, 2026-11-05), LLVM 23.1.5.
- Both target `x86_64-unknown-linux-gnu`; Python **3.14.10**, CachyOS Linux kernel `7.3.4-1-cachyos`.

Each command below ran with **T=1.85.0** and separately **T=stable**:

| Command | Rust 1.85.0 | Installed stable |
| --- | --- | --- |
| `cargo +T fmt --check` | exit 0 | exit 0 |
| `cargo +T clippy --locked --offline -- -D warnings` | exit 0 | exit 0 |
| `RUST_TEST_NOCAPTURE=1 cargo +T test --locked --offline` | exit 0; 166 tests + 2 doctests | exit 0; 166 tests + 2 doctests |
| `cargo +T build --release --locked --offline --target-dir target/provider-latency` | exit 0 | exit 0 |
| `bash /tmp/siliconwatch008-readiness/checks/strict-python-ci.sh` after that release build | exit 0; 153 tests | exit 0; 153 tests |
| `python3 -B -m unittest discover -s .github/scripts -p 'test_*.py' -v` | exit 0; 6 tests | exit 0; 6 tests |

These fresh results exactly reproduce the reported executable counts: library
100, main 2, CLI 7, daemon CLI 7, hwmon 9, identity 12, local API CLI 14, sampling
CLI 8 and cache integration 7, totaling **166**. Two compile-fail doctests pass by
rejecting invalid ownership/borrowing. Every suite has **zero failures, ignored
tests or skips**. Captured output contains no historical conditional-permission
skip notices. Expected panic/error diagnostics from fault-injection and compile-
fail tests do not represent failing tests.

The strict Python script is the unchanged CI fixture step from
`.github/workflows/alpha1.yml`: it verifies binary availability, Python version,
required discovery/modules/CLI cases and rejects all skips. It ran after each
respective release build. Release SHA-256 values remained:

- Rust 1.85.0: `0ac172511c5ffdcb83cfc2b2b331d3cc3d233e1baae7d7ea81a9d88cb3c3b4b2`.
- Stable: `be558a26b8babb18c78a35b60b304d1f3c6178a503067b17be8ca6fa71ea271f`.

The full fresh test runs include all 14 safe isolated API process fixtures and
unit tests for real socket backpressure/publication, strict framing, method and
version rejection, same-UID credentials, namespace modes, stale/refused endpoints,
replacement-safe cleanup, no extra acquisition, historical failure/recovery,
SIGINT/SIGTERM with blocked fixture acquisition, startup failure, and real signal
iterator wake on API normal/fatal/panic exit. Each created fixture process is
reaped; final inspection found no remaining worktree fixture executable process.

**Real cross-UID process isolation remains unverified.** Actual same-UID kernel
credentials, deliberately mismatching expected UID and injected lookup failures
passed. No privilege-switch account or production configuration was created.
Other architectures/distributions and non-Linux runtimes were not exercised.

### Supplemental all-target Clippy

`cargo +T clippy --locked --offline --all-targets -- -D warnings` exited **101**
on each toolchain, reporting the same two pre-existing
`clippy::non_octal_unix_permissions` findings:

- `tests/hwmon.rs:217`: `fs::Permissions::from_mode(0)`.
- `tests/identity.rs:368`: `fs::Permissions::from_mode(0)`.

Both files remain unchanged. These findings are separate from passing configured
Clippy and are not silently waived or fixed as part of this scope.

## 4. Security, correctness and dependency review

Focused manual review followed changed behavior into the existing daemon/cache,
sensor and identity contracts. Separate review passes covered transport/OS/
lifecycle, wire/schema/semantics, and dependency/privacy/preservation. Reviewers
included prior implementers; this is not an independent penetration test, a
dedicated security-plugin scan or an exhaustive audit of unchanged providers.
No confirmed blocker was identified in the reviewed paths.

| Area | Verified source behavior and evidence |
| --- | --- |
| Architecture and read-only scope | API is opt-in, exposes only two typed versioned methods and receives no owner/scan callback; no TCP, D-Bus, mutation, configuration or control path was added. Existing CLI/no-API behavior passes regressions. |
| Runtime namespace | Nonzero matching real/effective UIDs; validated absolute real-directory ancestry with trusted ownership/no other-user write; narrow root-owned sticky `/tmp` exception; private 0700 root/child and verified 0600 socket; overlong path rejected without truncation. |
| Peer/entropy boundary | Correctly sized kernel credentials checked before parsing; lookup failure/mismatch closes silently. Public 128-bit instance token uses bounded nonblocking kernel entropy, with checked partial/interrupted results and no weak fallback. |
| Existing paths and cleanup | Exclusive child creation refuses all existing/stale objects. Device/inode/type/UID checks precede deletion of only the owned socket and empty child. Replacements/uncertain identity are left intact; normal cleanup failures propagate. |
| Parser and output bounds | Typed duplicate/unknown-field rejection; first LF frame only; 4096-byte request/1 MiB response caps include LF. Collection/text preflight, finite/path checks and capped whole-response encoding preserve data or return an error rather than truncate. |
| Deadlines and slow peers | One active stream plus cancellation clone; absolute request/response budgets survive partial progress and interruption; late read completion is rechecked. Cancellation wakes active socket/idle wait. Kernel backlog and scheduler stalls are not hard-bounded. |
| Cache and concurrency | Cache→control try-lock order; contention distinct from poison; lifecycle dropped after assessment; bounded borrowed encoding completes and releases cache before socket I/O. Actual backpressure regression proves publication can complete while response output remains blocked. |
| Lifecycle/faults | Stop closes admission, cancels/joins API and cleans endpoint before acquisition join; final Stopped follows all created worker joins. API fatal exit/panic wakes signal coordination. Spawn-failure paths join already-created workers. Previously admitted acquisition may finish after Stopping. |
| Telemetry truth | Full existing Snapshot/identity serialization retains exact raw i64/text, separate canonical values/units, paths, issues and diagnostics; API versions remain independent. Acquisition failure is data, and historical success retains its own attempt. |
| Freshness | Service-current is the complete lifecycle/producer/latest/role/age conjunction; stale iff upper age exceeds threshold. Physical age and freshness on receipt remain unknown; per-row quality remains independent. |

Security assumptions remain explicit: trusted local filesystem behavior and
namespace control, with hostile same-UID/root actors excluded. Credentials identify
the connection-time principal, not an executable; authorized descriptors can be
passed onward. Identity-check cleanup is not atomic defense against hostile
same-UID replacement. A crash can require manual stale recovery.

### Direct libc and lockfile

The accepted Linux-only direct declaration remains `libc = "0.2.191"`; the locked
version was already present transitively. The sole lockfile relationship change
from base is adding libc to SiliconWatch's own dependency list. All **17 external
package records**, versions and checksums are unchanged, with no package added
or removed. Resolved `default,std` features match Prototype 008.

The cached crate declares Rust 1.65 and `MIT OR Apache-2.0`; metadata and both
license texts agree. Its archive checksum matches the lock, and **452 cached
source files** match the locked archive. The small production unsafe boundary
for UID, peer credentials and entropy checks sizes/return values and bounds its
retries. Fresh Rust 1.85 tests/builds establish actual minimum-toolchain
compatibility for this implementation. No package was installed or changed.
This is package/feature/license integrity review, not an online advisory audit
or a claim that upstream dependencies are vulnerability-free.

## 5. Public API documentation and schema

`python3 -B /tmp/siliconwatch009-readiness/contract-audit.py` passed, exit **0**.
Fresh dependency-free validation checked:

- All **seven** complete public JSON examples, **70** local schema references,
  **15** source-struct field sets, both status enums and **10** advertised limits.
- Independently recomputed time ordering/age/currentness/history for the three
  snapshot examples; preserved raw/Celsius values and invalid-row diagnostics.
- Nine error variants, 21 original invalid-message mutations, six extra valid
  shapes and ten extra invalid shapes.
- **10,219** decimal-range/spelling checks, including 10,000 deterministic random
  comparisons, u64/u128 endpoints, overflow and malformed encodings.

**Comprehensive Draft 2020-12 metaschema validation was not executed.** No suitable
installed validator was available; none was installed or downloaded. The audit
implements the validation keywords present and compares actual source shapes,
but is not a formally validated/conformance-tested JSON Schema implementation.
This limitation is retained, not represented as a passed metaschema check.

The schema is an exact emitter-shape contract; clients may tolerate future
additive response fields as the prose states. Schema validation cannot establish
raw duplicate-key rejection, lexical integer distinctions, UTF-8 byte budgets,
clock arithmetic or peer authority. Those remain runtime/prose responsibilities.

Examples are synthetic. The diagnostic Python command is development tooling,
not an application runtime dependency. Current-user source paths and diagnostics
are intentionally available to authorized readers; this API is not an
automatically redacted public export. Celsius stays the canonical default;
Fahrenheit preferences, calibration and GUI behavior remain outside acquisition.

## 6. DCO readiness, privacy and preservation

Configured author and committer identities are valid, match each other/configured
attribution, and match the previously published author and signoff. No placeholder
was detected; identity values are omitted from public review evidence. No Git
configuration was changed and no credentials were accessed.

There is **no Prototype 009 contribution commit**, so new contribution-signoff
verification is not claimed. The checker-unit suite passed; an empty contribution
range was not invoked. A later authorized publication must use full 40-character
base and contribution SHAs, verify the exact parent and manifest, and separately
check that the final DCO trailer matches the author. The repository checker alone
does not enforce that author match. No legal-identity/ownership certification is
implied by configuration consistency.

Focused privacy review identified and removed the single home-directory prefix
from the milestone locator. No remaining credential/private-home pattern finding
was identified in the proposed files, including the final readiness report.
Public checksums, synthetic instance tokens, `/fixture` and test-owned `/tmp`
paths, aggregate previously recorded hardware results and canonical project URLs
are intentional. Pattern checks are not a universal proof about arbitrary text.
No private measurements, credentials or machine-specific source identifiers were
copied into the intended publication set.

Fresh preservation comparisons confirmed:

- Original checkout **78** baseline files and intentional state, including
  agent-skills setup and Prototype 006 report edits.
- Prototype 007 **75** baseline files and clean state; Prototype 008 **81** baseline
  files and clean state, with their existing branches/HEADs unchanged.
- All **225** private evidence files retain recorded bytes/metadata, remain ignored
  and untracked, with no additions; private manifests retain restrictive modes.
- Ten prior local branch refs unchanged; no unexpected refs or history rewrites.
- Nineteen published historical artifacts match across all four worktrees; ten
  archives match their retained original payloads; two historical binaries retain
  recorded hashes.
- All **73** protected tracked Prototype 009 base files unchanged; approved proposal
  and Decisions unchanged from readiness entry.

Previous implementation evidence and temporary review artifacts were retained;
new verification uses a separate external readiness directory. The Prototype 004
historical limitation remains: capture sizes without original hashes cannot prove
historical byte identity. No missing hashes or evidence were reconstructed.

## 7. Known limitations, blockers and recommended authorization

No confirmed implementation/security/correctness blocker remains in this scoped
local review. The two historical supplemental lint findings and unavailable
comprehensive metaschema validation are explicit limitations. Publication itself
remains **unauthorized**, and hosted final-contribution CI remains unexecuted.

Accepted runtime limits include same-UID/root and trusted-filesystem assumptions,
real cross-UID testing unverified, manual stale recovery, one-client fairness and
kernel-backlog limits, whole-response rejection over caps, and no hard shutdown
or cancellation guarantee for blocked kernel/hardware/filesystem/output work.
Physical sensor age, receipt-time freshness and persistent physical identity are
not established. Known suspend requires cache recreation/restart; automatic
suspend detection is absent. The earlier hardware smoke is historical evidence
only and was not rerun here.

Recommend one next authorization: **publish this exact reviewed 23-file manifest**.
If separately approved, first fetch/reverify remote state and frozen hashes, then
make one DCO-signed-off contribution with an author-matching trailer, normally
push the feature branch, open one PR against `main` after duplicate checking,
and inspect completed hosted results for that exact contribution HEAD. The PR
should remain unmerged pending separate merge approval. This review performs none
of those actions and grants no deployment, GUI or Prototype 010 authority.

## 8. Evidence and continuation

External readiness evidence is under `/tmp/siliconwatch009-readiness/`:
`initial-verification.json`, `initial-files.json`, `checks/results.json`,
`checks/versions.log`, per-command logs, `transport-review.md`,
`contract-review.md`, `contract-review.json`, the rerunnable `contract-audit.py`,
`preservation-review.json`, `scopefindings.md`, `final-files.json` and
`final-audit.json`. Earlier implementation evidence remains in its original
external directories. These temporary review artifacts are not publication files.

The repository's **/handoff** skill was explicitly invoked. The continuation is
saved outside every worktree at `/tmp/siliconwatch009-readiness-handoff.md`.

## HANDOFF TO CHATGPT

Chris formally accepted **Prototype 009: Local Read-Only API** as technically complete on **November 17, 2026**, with its documented limitations. This task recorded that acceptance and completed local publication-readiness review. Acceptance does not establish production readiness or authorize deployment, broader permissions or hardware controls.

The milestone now contains a dated acceptance entry that preserves its original implementation-closeout record. README and public API status reflect acceptance, and the public milestone no longer contains the local home-directory prefix. The approved proposal remains unchanged. Decisions 5 and 43 retain their narrow current-user, peer-verified, read-only Unix-socket JSON policy. No functionality, source, tests, dependencies or schema changed during readiness review.

The exact proposed publication manifest now contains **23 files**: the accepted 22-file implementation set plus `docs/milestones/009-publication-readiness.md`. Final SHA-256 hashes and verification evidence are saved outside the worktrees. Only these intended publication files differ from the base; the index is empty.

Fresh checks passed on Rust **1.85.0** and installed stable **1.100.0**: formatting, configured warnings-denied Clippy, **166 executable Rust tests plus two compile-fail doctests**, and release builds. Each release passed **153 strict Python tests without skips**; **six DCO-checker tests** passed in each run. Safe isolated socket fixtures covered permissions, protocol limits, cache isolation, historical/failure states and shutdown. Supplemental all-target Clippy reports the same two historical permission-literal findings, separately documented and unchanged.

Focused source review found no confirmed security or correctness blocker. The direct Linux `libc` dependency remains version **0.2.191**, with unchanged features and all external lockfile records preserved. DCO author/committer configuration matches prior published attribution; no new contribution signoff can be verified before an authorized commit exists. All seven public JSON examples, schema references, source field/enum/limit mappings and additional semantic/boundary checks passed. Comprehensive Draft 2020-12 metaschema validation remains unavailable and is not claimed.

Original/007/008 worktrees, agent-skills setup, intentional Prototype 006 changes, existing branches, historical archives/binaries and all **225 private evidence files** remain preserved. No new hardware measurements, benchmark matrices or outlier investigation occurred. Known limits remain same-UID/root trust, real cross-UID testing unverified, manual stale recovery, no fairness or hard shutdown guarantee, and unknown physical age/freshness at receipt.

Branch remains **codex/alpha1-prototype009** at **231034f2822b9c91e8781c20579729374328280d**, without an upstream. Nothing was staged, committed, pushed or published; no Prototype 009 PR was created, and no hosted CI result is claimed. The repository's **/handoff** skill was invoked and an external continuation saved. **Recommended next authorization: separate approval to publish the exact reviewed 23-file manifest**, followed by final-HEAD hosted verification. Merge, deployment, GUI work and Prototype 010 remain outside authorization.
