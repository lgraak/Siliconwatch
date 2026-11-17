"""Provider evidence fixtures; no hardware, real scheduling, or root privileges."""

import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import provider_results as results


def registration(token="r001", entry="hwmon1", provider="fixture", channels=None):
    return {"token": token, "entry": entry, "path": f"/private/full/{entry}",
            "target": f"/private/hardware/controller-{token}/hwmon/{entry}",
            "provider": provider, "channels": channels or ["temp1_input"], "status": "ok"}


def samples(registrations, count=3, *, root="/private/full", durations=None):
    records, previous_start, previous_prepared = [], 0, 0
    for index in range(count):
        target = previous_start + 100_000_000 if index else 0
        start = max(target + 1000, previous_prepared)
        finish = start + (durations[index] if durations else 1_000_000)
        prepared = finish + 10
        sensors = []
        for reg in sorted(registrations, key=lambda r: r["entry"]):
            for channel in reg["channels"]:
                parent = str(Path(reg["target"]).parent.parent)
                sensors.append({"device": reg["provider"], "device_path": reg["target"],
                    "attribute": channel, "label": "private label", "source_path": f"{root}/{reg['entry']}/{channel}",
                    "status": "ok", "raw_text": "42000", "raw_value": 42000, "value": 42.0,
                    "unit": "°C", "error": None,
                    "identity": {"schema_version": 1, "candidate": f"private candidate {parent} {channel}",
                        "status": "best_effort", "issues": [],
                        "evidence": {"provider": reg["provider"], "parent": parent,
                            "device_link": parent, "canonical_parent": parent,
                            "channel": channel, "measurement": "temperature_input_millidegree_celsius"}}})
        records.append({"record_type": "siliconwatch.sample", "sampling_schema_version": 1,
            "sequence": index + 1, "requested_count": count, "interval_ms": 100,
            "timing": {"target_start_ns": target, "started_ns": start, "finished_ns": finish,
                "prepared_ns": prepared, "scan_duration_ns": finish - start,
                "lateness_ns": start - target, "wall_start_unix_ms": 100_000 - index,
                "wall_time_error": None},
            "freshness": {"basis": "current_attempt_observation_window" if sensors else "no_sensor_observations",
                "lower_bound_ns": prepared - finish if sensors else None,
                "upper_bound_ns": prepared - start if sensors else None,
                "observation_window_exceeds_interval": prepared - start > 100_000_000 if sensors else None,
                "physical_age_known": False},
            "snapshot": {"schema_version": 2, "root": root, "sensors": sensors, "warnings": []}, "error": None})
        previous_start, previous_prepared = start, prepared
    return records


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / "capture.jsonl"
        self.regs = [registration()]

    def write(self, records):
        self.path.write_text("".join(json.dumps(record) + "\n" for record in records))

    def inspect(self, records, count=None, registrations=None, warmup=0):
        self.write(records)
        config = {"count": count or len(records), "interval_ms": 100, "warmup_seconds": warmup}
        return results.inspect_capture(self.path, config, registrations or self.regs)

    def test_valid_counts_and_tail_neighbors_are_sanitized(self):
        records = samples(self.regs, durations=[1_000_000, 200_000_000, 1_000_000])
        result = self.inspect(records)
        self.assertTrue(result["coverage"]["complete"])
        self.assertTrue(result["coverage"]["usable"])
        self.assertEqual(result["coverage"]["successful_readings"], 3)
        self.assertEqual(result["tails"]["sequences"], [2])
        self.assertEqual(result["tails"]["neighbor_sequences"], [1, 2, 3])
        self.assertEqual(result["parsed"]["records"][1]["scan_duration_ns"], 200_000_000)
        self.assertNotIn("private", json.dumps(result))
        self.assertEqual(json.loads(self.path.read_text().splitlines()[0])["snapshot"]["sensors"][0]["raw_value"], 42000)

    def test_distinct_duplicate_provider_names_stay_separate(self):
        regs = self.regs + [registration("r002", "hwmon2")]
        result = self.inspect(samples(regs), registrations=regs)
        self.assertTrue(result["coverage"]["complete"])
        self.assertEqual([r["token"] for r in result["coverage"]["registrations"]], ["r001", "r002"])
        self.assertEqual(result["coverage"]["successful_readings"], 6)

    def test_failed_and_unsupported_readings_are_distinct(self):
        regs = [registration(channels=["freq1_input", "temp1_input", "temp2_input"])]
        records = samples(regs)
        for record in records:
            record["snapshot"]["sensors"][0]["status"] = "unsupported"
            record["snapshot"]["sensors"][1]["status"] = "permission_denied"
        result = self.inspect(records, registrations=regs)
        self.assertEqual(result["coverage"]["successful_readings"], 3)
        self.assertEqual(result["coverage"]["failed_readings"], 3)
        self.assertEqual(result["coverage"]["unsupported_readings"], 3)
        self.assertTrue(result["coverage"]["usable"])

    def test_all_unreadable_cohort_is_not_usable(self):
        records = samples(self.regs)
        for record in records:
            record["snapshot"]["sensors"][0]["status"] = "unavailable"
        result = self.inspect(records)
        self.assertTrue(result["coverage"]["complete"])
        self.assertFalse(result["coverage"]["usable"])
        self.assertEqual(result["coverage"]["unusable_snapshot_count"], 3)

    def test_missing_duplicate_and_reordered_channels_are_rejected(self):
        regs = [registration(channels=["temp1_input", "temp2_input"])]
        for mode in ("missing", "duplicate", "reordered"):
            records = samples(regs)
            rows = records[0]["snapshot"]["sensors"]
            if mode == "missing": rows.pop()
            elif mode == "duplicate": rows[1] = copy.deepcopy(rows[0])
            else: rows.reverse()
            result = self.inspect(records, registrations=regs)
            self.assertFalse(result["coverage"]["valid"])
            self.assertFalse(result["coverage"]["complete"])
            self.assertNotIn("private", json.dumps(result))

    def test_unexpected_registration_and_source_root_are_rejected(self):
        for field, value in (("device_path", "/other/private/device"),
                             ("source_path", "/other/private/hwmon1/temp1_input")):
            records = samples(self.regs)
            records[0]["snapshot"]["sensors"][0][field] = value
            result = self.inspect(records)
            self.assertFalse(result["coverage"]["valid"])
            self.assertNotIn("private", json.dumps(result))

    def test_empty_exit_zero_style_snapshot_does_not_establish_cohort(self):
        result = self.inspect(samples([], count=1), registrations=self.regs)
        self.assertTrue(result["parsed"]["complete"])
        self.assertFalse(result["coverage"]["complete"])
        self.assertEqual(result["coverage"]["missing_tokens"], ["r001"])

    def test_truncated_prefix_and_failed_attempt_remain_explicit(self):
        records = samples(self.regs)
        first = records[0]
        first["snapshot"], first["error"] = None, {"kind": "Io", "message": "private failure"}
        first["freshness"].update(basis="no_snapshot", lower_bound_ns=None,
            upper_bound_ns=None, observation_window_exceeds_interval=None)
        result = self.inspect(records[:2], count=3)
        self.assertFalse(result["parsed"]["complete"])
        self.assertEqual(result["parsed"]["emitted_count"], 2)
        self.assertEqual(result["parsed"]["failed_attempts"], 1)
        self.assertFalse(result["coverage"]["complete"])
        self.assertNotIn("private", json.dumps(result))

    def test_timing_and_schema_validation_reused(self):
        for mutate in (lambda r: r["timing"].update(scan_duration_ns=7),
                       lambda r: r.update(sampling_schema_version=9)):
            records = samples(self.regs)
            mutate(records[0])
            result = self.inspect(records)
            self.assertFalse(result["parsed"]["valid"])
            self.assertEqual(result["parsed"]["emitted_count"], 0)

    def test_quantile_thresholds_and_wall_clock_changes(self):
        result = self.inspect(samples(self.regs, count=301), warmup=10)
        self.assertIsNotNone(result["parsed"]["whole_run"]["scan_duration_ns"]["p95"])
        self.assertIsNone(result["parsed"]["whole_run"]["scan_duration_ns"]["p99"])
        self.assertEqual(result["parsed"]["records"][-1]["wall_start_unix_ms"], 99700)
        self.assertTrue(result["coverage"]["complete"])

    def test_record_and_line_bounds(self):
        self.write(samples(self.regs, count=1))
        with self.assertRaises(ValueError):
            results.inspect_capture(self.path, {"count": 302, "interval_ms": 100}, self.regs)
        self.path.write_text("x" * (results.MAX_LINE_BYTES + 1) + "\n")
        result = results.inspect_capture(self.path, {"count": 1, "interval_ms": 100}, self.regs)
        self.assertFalse(result["parsed"]["valid"])
        self.assertEqual(result["parsed"]["emitted_count"], 0)

    def test_identity_profiles_are_private_and_alias_changes_expected(self):
        self.write(samples(self.regs, root="/private/full"))
        full = results.identity_profile(self.path, self.regs)
        self.write(samples(self.regs, root="/private/alias"))
        isolated = results.identity_profile(self.path, self.regs)
        result = results.compare_identity_profiles(full, isolated, "r001")
        self.assertTrue(result["compatible"])
        self.assertTrue(result["root_changed"])
        self.assertEqual(result["source_path_changes"], 1)
        self.assertNotIn("private", json.dumps(result))

    def test_collision_change_is_explicit_but_evidence_change_rejected(self):
        self.write(samples(self.regs))
        full = results.identity_profile(self.path, self.regs)
        isolated = copy.deepcopy(full)
        identity = full["registrations"]["r001"]["temp1_input"]["identity"]
        identity.update(status="collision", issues=["duplicate identity candidate; every source is retained"])
        comparison = results.compare_identity_profiles(full, isolated, "r001")
        self.assertTrue(comparison["compatible"])
        self.assertEqual(comparison["changes"][0]["from_status"], "collision")
        isolated["registrations"]["r001"]["temp1_input"]["identity"]["evidence"]["parent"] = "/changed/private"
        self.assertFalse(results.compare_identity_profiles(full, isolated, "r001")["compatible"])

    def test_identity_unexpected_issues_and_missing_channel_rejected(self):
        self.write(samples(self.regs))
        full = results.identity_profile(self.path, self.regs)
        isolated = copy.deepcopy(full)
        isolated["registrations"]["r001"]["temp1_input"]["identity"]["issues"] = ["private new problem"]
        result = results.compare_identity_profiles(full, isolated, "r001")
        self.assertFalse(result["compatible"])
        self.assertNotIn("private", json.dumps(result))
        isolated["registrations"]["r001"] = {}
        self.assertFalse(results.compare_identity_profiles(full, isolated, "r001")["compatible"])

    def test_profile_size_and_missing_snapshot_are_explicit(self):
        self.write(samples(self.regs))
        with patch.object(results, "MAX_PROFILE_BYTES", 10), self.assertRaises(results.ParseError):
            results.identity_profile(self.path, self.regs)
        self.path.write_text("")
        with self.assertRaises(results.ParseError):
            results.identity_profile(self.path, self.regs)

    def test_report_is_reproducible_and_unknown_resources_not_zero(self):
        captured = self.inspect(samples(self.regs))
        case = {"id": "001-full", "phase": "primary", "role": "full_before", "token": None}
        document = {"schema_version": 1, "status": "incomplete", "environment": {},
            "inventory": [{"token": "r001", "provider": "fixture", "channel_count": 1, "status": "ok"}],
            "runs": [{"case": case, "resource": {"termination": "interrupted"},
                      **captured, "validity": {"complete": False, "actual_attempts": None}}]}
        text = results.render_report(document)
        self.assertEqual(text, results.render_report(json.loads(json.dumps(document))))
        self.assertIn("inherited launch", text)
        self.assertIn("N/A", text)
        self.assertIn("503.365", text)
        self.assertNotIn("private", text)
        with self.assertRaises(ValueError):
            results.render_report({"schema_version": 2})

    def test_report_exposes_reporting_failures_without_changing_legacy_output(self):
        document = {"schema_version": 1, "status": "stopped_error", "runs": []}
        legacy = results.render_report(document)
        document["reporting_errors"] = {}
        self.assertEqual(results.render_report(document), legacy)
        document["reporting_errors"]["stdout"] = {"type": "BrokenPipeError", "errno": 32}
        report = results.render_report(document)
        self.assertIn('"type": "BrokenPipeError"', report)
        self.assertIn('"errno": 32', report)
        self.assertIn('"stdout"', report)
        self.assertEqual(report.split("\n## Reporting failures")[0].rstrip(), legacy.rstrip())
        self.assertEqual(report, results.render_report(json.loads(json.dumps(document))))
        document["prior_stop_reason"] = "inventory_changed"
        self.assertIn("Earlier stop reason: **inventory_changed**.", results.render_report(document))

    def test_report_reads_recorded_global_and_per_case_bounds(self):
        smoke = {"id": "001-smoke", "count": 11, "interval_ms": 100,
                 "warmup_seconds": 0, "timeout_seconds": 5, "output_limit_bytes": 1024**2}
        primary = {"id": "002-primary", "count": 301, "interval_ms": 100,
                   "warmup_seconds": 10, "timeout_seconds": 40, "output_limit_bytes": 8 * 1024**2}
        document = {"schema_version": 1, "status": "incomplete",
                    "configuration": {"suite_timeout_seconds": 1500,
                        "suite_limit_bytes": 256 * 1024**2, "metadata_reserve_bytes": 64 * 1024**2,
                        "settle_seconds": 15, "observe_rss": False, "cases": [smoke, primary]},
                    "not_started": [primary["id"]],
                    "runs": [{"case": smoke, "effective_timeout_seconds": 4.75}]}
        report = results.render_report(document)
        self.assertIn('"suite_timeout_seconds": 1500', report)
        self.assertIn('"suite_limit_bytes": 268435456', report)
        self.assertIn('"metadata_reserve_bytes": 67108864', report)
        self.assertIn('"settle_seconds": 15', report)
        self.assertIn('"observe_rss": false', report)
        self.assertIn("| 001-smoke | 11 | 100 | 0 | 5 | 4.75 | 1048576 |", report)
        self.assertIn("| 002-primary | 301 | 100 | 10 | 40 | N/A | 8388608 |", report)
        self.assertEqual(report, results.render_report(json.loads(json.dumps(document))))


if __name__ == "__main__":
    unittest.main()
