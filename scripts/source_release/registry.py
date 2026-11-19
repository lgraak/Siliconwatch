"""Build Cargo's offline local registry and map every redistributed crate to its notices."""

from collections import defaultdict
from fnmatch import fnmatchcase
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tomllib

CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"
# Covers the resolved Linux crate archives, all locked index rows and the selected
# attribution files. Refresh only after reviewing the changed source distribution.
REVIEWED_SOURCE_INVENTORY = "c3c8e4249b1a2e6283d135bd43599672d396111423994a44006cfc1179ea1ec5"
SOURCE_SUPPLEMENTS = {
    "drm-fourcc-2.2.0": "docs/legal/source-notices/drm-fourcc-2.2.0.txt",
    "gbm-sys-0.4.0": "docs/legal/source-notices/gbm-MIT.txt",
    "skia-bindings-0.87.0": "docs/legal/source-notices/rust-skia-MIT.txt",
    "skia-safe-0.87.0": "docs/legal/source-notices/rust-skia-MIT.txt",
}
SLINT_REVISION = "70ad5e38344ed600c2fc763da9787f2e7ef3db7f"
SLINT_ARTWORK = {
    "i-slint-compiler-1.13.1": "internal/compiler",
    "i-slint-backend-linuxkms-1.13.1": "internal/backends/linuxkms",
}

LEGAL_NAME = re.compile(
    r"(?:^|[-_.])(?:licen[cs]es?|copying|copyrights?|notices?|unlicense|authors?|"
    r"attributions?|patents?|credits?|acknowledg(?:e)?ments?)(?:$|[-_.])", re.I)
SCTK_MPL_NOTICE = "sctk-adwaita-headerbar-MPL-2.0.txt"
SCTK_HEADERBAR = {
    "path": "src/lib.rs", "portion": "rounded_headerbar_shape",
    "sha256": "dcdcb830e216cba81df5c5c58aed243159f81bded4fd7748081d8ba3d4cdd1bc",
    "upstream_notice": "MPL-2.0", "license_file": SCTK_MPL_NOTICE,
}
CANTARELL_NOTICE = "Cantarell-0.0.5-OFL.txt"
CANTARELL_ASSET = "src/title/Cantarell-Regular.ttf"
CANTARELL_SHA256 = "d15f25bcdaba2e25f54c32d4e29e68fedeb1b7a1d8fbc83f7c4475916e93b55f"
FONT_EXTENSIONS = set(".ttf .ttc .otf .otc .otb .ttx .woff .woff2 .bdf .pcf .pfa .pfb .eot .fon .fnt".split())
# Values are paths within the original archive. The TTC contains its own
# original FontTools no-rights-reserved attribution in both faces.
FONT_NOTICES = {
    "fontdb-0.23.0": {"tests/fonts/Tuffy.ttf": ["tests/fonts/LICENSE.txt"]},
    "i-slint-common-1.13.1": {
        "sharedfontdb/DejaVuSans.ttf": ["sharedfontdb/DejaVuSans.ttf.license", "LICENSES/LicenseRef-DejaVu-Font.txt"]},
    "sctk-adwaita-0.10.1": {CANTARELL_ASSET: []},
    "ttf-parser-0.25.1": {
        "examples/wasm/TTC.ttc": ["examples/wasm/TTC.ttc"],
        "tests/fonts/bitmap.otb": ["LICENSE-MIT", "LICENSE-APACHE"],
        "tests/fonts/colr_1.ttf": ["tests/fonts/colr_1_LICENSE"],
        "tests/fonts/colr_1_variable.ttf": ["tests/fonts/colr_1_LICENSE"],
        "tests/fonts/demo.ttf": ["LICENSE-MIT", "LICENSE-APACHE"],
        "tests/fonts/demo.ttx": ["LICENSE-MIT", "LICENSE-APACHE"],
    },
}


def slint_supplements(directory):
    return sorted(p.relative_to(directory).as_posix() for p in directory.rglob("*") if p.is_file())


def slint_artwork(supplements, archive, slug):
    """Map each SVG in these locked Slint archives to its upstream workspace annotation."""
    rules = tomllib.loads((supplements / "REUSE.toml").read_text())["annotations"]
    art = {}
    with tarfile.open(archive, "r:gz") as crate:
        for member in crate:
            if not member.isfile() or not member.name.endswith(".svg"):
                continue
            rel = Path(member.name).relative_to(slug).as_posix()
            upstream = SLINT_ARTWORK[slug] + "/" + rel
            matches = []
            for rule in rules:
                patterns = rule["path"]
                if isinstance(patterns, str):
                    patterns = [patterns]
                if any(fnmatchcase(upstream, pat) for pat in patterns):
                    matches.append({"attribution": rule["SPDX-FileCopyrightText"], "license": rule["SPDX-License-Identifier"]})
            if not matches:
                raise ValueError(f"Unmapped Slint artwork: {upstream}")
            art[rel] = matches
    return art


def digest_file(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def canonical(record):
    return json.dumps(record, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def index_path(name):
    if not re.fullmatch(r"[a-zA-Z0-9_-]+", name):
        raise ValueError("Invalid registry package name")
    name = name.lower()
    if len(name) <= 2:
        return Path(str(len(name)), name)
    if len(name) == 3:
        return Path("3", name[0], name)
    return Path(name[:2], name[2:4], name)


def locked_index_rows(cache, packages):
    # Cargo 1.85's sparse cache v3 stores a header followed by NUL-delimited
    # version and original index JSON pairs. Emit just the locked records in
    # Cargo's documented local-registry format. Refuse foreign cache formats.
    raw = cache.read_bytes()
    if not raw.startswith(b"\x03\x02\x00\x00\x00") or not raw.endswith(b"\x00"):
        raise ValueError(f"Unrecognized Cargo sparse index: {cache}")
    fields = raw[5:-1].split(b"\x00")
    if len(fields) % 2 != 1:
        raise ValueError(f"Truncated Cargo sparse index: {cache}")
    wanted = {p["version"]: p for p in packages}
    rows = {}
    for version, blob in zip(fields[1::2], fields[2::2], strict=True):
        version = version.decode()
        if version not in wanted:
            continue
        row = json.loads(blob)
        package = wanted[version]
        if (version in rows or row["name"] != package["name"] or row["vers"] != version
                or row["cksum"] != package["checksum"]):
            raise ValueError(f"Index identity or checksum mismatch: {package['name']} {version}")
        # Publication time is informational and was added to the sparse index
        # retrospectively. Keep resolution metadata independent of cache age.
        row.pop("pubtime", None)
        rows[version] = canonical(row)
    if rows.keys() != wanted.keys():
        raise ValueError(f"Missing locked index entry: {cache}")
    return rows


def inspect_crate(path, name, version):
    prefix = f"{name}-{version}"
    with tarfile.open(path, "r:gz") as crate:
        package = tomllib.loads(crate.extractfile(f"{prefix}/Cargo.toml").read().decode())["package"]
        if (package["name"], package["version"]) != (name, version):
            raise ValueError(f"Crate identity mismatch: {prefix}")
        declared = package.get("license-file")
        notices = []
        fonts = {}
        for member in crate:
            if not member.isfile() or member.size == 0:
                continue
            rel = Path(member.name).relative_to(prefix)
            # AUTHORS, nested LICENSES directories and third-party subtrees can
            # carry attributions beyond the package's declared primary license.
            if (LEGAL_NAME.search(rel.name) and rel.suffix.lower() not in (".rs", ".py")
                    or "licenses" in {part.lower() for part in rel.parts[:-1]}
                    or rel.name == "REUSE.toml" or rel.as_posix() == ".reuse/dep5"
                    or rel.as_posix() == declared):
                notices.append(rel.as_posix())
            if rel.suffix.lower() in FONT_EXTENSIONS:
                fonts[rel.as_posix()] = hashlib.file_digest(crate.extractfile(member), "sha256").hexdigest()
        if declared and declared not in notices:
            raise ValueError(f"Missing declared license file in {prefix}: {declared}")
    mapping = FONT_NOTICES.get(prefix, {})
    if fonts.keys() != mapping.keys():
        raise ValueError(f"Unreviewed font resource inventory: {prefix}")
    resources = {}
    for rel, references in mapping.items():
        if set(references) - set(notices) - {rel}:
            raise ValueError(f"Mapped font notice missing: {prefix}/{rel}")
        resources[rel] = {"sha256": fonts[rel], "notices_in_archive": references}
    if prefix == "sctk-adwaita-0.10.1":
        if fonts[CANTARELL_ASSET] != CANTARELL_SHA256:
            raise ValueError("Embedded Cantarell font differs from its reviewed revision")
        resources[CANTARELL_ASSET]["additional_notice"] = "release-notices/" + CANTARELL_NOTICE
    return package.get("license"), sorted(notices), resources


def source_registry(root, target, lock, notices):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "+1.85.0", "metadata", "--format-version", "1", "--filter-platform", target,
         "--locked", "--offline"], cwd=root))
    resolved = {(p["name"], p["version"]): Path(p["manifest_path"]).parent
                for p in metadata["packages"] if p["source"] == CRATES_IO}
    stores = {folder.parent for folder in resolved.values()}
    if len(stores) != 1:
        raise ValueError("Expected the fetched dependencies in one crates.io registry cache")
    store = stores.pop()
    cargo_registry = store.parent.parent
    cache = cargo_registry / "cache" / store.name
    index = cargo_registry / "index" / store.name / ".cache"
    registry = root / "vendor/registry"
    registry.mkdir()
    grouped = defaultdict(list)
    for p in lock["package"]:
        if "source" in p:
            if p["source"] != CRATES_IO:
                raise ValueError("Unreviewed dependency source")
            grouped[p["name"]].append(p)

    shipped, index_only = [], []
    index_hash = hashlib.sha256()
    for name, packages in sorted(grouped.items()):
        rel = index_path(name)
        rows = locked_index_rows(index / rel, packages)
        dest = registry / "index" / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        contents = b"".join(rows[v] + b"\n" for v in sorted(rows))
        dest.write_bytes(contents)
        index_hash.update(contents)
        for p in sorted(packages, key=lambda package: package["version"]):
            version = p["version"]
            ident = (name, version)
            slug = f"{name}-{version}"
            record = {"name": name, "version": version, "crate_sha256": p["checksum"]}
            if ident not in resolved:
                index_only.append(record)
                continue
            archive = registry / (slug + ".crate")
            shutil.copyfile(cache / archive.name, archive)
            if digest_file(archive) != p["checksum"]:
                raise ValueError(f"Source crate differs from Cargo.lock: {slug}")
            license_id, included, resources = inspect_crate(archive, name, version)
            extra = []
            if ident in notices:
                extra = ["release-notices/" + path for path in notices[ident]]
            if slug in SOURCE_SUPPLEMENTS:
                extra.append(SOURCE_SUPPLEMENTS[slug])
            if version == "1.13.1" and (name.startswith("i-slint-") or name.split("-")[0] == "slint"):
                with tarfile.open(archive, "r:gz") as crate:
                    vcs = json.load(crate.extractfile(f"{slug}/.cargo_vcs_info.json"))
                if vcs["git"]["sha1"] != SLINT_REVISION or not vcs["path_in_vcs"]:
                    raise ValueError(f"Unreviewed Slint workspace revision: {slug}")
                record["workspace_annotations"] = {
                    "file": "release-notices/slint/REUSE.toml", "revision": SLINT_REVISION,
                    "upstream_directory": vcs["path_in_vcs"],
                }
                extra.append("release-notices/slint/REUSE.toml")
            if slug in SLINT_ARTWORK:
                if record["workspace_annotations"]["upstream_directory"] != SLINT_ARTWORK[slug]:
                    raise ValueError(f"Unreviewed Slint workspace artwork path: {slug}")
                record["artwork"] = slint_artwork(root / "release-notices/slint", archive, slug)
                extra = sorted(set(extra) | {"release-notices/slint/" + file
                                             for file in slint_supplements(root / "release-notices/slint")})
            if not included and not extra:
                raise ValueError(f"Source notice review incomplete: {slug}")
            if slug == "sctk-adwaita-0.10.1":
                if "release-notices/" + SCTK_MPL_NOTICE not in extra:
                    raise ValueError("Missing notice for the sctk-adwaita headerbar source")
                record["source_component"] = SCTK_HEADERBAR | {"license_file": "release-notices/" + SCTK_MPL_NOTICE}
            if resources:
                record["font_resources"] = resources
                for resource in resources.values():
                    if "additional_notice" in resource and resource["additional_notice"] not in extra:
                        raise ValueError(f"Missing bundled font notice: {slug}")
            shipped.append(record | {"archive": archive.name, "license": license_id,
                                     "notices_in_archive": included,
                                     "additional_notices": {path: digest_file(root / path) for path in extra}})
    if {(p["name"], p["version"]) for p in shipped} != resolved.keys():
        raise ValueError("Linux resolution and locked source inventory disagree")
    if {p.name for p in registry.glob("*.crate")} != {p["archive"] for p in shipped}:
        raise ValueError("Unmapped archive in source registry")
    inventory = {"schema_version": 1, "target": target, "registry_crates": shipped,
                 "index_only": index_only, "index_sha256": index_hash.hexdigest()}
    digest = hashlib.sha256(canonical(inventory)).hexdigest()
    if digest != REVIEWED_SOURCE_INVENTORY:
        raise ValueError(f"Distributed source or notice inventory requires review: {digest}")
    inventory["project_license"] = "LICENSE"
    inventory["local_patch"] = {
        "directory": "vendor/fontdue-0.9.5", "patch_notice": "vendor/fontdue-0.9.5-PATCH.md",
        "notices": ["LICENSE-APACHE", "LICENSE-MIT", "LICENSE-ZLIB"],
    }
    (root / "SOURCE-NOTICES.json").write_text(json.dumps(inventory, indent=2, sort_keys=True) + "\n")
