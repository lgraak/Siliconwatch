# Proposal for live daemon monitoring configuration

Status as of November 18, 2026. Pending Chris's approval.

Design Decision 43 currently permits peer-verified, current-user read access.
The GUI saves timing requests locally and shows a launch command. The existing
`get_info` method can report effective values for that daemon instance.

A future design should define a configuration authority independently from the
right to read telemetry. Chris must approve which principal can change the
sampling interval and stale threshold, and whether all processes with the
daemon UID can exercise that authority. `SO_PEERCRED` identifies the UID. A
same-UID writer policy therefore grants authority to every process running
under that account. An authority narrower than a UID needs a separate
enforceable boundary, with a documented threat model and credential lifecycle.

After approval, define a separately versioned write contract or distinct
control endpoint. Preserve v1 read-only semantics and require an explicit
daemon opt-in to live control. Specify how the client verifies the server and
how the server authenticates and authorizes each write before processing it.
The write should carry the target instance, expected configuration revision,
and the complete requested timing pair. Validate the proposed state at the
daemon and return the applied revision and effective values. Specify conflict,
busy, unavailable, restart, and invalid-setting responses. Apply a timing pair
atomically at an acquisition scheduling boundary. Decide explicitly whether
daemon overrides are session-only or persist across restarts, and how startup
flags, file ownership, and rollback interact with persistence.

Acceptance requires a separate approval of the authority model, protocol and
resource limits, and persistence semantics. Tests should cover unauthorized
peers, unauthorized same-UID callers when applicable, replay after restart,
concurrent revisions, malformed frames, a blocked provider, and unchanged
read-only v1 behavior. Scope the authority to software monitoring timings.
