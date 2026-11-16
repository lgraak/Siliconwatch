"""Deterministic format/statistics fixtures; no hardware or timing sleeps."""

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from overhead_results import parse_output, render_report, rss_summary, summarize


def sample(sequence=1, count=2, failed=False, sensors=True):
    target = (sequence - 1) * 100_001_000
    start = target + 1000
    snapshot = {
        "schema_version": 2, "root": "/private/machine/source",
        "sensors": [{"identity": {"schema_version": 1}, "status": "ok", "label": "private-label", "source_path": "/private/sensor"}] if sensors else [],
        "warnings": [],
    }
    observations = not failed and sensors
    return {
        "record_type": "siliconwatch.sample", "sampling_schema_version": 1,
        "sequence": sequence, "requested_count": count, "interval_ms": 100,
        "timing": {"target_start_ns": target, "started_ns": start, "finished_ns": start + 100,
                   "prepared_ns": start + 120, "scan_duration_ns": 100, "lateness_ns": 1000,
                   "wall_start_unix_ms": 100_000 - sequence, "wall_time_error": None},
        "freshness": {"basis": "no_snapshot" if failed else "current_attempt_observation_window" if sensors else "no_sensor_observations",
                      "lower_bound_ns": 20 if observations else None, "upper_bound_ns": 120 if observations else None,
                      "observation_window_exceeds_interval": False if observations else None, "physical_age_known": False},
        "snapshot": None if failed else snapshot,
        "error": {"kind": "PermissionDenied", "message": "/private/error"} if failed else None,
    }


def table(record, label="Core 0"):
    t, f = record["timing"], record["freshness"]
    lines = [
        f"Sample {record['sequence']}/{record['requested_count']}; interval 100 ms",
        f"Monotonic offsets (ns): target {t['target_start_ns']}, start {t['started_ns']}, finish {t['finished_ns']}, prepared {t['prepared_ns']}",
        f"Scan duration {t['scan_duration_ns']} ns; start lateness {t['lateness_ns']} ns; wall start {t['wall_start_unix_ms']} Unix ms",
        f"Observation basis: {f['basis']}",
    ]
    if f["lower_bound_ns"] is not None:
        lines.append(f"Observation-age bounds at preparation: {f['lower_bound_ns']}..{f['upper_bound_ns']} ns; window exceeds interval: false")
    lines.append("Physical sensor age is unknown; preparation precedes serialization and output.")
    if record["error"]:
        lines.append("Attempt error (PermissionDenied): /private/error")
    else:
        count = len(record["snapshot"]["sensors"])
        lines += ["SiliconWatch - Hardware Sensors", "", "DEVICE  SENSOR  VALUE  STATUS  IDENTITY", "--------------------"]
        if count:
            lines.append(f"coretemp  {label}  42.0 °C  ok  best_effort")
        lines += ["", f"{count} sensors discovered; {count} with status ok.", "Values are unvalidated hardware reports; 'ok' means read and parsed."]
        if count:
            lines.append("Identity v1 /private/path: private-id")
    return "\n".join(lines) + "\n\n"


class ParserTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "output"

    def parse(self, records=None, raw=None, output_format="json-lines", **kwargs):
        if raw is None:
            raw = "".join(json.dumps(record) + "\n" for record in records)
        self.path.write_bytes(raw.encode() if isinstance(raw, str) else raw)
        return parse_output(self.path, output_format, 2, 100, **kwargs)

    def test_valid_json_preserves_counts_but_removes_identifiers(self):
        result = self.parse([sample(), sample(2)])
        self.assertTrue(result["complete"])
        self.assertEqual(result["emitted_count"], 2)
        self.assertEqual(result["whole_run"]["sensor_status_observations"], {"ok": 2})
        self.assertNotIn("private", json.dumps(result))
        self.assertEqual(result["records"][1]["interval_deviation_ns"], 1000)

    def test_valid_table_matches_json_scalars_and_escaped_labels(self):
        records = [sample(), sample(2)]
        result = self.parse(raw="".join(table(r, "fake\\nSample 999/2; interval 100 ms") for r in records), output_format="table")
        self.assertTrue(result["complete"])
        self.assertEqual(result["records"], self.parse(records)["records"])

    def test_failed_attempt_then_recovery_remain_distinct(self):
        result = self.parse([sample(failed=True), sample(2)])
        self.assertTrue(result["complete"])
        self.assertEqual(result["failed_attempts"], 1)
        self.assertEqual(result["whole_run"]["failed_scan_duration_ns"]["n"], 1)
        self.assertEqual(result["whole_run"]["snapshot_scan_duration_ns"]["n"], 1)
        table_result = self.parse(raw=table(sample(failed=True)) + table(sample(2)), output_format="table")
        self.assertEqual(result["records"], table_result["records"])

    def test_empty_snapshots_have_no_freshness_bounds(self):
        records = [sample(sensors=False), sample(2, sensors=False)]
        self.assertTrue(self.parse(records)["complete"])
        self.assertTrue(self.parse(raw="".join(table(r) for r in records), output_format="table")["complete"])

    def test_missing_and_truncated_records_do_not_complete(self):
        result = self.parse([sample()])
        self.assertFalse(result["complete"])
        self.assertEqual(result["emitted_count"], 1)
        result = self.parse(raw=json.dumps(sample()) + "\n" + json.dumps(sample(2)))
        self.assertFalse(result["valid"])
        self.assertEqual(result["emitted_count"], 1)
        result = self.parse(raw=(table(sample()) + table(sample(2))).rstrip("\n") + "\n", output_format="table")
        self.assertEqual(result["emitted_count"], 1)
        self.assertFalse(result["complete"])

    def test_duplicate_out_of_order_and_extra_records_rejected(self):
        for records in ([sample(), sample()], [sample(2), sample()], [sample(), sample(2), sample(3)]):
            with self.subTest(records=len(records)):
                self.assertFalse(self.parse(records)["valid"])

    def test_duplicate_json_keys_and_nonfinite_constants_rejected(self):
        for raw in ('{"sequence":1,"sequence":2}\n', '{"value":NaN}\n', '[]\n', 'null\n'):
            self.assertFalse(self.parse(raw=raw)["valid"])

    def test_schema_and_configuration_mismatch(self):
        for key, value in (("sampling_schema_version", 2), ("sampling_schema_version", True), ("interval_ms", 1000), ("requested_count", 3)):
            record = sample()
            record[key] = value
            self.assertFalse(self.parse([record])["valid"])
        record = sample()
        record["snapshot"]["schema_version"] = 3
        self.assertFalse(self.parse([record])["valid"])

    def test_malformed_timing_freshness_and_scheduling_rejected(self):
        for field, value in (("started_ns", True), ("started_ns", -1), ("scan_duration_ns", 101), ("target_start_ns", 1), ("prepared_ns", 1000)):
            record = sample()
            record["timing"][field] = value
            self.assertFalse(self.parse([record])["valid"])
        record = sample()
        record["freshness"]["upper_bound_ns"] = 121
        self.assertFalse(self.parse([record])["valid"])
        record = sample(2)
        record["timing"]["target_start_ns"] += 1
        self.assertFalse(self.parse([sample(), record])["valid"])

    def test_wall_clock_may_reverse_and_may_be_unavailable(self):
        record = sample()
        record["timing"].update(wall_start_unix_ms=None, wall_time_error="clock unavailable")
        self.assertTrue(self.parse([record, sample(2)])["complete"])
        self.assertTrue(self.parse([sample(), sample(2)])["complete"])

    def test_warmup_and_first_lateness_are_explicit(self):
        result = self.parse([sample(), sample(2)], warmup_seconds=0.05)
        self.assertEqual(result["whole_run"]["record_count"], 2)
        self.assertEqual(result["post_warmup"]["record_count"], 1)
        self.assertEqual(result["whole_run"]["lateness_ns"]["n"], 1)
        self.assertEqual(result["first_start_lateness_ns"], 1000)

    def test_byte_limit_invalid_encoding_and_missing_file(self):
        for raw, limit in ((b"x" * 65 + b"\n", 64), (b"\xff\n", 64)):
            self.assertFalse(self.parse(raw=raw, max_line_bytes=limit)["valid"])
        result = parse_output(self.path.parent / "absent", "table", 2, 100)
        self.assertFalse(result["valid"])
        self.assertEqual(result["emitted_count"], 0)

    def test_table_count_mismatch_and_injected_header_rejected(self):
        raw = table(sample()).replace("1 sensors discovered", "2 sensors discovered")
        self.assertFalse(self.parse(raw=raw, output_format="table")["valid"])
        raw = table(sample()).replace("coretemp  Core 0  42.0 °C  ok  best_effort", "Sample 999/2; interval 100 ms")
        self.assertFalse(self.parse(raw=raw, output_format="table")["valid"])
        raw = table(sample()).replace("Identity v1 /private/path: private-id\n", "")
        self.assertFalse(self.parse(raw=raw, output_format="table")["valid"])

    def test_extreme_numeric_input_is_a_parse_error(self):
        raw = table(sample()).replace("Sample 1/2", "Sample " + "9" * 5000 + "/2")
        self.assertFalse(self.parse(raw=raw, output_format="table")["valid"])
        raw = '{"sequence":' + "9" * 5000 + '}\n'
        self.assertFalse(self.parse(raw=raw)["valid"])

    def test_mutually_exclusive_snapshot_and_error(self):
        for snapshot, error in ((None, None), (sample()["snapshot"], sample(failed=True)["error"])):
            record = sample()
            record.update(snapshot=snapshot, error=error)
            self.assertFalse(self.parse([record])["valid"])

    def test_invalid_parser_configuration_rejected(self):
        for count, interval, warmup in ((0, 100, 0), (10001, 100, 0), (2, 99, 0), (2, 100, -1), (2, 100, float("nan"))):
            with self.assertRaises(ValueError):
                parse_output(self.path, "json-lines", count, interval, warmup)


class StatisticsTests(unittest.TestCase):
    def test_empty_singleton_even_and_odd(self):
        self.assertEqual(summarize([]), {"n": 0, "median": None, "min": None, "max": None, "p95": None, "p99": None})
        self.assertEqual(summarize([8])["median"], 8)
        self.assertEqual(summarize([7, 1, 3, 5])["median"], 4)
        self.assertEqual(summarize([7, 1, 3])["median"], 3)

    def test_quantile_thresholds_and_nearest_rank(self):
        self.assertIsNone(summarize(range(199))["p95"])
        self.assertEqual(summarize(range(200))["p95"], 189)
        self.assertIsNone(summarize(range(999))["p99"])
        self.assertEqual(summarize(range(1000))["p99"], 989)
        self.assertEqual(summarize(range(1001))["p99"], 990)

    def test_invalid_observations_rejected(self):
        for values in ([True], [float("nan")], [float("inf")], ["2"], [2, "3"]):
            with self.assertRaises(ValueError):
                summarize(values)

    def test_rss_windows_missing_values_and_methods(self):
        samples = [{"offset_seconds": n, "rss_kib": rss, "method": "smaps_rollup", "read_seconds": 0.001} for n, rss in ((0, 10), (10, None), (11, 20), (12, 30), (13, 40))]
        summary = rss_summary(samples, 10)
        self.assertEqual(summary["whole_run"]["n"], 4)
        self.assertEqual(summary["post_warmup"]["median"], 30)
        self.assertEqual(summary["early_post_warmup"]["median"], 20)
        self.assertEqual(summary["late_post_warmup"]["median"], 35)
        self.assertEqual(summary["missing_observations"], 1)
        self.assertEqual(summary["methods"], ["smaps_rollup"])
        self.assertEqual(summary["read_duration_seconds"]["n"], 5)
        self.assertEqual(rss_summary(samples, 10, 20)["early_post_warmup"]["n"], 3)
        self.assertEqual(rss_summary(samples, 10, 20)["late_post_warmup"]["n"], 0)


class ReportTests(unittest.TestCase):
    def test_report_deterministic_and_missing_measurements_explicit(self):
        doc = {"schema_version": 1, "environment": {"rust": "1.85.0", "cpu": "fixture"}, "runs": [{"case": {"id": "test", "format": "json-lines", "count": 2}, "resource": {"termination": "timeout"}, "parsed": None}]}
        report = render_report(doc)
        self.assertEqual(report, render_report(copy.deepcopy(doc)))
        self.assertIn("N/A", report)
        self.assertIn("timeout", report)
        self.assertIn("not statistical confidence guarantees", report)
        self.assertIn("Requested counts and exit status never substitute", report)
        self.assertNotIn("NaN", report)

    def test_report_rejects_unknown_schema(self):
        with self.assertRaises(ValueError):
            render_report({"schema_version": 2})

    def test_incomplete_runs_excluded_from_repetition_summaries(self):
        doc = {"schema_version": 1, "runs": [{"case": {"id": "bad", "format": "table", "interval_ms": 100, "duration_seconds": 30}, "resource": {"cpu_percent": 900}, "validity": {"comparable": False}}]}
        report = render_report(doc)
        self.assertIn("900", report.split("## Repetition comparisons")[0])
        self.assertNotIn("900", report.split("## Repetition comparisons")[1])
        self.assertIn("inherited launch high-water", report)


if __name__ == "__main__":
    unittest.main()
