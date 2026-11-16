"""Suite guards and retained failures using fake children, never live hardware."""

import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import measure_overhead as harness


class OrchestrationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / "fixture-binary"
        self.binary.write_bytes(b"not an executable; spawning must be mocked")
        self.output = self.root / "results"
        self.inventory = [{"provider": "fixture", "exposed_input_count": 1}]

    def args(self, *options):
        return harness.arguments(["--binary", str(self.binary), "--output-dir", str(self.output), *options])

    def resources(self, termination="completed", exit_code=0):
        return {"termination": termination, "exit_code": exit_code,
                "signal": None, "reaped": True, "elapsed_seconds": 0.01,
                "total_cpu_seconds": 0.002, "user_cpu_seconds": 0.001,
                "system_cpu_seconds": 0.001, "cpu_percent": 20,
                "peak_rss_kib": 10, "rss_samples": [], "rss_method": None,
                "stdout_bytes": 1, "stderr_bytes": 0, "runner_cpu_seconds": 0.001}

    def run_mock_suite(self, termination="completed", *, interrupt=False, sink="file"):
        args, cases = self.args("--count", "2", "--repeats", "2", "--settle-seconds", "0", "--sink", sink)

        def child(_argv, output_dir, **kwargs):
            output_dir.mkdir()
            if sink == "file":
                (output_dir / "stdout.raw").write_bytes(b"{\n")
            if interrupt:
                kwargs["interrupt_event"].set()
            result = self.resources(termination, 0 if termination == "completed" else None)
            if sink == "null":
                result["stdout_bytes"] = None
            return result

        with patch.object(harness, "run_process", side_effect=child) as runner, \
             patch.object(harness, "inventory", return_value=self.inventory), \
             patch.object(harness, "collect_environment", return_value={"provider_inventory": self.inventory}), \
             patch.object(harness.shutil, "disk_usage", return_value=SimpleNamespace(free=10 * 1024**3)), \
             patch.object(harness.os, "getloadavg", return_value=(0, 0, 0)), \
             contextlib.redirect_stdout(io.StringIO()):
            code = harness.execute(args, cases)
        document = json.loads((self.output / "results.json").read_text())
        return code, runner.call_count, document

    def test_smoke_inventory_mismatch_blocks_before_launch_or_output_creation(self):
        args, cases = self.args("--matrix", "baseline", "--preflight-smoke", str(self.root / "smoke"))
        smoke = {"configuration": {"hwmon_root": "system"},
                 "environment": {"provider_inventory": []}, "runs": []}
        with patch.object(harness, "inventory", return_value=self.inventory), \
             patch.object(harness, "load_result", return_value=smoke), \
             patch.object(harness, "run_process") as runner, \
             self.assertRaisesRegex(ValueError, "cohort differs"):
            harness.execute(args, cases)
        runner.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_smoke_root_mismatch_blocks_even_with_matching_inventory(self):
        args, cases = self.args("--matrix", "baseline", "--preflight-smoke", str(self.root / "smoke"))
        smoke = {"configuration": {"hwmon_root": "custom (path omitted)"},
                 "environment": {"provider_inventory": self.inventory}, "runs": []}
        with patch.object(harness, "inventory", return_value=self.inventory), \
             patch.object(harness, "load_result", return_value=smoke), \
             patch.object(harness, "run_process") as runner, \
             self.assertRaisesRegex(ValueError, "cohort differs"):
            harness.execute(args, cases)
        runner.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_recurring_smoke_delay_rejects_long_case_before_execution(self):
        runs = [{"case": {"format": fmt, "count": 11, "duration_seconds": 1},
                 "resource": {"stdout_bytes": 1000, "elapsed_seconds": 1.5},
                 "parsed": {"complete": True}, "validity": {"complete": True}}
                for fmt in ("table", "json-lines")]
        smoke = {"binary_sha256": "fixture", "status": "complete", "runs": runs}
        with self.assertRaisesRegex(ValueError, "projected case runtime exceeds timeout"):
            harness.projection([harness.case(100, 3001, "table")], smoke, "fixture", 15)

    def test_exit_zero_with_malformed_output_retains_failure_and_stops(self):
        code, calls, document = self.run_mock_suite()
        self.assertEqual(code, 1)
        self.assertEqual(calls, 1)
        self.assertEqual(document["status"], "stopped_after_incomplete_run")
        self.assertEqual(len(document["not_started"]), 1)
        run = document["runs"][0]
        self.assertEqual(run["resource"]["exit_code"], 0)
        self.assertFalse(run["parsed"]["complete"])
        self.assertTrue(run["parsed"]["errors"])
        self.assertIsNone(run["validity"]["actual_attempts"])
        self.assertIsNone(run["validity"]["cpu_ms_per_attempt"])
        self.assertFalse(run["validity"]["comparable"])

    def test_timeout_is_retained_and_no_subsequent_case_starts(self):
        code, calls, document = self.run_mock_suite("timeout")
        self.assertEqual((code, calls), (1, 1))
        self.assertEqual(document["runs"][0]["resource"]["termination"], "timeout")
        self.assertIsNone(document["runs"][0]["validity"]["actual_attempts"])
        self.assertEqual(len(document["not_started"]), 1)

    def test_interruption_is_retained_and_no_subsequent_case_starts(self):
        code, calls, document = self.run_mock_suite("interrupted", interrupt=True)
        self.assertEqual((code, calls), (1, 1))
        self.assertEqual(document["runs"][0]["resource"]["termination"], "interrupted")
        self.assertIsNone(document["runs"][0]["validity"]["actual_attempts"])
        self.assertEqual(len(document["not_started"]), 1)

    def test_null_sink_can_finish_but_never_establishes_attempt_count(self):
        code, calls, document = self.run_mock_suite(sink="null")
        self.assertEqual((code, calls), (0, 2))
        self.assertEqual(document["status"], "complete")
        self.assertFalse(document["not_started"])
        for run in document["runs"]:
            self.assertIsNone(run["parsed"]["emitted_count"])
            self.assertIsNone(run["parsed"]["failed_attempts"])
            self.assertIsNone(run["validity"]["actual_attempts"])
            self.assertIsNone(run["validity"]["cpu_ms_per_attempt"])

    def test_configured_and_effective_timeouts_are_retained(self):
        for override, expected in ((None, 10.1), (0.25, 0.25), (60, 20)):
            with self.subTest(override=override):
                self.output = self.root / f"timeout-{override}"
                options = ["--count", "2", "--repeats", "1", "--settle-seconds", "0",
                           "--sink", "null", "--suite-timeout-seconds", "20"]
                if override is not None:
                    options.extend(["--timeout-seconds", str(override)])
                args, cases = self.args(*options)
                observed_timeouts = []

                def child(_argv, output_dir, **kwargs):
                    output_dir.mkdir()
                    observed_timeouts.append(kwargs["timeout_s"])
                    result = self.resources("timeout", None)
                    result["stdout_bytes"] = None
                    return result

                with patch.object(harness, "run_process", side_effect=child), \
                     patch.object(harness, "inventory", return_value=self.inventory), \
                     patch.object(harness, "collect_environment", return_value={}), \
                     patch.object(harness.shutil, "disk_usage", return_value=SimpleNamespace(free=10 * 1024**3)), \
                     patch.object(harness.os, "getloadavg", return_value=(0, 0, 0)), \
                     patch.object(harness.time, "monotonic", return_value=100.0), \
                     contextlib.redirect_stdout(io.StringIO()):
                    self.assertEqual(harness.execute(args, cases), 1)

                document = json.loads((self.output / "results.json").read_text())
                self.assertEqual(document["configuration"]["timeout_override_seconds"], override)
                self.assertEqual(observed_timeouts, [expected])
                self.assertEqual(document["runs"][0]["effective_timeout_seconds"], expected)
                self.assertIsNone(document["runs"][0]["validity"]["actual_attempts"])


if __name__ == "__main__":
    unittest.main()
