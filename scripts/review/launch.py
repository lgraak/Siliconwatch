"""Run one SiliconWatch review using controls from a maintainer-selected commit."""

import argparse
from contextlib import ExitStack
import fcntl
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib


PROJECT = "/mnt/project"
HOME = "/mnt/home"
CODEX_HOME = HOME + "/.codex"
STATE = "/mnt/state"
POLICY = Path("/etc/codex/requirements.toml")
ROLE = ".codex/agents/reviewer.toml"
SESSION = "scripts/review/session.py"
CONFIG = b'''project_doc_max_bytes = 0
default_permissions = "review"
web_search = "disabled"
approval_policy = "never"
sqlite_home = "/mnt/state"
log_dir = "/mnt/state"
skills.bundled.enabled = false
skills.include_instructions = false
shell_environment_policy.inherit = "none"
shell_environment_policy.set = { PATH = "/usr/bin:/bin" }
[permissions.review.filesystem]
":minimal" = "read"
"/mnt/project" = "read"
"/mnt/source" = "read"
[permissions.review.network]
enabled = false
[projects."/mnt/project"]
trust_level = "trusted"
[features]
apps = false
plugins = false
hooks = false
memories = false
request_permissions_tool = false
[mcp_servers.siliconwatch_policy_probe]
command = "/usr/bin/false"
'''


def control_blob(repository, commit, path):
    """Read a regular file from the trusted Git object store, ignoring replace refs."""
    git = ["/usr/bin/git", "--no-replace-objects", "-C", str(repository)]
    env = {"PATH": os.defpath, "GIT_CONFIG_NOSYSTEM": "1", "GIT_CONFIG_GLOBAL": "/dev/null"}
    entry = subprocess.check_output([*git, "ls-tree", commit, "--", path], env=env).split()
    if len(entry) != 4 or entry[:2] != [b"100644", b"blob"] or entry[3] != path.encode():
        raise ValueError(f"missing regular control file at {commit}:{path}")
    data = subprocess.check_output([*git, "cat-file", "blob", entry[2].decode()], env=env)
    if subprocess.check_output([*git, "hash-object", "--stdin"], input=data, env=env).strip() != entry[2]:
        raise ValueError(f"unverified Git blob for {path}")
    return data


def mcp_policy():
    try:
        policy = POLICY.read_bytes()
    except FileNotFoundError:
        if POLICY.is_symlink():
            raise ValueError(f"unreadable system policy at {POLICY}")
        return b"[mcp_servers]\n"
    if tomllib.loads(policy.decode()).get("mcp_servers") != {}:
        raise ValueError(f"existing {POLICY} must already enforce an empty MCP allowlist")
    return policy  # Retain all existing administrator requirements.


def in_snapshot(role, session, codex, commit, *, source=None, packet=b""):
    # Bubblewrap copies the exact bytes from already-open anonymous files. The
    # mount containing both discovery roots is read-only before Codex starts.
    source = Path.cwd() if source is None else Path(source)
    args = ["/usr/bin/bwrap", "--die-with-parent", "--unshare-user", "--unshare-pid",
            "--ro-bind", "/usr", "/usr", "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp",
            "--tmpfs", "/mnt", "--tmpfs", STATE, "--dir", CODEX_HOME,
            "--dir", PROJECT + "/.git/objects", "--dir", PROJECT + "/.git/refs",
            "--ro-bind", str(source.resolve(strict=True)), "/mnt/source",
            "--overlay-src", "/etc", "--tmp-overlay", "/etc", "--chdir", PROJECT]
    for path in map(Path, ("/bin", "/sbin", "/lib", "/lib64")):
        if path.is_symlink():
            args.extend(["--symlink", os.readlink(path), str(path)])
        elif path.is_dir():
            args.extend(["--ro-bind", str(path), str(path)])
    if not Path(codex).is_relative_to("/usr"):
        args.extend(["--ro-bind", codex, "/mnt/codex-cli"])
        codex = "/mnt/codex-cli"
    with ExitStack() as stack:
        descriptors = []
        for path, data in (
            (str(POLICY), mcp_policy()), (CODEX_HOME + "/config.toml", CONFIG),
            (PROJECT + "/" + ROLE, role), (PROJECT + "/review-packet.txt", packet), ("/mnt/session.py", session),
            (PROJECT + "/.git/HEAD", b"ref: refs/heads/review\n"),
        ):
            fd = os.memfd_create("siliconwatch-review", os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
            file = stack.enter_context(os.fdopen(fd, "w+b"))
            file.write(data)
            file.flush()
            fcntl.fcntl(fd, fcntl.F_ADD_SEALS, fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW
                        | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL)
            file.seek(0)
            descriptors.append(file.fileno())
            args.extend(["--ro-bind-data", str(file.fileno()), path])

        # Codex owns its transient state. None of these are configuration roots.
        for name in ("sessions", "tmp", "cache", "thread-writer-locks", "shell_snapshots"):
            args.extend(["--dir", f"{STATE}/{name}", "--symlink", f"{STATE}/{name}",
                         f"{CODEX_HOME}/{name}"])
        for name in ("installation_id", "session_index.jsonl", "session_index.jsonl.lock",
                     "auth.lock", "auth.json.lock", "models_cache.json"):
            args.extend(["--symlink", f"{STATE}/{name}", f"{CODEX_HOME}/{name}"])

        # Codex reads its normal login directly. The launcher never reads, copies
        # or refreshes credentials. An expired file login must be refreshed outside.
        auth = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")) / "auth.json"
        if auth.is_file():
            args.extend(["--ro-bind", str(auth.resolve(strict=True)), CODEX_HOME + "/auth.json"])
        env = {key: os.environ[key] for key in
               ("LANG", "LC_ALL", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY",
                "SSL_CERT_FILE", "SSL_CERT_DIR", "CODEX_API_KEY") if key in os.environ}
        env.update(HOME=HOME, CODEX_HOME=CODEX_HOME, XDG_CONFIG_HOME=HOME + "/.config",
                   TMPDIR=STATE + "/tmp", PATH=os.defpath, GIT_CONFIG_NOSYSTEM="1",
                   GIT_CONFIG_GLOBAL="/dev/null", GIT_NO_REPLACE_OBJECTS="1")
        args.extend(["--remount-ro", "/etc", "--remount-ro", "/mnt", "--",
                     "/usr/bin/python3", "-I", "/mnt/session.py", codex, commit,
                     hashlib.sha256(role).hexdigest()])
        return subprocess.run(args, env=env, pass_fds=descriptors).returncode


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("controls", help="full SHA of the inspected launcher, session and role")
    parser.add_argument("--source", type=Path, help="standalone checkout exposed read-only at /mnt/source")
    parser.add_argument("--repo", type=Path, default=Path.cwd(), help="trusted Git object store")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.controls):
        parser.error("use a full trusted commit SHA")
    # The invoked launcher itself is part of the maintainer's trust boundary.
    if Path(__file__).read_bytes() != control_blob(args.repo, args.controls, "scripts/review/launch.py"):
        parser.error("run the inspected launcher corresponding to the controls commit")
    role = control_blob(args.repo, args.controls, ROLE)
    session = control_blob(args.repo, args.controls, SESSION)
    codex = shutil.which("codex")
    if not codex:
        parser.error("Codex CLI is required")
    source = (args.source or args.repo).resolve(strict=True)
    if not (source / ".git").is_dir() or (source / ".git").is_symlink():
        parser.error("supply a standalone Git checkout as the review source")
    packet = sys.stdin.buffer.read()
    if not packet.strip():
        parser.error("missing review packet on stdin")
    return in_snapshot(role, session, str(Path(codex).resolve(strict=True)), args.controls,
                       source=source, packet=packet)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as exc:
        sys.exit(f"Independent review unavailable: {exc}")
