# Milestone 002 — Alpha 1, Prototype 002: Stable Sensor Source Identity

**Date:** 2026-11-16

**Status:** Complete — implemented and verified; awaiting Chris's review. This is experimental source matching, not validated physical identity or protection.

## 1. Milestone summary

Add deterministic, best-effort source identity to the accepted Prototype 001 snapshot reader, preserving its readings and diagnostic precedence. Changing only hwmon registration numbers leaves a candidate unchanged when the supported parent anchor, provider name, channel, and measurement semantics are unchanged. Collisions and inadequate evidence remain explicit. Nothing is persisted and no subsequent milestone has started.

Run as an ordinary user:

```bash
cargo run -- sensors
cargo run -- sensors --json
```

The existing `--hwmon-root PATH` fixture option remains available. Hardware access is read-only; no sysfs attributes are written.

## 2. Implementation details and identity rules

The new [identity module](../../src/identity.rs) collects evidence once per discovered hwmon directory, derives channel candidates, and checks ambiguity across the finished sensor snapshot. It uses the existing standard library, `serde`, and `serde_json`; no dependencies were added or removed. No provider framework, persistent registry, UUID database, hardware control, or service was introduced.

### Candidate construction

The identity schema version is **1**. A candidate is the compact JSON serialization of this fixed-order tuple, stored as a string:

```text
["siliconwatch.hwmon-source",1,provider,parent_path,input_attribute,measurement_semantics]
```

1. **Provider:** use the trimmed, nonempty hwmon `name` attribute. Missing, unreadable, empty, or control-containing names cannot generate candidates. The display fallback `hwmonN` never becomes identity evidence. The chip/provider name is not a unique device identifier or necessarily a driver name.
2. **Parent:** resolve the optional `device` symlink to a directory. Independently recognize canonical `<parent>/hwmon/hwmonN` and `<parent>/hwmonN` layouts outside the canonical scan root. These are bounded path-shape heuristics, not a universal sysfs hierarchy contract. An absent link permits the supported canonical fallback; a malformed, dangling, or unreadable link blocks a candidate rather than silently falling back.
3. **Agreement:** when both observations exist, they must resolve to the same directory. Conflicts retain both observations, leave the selected parent empty, and produce no candidate. A scan root or descendant, a non-UTF-8 parent, or any parent component exactly matching `hwmon` followed by digits is rejected as an anchor.
4. **Channel:** preserve the exact input attribute. Recognize canonical `tempN_input` and `fanN_input` for positive decimal N, and `inN_input` including zero. Leading-zero aliases and other input families have unknown/noncanonical identity semantics and receive no candidate. Their existing readings and raw data remain visible.
5. **Semantics:** respectively use `temperature_input_millidegree_celsius`, `fan_speed_input_rpm`, and `voltage_input_millivolt`. These describe the prototype's ABI interpretation, not a verified physical role or calibration.
6. **Encoding:** canonicalize paths, trim provider-edge whitespace, preserve case and exact channel spelling, and use ordered JSON fields with JSON escaping. Do not hash with process-dependent hashers, concatenate ambiguous delimiters, perform lossy path conversion, or include labels, aliases, readings, timestamps, or enumeration order.

Resolved absolute parent paths are topology evidence. They can still contain volatile `nvmeN`, `thermal_zoneN`, USB/HID identifiers, and mount-root components. No serial-number acquisition or universal hardware-ID adapter is implemented. Provider names and channel meaning can also change across drivers or kernels. These limits are consistent with the kernel's warnings about [sysfs hierarchy and device properties](https://docs.kernel.org/admin-guide/sysfs-rules.html); [hwmon names, labels, and units](https://docs.kernel.org/hwmon/sysfs-interface.html) do not establish physical sensor identity.

### Ambiguity and diagnostics

Identity status is independent of the existing reading `status`. It never changes a raw value, conversion, label, source path, or input/flag diagnostic. Multiple evidence problems are retained in `issues`; the first detected problem supplies the primary status in deterministic acquisition order.

| Identity status | Meaning | Candidate |
| --- | --- | --- |
| `best_effort` | Adequate supported evidence and no detected snapshot ambiguity | Present; conditional source candidate only |
| `insufficient_evidence` | Missing/invalid provider or parent evidence | Absent |
| `unreadable_evidence` | Evidence cannot be read/resolved or a device link is malformed | Absent |
| `conflicting_evidence` | Device-link and canonical-parent observations disagree | Absent |
| `ambiguous_channel` | Unknown/noncanonical channel, or multiple registrations share parent/provider | Absent for unknown semantics; retained for otherwise derivable candidates |
| `collision` | Two or more rows produce the same candidate | Retained on every member for diagnosis |

The collision pass marks **every** matching row; it never selects a winner, adds order-derived suffixes, merges rows, or discards data. Different canonical hwmon registrations sharing parent and provider are conservatively ambiguous even when their channel sets are disjoint. Registrations containing only unsupported input families participate in this check. Registrations with no discovered input rows do not participate. Collision takes precedence over the additional registration-ambiguity diagnostic. A non-null candidate alone is never an assurance of uniqueness or persistence.

### CLI and JSON compatibility

Experimental snapshot/error JSON **schema version 2** replaces version 1. All existing fields and exit behavior remain, with a new per-sensor `identity` object:

```json
{
  "schema_version": 1,
  "candidate": null,
  "status": "insufficient_evidence",
  "evidence": {
    "provider": "fixture-chip",
    "parent": null,
    "device_link": null,
    "canonical_parent": null,
    "channel": "temp1_input",
    "measurement": "temperature_input_millidegree_celsius"
  },
  "issues": ["no supported parent-device evidence"]
}
```

`device_link` and `canonical_parent` contain resolved observations; `parent` is the selected, agreeing anchor when available. The original `source_path` and `device_path` remain separate. The table retains its reading columns, adds `IDENTITY`, and follows the table with versioned candidates or explicit missing-candidate diagnostics. JSON consumers must accept schema 2 and inspect identity status. Snapshot paths still require UTF-8 for JSON. Identity metadata, like existing source paths, is not a privacy-redacted diagnostic export.

### Files changed

- [src/identity.rs](../../src/identity.rs): new representation, evidence reading, candidate encoding, and ambiguity checks.
- [src/hwmon.rs](../../src/hwmon.rs), [src/lib.rs](../../src/lib.rs), [src/main.rs](../../src/main.rs): integration, JSON version, and terminal identity output.
- [tests/identity.rs](../../tests/identity.rs): twelve new isolated fixture tests.
- [tests/cli.rs](../../tests/cli.rs), [tests/hwmon.rs](../../tests/hwmon.rs): schema expectations and CLI/JSON identity checks; all original tests retained.
- [README.md](../../README.md), [Cargo.toml](../../Cargo.toml), [Prototype 001 notes](../PROTOTYPE_001.md), this report: current status, package description, historical-document pointer, and milestone handoff. Cargo.lock and dependencies are unchanged.

## 3. Verification results

Executed on 2026-11-16 using Rust/Cargo 1.100.0 as UID 1000:

| Check | Actual result |
| --- | --- |
| `cargo fmt --check` | Passed, exit 0 |
| `cargo clippy -- -D warnings` | Passed, exit 0 |
| `cargo test` | **31 passed, 0 failed, 0 ignored**, no conditional skips |
| `cargo run -- sensors` | Passed, readable live table and identity diagnostics, exit 0 |
| `cargo run -- sensors --json` | Passed, schema 2 and candidate tuples independently parsed/checked, exit 0 |

The suite contains 3 unit tests, 7 CLI tests, 9 original reader tests, and 12 identity fixture tests. All 18 Prototype 001 tests remain, including both diagnostic-precedence regressions; only the expected snapshot schema number changed where required. Thirteen tests were added. Permission tests actually exercised mode-000 input denial and denied anchor traversal; neither took its privileged-user skip path (`RUST_TEST_NOCAPTURE=1` exposed any skip message).

Fixtures cover canonical-directory and class-alias renumbering, changed/removed/duplicate labels, reading-value independence, structured encoding of quotes/delimiters, parent/provider/channel separation, missing and unreadable evidence, malformed/dangling links, conflicting parents, collisions, swapped enumeration order, disjoint channel sets, unsupported-only registrations, and rejection of nested scan-root anchors. An early test run had one channel-name expectation mismatch (`temp1` versus the documented full `temp1_input`); the expectation was corrected and the final suite passed.

Application filesystem operations were reviewed as read-only; fixture mutations stay in temporary directories. No required check was blocked. Real reboot, physical re-enumeration/hotplug, replacement, other systems, Rust 1.85 builds, and performance budgets were not tested. Fixture renumbering is not a substitute for those hardware tests.

## 4. Hardware validation

- CachyOS, kernel `7.3.4-1-cachyos`, x86-64; AMD Ryzen AI 9 HX 370, Radeon 890M, and NVMe storage.
- Providers: `acpitz`, `nvme`, `amdgpu`, `k10temp`, `iwlwifi_1`, `it8613`, and `hidpp_battery_1`; the battery provider has no input rows.
- 24 inputs: 10 temperatures, 10 voltages, 2 fans, 1 frequency, and 1 power.
- Reading statuses: 22 `ok`, 2 `unsupported`; zero discovery warnings or permission failures.
- Identity statuses: 22 `best_effort`, 2 `ambiguous_channel` (GPU frequency/power semantics are outside this identity version). No live collisions, missing anchors, or conflicting evidence were observed.
- All seven device links agreed with the supported canonical-parent interpretation. Three registrations use the `/hwmon/hwmonN` layout and four use direct `/hwmonN` placement. No reboot or forced hardware re-enumeration was performed.
- Examples from the final table run: NVMe 42.9°C, GPU 38.0°C, CPU 45.1°C, IT8613 fan 0 RPM and voltage 2.805 V. Questionable IT8613 temperatures/voltages and AMD zero voltages remain unvalidated. `ok` still means parsed, not physically verified.

No serials, full hardware paths, or live candidate strings are included in this report.

## 5. Architecture and design review

The implementation advances Decision 10 with limited, explicit source matching; it does not complete persistent physical identity, aliases, or protection-safe rediscovery. Existing Rust/Linux-first, read-only, least-privilege, and raw-data preservation constraints remain intact. The one-shot prototype remains the approved staging step before any daemon/API. Decision 43 remains pending. No accepted design decision changed.

The parent-layout heuristic is intentionally bounded and marked best-effort. Kernel layout changes can produce explicit conflicts or different candidates. A single small module and two snapshot passes avoid a database, general provider framework, or persistent reconciliation system. No scope deviations or new dependencies were necessary.

## 6. Known issues and risks

- No unresolved confirmed bug was found in the focused implementation review. Review found and corrected two inference/ambiguity gaps, with fixture regressions: nested scan-root aliases and unsupported-only sibling registrations.
- A candidate is stable only while its supported anchor/provider/channel/semantics remain unchanged. Hardware replacement at the same topology can reuse it; a driver can change physical channel meaning without detectable evidence. Do not bind protection based on this candidate alone.
- Evidence is read sequentially, not atomically. Concurrent removal or re-registration can make a snapshot inconsistent. No persistent history exists to detect changes between scans.
- Canonical-parent inference is a path-shape heuristic, not subsystem-validated physical evidence. Missing serial-based identity, unsupported measurement semantics, and empty-registration ambiguity remain documented limits.
- UTF-8 JSON paths, unredacted path metadata, driver-specific conversion/calibration gaps, questionable readings, and unmeasured scan overhead remain inherited limitations.

## 7. Git status

- Branch: `codex/alpha1-prototype002`, based on accepted Prototype 001 checkpoint `bed3f17d7fb03ce3e500ecf5874c257a96241815`.
- Chris authorized preserving and publishing the existing design-decisions and Prototype 001 commits together with a new DCO-signed Prototype 002 commit, `feat: add stable hwmon sensor source identities`, through one pull request to `main`.
- The Prototype 002 checkpoint is the commit introducing this report. Resolve its SHA after committing with `git log --diff-filter=A -1 --format=%H -- docs/milestones/002-stable-sensor-identity.md`. Its own final hash cannot be embedded in its contents.
- At checkpoint preparation, the 11 reviewed Prototype 002 files are unstaged/uncommitted; no unrelated changes exist. The final chat handoff records the resulting commit SHA, actual push/PR/CI results, and post-operation working-tree status.
- Artwork, license, AGENTS.md, design decisions, Cargo.lock, and the Prototype 001 milestone report were preserved. No history rewrite, force-push, direct modification of remote `main`, or PR merge is authorized.

## 8. Recommended next milestone — proposal only

**Prototype 003: bounded foreground sampling with explicit timing.** Add an opt-in finite count and interval to the existing CLI, with serial scans and explicit sample timing/failures. Preserve the one-shot default and current identity/reading semantics. Do not introduce a background service, cache, API, database, alarms, or protection.

Acceptance: deterministic tests demonstrate bounded iteration, no overlapping scans, unchanged one-shot behavior, and no reuse of an old value as a fresh success after failure. Use an injectable clock/sampler boundary only where needed to avoid sleep-based tests. Validate a short ordinary-user hardware run and report scan latency. Blocking driver reads remain a timing risk; interval requests cannot guarantee completion deadlines. Chris must approve the contract before implementation; Prototype 003 has not begun.

## Revision history

| Date | Change |
| --- | --- |
| 2026-11-16 | Implemented Prototype 002, resolved review findings, ran 31 tests and live checks, and prepared handoff for Chris. |
| 2026-11-16 | Publication preflight: scoped diff review and formatting/Clippy/31 tests passed again; publication stopped before staging or committing because required predecessor commits are absent from GitHub. |
| 2026-11-16 | Chris authorized publishing the complete existing milestone history through one feature-branch PR. Fresh fetch/ancestry review and all 31 tests passed; prepared the DCO-signed Prototype 002 checkpoint without rewriting predecessors. |

## Publication preflight — 2026-11-16

**Initial publication attempt: Blocked; superseded by the combined-history authorization below.** Implementation remains complete. Chris authorized a DCO-signed commit, feature-branch push, and pull request targeting `main`, with the explicit condition: “If the remote branch has diverged or required commits are missing from GitHub, stop and report the issue before attempting to resolve it.”

The GitHub connector confirmed that remote `main` is identical to `a14d133072bbdb48f897afea266a9f3cd8d6e0f2`, the README commit. It found no `codex/alpha1-prototype002` branch. Comparing `main` to each required local predecessor returned **GitHub API error 404: Not Found**:

- `61043906faddb1a05e8182ebf41eb2b008c8ce2d` — `docs: document SiliconWatch architectural decisions`.
- `bed3f17d7fb03ce3e500ecf5874c257a96241815` — `feat: complete alpha1 hwmon prototype`.

The successful comparison against the known README commit establishes readable repository access; these results do not establish a write-authentication failure. Write permissions were not tested. No remote-history repair, predecessor publication, staging, commit, push, merge, or pull request was attempted.

Fresh publication checks passed `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`: **31 passed, 0 failed, 0 ignored, no conditional skips**. All 11 intended files were reviewed, with no unrelated files, generated binaries, or apparent credentials found. Live hardware commands were not rerun in this publication attempt; Section 4 records the preceding implementation validation.

At that stop, Chris had not authorized prerequisite publication. No solution was applied automatically; eight modified and three untracked Prototype 002 files remained unstaged. No new commit, push, PR, or publication CI result existed for that attempt.

## Combined-history publication authorization — 2026-11-16

Chris resolved the earlier stop condition by explicitly authorizing publication of both existing predecessor commits through `codex/alpha1-prototype002`. A fresh `git fetch origin` succeeded. Remote `main` and the common ancestor remain `a14d133072bbdb48f897afea266a9f3cd8d6e0f2`; the remote feature branch is absent. Both `61043906faddb1a05e8182ebf41eb2b008c8ce2d` and `bed3f17d7fb03ce3e500ecf5874c257a96241815` are ancestors of the current branch, retain DCO signoffs, and form the expected linear history. No unexpected remote change or divergence was found.

The full combined PR contains 17 intended project files; the Prototype 002 commit contains 11 reviewed files. Fresh formatting, Clippy with warnings denied, and automated tests passed again: **31 passed, 0 failed, 0 ignored, no conditional skips**. No live hardware retest was performed during publication preparation. The authorized plan is one new DCO-signed commit, an ordinary feature-branch push including the two preserved predecessors, and one PR targeting `main`. Publication results are reported after execution in the final chat handoff; authentication, permission, or divergence failures must stop publication rather than trigger history repair.

## HANDOFF TO CHATGPT

SiliconWatch Alpha 1 Prototype 002 is implemented and verified. Chris authorized publishing the complete local milestone history through the existing codex/alpha1-prototype002 branch and one pull request targeting GitHub main. This supersedes the earlier missing-history stop condition; the accepted design-decisions and Prototype 001 commits must remain unchanged with their DCO signoffs.

A fresh fetch succeeded. Remote main and the verified common ancestor are a14d133072bbdb48f897afea266a9f3cd8d6e0f2. The preserved predecessor commits are 61043906faddb1a05e8182ebf41eb2b008c8ce2d for accepted architectural decisions and bed3f17d7fb03ce3e500ecf5874c257a96241815 for the read-only hwmon Prototype 001. Both are ancestors of the working branch and retain their signoffs. No unexpected remote change or divergent feature branch was found.

The reviewed combined change set contains 17 intended project files. The new Prototype 002 checkpoint contains 11 files covering identity construction, ambiguity handling, reader/CLI integration, tests, and documentation. Its authorized message is `feat: add stable hwmon sensor source identities`. This report is prepared for that commit; its SHA can be resolved with `git log --diff-filter=A -1 --format=%H -- docs/milestones/002-stable-sensor-identity.md`. The final chat handoff supplies the exact new SHA, actual push result, PR URL, CI status, and final working-tree state after the publication operations.

Fresh cargo fmt --check, cargo clippy -- -D warnings, and cargo test all passed. All 31 tests passed, with zero failures, ignored tests, or conditional permission skips. The original diagnostic-precedence regressions remain covered. No new dependencies were added, and review found no unrelated files, generated binaries, apparent credentials, or sensitive machine identifiers in the publication set.

Prototype 001 provides one-shot temperature, voltage, and fan telemetry with raw source data. Prototype 002 adds versioned best-effort source candidates, explicit missing/unreadable/conflicting evidence, and collision handling without silently merging readings. Experimental snapshot JSON is schema 2. No daemon, scheduler, cache, service API, GUI, database, alarm engine, hardware control, or protective shutdown exists.

Previous live validation found 24 inputs: 22 successful readings with best-effort identities and two unsupported GPU inputs with ambiguous-channel identities. Hardware was not retested during publication preparation. Reboot/re-enumeration stability, replacement safety, other hardware, Rust 1.85 compatibility, and performance budgets remain unverified; source candidates are not universal physical identities.

The remaining publication actions are the authorized local commit, ordinary branch push, and single PR, followed by remote verification. No force-push, rebase, squash, cherry-pick, direct main update, branch-protection change, or merge is authorized. Any authentication, permission, or unexpected-divergence failure must be reported honestly. Prototype 003 remains only a proposal for bounded foreground sampling with explicit timing and deterministic failure handling; it requires separate review and has not begun.
