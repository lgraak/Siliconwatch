"""Offline tests for the mounted reviewer controls and the MCP policy preflight."""

import errno
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

if __package__:
    from . import launch, session
else:
    import launch
    import session


# This replaces only the session body. The production launcher creates the mounts.
OBSERVER = b'''
import json
import os
from pathlib import Path
import subprocess
import sys

assert sys.argv[1] == "/usr/bin/false"
role = Path.cwd() / ".codex/agents/reviewer.toml"
config = Path(os.environ["CODEX_HOME"]) / "config.toml"
policy = Path("/etc/codex/requirements.toml")
source = Path("/mnt/source/fixture.txt")
packet = Path.cwd() / "review-packet.txt"
for stage in ("preflight", "launch", "restored"):
    denied = []
    for path in (role, config, policy, role.with_name("injected.toml"), source, packet):
        try:
            with path.open("ab") as stream:
                stream.write(b"test")
        except OSError as exc:
            denied.append(exc.errno)
    print(json.dumps([stage, role.read_text(), config.read_text(), policy.read_text(),
                      sorted(p.name for p in role.parent.iterdir()), denied, source.read_text(), packet.read_text(),
                      Path("/inputs/home/.codex/config.toml").exists(),
                      subprocess.run([sys.argv[1]]).returncode]), flush=True)
    if stage != "restored" and input() != "continue":
        sys.exit(2)
'''

DRIVER = '''
import sys
sys.path.insert(0, "/review-code")
import launch
from pathlib import Path
root = Path("/inputs")
sys.exit(launch.in_snapshot((root / "checkout" / launch.ROLE).read_bytes(), (root / "observer").read_bytes(),
                            "/usr/bin/false", "0" * 40, source=root / "checkout",
                            packet=(root / "packet").read_bytes()))
'''


def isolated_host(root):
    # Only the operating-system binaries and synthetic fixture files enter the test.
    args = ["/usr/bin/bwrap", "--unshare-user", "--unshare-pid", "--die-with-parent",
            "--ro-bind", "/usr", "/usr", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
            "--dir", "/mnt", "--ro-bind", str(root), "/inputs",
            "--ro-bind", str(root / "etc"), "/etc",
            "--ro-bind", str(Path(__file__).resolve().parent), "/review-code"]
    for directory in ("/bin", "/lib", "/lib64", "/sbin"):
        path = Path(directory)
        if path.is_symlink():
            args.extend(["--symlink", os.readlink(path), directory])
        elif path.is_dir():
            args.extend(["--ro-bind", directory, directory])
    return args


def replace_text(path, content):
    temp = path.with_suffix(".replacement")
    temp.write_text(content)
    temp.replace(path)


class SnapshotTests(unittest.TestCase):
    def test_mcp_preflight_requires_effective_policy_denial(self):
        denied = {"name": session.PROBE, "enabled": False, "disabled_reason": session.DENIAL}
        with patch.object(session.subprocess, "check_output", return_value=json.dumps([denied])) as call:
            session.preflight("/usr/bin/false")
        call.assert_called_once_with(["/usr/bin/false", "mcp", "list", "--json"])
        self.assertEqual(tomllib.loads(launch.CONFIG.decode())["mcp_servers"],
                         {session.PROBE: {"command": "/usr/bin/false"}})

        for servers in ([], [dict(denied, enabled=True)], [dict(denied, disabled_reason=None)],
                        [dict(denied, name="other")], [denied, dict(denied, name="other")]):
            with self.subTest(servers=servers), patch.object(
                session.subprocess, "check_output", return_value=json.dumps(servers)
            ), self.assertRaises(ValueError):
                session.preflight("/usr/bin/false")

    def test_native_evidence_rejects_broader_permissions_or_aborted_session(self):
        fs = {"type": "restricted", "entries": [
            {"path": {"type": "special", "value": {"kind": "minimal"}}, "access": "read"},
            {"path": {"type": "path", "path": "/mnt/project"}, "access": "read"},
            {"path": {"type": "path", "path": "/mnt/source"}, "access": "read"},
        ]}
        profile = {"type": "managed", "file_system": fs, "network": "restricted"}
        context = {"cwd": str(session.PROJECT), "approval_policy": "never",
                   "permission_profile": profile}
        events = [
            {"type": "session_meta", "payload": {"id": "fixture"}},
            {"type": "turn_context", "payload": context},
            {"type": "event_msg", "payload": {"type": "task_complete", "last_agent_message": "report"}},
        ]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rollout.jsonl"
            path.write_text("".join(json.dumps(event) + "\n" for event in events))
            self.assertEqual(session.read_session(path, "fixture reviewer")[3], "report")
            aborted = {"type": "event_msg", "payload": {"type": "turn_aborted"}}
            extra_read = {"path": {"type": "path", "path": "/mnt/home"}, "access": "read"}
            writable = {"path": {"type": "path", "path": "/mnt/source"}, "access": "write"}
            for settings, tail in ((dict(profile, network="enabled"), []), (profile, [aborted]),
                (dict(profile, file_system=dict(fs, entries=fs["entries"] + [extra_read])), []),
                (dict(profile, file_system=dict(fs, entries=fs["entries"] + [writable])), [])):
                with self.subTest(settings=settings, tail=tail), self.assertRaises(ValueError):
                    context["permission_profile"] = settings
                    path.write_text("".join(json.dumps(event) + "\n" for event in events + tail))
                    session.read_session(path, "fixture reviewer")

    def test_incomplete_ambiguous_or_wrong_target_report_is_rejected(self):
        target = "a" * 40 + " worktree " + "b" * 64
        packet = "c" * 64
        complete = "Review status: complete " + target
        session.verify_report(f"Open important finding. Packet {packet}\n{complete}", packet, target)
        for status in ("", "Review status: incomplete", "Review status: complete " + target.replace("a" * 40, "d" * 40),
                       f"Review status: incomplete\n{complete}", complete + "\nmore"):
            with self.subTest(status=status), self.assertRaises(ValueError):
                session.verify_report(f"Packet {packet}\n{status}", packet, target)
        with self.assertRaises(ValueError):
            session.verify_report(complete, packet, target)

    def test_conflicting_or_repeated_metadata_in_one_log_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "child.jsonl"
            original = {"id": "child", "parent_thread_id": "parent", "agent_role": "reviewer"}
            for second in (original, dict(original, id="other"),
                           dict(original, parent_thread_id="other", agent_role="different")):
                with self.subTest(second=second):
                    path.write_text("".join(json.dumps({"type": "session_meta", "payload": meta}) + "\n"
                                            for meta in (original, second)))
                    with self.assertRaisesRegex(ValueError, "ambiguous native session metadata"):
                        session.read_session(path, "fixture instructions")

    def test_duplicate_native_session_ids_are_rejected(self):
        result = ({"id": "duplicate"}, [], True, "report", [], [])
        with patch.object(session, "read_session", return_value=result), self.assertRaises(ValueError):
            session.collect_sessions(["one", "two"], "instructions")

    def test_worktree_target_binds_exact_trailing_patch_bytes(self):
        patch_bytes = b"diff fixture\n"
        target = "a" * 40 + " worktree " + hashlib.sha256(patch_bytes).hexdigest()
        packet = f"Target: {target}\nRequirements\n--- BEGIN FROZEN GIT PATCH ---\n".encode() + patch_bytes
        with patch.object(session.subprocess, "check_output", return_value=b"commit\n"):
            self.assertEqual(session.review_target(packet, "/usr/bin/false"), target)
            with self.assertRaises(ValueError):
                session.review_target(packet.rstrip(), "/usr/bin/false")

    def test_concurrent_edit_uses_pinned_mounts(self):
        required = os.environ.get("SILICONWATCH_REQUIRE_REVIEW_SNAPSHOT") == "1"
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "etc/codex").mkdir(parents=True)
            ambient = root / "home/.codex"
            ambient.mkdir(parents=True)
            role = root / "checkout" / launch.ROLE
            role.parent.mkdir(parents=True)
            config = ambient / "config.toml"
            policy = root / "etc/codex/requirements.toml"
            packet = root / "packet"
            original_packet = "Review the frozen change.\n"
            original_role = 'name = "reviewer"\n'
            original_policy = "[mcp_servers]\n# retained administrative policy\n"
            original_config = "# synthetic ambient user settings\n"
            role.write_text(original_role)
            config.write_text(original_config)
            policy.write_text(original_policy)
            packet.write_text(original_packet)
            (root / "checkout/fixture.txt").write_text("intended source")
            (root / "observer").write_bytes(OBSERVER)
            host = isolated_host(root)
            env = {"HOME": "/inputs/home", "CODEX_HOME": "/inputs/home/.codex", "PATH": os.defpath}
            # Probe independently so bugs in the real launcher can never become skips.
            try:
                probe = subprocess.run([*host, "--", "/usr/bin/bwrap", "--unshare-user", "--unshare-pid",
                                        "--ro-bind", "/", "/", "--overlay-src", "/etc", "--tmp-overlay",
                                        "/etc", "--", "/usr/bin/true"], env=env, capture_output=True,
                                       text=True, timeout=15)
                limitation = ""
                if probe.returncode:
                    limitation = probe.stderr.strip() or f"Bubblewrap exited {probe.returncode}"
            except FileNotFoundError as exc:
                limitation = str(exc)
            driver = [*host, "--", "/usr/bin/python3", "-I", "-c", DRIVER]
            if limitation:
                try:
                    failed = subprocess.run(driver, env=env, input="", capture_output=True,
                                            text=True, timeout=15)
                except FileNotFoundError as exc:
                    self.assertEqual(exc.filename, "/usr/bin/bwrap")
                    self.assertFalse(Path(exc.filename).exists())
                else:
                    self.assertNotEqual(failed.returncode, 0)
                    self.assertEqual(failed.stdout, "")
                    self.assertIn(limitation, failed.stderr,
                                  "fixture launcher failed for a different reason than the prerequisite probe")
                if required:
                    self.fail(limitation)
                self.skipTest("verified launch failure; Bubblewrap user namespace/overlay unavailable: " + limitation)

            with subprocess.Popen(driver,
                                  env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, text=True) as process:
                def expect(stage):
                    self.assertTrue(select.select([process.stdout], [], [], 15)[0], stage + " timed out")
                    line = process.stdout.readline()
                    if not line:
                        self.fail(f"{stage} unavailable: {process.stderr.read()}")
                    self.assertEqual(json.loads(line), [stage, original_role, launch.CONFIG.decode(),
                        original_policy, ["reviewer.toml"], [errno.EROFS] * 6, "intended source",
                        original_packet, False, 1])

                def resume():
                    process.stdin.write("continue\n")
                    process.stdin.flush()

                try:
                    expect("preflight")
                    replace_text(role, 'name = "alternate"\n')
                    replace_text(config, '[mcp_servers.injected]\ncommand = "/usr/bin/false"\n')
                    replace_text(policy, '# changed administrative policy\n')
                    replace_text(packet, "replacement review packet\n")
                    sibling = role.with_name("injected.toml")
                    sibling.write_text('name = "injected"\n')
                    resume()
                    expect("launch")
                    replace_text(role, original_role)
                    replace_text(config, original_config)
                    replace_text(policy, original_policy)
                    replace_text(packet, original_packet)
                    sibling.unlink()
                    resume()
                    expect("restored")
                    self.assertEqual(process.wait(timeout=15), 0)
                finally:
                    if process.poll() is None:
                        process.kill()


if __name__ == "__main__":
    unittest.main()
