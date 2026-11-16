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
