#!/usr/bin/env python3
"""Check native package content, modes, provenance and manager ownership."""
import argparse
import hashlib
import json
from pathlib import Path
import stat


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def require(condition, message):
    if not condition:
        raise SystemExit(str(message))


def source_inputs(directory, version, commit):
    archive = f"siliconwatch-{version}-source.tar.gz"
    record = json.loads((directory / "SOURCE-PROVENANCE.json").read_text())
    require(record["version"] == version and record["commit"] == commit, "Source version/SHA mismatch")
    require(record["source_archive"] == archive and record["source_sha256"] == digest(directory / archive),
            "Source archive name or digest mismatch")
    notices = directory / "licenses"
    sums = notices / "SHA256SUMS"
    require(record["notices_sha256"] == digest(sums), "Notice manifest digest mismatch")
    expected = {}
    for line in sums.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        require(name not in expected and not Path(name).is_absolute() and ".." not in Path(name).parts,
                f"Invalid or duplicate notice path: {name}")
        expected[name] = checksum
    actual = {str(p.relative_to(notices)): digest(p) for p in notices.rglob("*") if p.is_file() and p != sums}
    require(expected and expected == actual, "Notice payload differs from its manifest")


def installed(manifest, manager_paths, version, commit):
    docs = "/usr/share/doc/siliconwatch/"
    licenses = "/usr/share/licenses/siliconwatch/"
    executables = {"/usr/bin/siliconwatch", "/usr/bin/siliconwatch-gui"}
    fixed = executables | {"/usr/share/applications/siliconwatch.desktop", "/usr/share/pixmaps/siliconwatch.png",
                           *(docs + suffix for suffix in ("INSTALL.md", "GUI.md", "copyright", "SOURCE-PROVENANCE.json",
                                                         "api/local-v1.md", "api/local-v1.schema.json"))}
    expected = {row["path"]: row for row in manifest}
    require(len(expected) == len(manifest) and fixed <= expected.keys(), "Missing or duplicate package payload")
    require(licenses + "SHA256SUMS" in expected, "Missing third-party notices")
    manager_files = set()
    allowed_dirs = {str(parent) for name in expected for parent in Path(name).parents}
    for name in manager_paths.read_text().splitlines():
        path = Path(name)
        require(path.is_absolute() and ".." not in path.parts, path)
        st = path.lstat()
        if stat.S_ISDIR(st.st_mode):
            require(str(path) in allowed_dirs, f"Unexpected owned directory {path}")
            continue
        manager_files.add(str(path))
        require(stat.S_ISREG(st.st_mode) and st.st_uid == st.st_gid == 0, f"Invalid type/ownership: {path}")
    require(manager_files == set(expected), f"Manager inventory mismatch: {manager_files ^ set(expected)}")
    for path, row in expected.items():
        require(path in fixed or path.startswith(licenses), f"Unexpected payload path: {path}")
        file = Path(path)
        mode = stat.S_IMODE(file.stat().st_mode)
        require(mode == (0o755 if path in executables else 0o644) and oct(mode) == row["mode"], f"Wrong mode: {path}")
        require(file.stat().st_size == row["size"] and digest(file) == row["sha256"], f"Altered contents: {path}")
    record = json.loads(Path(docs + "SOURCE-PROVENANCE.json").read_text())
    require(record["commit"] == commit and record["version"] == version, "Installed provenance mismatch")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["installed", "removed", "source"])
    parser.add_argument("version")
    parser.add_argument("commit")
    parser.add_argument("input", type=Path)
    parser.add_argument("manager_paths", type=Path, nargs="?")
    args = parser.parse_args()
    if args.mode == "source":
        source_inputs(args.input, args.version, args.commit)
        return
    manifest = json.loads(args.input.read_text())
    if args.mode == "installed":
        installed(manifest, args.manager_paths, args.version, args.commit)
    else:
        for row in manifest:
            path = Path(row["path"])
            require(not path.exists() and not path.is_symlink(), f"Remained after native removal: {path}")
    print(f"Verified {len(manifest)} {args.mode} files")


if __name__ == "__main__":
    main()
