"""Private side of the review launcher. Runs entirely inside the mounted snapshot."""

import hashlib
import json
import re
from pathlib import Path
import subprocess
import sys
import tomllib


PROJECT = Path("/mnt/project")
ROLE = PROJECT / ".codex/agents/reviewer.toml"
CONFIG = Path("/mnt/home/.codex/config.toml")
PROBE = "siliconwatch_policy_probe"
DENIAL = "requirements (/etc/codex/requirements.toml)"
PACKET = PROJECT / "review-packet.txt"
COORDINATOR = """Spawn one native child with agent_type=reviewer and fresh context
(fork_turns=none or fork_context=false). Its task is to read the entire immutable
/mnt/project/review-packet.txt, review the proposal specified there using /mnt/source,
and include the packet file's SHA-256 in its report. Wait for completion. Use only
the spawn and wait tools. If unavailable, report failure. Do not review it yourself.
"""


def preflight(codex):
    # A bare empty list would also pass if Codex silently ignored the policy.
    # List a harmless configured server and require the administrator policy to deny it.
    servers = json.loads(subprocess.check_output([codex, "mcp", "list", "--json"]))
    if len(servers) != 1:
        raise ValueError("unexpected effective MCP configuration")
    server = servers[0]
    if server["name"] != PROBE or server["enabled"] is not False or server["disabled_reason"] != DENIAL:
        raise ValueError("Codex did not enforce the snapshot's empty MCP allowlist")


def restricted_profile(context):
    profile = context["permission_profile"]
    fs = profile.get("file_system", {})
    entries = fs.get("entries", [])
    paths = set()
    for entry in entries:
        target = entry["path"]
        if entry["access"] != "read":
            return False
        if target == {"type": "special", "value": {"kind": "minimal"}}:
            paths.add(":minimal")
        elif target.get("type") == "path":
            path = target["path"]
            # Codex adds a per-session directory of executable aliases to :minimal.
            if not re.fullmatch(r"/mnt/home/\.codex/tmp/arg0-linux/codex-arg0[A-Za-z0-9]+", path):
                paths.add(path)
        else:
            return False
    return (profile.get("type") == "managed" and profile.get("network") == "restricted"
            and fs.get("type") == "restricted" and paths == {":minimal", "/mnt/project", "/mnt/source"})


def read_session(path, instructions):
    """Extract only evidence for this launch from the private native session logs."""
    meta = None
    contexts = 0
    calls = []
    envelopes = []
    spawns = []
    loaded = False
    report = None
    with path.open() as stream:
        for line in stream:
            event = json.loads(line)
            item = event["payload"]
            if event["type"] == "session_meta":
                if meta is not None or not isinstance(item.get("id"), str) or not item["id"]:
                    raise ValueError("ambiguous native session metadata")
                meta = item
            elif meta is None:
                raise ValueError("native session metadata must precede events")
            elif event["type"] == "turn_context":
                if (item["cwd"] != str(PROJECT) or item["approval_policy"] != "never"
                        or not restricted_profile(item)):
                    raise ValueError("unexpected Codex working root, approval policy or sandbox")
                contexts += 1
            elif event["type"] == "response_item":
                if item.get("type") == "agent_message":
                    envelopes.append(item)
                if item.get("type") in ("function_call", "custom_tool_call", "web_search_call"):
                    calls.append(item.get("name", item["type"]))
                if item.get("type") == "function_call" and item.get("name") == "spawn_agent":
                    spawns.append(json.loads(item["arguments"]))
                if item.get("type") == "message" and item.get("role") == "developer":
                    loaded |= any(instructions == block.get("text", "").strip() for block in item["content"])
            elif event["type"] == "event_msg":
                if item.get("type") in ("task_started", "turn_aborted"):
                    report = None
                elif item.get("type") == "task_complete":
                    report = item.get("last_agent_message")
    if not meta or not contexts or not report:
        raise ValueError("incomplete native Codex session evidence")
    return meta, spawns, loaded, report, calls, envelopes


def review_target(packet, codex):
    header, _, body = packet.partition(b"\n")
    match = re.fullmatch(rb"Target: ([0-9a-f]{40}) ([0-9a-f]{40}|worktree) ([0-9a-f]{64})", header)
    if not match:
        raise ValueError("missing valid Target header in frozen packet")
    base, head, digest = (value.decode() for value in match.groups())
    git = [codex, "sandbox", "--", "/usr/bin/git", "--no-replace-objects", "-C", "/mnt/source"]
    for rev in (base,) if head == "worktree" else (base, head):
        if subprocess.check_output([*git, "cat-file", "-t", rev]).strip() != b"commit":
            raise ValueError("packet target must reference commit objects")
    if head == "worktree":
        _, separator, patch = body.partition(b"\n--- BEGIN FROZEN GIT PATCH ---\n")
        if not separator:
            raise ValueError("missing frozen working-tree patch")
    else:
        patch = subprocess.check_output([*git, "diff", "--no-ext-diff", "--no-textconv",
                                         "--binary", "--full-index", base, head, "--"])
    if hashlib.sha256(patch).hexdigest() != digest:
        raise ValueError("review target differs from the frozen patch digest")
    return " ".join((base, head, digest))


def verify_report(report, packet_digest, target):
    statuses = [line for line in report.splitlines() if line.startswith("Review status:")]
    if (statuses != ["Review status: complete " + target]
            or report.rstrip().splitlines()[-1] != statuses[0] or packet_digest not in report):
        raise ValueError("review incomplete or reported target differs from the frozen proposal")


def collect_sessions(paths, instructions):
    records = {}
    for path in paths:
        record = read_session(path, instructions)
        identity = record[0]["id"]
        if identity in records:
            raise ValueError("duplicate native Codex session identity")
        records[identity] = record
    return records


def run(codex, controls, digest):
    role = ROLE.read_bytes()
    if hashlib.sha256(role).hexdigest() != digest:
        raise ValueError("mounted role differs from inspected bytes")
    definition = tomllib.loads(role.decode())
    instructions = definition["developer_instructions"].strip()
    if definition["name"] != "reviewer" or not instructions:
        raise ValueError("expected the named reviewer and inline developer instructions")
    packet = PACKET.read_bytes()
    packet_digest = hashlib.sha256(packet).hexdigest()
    preflight(codex)
    # Test the *effective* shell before inference, using harmless canaries.
    check = ('import os, sys; sys.exit(not (os.access("/mnt/project/review-packet.txt", os.R_OK)'
             ' and not os.access("/mnt/home/.codex/config.toml", os.R_OK)'
             ' and not os.access("/mnt/state", os.R_OK) and "CODEX_API_KEY" not in os.environ))')
    subprocess.run([codex, "sandbox", "--", sys.executable, "-I", "-c", check], check=True)
    target = review_target(packet, codex)
    result = subprocess.run([codex, "-a", "never", "exec",
                             "--ignore-rules", "--json", "-"], input=COORDINATOR,
                            stdout=subprocess.PIPE, text=True, check=True)
    events = [json.loads(line) for line in result.stdout.splitlines()]
    parent_ids = [e["thread_id"] for e in events if e["type"] == "thread.started"]
    if len(parent_ids) != 1 or not events or events[-1]["type"] != "turn.completed":
        raise ValueError("incomplete coordinator run")
    parent = parent_ids[0]
    records = collect_sessions(Path("/mnt/state/sessions").rglob("*.jsonl"), instructions)
    _, spawns, _, _, calls, _ = records.pop(parent)
    if set(calls) - {"spawn_agent", "wait_agent"}:
        raise ValueError("unexpected coordinator tool call")
    if len(spawns) != 1 or len(records) != 1:
        raise ValueError("expected one native spawn and one child")
    spawn = spawns[0]
    if spawn.get("agent_type") != "reviewer" or not (
        (spawn.get("fork_turns") == "none" and "fork_context" not in spawn)
        or (spawn.get("fork_context") is False and "fork_turns" not in spawn)
    ):
        raise ValueError("unverified role selection or fresh context")
    child, (meta, child_spawns, loaded, report, _, envelopes) = records.popitem()
    if meta.get("parent_thread_id") != parent or meta.get("agent_role") != "reviewer" or not loaded or child_spawns:
        raise ValueError("child identity or loaded developer instructions differ")
    if len(envelopes) != 1 or envelopes[0].get("author") != "/root":
        raise ValueError("unexpected child task or follow-up message")
    contents = envelopes[0]["content"]
    # The current CLI encrypts the initial routing text in both logs. Verify
    # delivery of that envelope; only the sealed packet may define the task.
    if not any(block.get("encrypted_content") == spawn["message"] for block in contents):
        raise ValueError("child routing envelope differs from the native spawn")
    verify_report(report, packet_digest, target)
    print(json.dumps({"review_status": "complete", "target": target, "packet_sha256": packet_digest, "controls_commit": controls, "config_sha256": hashlib.sha256(CONFIG.read_bytes()).hexdigest(),
                      "role_sha256": digest, "instruction_sha256": hashlib.sha256(instructions.encode()).hexdigest(),
                      "mcp_policy": DENIAL, "parent": parent, "child": child, "agent_type": "reviewer",
                      "fresh_context": True, "sandbox": "read-only", "approval": "never", "report": report}))


if __name__ == "__main__":
    try:
        run(*sys.argv[1:])
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as exc:
        sys.exit(f"Independent review unavailable: {exc}")
