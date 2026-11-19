"""Regression tests for offline corresponding-source preparation."""

import hashlib
import io
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import prepare


class SourcePreparationTests(unittest.TestCase):
    def test_source_tar_is_reproducible_across_mtime_and_group_write_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            root = work / "siliconwatch-1.0.0-source"
            root.mkdir()
            source = root / "build.sh"
            source.write_bytes(b"#!/bin/sh\n")
            source.chmod(0o775)
            first = work / "first.tar.gz"
            second = work / "second.tar.gz"
            prepare.write_archive(root, first, "1000000000")
            os.utime(source, (2000000000, 2000000000))
            source.chmod(0o755)
            prepare.write_archive(root, second, "1000000000")
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with tarfile.open(first, "r:gz") as tar:
                member = tar.getmember(f"{root.name}/build.sh")
                self.assertEqual(member.mode, 0o755)
                self.assertEqual(member.mtime, 1000000000)
                self.assertEqual((member.uid, member.gid), (0, 0))
                self.assertEqual(tar.extractfile(member).read(), source.read_bytes())

    def test_sctk_binary_notices_map_embedded_font_and_headerbar_source(self):
        source = Path(__file__).resolve().parents[2]
        with tempfile.TemporaryDirectory() as directory:
            licenses = Path(directory)
            entry = {"license": "MIT", "license_files": ["crate-MIT.txt"]}
            prepare.add_sctk_notices(source, licenses, entry)
            font, = entry["bundled_resources"]
            self.assertEqual(font["license"], "OFL-1.1")
            terms = (licenses / font["license_file"]).read_bytes()
            # The pinned GNOME 0.0.5 COPYING with LF line endings.
            self.assertEqual(hashlib.sha256(terms).hexdigest(),
                             "1ab845e2ae013714423d9a18c6d92368e9b13794440a388db4f8eed469eab0ea")
            self.assertIn(b"Understanding Limited", terms)
            self.assertIn(b"Jakub Steiner", terms)
            header = entry["source_component"]
            self.assertEqual(header["path"], "src/lib.rs")
            self.assertEqual(header["portion"], "rounded_headerbar_shape")
            self.assertIn("Mozilla Public License", (licenses / header["license_file"]).read_text())
            self.assertEqual(entry["license"], "MIT")
            self.assertEqual(set(entry["license_files"]), {"crate-MIT.txt", font["license_file"], header["license_file"]})

    def test_changed_notice_archive_is_rejected_before_extraction(self):
        with tempfile.TemporaryDirectory() as directory:
            work = Path(directory)
            archive = work / "unknown.tar"
            with tarfile.open(archive, "w") as tar:
                member = tarfile.TarInfo("usr/share/licenses/siliconwatch/example")
                member.size = 1
                tar.addfile(member, io.BytesIO(b"x"))
            licenses = work / "licenses"
            with patch.object(prepare, "dependency_inventory", return_value=set()):
                with self.assertRaisesRegex(ValueError, "archive checksum"):
                    prepare.prepare_notices(work, work, licenses, archive, "0.1.0-alpha.2")
            self.assertFalse(licenses.exists())

    def test_changed_crate_checksum_invalidates_reviewed_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.lock").write_text(
                '[[package]]\nname="sample"\nversion="1.0.0"\n'
                'source="registry+https://github.com/rust-lang/crates.io-index"\n'
                'checksum="' + "0" * 64 + '"\n'
            )
            with patch.object(prepare, "output", return_value="sample v1.0.0"):
                with self.assertRaisesRegex(ValueError, "dependency closure changed"):
                    prepare.dependency_inventory(root)


if __name__ == "__main__":
    unittest.main()
