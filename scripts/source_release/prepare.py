#!/usr/bin/env python3
"""Assemble matching offline-buildable source and reviewed notices for Linux packages."""

import argparse
import gzip
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib
import urllib.request

from registry import (
    CANTARELL_ASSET, CANTARELL_NOTICE, CANTARELL_SHA256,
    SCTK_HEADERBAR, SCTK_MPL_NOTICE, slint_supplements, source_registry,
)

TARGET = "x86_64-unknown-linux-gnu"
PROJECTS = {"siliconwatch", "siliconwatch-gui"}
REVIEWED_PACKAGE = "siliconwatch-0.1.0-alpha.1-linux-x86_64.pkg.tar.zst"
REVIEWED_URL = f"https://github.com/lgraak/Siliconwatch/releases/download/v0.1.0-alpha.1/{REVIEWED_PACKAGE}"
REVIEWED_SHA256 = "8b4b606834079946aaaffe3a4ffc12067fa7448c40b26d2855fd2e8593f0c2b4"
# Registry names, versions and immutable crate checksums covered by the Alpha 1 review.
REVIEWED_REGISTRY = "e358a73cbfa6dd409f68a995363f4785d521df503640392c1a0812b65713fe4c"
REVIEWED_FONTDUE_TREE = "dfd664127501feff001d067ea6fd245cf1b09493"


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def output(*args, cwd):
    return subprocess.check_output(args, cwd=cwd, text=True).strip()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def read_toml(path):
    with path.open("rb") as stream:
        return tomllib.load(stream)


def selected_packages(source, edges):
    lines = output(
        "cargo", "+1.85.0", "tree", "--workspace", "--locked", "--offline",
        "--target", TARGET, "--edges", edges, "--prefix", "none",
        "--format", "{p}", cwd=source,
    ).splitlines()
    selected = set()
    for line in lines:
        if line:
            name, version, *_ = line.split()
            if not version.startswith("v"):
                raise ValueError(f"unrecognized Cargo tree entry: {line}")
            selected.add((name, version[1:]))
    return selected


def dependency_inventory(source):
    selected = selected_packages(source, "normal,build")
    packages = read_toml(source / "Cargo.lock")["package"]
    crates = sorted((p["name"], p["version"], p["checksum"]) for p in packages
                    if "source" in p and (p["name"], p["version"]) in selected)
    digest = hashlib.sha256(json.dumps(crates, separators=(",", ":")).encode()).hexdigest()
    if digest != REVIEWED_REGISTRY:
        raise ValueError("Linux dependency closure changed; update the reviewed notice inputs and checksum")
    return selected


def add_sctk_notices(source, licenses, entry):
    for filename in (CANTARELL_NOTICE, SCTK_MPL_NOTICE):
        shutil.copyfile(source / "docs/legal/source-notices" / filename, licenses / filename)
        entry["license_files"].append(filename)
    entry["bundled_resources"] = [{
        "path": CANTARELL_ASSET, "sha256": CANTARELL_SHA256,
        "license": "OFL-1.1", "license_file": CANTARELL_NOTICE,
    }]
    entry["source_component"] = SCTK_HEADERBAR


def prepare_notices(source, work, licenses, archive, version):
    selected = dependency_inventory(source)
    if archive is None:
        archive = work / REVIEWED_PACKAGE
        with urllib.request.urlopen(REVIEWED_URL, timeout=90) as response, archive.open("xb") as out:
            shutil.copyfileobj(response, out)
    if sha256(archive) != REVIEWED_SHA256:
        raise ValueError("reviewed Alpha 1 notice archive checksum mismatch")
    licenses.mkdir()
    subprocess.run(
        ["tar", "-xf", archive, "--strip-components=4", "-C", licenses, "usr/share/licenses/siliconwatch"],
        check=True,
    )
    index_path = licenses / "THIRD-PARTY.json"
    index = json.loads(index_path.read_text())
    supplement = "NOTICE-simd_helpers-0.1.0.txt"
    shutil.copyfile(source / "docs/legal" / supplement, licenses / supplement)
    shutil.copyfile(source / "LICENSE", licenses / "LICENSE-GPL-3.0")
    entries = index["rust_packages"]
    for entry in entries:
        if entry["name"] in PROJECTS:
            entry["version"] = version
        if (entry["name"], entry["version"]) == ("sctk-adwaita", "0.10.1"):
            add_sctk_notices(source, licenses, entry)
        if (entry["name"], entry["version"]) == ("i-slint-compiler", "1.13.1"):
            shutil.copytree(source / "docs/legal/source-notices/slint", licenses / "slint")
            entry["license_files"].extend("slint/" + f for f in slint_supplements(licenses / "slint"))
            entry["artwork_attributions"] = {
                "compiled_style": "fluent", "upstream_directory": "internal/compiler",
                "workspace_annotations": "slint/REUSE.toml", "review": "slint/ASSET-NOTICES.md",
            }
        if entry["name"] == "simd_helpers":
            entry["license_files"] = [supplement]
            entry["notice_source"] = "upstream_manifest_and_reviewed_supplement"
        if not entry["license_files"] or any(not (licenses / p).is_file() for p in entry["license_files"]):
            raise ValueError(f"missing reviewed notices for {entry['name']}")
    if selected != {(p["name"], p["version"]) for p in entries}:
        raise ValueError("active Cargo packages differ from the reviewed notice inventory")
    runtime = {(p["name"], p["version"]) for p in entries if p["dependency_role"] == "normal_non_proc_macro"}
    if runtime != selected_packages(source, "normal,no-proc-macro"):
        raise ValueError("normal and build dependency roles changed; review the notice classifications")
    index["source_availability"] = "The matching release source bundle includes the project and the locked source archives needed by the x86-64 Linux Cargo resolver."
    write_json(index_path, index)
    (licenses / "README-NOTICES.md").write_text(
        "# SiliconWatch third-party notices\n\n"
        "THIRD-PARTY.json lists the locked x86-64 Linux build and normal dependencies. "
        "The notice set starts with the checksum-pinned Alpha 1 package. The sctk-adwaita and Slint entries add the bundled resource credits and terms. "
        "The simd_helpers supplement records the upstream MIT declaration and reference terms. "
        "SOURCE-NOTICES.json in the corresponding-source bundle lists its redistributed registry crates. The bundle also includes patched fontdue.\n"
    )
    # The old manifest describes a different package. Regenerate the inventory from these exact files.
    inventory = licenses / "NOTICE-PAYLOAD-INVENTORY.json"
    sums = licenses / "SHA256SUMS"
    files = sorted(p for p in licenses.rglob("*") if p.is_file() and p not in (inventory, sums))
    write_json(inventory, {"files": [
        {"file": p.relative_to(licenses).as_posix(), "bytes": p.stat().st_size, "sha256": sha256(p)}
        for p in files
    ]})
    sums.write_text("".join(f"{sha256(p)}  {p.relative_to(licenses).as_posix()}\n"
                            for p in sorted(files + [inventory])))
    return {(entry["name"], entry["version"]): entry["license_files"] for entry in entries}


def write_archive(root, path, timestamp):
    # The two archive headers use fixed metadata so a second run produces the same bytes.
    command = ["tar", "--sort=name", "--format=gnu", "--owner=0", "--group=0", "--numeric-owner",
               f"--mtime=@{timestamp}", "--mode=u+rwX,go+rX,go-w", "-cf", "-", root.name]
    with path.open("xb") as out, gzip.GzipFile(filename="", fileobj=out, mode="wb", compresslevel=1, mtime=0) as zipped:
        with subprocess.Popen(command, cwd=root.parent, stdout=subprocess.PIPE) as proc:
            shutil.copyfileobj(proc.stdout, zipped, length=1024 * 1024)
            if proc.wait():
                raise RuntimeError("could not archive vendored source")


def prepare(checkout, commit, version, destination, reviewed):
    checkout = checkout.resolve()
    if not re.fullmatch(r"[0-9a-f]{40}", commit) or output("git", "rev-parse", "HEAD", cwd=checkout) != commit:
        raise ValueError("checkout HEAD must match the full candidate commit")
    number = r"(?:0|[1-9][0-9]*)"
    if not re.fullmatch(rf"{number}\.{number}\.{number}(?:-(?:alpha|beta|rc)\.{number})?", version):
        raise ValueError("invalid native release version")
    if output("git", "rev-parse", f"{commit}:vendor/fontdue-0.9.5", cwd=checkout) != REVIEWED_FONTDUE_TREE:
        raise ValueError("vendored fontdue changed; review patch and notices")
    destination = destination.resolve()
    if destination.exists() or destination.is_relative_to(checkout):
        raise ValueError("source output must be a new directory outside the checkout")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination.parent) as temp:
        work = Path(temp)
        result = work / "output"
        result.mkdir()
        root = work / f"siliconwatch-{version}-source"
        root.mkdir()
        snapshot = work / "project.tar"
        subprocess.run(["git", "archive", "--format=tar", "-o", snapshot, commit], cwd=checkout, check=True)
        subprocess.run(["tar", "-xf", snapshot, "-C", root], check=True)
        for manifest in (root / "Cargo.toml", root / "gui/Cargo.toml"):
            if read_toml(manifest)["package"]["version"] != version:
                raise ValueError(f"release version does not match {manifest.relative_to(root)}")
        licenses = result / "licenses"
        notices = prepare_notices(root, work, licenses, reviewed, version)
        config = root / ".cargo/config.toml"
        registry = root / "vendor/registry"
        if config.exists() or registry.exists() or (root / ".cargo/config").exists():
            raise ValueError("source-replacement configuration requires review")
        shutil.copytree(licenses, root / "release-notices")
        source_registry(root, TARGET, read_toml(root / "Cargo.lock"), notices)
        config.parent.mkdir(exist_ok=True)
        config.write_text('[source.crates-io]\nreplace-with = "release-vendor"\n\n'
                          '[source.release-vendor]\nlocal-registry = "vendor/registry"\n')
        (root / "SOURCE-REVISION").write_text(commit + "\n")
        (root / "BUILD-FROM-SOURCE.md").write_text(
            "# Build the release source\n\nInstall Rust 1.85.0 and the native Linux development libraries listed for your distribution in "
            "packaging/linux/container.sh (the build section). From this directory run "
            "`cargo +1.85.0 build --workspace --locked --offline --release`. "
            "Cargo unpacks the included original .crate source archives and uses the local fontdue patch. "
            "SOURCE-REVISION records the Git commit. SOURCE-NOTICES.json maps every distributed registry archive to its notices and identifies resolver-only entries. release-notices/ contains the Linux binary notices. The source bundle targets x86_64-unknown-linux-gnu.\n"
        )
        archive = result / f"{root.name}.tar.gz"
        timestamp = output("git", "show", "-s", "--format=%ct", commit, cwd=checkout)
        write_archive(root, archive, timestamp)
        write_json(result / "SOURCE-PROVENANCE.json", {
            "schema_version": 1, "version": version, "commit": commit, "target": TARGET,
            "cargo_lock_sha256": sha256(root / "Cargo.lock"),
            "source_archive": archive.name, "source_sha256": sha256(archive),
            "notices_sha256": sha256(licenses / "SHA256SUMS"),
        })
        result.rename(destination)


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--source", type=Path, required=True)
    p.add_argument("--commit", required=True)
    p.add_argument("--version", required=True, help="Cargo release version without a v prefix")
    p.add_argument("--output", type=Path, required=True)
    p.add_argument("--reviewed-package", type=Path, help="cached checksum-pinned Alpha 1 archive for offline operation")
    args = p.parse_args()
    prepare(args.source, args.commit, args.version, args.output, args.reviewed_package)
