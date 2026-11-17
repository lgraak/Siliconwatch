"""Deterministic selection/cleanup fixtures; no access to real sysfs."""

import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import provider_selection as selection
from overhead_process import run_process


class FixtureCase(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / "class"
        self.root.mkdir()
        self.aliases = self.base / "aliases"
        self.aliases.mkdir(mode=0o700)

    def registration(self, number=0, provider="chip", channels=None, parent=None):
        parent = self.base / "hardware" / (parent or f"controller-{number}")
        target = parent / "hwmon" / f"hwmon{number}"
        target.mkdir(parents=True)
        (target / "name").write_text(provider + "\n")
        (target / "device").symlink_to(parent, target_is_directory=True)
        for channel, value in (channels if channels is not None else {"temp1_input": "42000"}).items():
            (target / channel).write_text(value)
        (self.root / f"hwmon{number}").symlink_to(target, target_is_directory=True)
        return target


class SelectionTests(FixtureCase):
    def test_enumeration_preserves_sorted_channels_and_ignores_other_entries(self):
        self.registration(10, "later", {"temp2_input": "2", "in0_input": "3", "temp1_label": "display"})
        self.registration(2, "earlier", {"odd_input": "2", "fan1_input": "3"})
        (self.root / "unrelated").mkdir()
        items = selection.enumerate_registrations(self.root)
        self.assertEqual([i["entry"] for i in items], ["hwmon10", "hwmon2"])
        self.assertEqual([i["token"] for i in items], ["r001", "r002"])
        self.assertEqual(items[0]["channels"], ["in0_input", "temp2_input"])
        self.assertEqual(items[1]["channels"], ["fan1_input", "odd_input"])
        self.assertEqual(selection.fingerprint(items), selection.fingerprint(selection.enumerate_registrations(self.root)))
        json.dumps(items, allow_nan=False)

    def test_private_inventory_is_not_exported(self):
        self.registration()
        items = selection.enumerate_registrations(self.root)
        public = selection.public_inventory(items)
        self.assertEqual(public[0]["provider"], "chip")
        self.assertEqual(public[0]["channel_count"], 1)
        self.assertNotIn(str(self.base), json.dumps(public))
        self.assertNotIn("target_identity", public[0])
        self.assertNotIn("device_link", public[0])
        self.assertNotIn("channels", public[0])

    def test_duplicate_names_require_exact_tokens(self):
        self.registration(0, "same")
        self.registration(1, "same")
        items = selection.enumerate_registrations(self.root)
        with self.assertRaisesRegex(selection.SelectionError, "ambiguous_selection"):
            selection.select_registrations(items, ["same"])
        self.assertEqual(selection.select_registrations(items, ["r002"]), [items[1]])

    def test_duplicate_targets_are_explicit_and_not_selected(self):
        target = self.registration()
        (self.root / "hwmon1").symlink_to(target, target_is_directory=True)
        items = selection.enumerate_registrations(self.root)
        self.assertEqual([i["status"] for i in items], ["duplicate_target", "duplicate_target"])
        for selected in (None, ["r001"]):
            with self.assertRaises(selection.SelectionError):
                selection.select_registrations(items, selected)

    def test_shared_provider_and_parent_remain_distinct_registrations(self):
        self.registration(0, "chip", parent="same-parent")
        self.registration(1, "chip", parent="same-parent")
        items = selection.enumerate_registrations(self.root)
        self.assertEqual(items[0]["device_link"], items[1]["device_link"])
        self.assertNotEqual(items[0]["target"], items[1]["target"])
        self.assertEqual(len(selection.select_registrations(items)), 2)
        with self.assertRaisesRegex(selection.SelectionError, "ambiguous_selection"):
            selection.select_registrations(items, ["chip"])

    def test_duplicate_token_and_name_requests_are_rejected(self):
        self.registration()
        items = selection.enumerate_registrations(self.root)
        for requested in (["r001", "r001"], ["r001", "chip"]):
            with self.subTest(requested=requested), self.assertRaisesRegex(selection.SelectionError, "duplicate_selection"):
                selection.select_registrations(items, requested)

    def test_invalid_selections_and_counts_are_rejected(self):
        self.registration()
        items = selection.enumerate_registrations(self.root)
        for requested in ([], [""], [None], ["unknown"], ["../hwmon0"], "r001"):
            with self.subTest(requested=requested), self.assertRaises(selection.SelectionError):
                selection.select_registrations(items, requested)
        for limit in (0, 6, True, 1.5):
            with self.subTest(limit=limit), self.assertRaises(selection.SelectionError):
                selection.select_registrations(items, max_count=limit)

    def test_more_than_five_requires_explicit_subset(self):
        for number in range(6):
            self.registration(number, f"chip-{number}")
        items = selection.enumerate_registrations(self.root)
        with self.assertRaisesRegex(selection.SelectionError, "too_many"):
            selection.select_registrations(items)
        self.assertEqual(len(selection.select_registrations(items, ["r001", "r006"])), 2)

    def test_selection_order_does_not_depend_on_argument_order(self):
        self.registration(0, "first")
        self.registration(1, "second")
        items = selection.enumerate_registrations(self.root)
        self.assertEqual(selection.select_registrations(items, ["second", "first"]), items)

    def test_missing_root_and_nondirectory_are_safe_errors(self):
        for path in (self.base / "missing", self.base / "file"):
            if path.name == "file":
                path.write_text("fixture")
            with self.subTest(path=path), self.assertRaises(selection.SelectionError) as caught:
                selection.enumerate_registrations(path)
            self.assertNotIn(str(self.base), str(caught.exception))

    def test_missing_name_and_dangling_registration_remain_visible(self):
        target = self.registration()
        (target / "name").unlink()
        (self.root / "hwmon1").symlink_to(self.base / "missing")
        items = selection.enumerate_registrations(self.root)
        self.assertEqual(items[0]["issues"], ["provider_missing"])
        self.assertEqual(items[1]["issues"], ["registration_missing"])
        self.assertEqual([i["status"] for i in items], ["unavailable", "unavailable"])

    def test_zero_input_registration_is_visible_but_not_selected(self):
        self.registration(0, "empty", {})
        self.registration(1, "readable")
        items = selection.enumerate_registrations(self.root)
        self.assertEqual(items[0]["status"], "zero_inputs")
        self.assertEqual(selection.select_registrations(items), [items[1]])

    def test_unreadable_name_is_deterministic_and_private(self):
        self.registration()
        original_open = Path.open
        def denied(path, *args, **kwargs):
            if path.name == "name":
                raise PermissionError("secret local path should not leak")
            return original_open(path, *args, **kwargs)
        with patch.object(Path, "open", denied):
            item = selection.enumerate_registrations(self.root)[0]
        self.assertEqual(item["status"], "unavailable")
        self.assertEqual(item["issues"], ["provider_permission_denied"])
        self.assertNotIn("secret", json.dumps(selection.public_inventory([item])))

    def test_unreadable_registration_enumeration_is_explicit(self):
        self.registration()
        original = selection._names
        def denied(path):
            if Path(path).name == "hwmon0":
                raise PermissionError()
            return original(path)
        with patch.object(selection, "_names", side_effect=denied):
            item = selection.enumerate_registrations(self.root)[0]
        self.assertEqual(item["status"], "unavailable")
        self.assertEqual(item["issues"], ["registration_permission_denied"])

    def test_provider_text_and_entry_limits(self):
        target = self.registration()
        (target / "name").write_text("x" * 300)
        self.assertEqual(selection.enumerate_registrations(self.root)[0]["issues"], ["provider_invalid"])
        with patch.object(selection, "MAX_DIRECTORY_ENTRIES", 1), self.assertRaisesRegex(selection.SelectionError, "directory_entry_limit"):
            selection.enumerate_registrations(self.root)
        with patch.object(selection, "MAX_REGISTRATIONS", 0), self.assertRaisesRegex(selection.SelectionError, "registration_limit"):
            selection.enumerate_registrations(self.root)

    def test_identity_link_failure_does_not_invent_read_failure(self):
        target = self.registration()
        (target / "device").unlink()
        (target / "device").symlink_to(self.base / "missing")
        item = selection.enumerate_registrations(self.root)[0]
        self.assertEqual(item["status"], "ok")
        self.assertEqual(item["device_link_status"], "missing")
        self.assertIsNone(item["device_link"])

    def test_raw_reading_changes_do_not_invalidate_registration(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        (target / "temp1_input").write_text("99999")
        selection.validate_registration(item)

    def test_renumbering_and_changed_channels_invalidate_selection(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        (target / "fan1_input").write_text("1000")
        with self.assertRaisesRegex(selection.SelectionError, "registration_changed"):
            selection.validate_registration(item)
        (target / "fan1_input").unlink()
        (self.root / "hwmon0").rename(self.root / "hwmon3")
        with self.assertRaisesRegex(selection.SelectionError, "registration_changed"):
            selection.validate_registration(item)

    def test_retargeted_registration_is_not_silently_rebound(self):
        self.registration(0)
        other = self.registration(1)
        item = selection.enumerate_registrations(self.root)[0]
        path = Path(item["path"])
        path.unlink()
        path.symlink_to(other)
        with self.assertRaisesRegex(selection.SelectionError, "registration_changed"):
            selection.validate_registration(item)

    def test_alias_root_preserves_target_basename_modes_and_contents(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        original = {p.name: (p.read_bytes(), stat.S_IMODE(p.stat().st_mode))
                    for p in target.iterdir() if p.is_file()}
        with selection.alias_root(self.aliases, item) as root:
            self.assertEqual(stat.S_IMODE(root.stat().st_mode), 0o700)
            self.assertEqual([p.name for p in root.iterdir()], ["hwmon0"])
            self.assertTrue((root / "hwmon0").is_symlink())
            self.assertEqual((root / "hwmon0").resolve(), target)
            self.assertEqual((root / "hwmon0" / "temp1_input").read_text(), "42000")
        self.assertFalse(root.exists())
        self.assertEqual(list(self.aliases.iterdir()), [])
        self.assertEqual(original, {p.name: (p.read_bytes(), stat.S_IMODE(p.stat().st_mode))
                                    for p in target.iterdir() if p.is_file()})

    def test_exception_still_cleans_alias_without_touching_target(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        with self.assertRaisesRegex(RuntimeError, "fixture"):
            with selection.alias_root(self.aliases, item):
                raise RuntimeError("fixture")
        self.assertTrue(target.is_dir())
        self.assertEqual(list(self.aliases.iterdir()), [])

    def test_target_change_on_context_exit_is_detected_and_alias_cleaned(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        with self.assertRaisesRegex(selection.SelectionError, "registration_changed"):
            with selection.alias_root(self.aliases, item):
                (target / "fan1_input").write_text("1")
        self.assertTrue(target.is_dir())
        self.assertEqual(list(self.aliases.iterdir()), [])

    def test_unsafe_parent_and_source_tree_are_rejected(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        self.aliases.chmod(0o755)
        with self.assertRaisesRegex(selection.SelectionError, "not_private"):
            with selection.alias_root(self.aliases, item):
                self.fail("must not yield")
        target.chmod(0o700)
        with self.assertRaisesRegex(selection.SelectionError, "inside_source_tree"):
            with selection.alias_root(target, item):
                self.fail("must not yield")
        self.aliases.chmod(0o700)
        linked = self.base / "linked-parent"
        linked.symlink_to(self.aliases)
        with self.assertRaisesRegex(selection.SelectionError, "not_plain_directory"):
            with selection.alias_root(linked, item):
                self.fail("must not yield")

    def test_unexpected_cleanup_contents_are_preserved_not_recursively_deleted(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        with self.assertRaisesRegex(selection.SelectionError, "unexpected_entries"):
            with selection.alias_root(self.aliases, item) as root:
                (root / "unexpected").write_text("retain")
        self.assertEqual((root / "unexpected").read_text(), "retain")
        self.assertEqual((target / "temp1_input").read_text(), "42000")

    def test_substituted_root_symlink_is_never_followed_during_cleanup(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        with self.assertRaisesRegex(selection.SelectionError, "cleanup_failed"):
            with selection.alias_root(self.aliases, item) as root:
                moved = root.with_name(root.name + "-moved")
                root.rename(moved)
                root.symlink_to(target)
        self.assertTrue(root.is_symlink())
        self.assertEqual(list(moved.iterdir()), [])
        self.assertEqual((target / "temp1_input").read_text(), "42000")

    def test_failed_link_creation_removes_only_empty_owned_root(self):
        target = self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        with patch.object(selection.os, "symlink", side_effect=PermissionError()), self.assertRaises(PermissionError):
            with selection.alias_root(self.aliases, item):
                self.fail("must not yield")
        self.assertEqual(list(self.aliases.iterdir()), [])
        self.assertEqual((target / "temp1_input").read_text(), "42000")

    def test_cleanup_rewinds_held_directory_stream_before_enumeration(self):
        self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        listdir, lseek = os.listdir, os.lseek
        rewound = set()

        def stale_until_rewind(path):
            if isinstance(path, int) and path not in rewound:
                return []
            return listdir(path)

        def rewind(fd, offset, how):
            if offset == 0 and how == os.SEEK_SET:
                rewound.add(fd)
            return lseek(fd, offset, how)

        with patch.object(selection.os, "listdir", side_effect=stale_until_rewind), \
             patch.object(selection.os, "lseek", side_effect=rewind):
            with selection.alias_root(self.aliases, item) as root:
                self.assertTrue((root / item["entry"]).is_symlink())
        self.assertTrue(rewound)
        self.assertFalse(root.exists())
        self.assertEqual(list(self.aliases.iterdir()), [])

    def test_workspace_alias_cleanup_with_owned_process_runner(self):
        # Exercise the same filesystem as real private evidence, not only /tmp.
        # The child prints fixture data; neither it nor the fixture touches sysfs.
        parent = Path(__file__).resolve().parents[2] / "target"
        parent.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="provider-alias-test-", dir=parent) as temporary:
            self.base = Path(temporary)
            self.root = self.base / "class"
            self.root.mkdir()
            self.aliases = self.base / "aliases"
            self.aliases.mkdir(mode=0o700)
            target = self.registration()
            item = selection.enumerate_registrations(self.root)[0]
            with selection.alias_root(self.aliases, item) as root:
                resource = run_process([sys.executable, "-B", "-c", "print('fixture')"],
                    self.base / "capture", timeout_s=3, output_limit_bytes=1024,
                    observe_rss=False)
            self.assertEqual(resource["termination"], "completed")
            self.assertEqual(resource["exit_code"], 0)
            self.assertTrue(resource["reaped"])
            self.assertFalse(root.exists())
            self.assertEqual(list(self.aliases.iterdir()), [])
            self.assertEqual((target / "temp1_input").read_text(), "42000")


_BINARY = Path(__file__).resolve().parents[2] / "target/provider-latency/release/siliconwatch"


@unittest.skipUnless(_BINARY.is_file(), "build Rust 1.85 release binary in target/provider-latency first")
class CliAliasTests(FixtureCase):
    def command(self, root, *args):
        result = subprocess.run([str(_BINARY), *args, "--hwmon-root", str(root)],
                                stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, text=True, timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def snapshot(self, root):
        return json.loads(self.command(root, "sensors", "--json"))

    def test_fixture_alias_preserves_raw_values_order_errors_and_identity(self):
        target = self.registration(0, channels={"temp1_input": "42000", "in0_input": "1152",
                    "fan1_input": "0", "freq1_input": "250000", "temp2_input": "malformed",
                    "temp3_input": "33000", "temp4_input": "1"})
        self.registration(1, "other", {"temp1_input": "1000"})
        (target / "temp1_label").write_text("Duplicate label")
        (target / "temp2_label").write_text("Duplicate label")
        (target / "temp2_enable").write_text("malformed")
        (target / "temp2_fault").write_text("1")
        (target / "temp3_enable").write_text("0")
        (target / "temp3_fault").write_text("malformed")
        (target / "temp4_input").unlink()
        (target / "temp4_input").symlink_to(self.base / "absent-input")
        (target / "temp4_enable").write_text("malformed")
        item = selection.enumerate_registrations(self.root)[0]
        full = self.snapshot(self.root)
        expected = [sensor for sensor in full["sensors"] if sensor["device_path"] == str(target)]
        with selection.alias_root(self.aliases, item) as alias:
            isolated = self.snapshot(alias)
        self.assertEqual(isolated["schema_version"], 2)
        self.assertEqual([s["attribute"] for s in isolated["sensors"]], item["channels"])
        self.assertEqual(len(isolated["sensors"]), len(expected))
        for before, after in zip(expected, isolated["sensors"]):
            self.assertNotEqual(before["source_path"], after["source_path"])
            self.assertEqual({k: v for k, v in before.items() if k != "source_path"},
                             {k: v for k, v in after.items() if k != "source_path"})
            self.assertEqual(after["identity"]["schema_version"], 1)
        sensors = {s["attribute"]: s for s in isolated["sensors"]}
        self.assertEqual(sensors["temp1_input"]["value"], 42.0)
        self.assertEqual(sensors["in0_input"]["value"], 1.152)
        self.assertEqual(sensors["fan1_input"]["value"], 0.0)
        self.assertEqual(sensors["freq1_input"]["status"], "unsupported")
        self.assertTrue(sensors["temp2_input"]["error"].startswith("invalid integer:"))
        self.assertEqual(sensors["temp2_input"]["raw_text"], "malformed")
        self.assertEqual(sensors["temp3_input"]["error"], "sensor is disabled")
        self.assertEqual(sensors["temp3_input"]["raw_value"], 33000)
        self.assertEqual(sensors["temp4_input"]["status"], "unavailable")
        self.assertNotIn("enable flag", sensors["temp4_input"]["error"])
        self.assertEqual([w["message"] for w in full["warnings"]], [w["message"] for w in isolated["warnings"]])

    def test_isolation_exposes_collision_context_difference_without_rewriting_candidates(self):
        self.registration(0, parent="shared")
        self.registration(1, parent="shared")
        items = selection.enumerate_registrations(self.root)
        full = self.snapshot(self.root)
        self.assertEqual([s["identity"]["status"] for s in full["sensors"]], ["collision", "collision"])
        with selection.alias_root(self.aliases, items[0]) as alias:
            isolated = self.snapshot(alias)
        before, after = full["sensors"][0], isolated["sensors"][0]
        self.assertEqual(before["identity"]["candidate"], after["identity"]["candidate"])
        self.assertEqual(after["identity"]["status"], "best_effort")
        self.assertEqual(before["raw_value"], after["raw_value"])
        self.assertEqual(before["device_path"], after["device_path"])

    def test_root_layout_can_change_identity_meaning_and_must_be_reported(self):
        # Deliberately unlike real class symlink topology: this demonstrates why
        # equivalence is conditional on original root/collision context.
        target = self.root / "hwmon0"
        target.mkdir()
        (target / "name").write_text("fixture")
        (target / "temp1_input").write_text("42000")
        item = selection.enumerate_registrations(self.root)[0]
        before = self.snapshot(self.root)["sensors"][0]
        with selection.alias_root(self.aliases, item) as alias:
            after = self.snapshot(alias)["sensors"][0]
        self.assertEqual(before["identity"]["status"], "insufficient_evidence")
        self.assertEqual(after["identity"]["status"], "best_effort")
        self.assertIsNone(before["identity"]["candidate"])
        self.assertIsNotNone(after["identity"]["candidate"])
        self.assertEqual(before["raw_value"], after["raw_value"])
        self.assertEqual(before["device_path"], after["device_path"])

    def test_existing_table_and_sampling_json_lines_use_unchanged_contract(self):
        self.registration()
        item = selection.enumerate_registrations(self.root)[0]
        with selection.alias_root(self.aliases, item) as alias:
            snapshot = self.snapshot(alias)
            table = self.command(alias, "sensors")
            lines = self.command(alias, "sample", "--count", "2", "--interval-ms", "100", "--json-lines")
            records = [json.loads(line) for line in lines.splitlines()]
        self.assertIn("chip", table)
        self.assertIn("42", table)
        self.assertEqual([r["sequence"] for r in records], [1, 2])
        for record in records:
            self.assertEqual(record["sampling_schema_version"], 1)
            self.assertEqual(record["snapshot"], snapshot)
            self.assertIsNone(record["error"])
            timing, freshness = record["timing"], record["freshness"]
            self.assertEqual(timing["scan_duration_ns"], timing["finished_ns"] - timing["started_ns"])
            self.assertEqual(timing["lateness_ns"], timing["started_ns"] - timing["target_start_ns"])
            self.assertEqual(freshness["basis"], "current_attempt_observation_window")
            self.assertEqual(freshness["lower_bound_ns"], timing["prepared_ns"] - timing["finished_ns"])
            self.assertEqual(freshness["upper_bound_ns"], timing["prepared_ns"] - timing["started_ns"])
            self.assertFalse(freshness["physical_age_known"])
        self.assertEqual(records[1]["timing"]["target_start_ns"], records[0]["timing"]["started_ns"] + 100_000_000)


if __name__ == "__main__":
    unittest.main()
