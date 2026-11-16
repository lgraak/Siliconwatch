# Milestone 001 — Alpha 1, Prototype 001: hwmon discovery

**Review, closeout, and final acceptance date:** 2026-11-16

**Status:** Complete — accepted by Chris for Alpha 1 development. This is an experimental read-only snapshot prototype, not production monitoring or validated protection.

## 1. Milestone summary

Prove read-only discovery and display of actual Linux hardware sensors before implementing a daemon, API, or GUI. The existing Rust snapshot CLI meets that objective. The initial review established permanent report requirements in [AGENTS.md](../../AGENTS.md). This closeout fixes the accepted review's diagnostic issue, adds deterministic regressions, and reruns all five required checks. Prototype 002 remains a proposal only.

Finalization reviewed the complete change set, reran all five checks, recorded Chris's acceptance, and prepared the authorized local DCO-signed checkpoint. No Prototype 002 implementation or additional application changes were made during finalization.

## 2. Features and implementation decisions

- Discover numbered hwmon directories and their input attributes, including symlinked devices and sparse channels.
- Read optional labels and standard temperature, fan, and voltage inputs. Convert millidegrees Celsius and millivolts by dividing by 1,000; RPM is unchanged.
- Preserve trimmed raw text, numeric values, source paths, and resolved device directories.
- Report per-channel parsing, availability, permission, unsupported-input, and fault statuses. Read optional enable/fault flags without changing them.
- Provide a terminal table and experimental JSON with `schema_version: 1`; retain the one-shot commands `cargo run -- sensors` and `cargo run -- sensors --json`.
- Use `--hwmon-root PATH` for fixtures. Escape terminal control characters and buffer JSON serialization to avoid partial output on serialization failures.
- Keep CLI parsing and fixtures in the standard library. Direct dependencies are `serde` and `serde_json`; a lockfile is present. No dependencies were added during this review.

**Closeout correction:** `Unsupported` previously conflated a readable unknown input family with an unsupported I/O operation when deciding whether flags could replace a diagnostic. Optional flags could overwrite an input-read failure or an earlier unsupported flag-read failure. A small boolean now records whether an input failure or first flag problem already owns the primary diagnostic. Readable unknown families remain eligible for disabled/fault qualification; later malformed or unreadable flags still produce warnings.

A private text-reader callback permits deterministic I/O-error injection through the real parsing and flag logic. Production still uses `fs::read_to_string`. No public API, JSON schema, status variants, unit conversions, raw-reading fields, source metadata, flag order, or optional-file behavior changed. No unrelated refactoring or dependencies were introduced.

## 3. Important files

| Area | Files |
| --- | --- |
| Reader and CLI | [src/hwmon.rs](../../src/hwmon.rs), [src/main.rs](../../src/main.rs), [src/lib.rs](../../src/lib.rs) |
| Build | [Cargo.toml](../../Cargo.toml), [Cargo.lock](../../Cargo.lock), [.gitignore](../../.gitignore) |
| Tests | [tests/hwmon.rs](../../tests/hwmon.rs), [tests/cli.rs](../../tests/cli.rs), [fixture helper](../../tests/common/mod.rs) |
| Usage | [README.md](../../README.md), [prototype notes](../PROTOTYPE_001.md) |
| Permanent handoff process | [AGENTS.md](../../AGENTS.md), this report |

The initial review changed only `AGENTS.md` and this report. The closeout changed only `src/hwmon.rs`, `tests/hwmon.rs`, and this report. Existing uncommitted work, AGENTS.md, dependencies, license, artwork, and design decisions were preserved.

The checkpoint includes all 13 files listed above, including this report. README changes belong to Prototype 001: they replace planning-only status with the implemented CLI and add usage instructions. Finalization changed only this report and the next-step link in the prototype notes. Full-file review and targeted sensitive-content checks found no secrets, personal machine identifiers, generated binaries, hardware dumps, or temporary files in the checkpoint set. Cargo's generated lockfile is intentionally included for reproducibility; `/target/` is ignored. License, artwork, and the accepted design-decision record remain unchanged.

## 4. Actual verification results

Rerun for final acceptance on 2026-11-16 with Rust/Cargo 1.100.0 as the normal user, UID 1000:

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Passed, exit 0 |
| `cargo clippy -- -D warnings` | Passed, exit 0 |
| `cargo test` | 18 passed, 0 failed, 0 ignored; no conditional permission skip |
| `cargo run -- sensors` | Passed; readable live table, exit 0 |
| `cargo run -- sensors --json` | Passed; stdout independently parsed as JSON, exit 0 |
| Read-only access review | Application filesystem calls perform reads, metadata queries, and enumeration; no sysfs write, privilege escalation, module-loading, or shutdown path found |
| Change hygiene | Full working-tree review, whitespace checks (including untracked files), and local Markdown link checks passed; all 13 checkpoint files belong to Prototype 001 and its handoff process |

The 18 tests comprise three unit tests, six CLI tests, and nine reader/fixture tests. Existing coverage includes conversions, negative temperatures, zero RPM, malformed/overflowing inputs, missing files/root, symlinks, labels, flags, permission denial, JSON escaping, and non-UTF-8 path errors. Fixture writes are confined to temporary directories. Test output capture was disabled with `RUST_TEST_NOCAPTURE=1` to confirm the permission test actually exercised its assertion.

Three new regression tests cover 36 combinations: 25 input-error/flag combinations, six earlier-unsupported-flag/later-flag combinations, and five filesystem cases covering malformed input, disabled/faulted readable inputs, unknown families, and inactive flags. Assertions cover the primary status and message, retained raw values/conversions/metadata, and secondary warnings. Read-error cases include unsupported operations, permission denial, missing input, invalid data, and other I/O errors.

Before the correction, `cargo test --lib` reproduced both overwrite bugs: one existing unit test passed and both new unit tests failed. After the correction all three unit tests passed; the final complete suite passed all 18. These injected I/O failures are deterministic regressions, not a claim of reproducing those errors on live hardware.

No required check was blocked. Rust 1.85 compatibility, other distributions/hardware, real hotplug races, denied device directories, actual filesystem permission denial on optional flags, multi-device ordering, and performance budgets were not independently tested. Optional-flag permission errors were tested through injection; input permission denial was also exercised using a real mode-000 fixture.

## 5. Hardware validation

- Platform: CachyOS, kernel `7.3.4-1-cachyos`, x86-64; Ryzen AI 9 HX 370, Radeon 890M, and NVMe storage.
- Detected hwmon names: `acpitz`, `nvme`, `amdgpu`, `k10temp`, `iwlwifi_1`, `it8613`, and `hidpp_battery_1`. The battery device had no input attributes.
- Discovered 24 inputs: 10 temperature, 10 voltage, two fan, one frequency, and one power input.
- Results: 22 `ok`, two `unsupported`, zero discovery warnings, and no permission-denied channels in the live snapshot.
- Unsupported GPU inputs: `freq1_input` and `power1_input`; raw data remains available in JSON.

Representative readings from the terminal run:

```text
DEVICE   SENSOR        VALUE  STATUS
nvme     Composite   43.9 °C  ok
amdgpu   edge        43.0 °C  ok
k10temp  Tctl        47.8 °C  ok
it8613   fan2_input     0 RPM  ok
it8613   in0_input   2.805 V  ok
```

Readings are samples, not calibrated ground truth. IT8613 again reported questionable -1°C temperatures, repeated voltages, and zero fan speeds; AMD voltage channels reported zero. These observations do not establish physical failure. `ok` means read and parsed, not physically validated.

## 6. Known bugs, limitations, and risks

**Resolved diagnostic bug:** the input/flag precedence issue identified in the initial review was reproduced with deterministic injected I/O errors and corrected, including the related earlier-flag overwrite case. Both regressions fail with the old logic and pass with the fix. No additional confirmed bug was found in this focused closeout; the unsupported-I/O condition was not observed on live hardware.

**Known limitations:** exceptional thermistor conversions, board voltage scaling, physical validation, and other input-family conversions are absent. Sampling is sequential and non-atomic. There are no timestamps, stable sensor IDs, polling cache, service API, daemon, GUI, plugins, database, alarms, or protective actions. Snapshot paths must be UTF-8 for JSON; otherwise it returns an error object. JSON exposes paths/labels and is not a privacy-redacted report.

**Unverified risks:** slow driver reads may delay a scan; CPU, memory, and frame-time overhead have not been benchmarked. Re-enumeration, hardware replacement, and different driver semantics may invalidate identity assumptions. Tests cannot establish physical sensor accuracy.

## 7. Architecture and design review

The prototype follows Rust/Linux-first and read-only constraints. The explicitly requested telemetry proof precedes the accepted daemon/API direction; this is a scoped sequencing exception, not an architectural reversal. JSON is experimental and does not implement the versioned local service API. Decision 43 remains pending. No accepted decision was changed, and no speculative provider framework or persistence layer was introduced.

The current canonical `device_path` still includes `hwmonN`, so it is useful provenance but not persistent identity. Resolving a parent device can improve stability, but paths may also contain volatile NVMe, thermal-zone, USB, or HID numbering. There is no evidence that the overall architecture is impractical. The closeout adds only diagnostic state and a private test seam; it makes no architectural revision. The next proposal is limited to source identity and collision semantics.

## 8. Git status

- Branch: `codex/alpha1-prototype001`.
- Chris authorized one local checkpoint, with message `feat: complete alpha1 hwmon prototype`, using `git commit -s`.
- Parent commit: `61043906faddb1a05e8182ebf41eb2b008c8ce2d`, the prior signed-off documentation commit.
- This report is prepared for the checkpoint itself. At preparation, README is modified and the other 12 reviewed files are untracked; no unrelated changes or staged files are present. The final chat handoff supplies the resulting SHA and verified post-commit working-tree status.
- The checkpoint is the commit introducing this report. After committing, resolve its immutable SHA with `git log --diff-filter=A -1 --format=%H -- docs/milestones/001-hwmon-prototype.md`; a commit cannot embed its own final hash.
- No push, merge, or pull request is authorized or performed. Later Git operations still require explicit authorization and DCO signoff.

## 9. Recommended next milestone — proposal only

**Alpha 1, Prototype 002: Stable Sensor Source Identity.** The diagnostic prerequisite is now resolved. No additional feature prerequisite is evident; Chris should approve the identity and ambiguity contract before implementation.

**Objective and scope:** extend the one-shot reader with versioned source identity based on adequate parent-device evidence, provider identity, and input attribute. Preserve original paths and expose supporting identity evidence. Distinguish insufficient evidence from collisions; never silently merge or discard readings. Display labels and volatile hwmon indices must not be authoritative identity. Do not add a UUID registry, database, topology-reconciliation framework, or protective features.

**Implementation approach, subject to review:** keep identity derivation alongside the existing reader, using a small structured result separate from reading `status`. First derive versioned candidate keys from supported evidence; then detect duplicate candidates across the complete snapshot. Mark all members of a collision ambiguous instead of choosing a first-seen winner or adding order-dependent suffixes. Preserve every sensor and its provenance. Document the supported anchors and experimental JSON change; use existing dependencies where practical.

**Explicit exclusions:** polling scheduler, sensor cache, background daemon, D-Bus or any other service API, SQLite or other persistence, GUI, hardware control, and protective shutdowns. No dependency requiring them has been identified. Any unavoidable dependency change would require Chris's separate approval.

**Acceptance criteria and expected tests:**

1. Changing only `hwmonN` registration preserves identity when the parent anchor, provider, and channel are unchanged; separate fixture scans demonstrate this.
2. Label changes leave identity unchanged. Identical labels on distinct devices, and distinct channels/providers, remain distinguishable.
3. Missing, unreadable, or insufficient anchors explicitly lack a confirmed stable identity; fixture tests cover each fallback.
4. Colliding candidate identities remain visible and explicitly ambiguous. Enumeration-order permutations must not select a winner or silently merge readings.
5. Existing raw readings, source metadata, statuses, one-shot commands, and read-only access remain intact. Document/version experimental JSON changes and pass formatting, Clippy, and all regression tests.
6. Inspect available hardware anchors read-only and report the limits of the evidence; do not claim reboot/replacement stability without testing it.

**Risks and dependencies:** agree on identity evidence, encoding/versioning, and collision/fallback representation before implementation. A parent path can retain volatile components; replacement hardware at the same topology can be indistinguishable. Stability under hwmon renumbering is a limited source-identity contract, not universal persistent physical identity or full completion of Decision 10. Decision 43 remains pending and does not block this one-shot reader. Prototype 002 has not begun.

## Review history

| Date | Stage | Outcome |
| --- | --- | --- |
| 2026-11-16 | Initial milestone review | Established AGENTS.md handoffs; 15 tests and live commands passed. Identified optional-flag diagnostic overwrite. Identity plus polling/cache was a proposal only. |
| 2026-11-16 | Prototype 001 closeout | Reproduced and corrected input and earlier-flag diagnostic overwrites; 18 tests and all required checks passed. Chris narrowed the next proposal to identity/collisions only, superseding the earlier polling/cache proposal. |
| 2026-11-16 | Final acceptance and checkpoint preparation | Chris accepted Prototype 001 for Alpha 1 development and authorized one local DCO-signed checkpoint. All five required commands passed again, including all 18 tests. Reviewed all 13 checkpoint files; Prototype 002 remains unimplemented. |

## HANDOFF TO CHATGPT

SiliconWatch Alpha 1, Prototype 001 is complete and accepted by Chris for Alpha 1 development. It proves read-only Linux hwmon discovery using a Rust snapshot CLI with terminal and JSON output. Temperatures, voltages, fans, labels, raw values, source metadata, and per-sensor errors are supported. This acceptance does not establish production monitoring or protective-action readiness. No accepted architectural decision was changed; the only direct dependencies remain serde and serde_json.

The diagnostic precedence correction preserves input failures and the first optional-flag problem, including unsupported I/O errors. Later flag errors remain warnings, and readable unknown families can still receive disabled/fault qualification. Raw readings and existing status semantics remain intact. Deterministic injected-error regressions previously failed with the old logic and now pass; three added tests cover 36 combinations.

Final verification on Rust/Cargo 1.100.0 passed `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo run -- sensors`, and `cargo run -- sensors --json`. All 18 tests passed, with zero failures, ignored tests, or conditional skips. Both precedence regressions passed. The input-permission fixture exercised real denial as UID 1000, and live JSON was independently parsed. No required check was blocked.

CachyOS kernel 7.3.4-1-cachyos on AMD Ryzen/Radeon hardware exposed 24 inputs: 10 temperatures, 10 voltages, two fans, and two unsupported GPU frequency/power inputs. Twenty-two parsed successfully, with no warnings or permission failures. Representative readings were NVMe 43.9°C, GPU 43.0°C, and CPU 47.8°C. IT8613 values remain questionable; `ok` means parsed, not physically validated. Other hardware, minimum Rust 1.85, hotplug races, performance budgets, calibration, and persistent identity remain unverified or unimplemented. No hardware settings were changed.

The reviewed checkpoint set contains 13 files: README.md, .gitignore, AGENTS.md, Cargo.toml, Cargo.lock, docs/PROTOTYPE_001.md, docs/milestones/001-hwmon-prototype.md, src/lib.rs, src/main.rs, src/hwmon.rs, tests/cli.rs, tests/hwmon.rs, and tests/common/mod.rs. README changes belong to Prototype 001. No secrets, sensitive machine identifiers, generated binaries, temporary outputs, or unrelated changes were found in this set; build output is ignored.

Chris authorized one local DCO-signed checkpoint on codex/alpha1-prototype001 with message `feat: complete alpha1 hwmon prototype`. This report is prepared for that commit; its parent is 61043906faddb1a05e8182ebf41eb2b008c8ce2d. The final chat handoff records the resulting SHA and post-commit working-tree status. The checkpoint can also be identified with `git log --diff-filter=A -1 --format=%H -- docs/milestones/001-hwmon-prototype.md`. No push, merge, or pull request is authorized or performed.

Recommended next milestone: Alpha 1, Prototype 002, Stable Sensor Source Identity. Derive versioned candidates from supported hardware/provider/channel evidence, keep labels separate, explicitly report missing evidence, and mark collisions without selecting an enumeration-order winner. Acceptance requires fixture tests for hwmon renumbering, duplicate or changed labels, missing/unreadable anchors, collisions, and order independence while preserving current readings and passing existing checks. Parent paths may retain volatile components; do not promise universal identity across reboot or replacement. Polling scheduler, sensor cache, background daemon, D-Bus API, SQLite, GUI, hardware control, and protective shutdowns are excluded. Chris's remaining decision is approval or revision of this identity/ambiguity contract before implementation. Decision 43 remains pending. Prototype 002 has not begun; stop for review.
