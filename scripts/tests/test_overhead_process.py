"""Hardware-independent process/measurement fixtures for the benchmark runner."""

import json
import os
from pathlib import Path
import signal
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from overhead_process import read_rss, run_process


class ProcessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name)

    def run_child(self, source, **kwargs):
        options = dict(timeout_s=2.0, output_limit_bytes=65536,
                       rss_interval_s=0.01, grace_s=0.05, reap_timeout_s=0.5)
        options.update(kwargs)
        return run_process([sys.executable, "-c", source], self.path / "capture", **options)

    def test_exit_and_private_capture_and_resource_units(self):
        result = self.run_child("import sys; print('output'); print('error', file=sys.stderr)")
        self.assertEqual((result["termination"], result["exit_code"]), ("completed", 0))
        self.assertTrue(result["reaped"])
        self.assertEqual((self.path / "capture/stdout.raw").read_bytes(), b"output\n")
        self.assertEqual((self.path / "capture/stderr.raw").read_bytes(), b"error\n")
        self.assertEqual(result["stdout_bytes"], 7)
        self.assertEqual(result["stderr_bytes"], 6)
        self.assertEqual((self.path / "capture/stdout.raw").stat().st_mode & 0o777, 0o600)
        self.assertGreater(result["elapsed_seconds"], 0)
        self.assertEqual(result["total_cpu_seconds"], result["user_cpu_seconds"] + result["system_cpu_seconds"])
        self.assertAlmostEqual(result["cpu_percent"], 100 * result["total_cpu_seconds"] / result["elapsed_seconds"])
        self.assertGreater(result["peak_rss_kib"], 0)
        self.assertEqual(result["peak_rss_scope"], "kernel_child_lifetime_including_inherited_launch_peak")
        json.dumps(result, allow_nan=False)

    def test_nonzero_exit_is_not_claimed_successful_data(self):
        result = self.run_child("raise SystemExit(7)")
        self.assertEqual(result["exit_code"], 7)
        self.assertEqual(result["termination"], "completed")
        self.assertNotIn("completed_attempts", result)

    def test_timeout_escalates_child_ignoring_term(self):
        result = self.run_child("import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); time.sleep(30)", timeout_s=0.2)
        self.assertEqual(result["termination"], "timeout")
        self.assertEqual(result["signal"], signal.SIGKILL)
        self.assertTrue(result["reaped"])

    def test_preexisting_interruption_does_not_launch(self):
        event = threading.Event()
        event.set()
        result = self.run_child("raise RuntimeError('must not execute')", interrupt_event=event)
        self.assertEqual(result["termination"], "interrupted")
        self.assertIsNone(result["total_cpu_seconds"])
        self.assertEqual(result["stdout_bytes"], 0)

    def test_running_child_interruption(self):
        class StopAfterLaunch:
            calls = 0

            def is_set(self):
                self.calls += 1
                return self.calls >= 3

        result = self.run_child("import time; time.sleep(30)", interrupt_event=StopAfterLaunch())
        self.assertEqual(result["termination"], "interrupted")
        self.assertTrue(result["reaped"])
        self.assertEqual(result["signal"], signal.SIGTERM)

    def test_keyboard_interrupt_is_recorded_and_child_reaped(self):
        class RaiseOnceAfterLaunch:
            calls = 0

            def is_set(self):
                self.calls += 1
                if self.calls == 3:
                    raise KeyboardInterrupt
                return False

        result = self.run_child("import time; time.sleep(30)", interrupt_event=RaiseOnceAfterLaunch())
        self.assertEqual(result["termination"], "interrupted")
        self.assertTrue(result["reaped"])

    def test_combined_output_budget_is_hard_bound(self):
        result = self.run_child("import os; os.write(1, b'x' * 1000000); os.write(2, b'y' * 1000000)", output_limit_bytes=1234)
        self.assertEqual(result["termination"], "output_limit")
        self.assertTrue(result["stdout_truncated"] or result["stderr_truncated"])
        size = sum(p.stat().st_size for p in (self.path / "capture").iterdir())
        self.assertEqual(size, 1234)
        self.assertEqual(size, result["stdout_bytes"] + result["stderr_bytes"])
        self.assertTrue(result["reaped"])

    def test_stderr_counts_toward_combined_limit(self):
        result = self.run_child("import os; os.write(2,b'x'*20000)", output_limit_bytes=100)
        self.assertEqual(result["termination"], "output_limit")
        self.assertEqual(result["stderr_bytes"], 100)
        self.assertTrue(result["stderr_truncated"])

    def test_exact_budget_without_excess_is_complete(self):
        result = self.run_child("import os; os.write(1,b'x'*100)", output_limit_bytes=100)
        self.assertEqual(result["termination"], "completed")
        self.assertFalse(result["stdout_truncated"])

    def test_null_output_is_unmeasured_not_zero(self):
        result = self.run_child("print('discarded')", sink="null", observe_rss=False)
        self.assertEqual(result["exit_code"], 0)
        self.assertIsNone(result["stdout_bytes"])
        self.assertFalse((self.path / "capture/stdout.raw").exists())
        self.assertEqual(result["rss_samples"], [])
        self.assertIsNone(result["rss_method"])

    def test_missing_executable_records_setup_error_and_null_metrics(self):
        result = run_process([str(self.path / "missing")], self.path / "capture", timeout_s=1, output_limit_bytes=100)
        self.assertEqual(result["termination"], "setup_error")
        self.assertEqual(result["setup_error"]["errno"], 2)
        self.assertIsNone(result["peak_rss_kib"])
        self.assertIsNone(result["total_cpu_seconds"])
        self.assertNotIn(str(self.path), json.dumps(result))

    def test_existing_capture_is_not_overwritten(self):
        (self.path / "capture").mkdir()
        with self.assertRaises(FileExistsError):
            self.run_child("print('must not execute')")

    def test_capture_failure_records_error_and_reaps_child(self):
        with patch("overhead_process.os.write", side_effect=OSError(28, "private path must not appear")):
            result = self.run_child("import os,time; os.write(1,b'ready'); time.sleep(30)")
        self.assertEqual(result["termination"], "capture_error")
        self.assertTrue(result["reaped"])
        self.assertEqual(result["setup_error"], {"type": "OSError", "errno": 28})
        self.assertNotIn("private path", json.dumps(result))

    def test_missing_rss_samples_remain_null(self):
        with patch("overhead_process.read_rss", return_value=(None, None)):
            result = self.run_child("import time; time.sleep(0.08)")
        self.assertGreater(len(result["rss_samples"]), 0)
        self.assertTrue(all(sample["rss_kib"] is None for sample in result["rss_samples"]))
        self.assertEqual(result["sampled_read_failures"], len(result["rss_samples"]))

    def test_allocation_child_records_observed_and_kernel_memory_separately(self):
        result = self.run_child("import time; allocation=bytearray(16*1024*1024); time.sleep(0.1)")
        observed = [sample["rss_kib"] for sample in result["rss_samples"] if sample["rss_kib"] is not None]
        self.assertTrue(observed)
        self.assertGreater(max(observed), 16 * 1024)
        self.assertGreater(result["peak_rss_kib"], 16 * 1024)
        self.assertIn(result["rss_method"], ("smaps_rollup", "status_vmrss_approximate"))
        # No exact allocator/peak equivalence assertion: the quantities differ.

    def test_invalid_configuration_fails_before_capture(self):
        for option in ({"timeout_s": 0}, {"timeout_s": float("nan")},
                       {"rss_interval_s": 0}, {"output_limit_bytes": 0},
                       {"output_limit_bytes": True}, {"sink": "terminal"},
                       {"timeout_s": 1000, "rss_interval_s": 0.001}):
            with self.subTest(option=option), self.assertRaises(ValueError):
                self.run_child("pass", **option)
        self.assertFalse((self.path / "capture").exists())


class RssFixtureTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.proc = self.root / "123"
        self.proc.mkdir()
        self.write_stat("start1")

    def write_stat(self, start):
        (self.proc / "stat").write_text("123 (name ) with spaces) " + " ".join(["S"] + ["0"] * 18 + [start]) + "\n")

    def test_prefers_smaps_and_retains_method_after_failure(self):
        (self.proc / "smaps_rollup").write_text("Rss: 4096 kB\n")
        (self.proc / "status").write_text("VmRSS: 2048 kB\n")
        self.assertEqual(read_rss(123, "start1", proc_root=self.root), (4096, "smaps_rollup"))
        (self.proc / "smaps_rollup").unlink()
        self.assertEqual(read_rss(123, "start1", "smaps_rollup", self.root), (None, "smaps_rollup"))

    def test_fallback_is_explicitly_approximate(self):
        (self.proc / "status").write_text("VmRSS: 2048 kB\n")
        self.assertEqual(read_rss(123, "start1", proc_root=self.root), (2048, "status_vmrss_approximate"))

    def test_reused_pid_is_rejected(self):
        (self.proc / "smaps_rollup").write_text("Rss: 4096 kB\n")
        self.write_stat("start2")
        self.assertEqual(read_rss(123, "start1", proc_root=self.root), (None, None))

    def test_missing_malformed_and_negative_values_are_unavailable(self):
        for text in ("Rss: abc kB\n", "Rss: -1 kB\n", "Rss: 12 bytes\n", "Nothing\n"):
            with self.subTest(text=text):
                (self.proc / "smaps_rollup").write_text(text)
                self.assertEqual(read_rss(123, "start1", proc_root=self.root), (None, None))


if __name__ == "__main__":
    unittest.main()
