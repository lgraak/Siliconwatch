# SiliconWatch Design Decisions

> **Status:** Design record, pre-alpha. Decisions 1–42 are accepted project direction, not claims of implemented functionality. Decision 43 is approved for Alpha 1 current-user, read-only local access; broader permissions remain pending. This document is a planning reference, not a release or security certification.

SiliconWatch is an independent Rust-based Linux hardware monitoring project inspired by the depth of HWiNFO64. The initial engineering target is an efficient headless daemon, CLI, and local API. A native GUI follows as an API client.

**Project baseline:** Linux-first, initially x86-64, with a versioned local API foundational to the headless daemon and CLI. The GUI is an optional API client. Licensing is [GNU GPLv3](../LICENSE); contribution signoffs follow Decision 41.

## Decision conventions

- Decision numbers are stable identifiers grouped by topic, not delivery order.
- **Accepted** means the product behavior has been agreed upon, not that it has been implemented or validated.
- **Pending** decisions must not be treated as implementation requirements until approved.
- Changes to accepted decisions require an explicit new entry in the revision log and an update to the affected entry; do not silently reinterpret them.
- Protective actions remain **disabled until explicitly authorized**. Notification severity, imported profiles, simulations, and escalation do not confer authorization.
- Monitoring access, configuration authority, and authorization for disruptive actions are distinct. Decision 43 approves only Alpha 1 current-user, peer-verified read access. Multiuser sharing, configuration writes and disruptive-action permissions remain pending.
- Implementation details not settled here should be recorded as proposals rather than attributed to these decisions.

## Service, interfaces, and distribution

### 1. Start at boot — Accepted

Install the system-wide monitoring daemon to start at boot by default, with prominent disclosure and an opt-out. Boot-time monitoring must not silently activate protective shutdowns.

**Why:** Server and unattended desktop alarms need coverage without a GUI session.

### 5. Local-first, remote-ready interfaces — Accepted

**Initial direction — 2026-11-16:** Implement the local API first, using D-Bus as the initial direction, while keeping interfaces suitable for future authenticated remote access.

**Why:** Headless CLI and future GUI should reuse the same sensor cache and command contracts.

**Alpha 1 policy approved by Chris — 2026-11-17:** Use an explicitly enabled pathname Unix-domain socket with versioned JSON requests/responses and the current-user read-only policy in Decision 43. The API consumes the existing shared cache and never triggers hardware acquisition. D-Bus and system-wide access are deferred. This records the approved Prototype 009 change in direction while preserving the earlier rationale; it does not authorize TCP, remote access, deployment or hardware controls.

### 6. Pull and push remote telemetry — Accepted

Support both models eventually. Implement authenticated pull first; central collection remains optional, and local alarms must continue independently.

**Why:** Both workstation inspection and a small centralized Proxmox collector should be possible.

### 7. Remote authentication — Accepted

Use mTLS for machine-to-machine connections and scoped API tokens for third-party integrations. Remote exposure is opt-in. Remote API authorization never implies permission to shut down a host.

**Why:** Hardware inventory and telemetry should not be broadly exposed on a network.

### 14. Packaging — Accepted

Develop with Cargo. Plan separate packages for the headless daemon/CLI and optional GUI, with native Debian and Arch packaging plus standalone binaries.

**Why:** Proxmox nodes should not need a desktop stack installed.

### 20. Distribution support tiers — Accepted

Primary: Debian/Proxmox and Arch/CachyOS. Secondary: Ubuntu LTS and Fedora. Other distributions receive best-effort support.

**Why:** Prioritize systems available for repeatable testing.

### 21. Architecture support — Accepted

Develop and validate x86-64 first. Keep the core portable so ARM64 builds and architecture-specific providers can follow.

**Why:** Avoid architecture assumptions in shared code without delaying the first release.

### 22. Updates — Accepted

The operating system package manager controls updates. SiliconWatch does not self-update or reboot automatically. It may optionally notify about releases and must back up configuration before migrations.

**Why:** Preserve operator control and recoverability, especially on servers.

### 24. API lifecycle — Accepted

Version interfaces and publish schemas from the outset. Support capability negotiation. Mark pre-1.0 APIs experimental and maintain backward compatibility after stabilization.

**Why:** Future GUI clients and third-party integrations need explicit compatibility expectations.

### 26. Shared service and user preferences — Accepted

Run one system-wide daemon with shared telemetry and administrative alarm policy. Keep layouts, dashboards, and UI preferences separate by user.

**Why:** Avoid duplicate hardware polling while supporting multi-user systems.

### 30. First-run experience — Accepted

Discover and show sensors automatically with safe defaults. Offer an optional GUI setup wizard and equivalent headless CLI configuration. No disruptive actions are enabled automatically.

**Why:** Work out of the box on desktops and servers without privileging one interface.

## Sensors, calculation, and data quality

### 2. Sensor discovery and confidence — Accepted

Display all discovered sensors, flag questionable readings, and require validation before any questionable source can drive disruptive protection.

**Why:** Linux hardware interfaces sometimes expose mislabeled, disconnected, or implausible readings.

### 4. Adaptive per-sensor polling — Accepted

Use configurable adaptive intervals, central caching, overhead monitoring, and protection-aware minimum sampling rates.

**Why:** Preserve gaming frame times without slowing safety-related observations excessively.

### 8. Provider architecture — Accepted

Use built-in Rust providers for common hardware, with an optional versioned plugin boundary for specialized hardware.

**Why:** Deliver reliable baseline support while allowing hardware-specific extensions.

### 10. Persistent sensor identity — Accepted

Assign persistent hardware-backed identities, with rediscovery, aliases, and explicit ambiguity detection. Never silently attach an alarm to a different sensor following hardware changes.

**Why:** Sensor index order and display names are not durable identifiers.

### 12. Virtual sensor formulas — Accepted

Provide a safe, unit-aware formula engine for arithmetic, conditionals, moving averages, rates of change, and chained calculated sensors.

**Why:** Cooling-loop analysis needs derived values such as average radiator intake/exhaust temperatures and deltas.

### 19. Hardware control boundary — Accepted

Keep the core a read-only telemetry collector for physical and calculated sensors. Expose data to optional third-party fan/pump controllers via API; do not implement direct hardware fan or pump control in the core. Separately authorized protective shutdowns remain an explicit exception to read-only operation.

**Why:** Separate monitoring from continuous actuator control and its failure modes.

### 33. Suspend/resume handling — Accepted

Invalidate stale samples on resume, refresh affected providers, use fresh validated samples in alarm evaluation, and surface missing sensors explicitly. Do not blindly disable thermal protection because a source temporarily disappears.

**Why:** Resuming from sleep can invalidate sensor handles and time assumptions.

### 35. Canonical and alternative readings — Accepted

Prefer a canonical reading while retaining expandable alternative provider values and raw source data. Deduplicate only when identity and measurement semantics justify it. Allow explicit provider selection.

**Why:** Similar labels do not necessarily mean two sensors measure the same physical quantity.

### 36. Sensor calibration — Accepted

Support offsets, scaling, and optional multipoint calibration. Preserve raw readings, label calibrated values, and require separate validation before calibrated data can drive disruptive protection.

**Why:** External temperature probes and flow meters may need correction without losing original evidence.

### 37. Statistics scope — Accepted

Support session, rolling-window, and historical sensor statistics. Users choose the period and can explicitly reset session statistics without deleting historical records.

**Why:** A peak since boot answers a different question from a peak during a gaming session.

## History, configuration, and portability

### 3. Historical data model — Accepted

Default to 15 minutes of detailed in-memory samples, seven days of downsampled disk history, and detailed alarm snapshots. Make retention configurable.

**Why:** Retain useful incident context without excessive continuous disk writes.

### 9. Configuration and persistence — Accepted

Use human-readable TOML for settings, formulas, and alarm rules; SQLite for history and events. GUI and CLI changes pass through daemon validation rather than directly editing live service state.

**Why:** Make configuration auditable, portable, and consistently validated.

### 17. Portable diagnostic reports — Accepted

Generate reports locally, redact sensitive identifiers by default, and require explicit opt-in for extra details. Never automatically upload reports.

**Why:** Troubleshooting bundles should be shareable without unintentionally exposing host identity or network details.

### 29. Reusable profiles — Accepted

Allow profiles and templates to be exported and imported. Require sensor mapping and validation on the destination machine. Never import shutdown permission automatically.

**Why:** Configurations should be reusable across heterogeneous hardware without unsafe identity assumptions.

### 31. Network-outage durability — Accepted

Continue local telemetry and alarm recording during outages, and synchronize missing history/events after reconnection within configurable storage, bandwidth, and retention limits.

**Why:** Network interruptions must not erase incident evidence or interrupt local monitoring.

### 38. Retention under storage pressure — Accepted

Enforce configurable age and disk budgets. Prioritize critical events and audit records, downsample or remove routine samples first, and never let persistence pressure block acquisition or alarm evaluation.

**Why:** A full disk must not stall the active protection engine.

## Alarms, safety, and notifications

### 11. Notification channels — Accepted

Support local notifications and persistent logs, plus optional per-rule webhook, email, ntfy, and Gotify providers.

**Why:** Desktop users and unattended hosts need different delivery paths.

### 13. First-run alarm templates — Accepted

Offer hardware-aware, notification-only templates. Users choose which to activate. No shutdown authorization is granted by setup.

**Why:** Useful initial warnings should not cause surprise shutdowns.

### 15. Least-privilege execution — Accepted

Run the daemon and GUI unprivileged by default. Introduce narrow privileged helpers only for necessary operations and separate authorization for disruptive actions.

**Why:** Reading telemetry should not require unrestricted root execution.

### 16. Post-shutdown recovery notices — Accepted

After a protective shutdown, show a persistent unacknowledged warning explaining the trigger, history, and current sensor state. Do not restrict subsequent system operation.

**Why:** Users returning to an unexpectedly powered-off machine need the cause and supporting evidence.

### 25. Daemon supervision — Accepted

Use systemd restart/watchdog supervision, persistent service-failure events, health reporting, and optional remote alerts. Daemon failure itself must not initiate shutdown.

**Why:** Monitoring failures require visibility, not an automatic destructive response.

### 27. Notification escalation — Accepted

Support optional per-rule escalation with configurable delays, additional channels, rate limits, and acknowledgment handling. Escalation never grants shutdown authority.

**Why:** A missed alert can be routed more aggressively without broadening its permissions.

### 32. Maintenance mode — Accepted

Provide time-limited maintenance with independent notification suppression and protective-action controls. Keep recording/auditing, preserve protection by default, and automatically restore normal operation.

**Why:** Maintenance should reduce alert noise without silently removing safeguards.

### 34. Unreviewed alarms — Accepted

On login or GUI reopening, group significant unreviewed alarms, including resolved and still-active conditions. Preserve acknowledgment state and avoid notification storms.

**Why:** Users need an intelligible incident summary after being away.

### 42. Protection-rule simulation — Accepted

Support injected simulated readings, historical replay, and dry-run evaluation. Simulations may produce simulated alarm outcomes but **must never call the real shutdown mechanism**. Enabling an actual disruptive action remains separately authorized.

**Why:** Test thresholds, timing, noise, and dropout behavior without endangering a running system. Simulation does not validate the physical sensor itself.

## User experience, performance, and release governance

### 18. GUI presentation — Accepted

Offer a dense HWiNFO-style grouped sensor table as the default GUI view, along with customizable dashboards.

**Why:** Power users need comprehensive detail, while dashboards highlight selected relationships.

### 23. Performance gates — Accepted

Run lightweight CI benchmarks on pull requests and comprehensive release benchmarks. Document CPU, memory, polling, and gaming frame-time budgets.

**Why:** Low overhead is a primary product requirement, not a late optimization.

### 28. Accessibility — Accepted

Design GUI workflows for keyboard navigation, screen readers, scalable controls, configurable shortcuts, and one-handed use. Consider optional voice integrations later.

**Why:** Accessibility should influence architecture and interactions before GUI implementation.

### 39. Community plugins — Accepted

Allow verified official plugins and explicitly approved community plugins. Require versioned manifests and permission declarations, process isolation, and no implicit privileged access.

**Why:** Extensions increase hardware reach but also introduce supply-chain and process-safety concerns.

### 40. Release stages — Accepted

Stage delivery as experimental headless alpha, native GUI beta, expanded monitoring and alarms, and stable 1.0 only after protection features have been validated and APIs stabilized.

**Why:** Avoid representing pre-alpha monitoring or unvalidated protection as production-ready.

### 41. Contributor provenance — Accepted

Require Developer Certificate of Origin (DCO) commit signoffs, checked in contributions. Contributors retain their copyright; no CLA or copyright assignment.

**Why:** Provide a lightweight statement of contribution rights and respect for proprietary third-party work.

## Local API permissions and remaining decisions

### 43. Local API permissions — Approved for Alpha 1 current-user reads; broader policy Pending

**Original question — 2026-11-16 (historical):** Determine local read access, administrator configuration rights, and independent authorization for disruptive actions across D-Bus, CLI, GUI, and shared systems. No option is accepted yet.

**Original constraints:** Decisions 7, 15, and 26 require remote authentication, least-privilege operation, and a shared system-wide daemon as project direction. Do not infer broader permissions from those constraints alone.

**Alpha 1 policy approved by Chris — 2026-11-17:** Prototype 009 may expose only versioned informational and snapshot reads through an opt-in local pathname Unix socket. Require the daemon's real and effective UIDs to match and be nonzero, a private owner-controlled runtime namespace and restrictive socket permissions, and kernel peer credentials matching the daemon's effective UID before parsing requests. The endpoint grants no configuration, acquisition-scheduling or hardware-control authority. Environment paths require validation; existing/stale endpoints must not be overwritten or automatically removed. Read-only clients consume the same cache as the acquisition owner.

This approval does **not** authorize remote API access, multiuser/system-wide sharing, write operations, privileged execution or hardware controls. Same-UID programs and root are outside the isolation boundary. Decision 26 remains the future shared-service direction; its permissions, D-Bus, administrator configuration rights and disruptive-action authorization require separate review. See the [approved implementation proposal](milestones/009-local-readonly-api-proposal.md) and [implementation report](milestones/009-local-readonly-api.md). Policy approval authorizes local implementation and verification, not production readiness or milestone acceptance.

#### Prototype 005 completion authorization — 2026-11-16

At Chris's request, this entry records a separate, limited development-execution authorization. **Decision 43's local API permissions policy remains Pending; no permission option is approved by this entry.**

Chris authorizes the bounded completion protocol in the [Prototype 005 completion proposal](milestones/005-provider-latency-completion-proposal.md):

- Implement the minimal Python-only single-block completion mode and deterministic fixture tests, including cleanup and signal handling through final evidence preservation. Use the existing Rust 1.85.0 release binary without rebuilding it; preserve application source, dependencies, schemas, sensor identity and sampling semantics.
- Run six short smoke cases. Only if all preflight gates and smoke cases pass, run one fresh seven-case block in this order: full scan before, amdgpu, k10temp, Wi-Fi, acpitz, NVMe, full scan after. Retain the proposal's 11-attempt smokes, 301-attempt primary cases, 100 ms interval and JSON Lines output.
- Limit the new session to **600 seconds total**, including preflight, smoke, settling, measurements and finalization, and **128 MiB of new evidence**. Keep combined preserved and new local measurement evidence within the proposal's 256 MiB limit.
- Permit **no retries or additional hardware runs without separate authorization**. Stop on failed preflight, identity validation, resource limits or smoke tests; retain failure evidence rather than silently continuing.
- Preserve all original and incomplete measurements, hashes, timestamps and recovery records. Store new evidence separately. Do not reconstruct unavailable CPU accounting, exit statuses, total attempt counts or execution durations.

**Why a fresh block is required:** the original third block contains an earlier full-before control and completed amdgpu, k10temp and Wi-Fi observations, but acpitz was interrupted, NVMe was unstarted and the closing full-scan control was never recorded. A control measured in a later session cannot establish the earlier session's intervening workload, thermal or scheduling conditions, so it cannot retroactively provide equivalent bracketing. Report original blocks one and two plus the fresh complete block as distinct-session comparisons; preserve the original incomplete block separately. Do not relabel new measurements as resumed historical cases or infer driver causality from latency associations.

**Prototype 005 acceptance at execution authorization: Pending.** Execution authorization does not constitute milestone acceptance. Completion must be assessed against actual verification results, preserved evidence and the approved acceptance criteria. The existing [milestone report](milestones/005-provider-latency-characterization.md) and [measurement report](benchmarks/005-provider-latency-results.md) retain the partial experiment; this historical entry records the subsequent execution authorization without rewriting that history or the proposal's earlier unapproved status.

No commit, push, merge or pull request is authorized. Prototype 006 must not begin. This documentation update itself performs no benchmark-code modification or hardware characterization; stop after the update and handoff, as Chris requested.

#### Prototype 005 technical acceptance — 2026-11-16

**Chris formally accepted Alpha 1, Prototype 005 as technically complete with its documented limitations.** The [formal acceptance record](milestones/005-provider-latency-characterization.md#formal-acceptance-and-publication-review--2026-11-16) records successful deterministic validation, the completed authorized characterization, independent evidence verification and preservation of original observations. The original third block remains incomplete; the fresh seven-case block is a separate cohort. Exit 143 and the historical 503.365 ms event remain unexplained, and NVMe-registration latency association does not establish driver causality.

This is milestone acceptance, separate from the earlier hardware execution authorization and from any future Git publication authorization. **Decision 43's local API permissions policy remains Pending; no local API access option is approved.** No additional hardware runs, staging, commits, push, merge, PR or Prototype 006 work are authorized by acceptance. See the [publication-readiness review](milestones/005-publication-readiness.md) for the audited proposed change set and remaining limitations.

## Cross-cutting implementation invariants

1. **Acquisition is shared:** Providers feed one daemon-owned sensor cache; CLI, GUI, history, formulas, alarms, and remote clients consume validated data rather than independently polling hardware.
2. **Trust is explicit:** Raw, calibrated, derived, stale, missing, and questionable readings remain distinguishable. Identity ambiguity cannot silently retarget protection.
3. **Effects are gated:** Notification, escalation, profile import, simulated tests, and access to telemetry never imply shutdown authorization. Only a separately authorized real protection path may invoke a privileged helper.
4. **Local operation is independent:** Loss of network, GUI, dashboard, or database write availability must not by itself stop local sensor collection or live alarm evaluation.
5. **Performance is measurable:** Provider poll frequency, cache behavior, storage load, and client rendering must fit documented system overhead and frame-time budgets.
6. **Experimental status is visible:** Planned capabilities, implemented capabilities, and validated protection behavior must be distinguished in docs and releases.

## Integration review questions — not accepted decisions

These questions identify wording tensions and implementation details left open by the supplied record. They do not amend Decisions 1–42, select a policy for Decision 43, or authorize protective actions.

| Area | Related decisions | Clarification requested |
| --- | --- | --- |
| Meaning of validation | 2, 35, 36; invariant 1 | Does “validated data” mean structurally valid cache records that can still be flagged questionable, or only readings approved for protection? The latter interpretation conflicts with displaying questionable readings and preserving raw alternatives. |
| Local permissions and source trust | 2, 7, 10, 15, 26, 35, 36, 43 | Alpha 1 current-user, peer-verified read access is approved under Decision 43. Multiuser access, configuration, validation of sensor mappings and independent disruptive-action authorization remain pending. What changes to provider, identity or calibration invalidate prior validation? |
| Derived data, timing, and missing sensors | 4, 10, 12, 25, 33, 35, 36, 37 | How do stale, missing, questionable, or simulated inputs propagate through formulas and provider fallback? How do adaptive polling and suspend gaps affect moving averages, rate calculations, alarm timing, and statistics? What observable degraded state applies when protection lacks fresh evidence? |
| Simulation and external effects | 11, 19, 39, 42; invariants 1–3 | How are replay and simulated samples isolated from live protection while preserving their provenance? May simulations emit real notifications, webhooks, or controller-facing events that could trigger external actions? The prohibition on calling the real shutdown mechanism remains unchanged. |
| Persistence and recovery | 3, 16, 25, 31, 38; invariant 4 | How are incident evidence and service-failure events reported when storage is full or unavailable? What happens if an already authorized protective action cannot persist its explanation? How are retention gaps represented during historical replay or reconnect synchronization? |
| Standalone updates | 14, 22 | What is the supported update and configuration-backup workflow for standalone binaries when updates are described as controlled by the operating system package manager? The accepted prohibition on self-updating and automatic rebooting remains unchanged. |
| Performance and release gates | 4, 12, 23, 40 | What numeric CPU, memory, polling, and frame-time budgets, reference workloads, and protection-validation criteria define each release gate? What happens when minimum protection sampling rates exceed hardware capability or the overhead budget, and which features belong in the initial headless alpha? |
| Diagnostic privacy | 7, 9, 17, 29 | Which identifiers, credentials, configuration fields, and log contents must default redaction cover, and how are optional extra details reviewed before opt-in? Local report generation and the prohibition on automatic upload remain unchanged. |

## Revision history

| Date | Change | Status |
| --- | --- | --- |
| 2026-11-16 | Initial consolidated record of decisions 1–42 from project discussions; decision 43 explicitly pending. | Draft for review |
| 2026-11-16 | Integrated the supplied record into `docs/`; preserved all accepted decision text and pending Decision 43; added explicit project context, consistent heading spacing, and non-normative review questions. | Integration draft; awaiting review |
| 2026-11-16 | Added Chris's bounded Prototype 005 completion authorization under Decision 43, preserving its local API permissions question, numbering and history; documented separate evidence and non-retroactive bracketing. | Execution authorized within stated bounds; Prototype 005 acceptance Pending; local API permissions Pending |
| 2026-11-16 | Recorded Chris's subsequent formal Prototype 005 technical acceptance with documented limitations and a separate publication-readiness review; retained all earlier authorization history. | Prototype 005 Accepted; publication not authorized; local API permissions Pending |
| 2026-11-17 | Recorded Chris's Prototype 009 implementation authorization: Alpha 1 pathname Unix socket/versioned JSON replaces the initial D-Bus direction; current-user, peer-verified read-only access is approved. Prior decision and Prototype 005 history retained. | Decisions 5/43 updated for the narrow Alpha 1 policy; broader permissions Pending; local implementation/testing only |

## Next steps

1. Review this consolidation against the agreed conversations. Correct any wording that overstates a decision.
2. Review the remaining Decision 43 permissions separately; do not expand the approved Alpha 1 read-only policy implicitly.
3. Convert major sections into architecture and implementation specifications as engineering begins.
4. Keep this record synchronized with later explicit decisions and their revision dates.
