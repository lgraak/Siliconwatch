"""Harness orchestration contracts without live hardware or long waits."""
import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import measure_overhead as m


class MatrixTests(unittest.TestCase):
    def test_full_matrix_exact_counts_and_spans(self):
        cases = m.generate_matrix("baseline")
        self.assertEqual(len(cases), 24)
        self.assertEqual(sum(c["duration_seconds"] for c in cases), 3960)
        self.assertEqual(len({c["id"] for c in cases}), 24)
        configurations = {(c["interval_ms"], c["count"], c["format"]) for c in cases}
        self.assertEqual(len(configurations), 8)
        for config in configurations:
            self.assertEqual(sum((c["interval_ms"], c["count"], c["format"]) == config for c in cases), 3)
        self.assertEqual(cases, m.generate_matrix("baseline"))

    def test_controls_and_smoke(self):
        controls = m.generate_matrix("controls")
        self.assertEqual(len(controls), 12)
        self.assertEqual(sum(c["duration_seconds"] for c in controls), 360)
        self.assertTrue(all(not c["observe_rss"] for c in controls))
        self.assertEqual(len(m.generate_matrix("smoke", 1)), 4)

    def test_bounds_and_arguments(self):
        for i, n in ((99, 1), (60001, 1), (100, 0), (100, 10001)):
            with self.assertRaises(ValueError):
                m.case(i, n, "table")
        for args in (["--count", "0", "--plan"], ["--matrix", "baseline", "--settle-seconds", "0", "--plan"],
                     ["--count", "1", "--warmup-seconds", "nan", "--plan"], ["--matrix", "smoke", "--count", "1", "--plan"]):
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
                m.arguments(args)
        args, cases = m.arguments(["--count", "2", "--repeats", "1", "--plan"])
        self.assertEqual(cases[0]["count"], 2)

    def test_child_argv_no_shell(self):
        args = m.child_args(Path("/tmp/a b"), m.case(100, 2, "json-lines"), Path("/tmp/$(not-run)"))
        self.assertEqual(args[0], "/tmp/a b")
        self.assertEqual(args[-1], "/tmp/$(not-run)")


class ResultsTests(unittest.TestCase):
    def test_no_inferred_attempt_counts(self):
        r = {"termination": "completed", "exit_code": 0, "total_cpu_seconds": .1}
        self.assertIsNone(m.observed_validity(r, {"complete": False, "emitted_count": None})["actual_attempts"])
        p = {"complete": True, "emitted_count": 2}
        self.assertEqual(m.observed_validity(r, p)["cpu_ms_per_attempt"], 50)
        self.assertIsNone(m.observed_validity(dict(r, termination="timeout"), p)["actual_attempts"])

    def test_rss_missing_and_windows(self):
        r = {"elapsed_seconds": 30, "rss_samples": [
            {"offset_seconds": 1, "rss_kib": 100}, {"offset_seconds": 11, "rss_kib": None},
            {"offset_seconds": 12, "rss_kib": 200}, {"offset_seconds": 25, "rss_kib": 300}]}
        s = m.rss_summary(r, 10)
        self.assertEqual(s["missing_observations"], 1)
        self.assertEqual(s["post_warmup"]["median"], 250)
        self.assertEqual(s["early_post_warmup"]["median"], 200)
        self.assertIsNone(m.rss_summary({"elapsed_seconds": 1}, 10)["whole_run"]["median"])

    def test_projection_requires_verified_matching_binary(self):
        runs = [{"case": {"format": f, "count": 11, "duration_seconds": 1},
                 "resource": {"stdout_bytes": 1100, "elapsed_seconds": 1.1},
                 "parsed": {"complete": True}, "validity": {"complete": True}}
                for f in ("table", "json-lines")]
        smoke = {"binary_sha256": "abc", "status": "complete", "runs": runs}
        p = m.projection(m.generate_matrix("baseline"), smoke, "abc", 15)
        self.assertLess(p["estimated_seconds_including_settling"], 5400)
        self.assertLess(p["estimated_bytes_including_reserve"], 1024 * m.MIB)
        with self.assertRaises(ValueError):
            m.projection([], smoke, "wrong", 15)
        with self.assertRaises(ValueError):
            m.projection([], dict(smoke, status="interrupted"), "abc", 15)

    def test_summarize_reproducible_without_spawn(self):
        doc = {"schema_version": 1, "environment": {}, "runs": [], "status": "complete"}
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            m.save(doc, path)
            first = (path / "report.md").read_text()
            m.save(m.load_result(path), path)
            self.assertEqual(first, (path / "report.md").read_text())
            with patch.object(m, "run_process", side_effect=AssertionError("must not spawn")):
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    self.assertEqual(m.main(["--summarize", str(path)]), 0)
                self.assertEqual(out.getvalue(), first)

    def test_metadata_projection_and_atomic_storage_reserve(self):
        smoke = {"binary_sha256": "abc", "status": "complete", "runs": [
            {"case": {"format": f, "count": 11, "duration_seconds": 1},
             "resource": {"stdout_bytes": 1100, "elapsed_seconds": 1.02},
             "parsed": {"complete": True, "records": [{"timing": "x" * 700}]},
             "validity": {"complete": True}} for f in ("table", "json-lines")]}
        result = m.projection(m.generate_matrix("baseline"), smoke, "abc", 15)
        self.assertLess(result["estimated_metadata_bytes"], m.MAX_ENCODED_RESULT_BYTES)
        self.assertLess(2 * m.MAX_ENCODED_RESULT_BYTES + m.MIB, m.MAX_RESULT_BYTES)
        for r in smoke["runs"]:
            r["parsed"]["records"] = [{"timing": "x" * 3000}]
        with self.assertRaises(ValueError):
            m.projection(m.generate_matrix("baseline"), smoke, "abc", 15)


if __name__ == "__main__":
    unittest.main()
