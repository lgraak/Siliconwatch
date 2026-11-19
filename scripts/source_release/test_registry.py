"""Exercise the actual archive and notice boundary independently of a developer's Cargo cache."""

import hashlib
import io
import json
from pathlib import Path
import re
import shutil
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import registry


class SourceRegistryTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.work = Path(temp.name)
        self.root = self.work / "source"
        (self.root / "vendor").mkdir(parents=True)
        self.store = self.work / "registry"
        self.available = self.store / "cache/crates-io"
        self.available.mkdir(parents=True)
        self.archive = self.available / "native-1.0.0.crate"
        self.make_archive({"LICENSE-MIT": b"license", "assets/NOTICE.txt": b"asset attribution"})
        self.lock = {"package": []}
        self.add_index("native", registry.digest_file(self.archive))
        self.add_index("block2", "1" * 64)
        self.add_index("clipboard-win", "2" * 64)
        manifest = str(self.store / "src/crates-io/native-1.0.0/Cargo.toml")
        self.resolution = {"packages": [{"name": "native", "version": "1.0.0",
                                         "source": registry.CRATES_IO, "manifest_path": manifest}]}

    def make_archive(self, files):
        files = {"Cargo.toml": b'[package]\nname="native"\nversion="1.0.0"\nlicense="MIT"\n'} | files
        with tarfile.open(self.archive, "w:gz") as archive:
            for name, data in files.items():
                member = tarfile.TarInfo("native-1.0.0/" + name)
                member.size = len(data)
                archive.addfile(member, io.BytesIO(data))

    def add_index(self, name, checksum):
        self.lock["package"].append({"name": name, "version": "1.0.0",
                                     "source": registry.CRATES_IO, "checksum": checksum})
        row = {"name": name, "vers": "1.0.0", "cksum": checksum, "features": {}, "deps": []}
        cache = self.store / "index/crates-io/.cache" / registry.index_path(name)
        cache.parent.mkdir(parents=True, exist_ok=True)
        cache.write_bytes(b"\x03\x02\x00\x00\x00etag: fixture\x001.0.0\x00" + json.dumps(row).encode() + b"\x00")

    def build(self, notices=None):
        with patch.object(registry.subprocess, "check_output", return_value=json.dumps(self.resolution)):
            registry.source_registry(self.root, "x86_64-unknown-linux-gnu", self.lock, notices or {})

    def reviewed_fixture(self, notices):
        with self.assertRaisesRegex(ValueError, "requires review: [a-f0-9]{64}$") as raised:
            self.build(notices)
        fixture_digest = re.search(r"[a-f0-9]{64}$", str(raised.exception))[0]
        shutil.rmtree(self.root / "vendor/registry")
        return patch.object(registry, "REVIEWED_SOURCE_INVENTORY", fixture_digest)

    def test_index_only_crates_have_no_source_and_native_notices_include_nested_attribution(self):
        with self.reviewed_fixture({}):
            self.build()
        report = json.loads((self.root / "SOURCE-NOTICES.json").read_text())
        self.assertEqual([entry["name"] for entry in report["index_only"]], ["block2", "clipboard-win"])
        self.assertEqual([p.name for p in (self.root / "vendor/registry").glob("*.crate")], [self.archive.name])
        for entry in report["index_only"]:
            index = self.root / "vendor/registry/index" / registry.index_path(entry["name"])
            self.assertEqual(json.loads(index.read_bytes())["cksum"], entry["crate_sha256"])
        package, = report["registry_crates"]
        self.assertEqual(package["notices_in_archive"], ["LICENSE-MIT", "assets/NOTICE.txt"])
        self.assertEqual(package["crate_sha256"], hashlib.sha256(self.archive.read_bytes()).hexdigest())

    def test_suffixed_attributions_plural_notices_and_patent_files(self):
        names = ["assets/font.ttf.license", "assets/colr_1_LICENSE", "NOTICES.md", "PATENTS", "ATTRIBUTION.md", "REUSE.toml", ".reuse/dep5"]
        self.make_archive({path: b"original terms" for path in names})
        _, notices, _ = registry.inspect_crate(self.archive, "native", "1.0.0")
        self.assertEqual(notices, sorted(names))

    def test_slint_artwork_requires_attribution_for_every_distributed_svg(self):
        self.make_archive({"widgets/fluent/_open.svg": b"<svg/>", "extra.svg": b"<svg/>"})
        (self.root / "REUSE.toml").write_text(
            '[[annotations]]\npath="internal/compiler/widgets/fluent/_**.svg"\n'
            'SPDX-FileCopyrightText="Microsoft"\nSPDX-License-Identifier="MIT"\n')
        with patch.dict(registry.SLINT_ARTWORK, {"native-1.0.0": "internal/compiler"}):
            with self.assertRaisesRegex(ValueError, "Unmapped Slint artwork"):
                registry.slint_artwork(self.root, self.archive, "native-1.0.0")
            self.make_archive({"widgets/fluent/_open.svg": b"<svg/>"})
            art = registry.slint_artwork(self.root, self.archive, "native-1.0.0")
        self.assertEqual(art["widgets/fluent/_open.svg"], [{"attribution": "Microsoft", "license": "MIT"}])

    def test_unmapped_font_fails_even_with_a_crate_license(self):
        self.make_archive({"LICENSE": b"crate license", "data/font.ttf": b"font bytes"})
        with self.assertRaisesRegex(ValueError, "Unreviewed font resource"):
            registry.inspect_crate(self.archive, "native", "1.0.0")

    def test_changed_supplement_requires_new_review(self):
        notices = {("native", "1.0.0"): ["upstream.txt"]}
        supplement = self.root / "release-notices/upstream.txt"
        supplement.parent.mkdir()
        supplement.write_text("pinned workspace copyright")
        with self.reviewed_fixture(notices):
            supplement.write_text("different attribution")
            with self.assertRaisesRegex(ValueError, "requires review"):
                self.build(notices)

    def test_missing_mapped_notice_fails_before_sealing(self):
        with self.assertRaises(FileNotFoundError):
            self.build({("native", "1.0.0"): ["absent.txt"]})

    def test_wrong_archive_checksum_fails_before_notices(self):
        self.archive.write_bytes(b"unexpected archive")
        with self.assertRaisesRegex(ValueError, "differs from Cargo.lock"):
            self.build()

    def test_new_crate_without_a_license_mapping_fails(self):
        self.make_archive({"src/lib.rs": b"// code"})
        self.lock["package"].clear()
        self.add_index("native", registry.digest_file(self.archive))
        with self.assertRaisesRegex(ValueError, "notice review incomplete"):
            self.build()

    def test_incorrect_resolver_metadata_for_excluded_source_fails(self):
        self.lock["package"][1]["checksum"] = "a" * 64
        with self.assertRaisesRegex(ValueError, "Index identity or checksum"):
            self.build()


if __name__ == "__main__":
    unittest.main()
