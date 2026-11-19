"""Exercise source binding and independent checksum failures without native package tools."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("verify_payload", Path(__file__).with_name("verify-payload.py"))
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class SourceInputs(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = Path(tmp.name)
        self.version = "0.1.0-alpha.2"
        self.commit = "1" * 40
        self.filename = f"siliconwatch-{self.version}-source.tar.gz"
        (self.path / self.filename).write_bytes(b"source")
        notices = self.path / "licenses"
        notices.mkdir()
        (notices / "LICENSE").write_bytes(b"terms")
        (notices / "SHA256SUMS").write_text(hashlib.sha256(b"terms").hexdigest() + "  LICENSE\n")
        (self.path / "SOURCE-PROVENANCE.json").write_text(json.dumps({
            "version": self.version, "commit": self.commit, "source_archive": self.filename,
            "source_sha256": verify.digest(self.path / self.filename),
            "notices_sha256": verify.digest(notices / "SHA256SUMS"),
        }))

    def test_bound_input_passes(self):
        verify.source_inputs(self.path, self.version, self.commit)

    def test_different_commit_fails(self):
        with self.assertRaisesRegex(SystemExit, "version/SHA"):
            verify.source_inputs(self.path, self.version, "2" * 40)

    def test_changed_archive_fails(self):
        (self.path / self.filename).write_bytes(b"changed")
        with self.assertRaisesRegex(SystemExit, "archive"):
            verify.source_inputs(self.path, self.version, self.commit)

    def test_unexpected_notice_fails(self):
        (self.path / "licenses/another").write_bytes(b"unexpected")
        with self.assertRaisesRegex(SystemExit, "Notice payload"):
            verify.source_inputs(self.path, self.version, self.commit)


class NativeReceipt(unittest.TestCase):
    def test_failed_retry_invalidates_a_previous_install_receipt(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            receipt = directory / "install-debian.json"
            receipt.write_text("earlier success")
            docker = directory / "docker"
            docker.write_text("#!/bin/sh\nexit 23\n")
            docker.chmod(0o755)
            runner = Path(__file__).with_name("run-native.sh")
            result = subprocess.run(["bash", str(runner), "verify", "debian", "v0.1.0-alpha.2",
                                     "ab" * 20, str(directory)],
                                    env=os.environ | {"PATH": str(directory) + os.pathsep + os.environ["PATH"]},
                                    capture_output=True, check=False)
            self.assertEqual(result.returncode, 23, result.stderr)
            self.assertFalse(receipt.exists())
            self.assertEqual(list(directory.glob(".verify-*")), [])


if __name__ == "__main__":
    unittest.main()
