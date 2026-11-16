"""Filesystem fixtures and mocked host inputs; no sysfs writes."""

import hashlib
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import overhead_provenance as provenance


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)

    def device(self, number, name, files=()):
        path = self.root / f"hwmon{number}"
        path.mkdir()
        (path / "name").write_text(name)
        for name in files:
            (path / name).write_text("unused")
        return path

    def test_inventory_retains_empty_duplicate_and_unsupported_providers(self):
        self.device(5, "nvme\n", ["temp1_input", "temp2_input", "temp1_label"])
        self.device(0, "battery")
        self.device(9, "nvme", ["temp3_input", "freq1_input", "in0_input", "mystery2_input"])
        actual = provenance.inventory(self.root)
        self.assertEqual(len(actual), 3)
        self.assertEqual(sum(row["exposed_input_count"] for row in actual), 6)
        self.assertEqual([row["provider"] for row in actual].count("nvme"), 2)
        battery = next(row for row in actual if row["provider"] == "battery")
        self.assertEqual(battery["exposed_input_count"], 0)
        self.assertNotIn("hwmon", json.dumps(actual))
        self.assertNotIn(str(self.root), json.dumps(actual))

    def test_inventory_renumbering_is_deterministic(self):
        a = self.device(1, "gpu", ["temp1_input"])
        b = self.device(9, "cpu", ["temp1_input", "fan1_input"])
        before = provenance.inventory(self.root)
        a.rename(self.root / "hwmon20")
        b.rename(self.root / "hwmon0")
        self.assertEqual(provenance.inventory(self.root), before)

    def test_missing_root_and_invalid_name_are_explicit(self):
        self.assertEqual(provenance.inventory(self.root / "missing")[0]["discovery_status"], "missing")
        self.device(0, "bad\nsecret-path", ["temp1_input"])
        row = provenance.inventory(self.root)[0]
        self.assertIsNone(row["provider"])
        self.assertEqual(row["name_status"], "unavailable_or_invalid")
        self.assertNotIn("secret", json.dumps(row))

    def test_permission_failure_does_not_export_exception_path(self):
        with patch.object(Path, "iterdir", side_effect=PermissionError("/secret/path")):
            result = provenance.inventory(self.root)
        self.assertEqual(result[0]["discovery_status"], "permission_denied")
        self.assertNotIn("secret", json.dumps(result))

    def test_bounded_reads_and_hash(self):
        path = self.root / "binary"
        path.write_bytes(b"abc")
        self.assertEqual(provenance.file_sha256(path), hashlib.sha256(b"abc").hexdigest())
        self.assertIsNone(provenance._read(path, 2))
        self.assertIsNone(provenance.file_sha256(self.root / "missing"))

    def test_filesystem_selects_longest_mount_and_omits_devices(self):
        mountinfo = self.root / "mountinfo"
        mountinfo.write_text("1 0 0:1 / / rw - rootfs secret-source rw\n"
                            f"2 1 0:2 / {self.root} rw - tmpfs secret-device rw\n")
        self.assertEqual(provenance._filesystem(self.root / "results", mountinfo), "tmpfs")

    def test_environment_allowlist_missing_values_and_no_uname_hostname(self):
        binary = self.root / "siliconwatch"
        binary.write_bytes(b"binary")
        with patch.object(provenance, "_fields", return_value={}), \
             patch.object(provenance, "_command", return_value=None), \
             patch.object(provenance, "inventory", return_value=[]), \
             patch.object(provenance, "_power", return_value=[]), \
             patch.object(provenance, "_cpu_quota", return_value={"status": "unavailable"}), \
             patch.object(provenance.platform, "node", side_effect=AssertionError("hostname read")):
            result = provenance.collect_environment(binary, self.root, self.root)
        self.assertIsNone(result["memory_total_kib"])
        self.assertIsNone(result["git_revision"])
        self.assertIsNone(result["installed_rust_1_85"])
        self.assertEqual(result["binary_sha256"], hashlib.sha256(b"binary").hexdigest())
        self.assertNotIn(str(self.root), json.dumps(result))
        self.assertNotIn("environ", result)


if __name__ == "__main__":
    unittest.main()
