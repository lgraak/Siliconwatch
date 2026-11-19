"""Encrypt candidate inputs and outputs before handing them to GitHub Actions storage."""
import argparse
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile

MAX_BYTES = 2 * 1024**3


def crypt(source, target, key, home, *, decrypt):
    if not re.fullmatch(r"[0-9a-fA-F]{64}", key):
        raise ValueError("CANDIDATE_KEY must contain a 256-bit key encoded as 64 hex digits")
    options = ["--max-output", str(MAX_BYTES), "--decrypt"] if decrypt else ["--symmetric", "--cipher-algo", "AES256", "--s2k-count", "65011712", "--compress-algo", "none"]
    subprocess.run(
        ["gpg", "--batch", "--no-tty", "--homedir", str(home), "--pinentry-mode", "loopback",
         "--no-symkey-cache", "--passphrase-fd", "0", "--output", str(target), *options, str(source)],
        input=key + "\n", text=True, check=True,
        env={k: v for k, v in os.environ.items() if k not in ("CANDIDATE_KEY", "GH_TOKEN", "GITHUB_TOKEN")},
    )


def pack(source, target, key):
    if source.is_symlink() or not source.is_dir() or os.path.lexists(target):
        raise ValueError("Expected an input directory and an unused envelope path")
    with tempfile.TemporaryDirectory(dir=target.parent) as temp:
        temp = Path(temp)
        archive = temp / "payload.tar"
        with tarfile.open(archive, "w", dereference=True) as out:
            for path in sorted(source.rglob("*")):
                if path.is_symlink() or not (path.is_dir() or path.is_file()):
                    raise ValueError(f"Unexpected file type: {path}")
                out.add(path, arcname=path.relative_to(source), recursive=False)
        if archive.stat().st_size > MAX_BYTES:
            raise ValueError("Release envelope exceeds the size limit")
        crypt(archive, temp / "payload.gpg", key, temp, decrypt=False)
        (temp / "payload.gpg").rename(target)


def unpack(source, target, key, *, expected=None):
    if os.path.lexists(target) or source.is_symlink() or not source.is_file():
        raise ValueError("Expected an encrypted file and an unused output directory")
    with tempfile.TemporaryDirectory(dir=target.parent) as temp:
        temp = Path(temp)
        archive, output = temp / "payload.tar", temp / "output"
        output.mkdir()
        crypt(source, archive, key, temp, decrypt=True)
        seen = set()
        with tarfile.open(archive, "r:") as inp:
            for member in inp:
                if member.isdir() and member.name == ".":
                    continue
                path = Path(member.name)
                regular = member.type in (tarfile.REGTYPE, tarfile.AREGTYPE) and not member.sparse
                if (not path.parts or path.is_absolute() or ".." in path.parts or path in seen
                        or not (member.isdir() or regular) or len(seen) >= 65536):
                    raise ValueError(f"Invalid envelope member: {member.name!r}")
                if expected is not None and (not regular or str(path) not in expected):
                    raise ValueError("Unexpected candidate file")
                seen.add(path)
                dest = output / path
                if member.isdir():
                    dest.mkdir(parents=True, exist_ok=True)
                else:
                    dest.parent.mkdir(parents=True, exist_ok=True)
                    with inp.extractfile(member) as src, dest.open("xb") as dst:
                        shutil.copyfileobj(src, dst)
        if expected is not None and {str(path) for path in seen} != expected:
            raise ValueError("Candidate envelope is incomplete")
        output.rename(target)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("pack", "unpack"))
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    args = parser.parse_args()
    key = os.environ.get("CANDIDATE_KEY", "")
    (pack if args.operation == "pack" else unpack)(args.source, args.target, key)
