#!/usr/bin/env python3
"""Seal or verify the exact files submitted for release approval."""

import argparse
import gzip
import hashlib
import io
import json
import os
import re
import shutil
import tarfile
import tempfile
from pathlib import Path

NUMBER = r"(?:0|[1-9][0-9]*)"
VERSION = re.compile(rf"v{NUMBER}\.{NUMBER}\.{NUMBER}(?:-(?:alpha|beta|rc)\.{NUMBER})?")
MAX_REPORT_FILE_BYTES = 16 * 1024**2
MAX_REPORT_ARCHIVE_BYTES = 64 * 1024**2
MAX_RECEIPT_BYTES = 4096
PACKAGE_SUFFIX = {"debian": "amd64.deb", "ubuntu": "amd64.deb", "fedora": "x86_64.rpm", "arch": "x86_64.pkg.tar.zst"}
REPORTS = {
    *(f"build-{distro}.log" for distro in ("debian", "fedora", "arch")),
    *(f"payload-{distro}.json" for distro in ("debian", "fedora", "arch")),
    *(f"install-{distro}.{kind}" for distro in PACKAGE_SUFFIX for kind in ("log", "json")),
    "ci.json", "SOURCE-PROVENANCE.json",
}


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def check_identity(version, commit):
    require(VERSION.fullmatch(version), "Unsupported release tag")
    require(re.fullmatch(r"[a-f0-9]{40}", commit), "Expected a full source commit SHA")


def package_name(version, distro):
    return f"siliconwatch-{version[1:]}-linux-{PACKAGE_SUFFIX[distro]}"


def packages(version):
    return {package_name(version, distro) for distro in PACKAGE_SUFFIX}


def source_name(version):
    return f"siliconwatch-{version[1:]}-source.tar.gz"


def payload_names(version):
    return packages(version) | {source_name(version), "RELEASE-NOTES.md", "verification-reports.tar.gz"}


def bundle_names(version):
    return payload_names(version) | {"SHA256SUMS", "candidate.json"}


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def regular(path):
    require(not path.is_symlink() and path.is_file() and path.stat().st_size > 0,
            f"Expected a nonempty regular file: {path.name}")


def exact_files(directory, expected):
    require(not directory.is_symlink() and directory.is_dir(), "Expected a real directory")
    actual = {path.name for path in directory.iterdir()}
    require(actual == expected,
            f"Unexpected inventory in {directory.name}: missing {sorted(expected-actual)}, extra {sorted(actual-expected)}")
    for name in expected:
        regular(directory / name)


def metadata(directory, names):
    return {name: {"bytes": (directory / name).stat().st_size, "sha256": sha256(directory / name)}
            for name in sorted(names)}


def checksums(files):
    return "".join(f"{info['sha256']}  {name}\n" for name, info in sorted(files.items()))


class NativeReportInfo(tarfile.TarInfo):
    """Reject extended headers before tarfile follows or expands them."""

    @classmethod
    def fromtarfile(cls, archive):
        # Only the ordinary headers emitted by seal are part of this format.
        # Inspect the physical header before tarfile interprets extension records.
        offset = archive.fileobj.tell()
        header = archive.fileobj.read(tarfile.BLOCKSIZE)
        member = tarfile.TarInfo.frombuf(header, archive.encoding, archive.errors)
        require(member.type in (tarfile.REGTYPE, tarfile.AREGTYPE) and member.name in REPORTS,
                f"Unexpected verification report member: {member.name}")
        require(0 < member.size <= MAX_REPORT_FILE_BYTES, f"Invalid verification report size: {member.name}")
        archive.fileobj.seek(offset)
        return super().fromtarfile(archive)


def verify_install_receipts(archive_path, version, commit, files):
    expected = {}
    for distro in PACKAGE_SUFFIX:
        name = package_name(version, distro)
        expected[f"install-{distro}.json"] = {
            "schema": "siliconwatch.install.v1", "result": "passed", "distro": distro,
            "tag": version, "source_commit": commit, "package": name, "sha256": files[name]["sha256"],
        }
    def unique_fields(pairs):
        record = dict(pairs)
        require(len(record) == len(pairs), "Duplicate field in install receipt")
        return record

    # Cap the actual expanded bytes before parsing any tar extension, member or padding.
    with gzip.open(archive_path, "rb") as compressed:
        expanded = compressed.read(MAX_REPORT_ARCHIVE_BYTES + 1)
    require(len(expanded) <= MAX_REPORT_ARCHIVE_BYTES, "Expanded verification report archive exceeds size limit")
    seen = set()
    with tarfile.open(fileobj=io.BytesIO(expanded), mode="r:", tarinfo=NativeReportInfo) as archive:
        for member in archive:
            require(member.name not in seen, f"Duplicate verification report member: {member.name}")
            seen.add(member.name)
            if member.name in expected:
                require(member.size <= MAX_RECEIPT_BYTES, f"Invalid install receipt size: {member.name}")
                with archive.extractfile(member) as stream:
                    receipt = json.load(stream, object_pairs_hook=unique_fields)
                require(receipt == expected[member.name], f"Install receipt mismatch: {member.name}")
        padding = memoryview(expanded)[archive.offset:]
        require(len(padding) >= 2 * tarfile.BLOCKSIZE and len(padding) % tarfile.BLOCKSIZE == 0 and not any(padding),
                "Unexpected data or incomplete padding at the end of verification reports")
    require(seen == REPORTS, f"Missing verification reports: {sorted(REPORTS - seen)}")


def verify(directory, version, commit, digest):
    check_identity(version, commit)
    require(re.fullmatch(r"[0-9a-f]{64}", digest), "Expected an approved SHA-256 digest")
    exact_files(directory, bundle_names(version))
    manifest = directory / "candidate.json"
    require(sha256(manifest) == digest, "Candidate manifest differs from approval")
    files = metadata(directory, payload_names(version))
    require((directory / "SHA256SUMS").read_bytes() == checksums(files).encode(), "Invalid SHA256SUMS")
    files.update(metadata(directory, {"SHA256SUMS"}))
    expected = {"schema": 1, "tag": version, "source_commit": commit, "files": files}
    require(json.loads(manifest.read_bytes()) == expected, "Version, source commit or candidate file mismatch")
    require((directory / "RELEASE-NOTES.md").read_text().splitlines()[0] == f"# SiliconWatch {version}",
            "Release notes do not identify the candidate")
    verify_install_receipts(directory / "verification-reports.tar.gz", version, commit, files)
    return files | metadata(directory, {"candidate.json"})


def seal(version, commit, package_dir, source, notes, reports, output):
    check_identity(version, commit)
    exact_files(package_dir, packages(version))
    exact_files(reports, REPORTS)
    sizes = [(reports / name).stat().st_size for name in REPORTS]
    require(max(sizes) <= MAX_REPORT_FILE_BYTES and sum(sizes) <= MAX_REPORT_ARCHIVE_BYTES,
            "Verification report inputs exceed size limits")
    regular(source)
    regular(notes)
    require(source.name == source_name(version), "Unexpected corresponding-source archive name")
    ci = json.loads((reports / "ci.json").read_bytes())
    require(ci.get("result") == "success" and ci.get("source_commit") == commit, "CI verification mismatch")
    provenance_path = reports / "SOURCE-PROVENANCE.json"
    provenance = json.loads(provenance_path.read_bytes())
    required = {"schema_version": 1, "commit": commit, "version": version[1:],
                "source_archive": source.name, "source_sha256": sha256(source)}
    require(all(provenance.get(key) == value for key, value in required.items()),
            "Corresponding source does not match the candidate")
    provenance_hash = sha256(provenance_path)
    for name in sorted(REPORTS):
        if name.startswith("payload-"):
            rows = json.loads((reports / name).read_bytes())
            require(isinstance(rows, list), f"Invalid native payload manifest: {name}")
            packaged = [row.get("sha256") for row in rows
                        if row.get("path") == "/usr/share/doc/siliconwatch/SOURCE-PROVENANCE.json"]
            require(packaged == [provenance_hash], f"Packaged provenance differs: {name}")
    require(not os.path.lexists(output), "Candidate destination already exists")
    with tempfile.TemporaryDirectory(dir=output.parent) as scratch:
        directory = Path(scratch) / "bundle"
        directory.mkdir()
        for name in packages(version):
            shutil.copyfile(package_dir / name, directory / name)
        shutil.copyfile(source, directory / source.name)
        shutil.copyfile(notes, directory / "RELEASE-NOTES.md")
        with gzip.GzipFile(directory / "verification-reports.tar.gz", "wb", mtime=0) as compressed:
            with tarfile.open(fileobj=compressed, mode="w|", format=tarfile.USTAR_FORMAT) as archive:
                for name in sorted(REPORTS):
                    info = tarfile.TarInfo(name)
                    info.size = (reports / name).stat().st_size
                    with (reports / name).open("rb") as report:
                        archive.addfile(info, report)
        files = metadata(directory, payload_names(version))
        (directory / "SHA256SUMS").write_text(checksums(files))
        files.update(metadata(directory, {"SHA256SUMS"}))
        manifest = {"schema": 1, "tag": version, "source_commit": commit, "files": files}
        (directory / "candidate.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        digest = sha256(directory / "candidate.json")
        verify(directory, version, commit, digest)
        directory.rename(output)
    return digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("seal")
    check = commands.add_parser("verify")
    for action in (build, check):
        action.add_argument("--version", required=True)
        action.add_argument("--commit", required=True)
    for option in ("packages", "source", "notes", "reports", "output"):
        build.add_argument("--" + option, required=True, type=Path)
    check.add_argument("--candidate", required=True, type=Path)
    check.add_argument("--digest", required=True)
    args = parser.parse_args()
    if args.command == "seal":
        print(seal(args.version, args.commit, args.packages, args.source, args.notes, args.reports, args.output))
    else:
        verify(args.candidate, args.version, args.commit, args.digest)
        print("Candidate contents match the approved digest, version and source commit.")


if __name__ == "__main__":
    main()
