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
2. **Start from trusted instructions.** Launch a fresh coordinator at a checkout with previously trusted review controls. If the proposal changes Codex configuration, agent definitions, hooks, skills or project instructions, launch from a separate trusted base checkout and pass the proposal as data. Read design guidance at the named base or head. For the first installation, inspect the new role and all local configuration before trusting the checkout, disable automatic project instructions for the bootstrap (`-c project_doc_max_bytes=0`), and pass the base guidance explicitly. Record the inspected role's digest. The maintainer decides whether to adopt it.
3. **Spawn and verify.** Begin the packet with “Spawn the project agent type `reviewer` with fresh context, give it only the packet below, wait for completion and return its report and child ID.” Use a separate read-only coordinator because the child inherits its parent's permissions. Preserve runtime evidence for the selected role, parent and distinct child, such as the native spawn arguments and child thread metadata. A task nickname or a coordinator's own prose cannot establish this. If the role or its spawn cannot be verified, report independent review as unavailable and leave the gate open.
4. **Resolve findings.** Return the child's findings to the implementing agent. For each, record the correction and test, evidence that resolves it, or an open decision for the maintainer. After remediation or any substantive change, freeze the entire updated proposal and request a fresh reviewer with the prior findings as questions to verify. Keep unresolved findings visible. Withhold a merge-ready recommendation while a blocking or important finding remains open.
5. **Handoff.** Include the target SHAs or digest, reviewer identity, findings and dispositions, checks run and remaining gaps. CI and DCO remain separate mandatory checks. Label the result as an AI code review. Human GitHub approvals and protected release approvals come from the designated people.

For a CLI review, run this Bash example from the trusted checkout. Keep the packet and output outside the checkout. Add any needed per-session overrides to `review_flags`, for example `-c mcp_servers.node_repl.enabled=false` on a host with that server. Use the same flags for the preflight and review. The preflight reports enabled MCP server names and blocks launch until each is disabled. The review ignores execution-policy allow rules that could otherwise run shell commands outside the sandbox.

```bash
set -o pipefail
review_flags=(--disable apps --disable plugins --disable hooks --disable memories)
codex "${review_flags[@]}" mcp list --json |
    python3 -c 'import json, sys; names = [s["name"] for s in json.load(sys.stdin) if s["enabled"]]; sys.exit(", ".join(names) if names else 0)' &&
    codex "${review_flags[@]}" -a never exec --ignore-rules --sandbox read-only --json - < /path/to/review-packet.txt
```

The CLI summary may omit the selected role. In that case, use the child thread's Codex metadata (`thread/read`) or the recorded native `spawn_agent` call and corresponding child result to verify the handoff.

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
