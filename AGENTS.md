# SiliconWatch development workflow

Read `README.md`, `docs/DESIGN_DECISIONS.md`, and any instructions in the affected directories before making changes. Design decisions guide implementation; retain essential safety and security constraints and flag proposed changes explicitly.

## Working principles

- Build incrementally. Prefer working code over speculative architecture.
- Keep milestones small, testable, and aligned with the approved Alpha 1 scope. Do not expand that scope without explicit approval.
- Avoid premature abstractions and unnecessary dependencies.
- Maintain read-only hardware monitoring until protective features are separately approved.
- Preserve project flexibility, existing content, and unrelated changes.
- Use feature branches for development.
- Never perform disruptive hardware actions without authorization.
- Do not automatically begin the next milestone.
- Do not commit, push, merge, or create pull requests unless explicitly instructed. Follow the DCO signoff policy when committing.
- Surface blockers and uncertainty honestly. Never report an unverified feature as tested.
- Finish every completed milestone with the report below, then stop and wait for review.

## Quality requirements

- Run formatting, Clippy with warnings denied, and automated tests when applicable.
- Test against real hardware when available, without changing hardware settings.
- Distinguish verified behavior from assumptions and report failures, unexecuted checks, and limitations honestly.
- Preserve essential safety and security constraints while allowing evidence-based architectural revisions; document proposed deviations rather than silently changing accepted decisions.
- Follow DCO signoff requirements for every authorized contribution commit.

## Independent review before merge

Before presenting substantive changes for merge, the implementing agent must obtain a review from the project-scoped Codex agent `reviewer`. This applies to runtime behavior, APIs, stored settings, concurrency, permissions, CI, packaging and release controls. One reviewer is sufficient. Typo-only and formatting-only edits can skip this step.

1. **Freeze the proposal.** Supply the original requirements, observable acceptance criteria, relevant paths, accepted design decisions and exact diff. For commits, give full base and head SHAs and `git diff --no-ext-diff --no-textconv --binary --full-index BASE HEAD`. For local work, give the full base SHA, a saved full-index binary patch and its SHA-256. Check its inventory against all intended staged, unstaged and new files, deletions and mode changes. Review the frozen objects or patch. Verify live files against their postimages before relying on them. Leave the builder's rationale, self-assessment and desired verdict out of the packet.
2. **Select trusted controls.** Use a maintainer-inspected full commit SHA for the launcher, session verifier and reviewer role. For a change to review controls, use the previously trusted revision. For first-install qualification, inspect those three files independently as proposal data and test the candidate launcher. The maintainer selects the trusted controls revision for routine use at acceptance. Address the source checkout as `/mnt/source` and supply the applicable base guidance as review data. For uncommitted proposals outside that checkout, include the frozen patch in the packet.
3. **Run the snapshot launcher.** Start a fresh coordinator using the command below. It spawns the named project role with only the packet. Require a successful exit and its JSON receipt showing the actual role, fresh context, distinct parent and child, loaded instructions, read-only sandbox and denied approvals. An unavailable prerequisite or missing native evidence leaves the review gate open.
4. **Resolve findings.** Return the child's findings to the implementing agent. For each, record the correction and test, evidence that resolves it, or an open decision for the maintainer. After remediation or any substantive change, freeze the entire updated proposal and request a fresh reviewer with the prior findings as questions to verify. Keep unresolved findings visible. Withhold a merge-ready recommendation while a blocking or important finding remains open.
5. **Handoff.** Include the target SHAs or digest, reviewer identity, findings and dispositions, checks run and remaining gaps. CI and DCO remain separate mandatory checks. Label the result as an AI code review. Human GitHub approvals and protected release approvals come from the designated people.

### Running a review on Linux

Run the inspected launcher from a trusted checkout, with system Python 3.11+, Codex CLI and Bubblewrap with user namespaces and temporary overlay support. Set `controls` to the full SHA selected above. Put the packet and receipt outside the checkout. Use `--source /path/to/standalone/checkout` if the proposal lives in a different repository checkout.

```bash
python3 -I /path/to/trusted/scripts/review/launch.py --repo /path/to/trusted "$controls" \
    < /path/to/review-packet.txt > /path/to/review-receipt.json
```

The launcher reads the role and verifier from Git objects. It seals those bytes, the review packet, a minimal Codex configuration and the empty MCP allowlist before mounting private copies. Both preflight and the complete Codex session use the same read-only discovery roots. The preflight requires Codex to report a harmless configured server disabled by that policy. The Codex client uses its normal file login read-only or an existing `CODEX_API_KEY`. Model shell commands can read the frozen project and source checkout under a restricted filesystem profile. A pre-inference check verifies that a private control file and session state are inaccessible to the shell. Codex keeps transient session data in private memory-backed storage. The receipt includes the child's report, the packet digest confirmed by that child, and the control, configuration and instruction identifiers.

The trusted boundary includes the launcher checkout and object store, installed executables, the Linux kernel, and the account or machine administrator. Run on a standalone or compatibly managed machine. An existing system requirements file must already enforce an empty MCP allowlist. The launcher preserves it and fails when it cannot verify that policy. Administer a change to higher-priority organization policy separately from an active review. New Codex versions must provide the required native evidence. Run `SILICONWATCH_REQUIRE_REVIEW_SNAPSHOT=1 python3 -B -m unittest discover -s scripts/review` to require the live mount race test on a review host.

## Required milestone handoff

Save each completed report under `docs/milestones/NNN-milestone-name.md`, using a zero-padded milestone number and a descriptive filename. Preserve prior reports and clearly date later review updates.

The report must cover milestone name and status, implemented features, important files, introduced dependencies, actual verification and hardware results, known bugs and risks, design deviations, Git status, and one recommended next milestone with acceptance criteria. Use the detailed structure below.

### 1. Milestone Summary

Include the milestone number and name, objective, completion status (`Complete`, `Partial`, or `Blocked`), and a brief account of what was accomplished.

### 2. Implementation Details

List implemented features, major technical decisions, important files created or modified, dependencies added or removed, and deviations from the requirements with reasons.

### 3. Verification Results

Report actual results for applicable checks:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`
- Functional execution tests
- Hardware tests
- Security or permission checks

Include passed and failed test counts. Clearly distinguish tests actually executed, tests that could not be executed, and features not yet tested. Describe any conditional skips.

### 4. Hardware Validation

When relevant, report the operating system and kernel, hardware tested, sensor providers, sensor counts and categories, example readings, missing or unsupported sensors, permission problems, and unexpected behavior. Omit sensitive machine identifiers unless necessary.

### 5. Architecture and Design Review

Explain alignment with accepted design decisions, impractical decisions or incorrect assumptions discovered, recommended architectural changes, and any unnecessary complexity. Decisions are guidance, not immutable requirements; propose changes when implementation evidence warrants them.

### 6. Known Issues and Risks

List confirmed bugs, untested functionality, performance concerns, security concerns, technical debt, and potential compatibility problems. Distinguish confirmed issues from suspected risks.

### 7. Git Status

Report branch, commit hash if committed, staged/unstaged changes, push status, working-tree status, and pull request URL if applicable. Do not infer permission to commit, push, merge, or create a pull request.

### 8. Recommended Next Milestone

Recommend one small milestone with its objective, proposed scope, acceptance criteria, expected tests, and risks, dependencies, or blockers. Keep it aligned with the Alpha 1 scope freeze. Do not begin it automatically.

### 9. ChatGPT Handoff Summary

End the report with a section titled **HANDOFF TO CHATGPT**. Make it self-contained and easy to copy into an existing ChatGPT conversation, approximately 300–600 words unless complexity justifies more.

Include what was requested, implemented, and verified; what remains incomplete; important technical decisions; issues requiring Chris's attention; Git status; and the recommended next action. Include this full, approximately 300–600-word handoff in the final chat response as well as the saved report. Then stop and wait for review.
