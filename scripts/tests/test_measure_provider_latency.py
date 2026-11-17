"""Protocol orchestration fixtures; fake timing/children, never real hardware."""

import contextlib
import io
import itertools
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import measure_provider_latency as harness
from provider_selection import SelectionError, enumerate_registrations
from test_provider_results import samples


class Event:
    def __init__(self):
        self.stopped = False

    def is_set(self):
        return self.stopped

    def set(self):
        self.stopped = True

    def wait(self, _seconds):
        return self.stopped


class ProtocolTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / 'class'
        self.root.mkdir()
        self.targets = []
        for n in range(2):
            target = self.base / f'devices/controller{n}/hwmon/hwmon{n}'
            target.mkdir(parents=True)
            (target / 'name').write_text(f'fixture{n}')
            (target / 'temp1_input').write_text('42000')
            (self.root / target.name).symlink_to(target)
            self.targets.append(target)
        self.regs = enumerate_registrations(self.root)
        self.binary = self.base / 'fixture-binary'
        self.binary.write_bytes(b'fixture; actual spawning must be mocked')
        self.output = self.base / 'result'
        self.event = Event()
        self.calls = []

    def args(self, *extra):
        return harness.arguments(['--run', '--binary', str(self.binary), '--hwmon-root', str(self.root),
                                  '--output-dir', str(self.output), *extra])

    def child(self, argv, output, **kwargs):
        self.calls.append((argv, kwargs))
        output.mkdir()
        scan_root = Path(argv[-1])
        regs = [r for r in self.regs if (scan_root / r['entry']).exists()]
        count = int(argv[argv.index('--count') + 1])
        data = ''.join(json.dumps(record) + '\n' for record in samples(regs, count, root=str(scan_root))).encode()
        (output / 'stdout.raw').write_bytes(data)
        (output / 'stderr.raw').write_bytes(b'')
        elapsed = (count - 1) * .1 + .02
        return {'termination': 'completed', 'exit_code': 0, 'reaped': True,
                'elapsed_seconds': elapsed, 'stdout_bytes': len(data), 'stderr_bytes': 0,
                'total_cpu_seconds': .003, 'user_cpu_seconds': .002, 'system_cpu_seconds': .001,
                'cpu_percent': .3 / elapsed, 'runner_cpu_seconds': .001, 'peak_rss_kib': 100,
                'peak_rss_scope': 'kernel_child_lifetime_including_inherited_launch_peak',
                'rss_samples': [], 'rss_method': None}

    def execute(self, *options, child=None, extra=None):
        with contextlib.ExitStack() as stack:
            stack.enter_context(patch.object(harness, 'run_process', side_effect=child or self.child))
            stack.enter_context(patch.object(harness, 'collect_environment', return_value={'test': True}))
            stack.enter_context(patch.object(harness.threading, 'Event', return_value=self.event))
            stack.enter_context(patch.object(harness.time, 'monotonic', return_value=100.0))
            stack.enter_context(patch.object(harness.os, 'getloadavg', return_value=(0, 0, 0)))
            stack.enter_context(patch.object(harness.shutil, 'disk_usage', return_value=SimpleNamespace(free=10 * 1024**3)))
            stack.enter_context(contextlib.redirect_stdout(io.StringIO()))
            if extra is not None:
                stack.enter_context(extra)
            code = harness.execute(self.args(*options))
        document = harness.load_result(self.output)
        return code, document

    def completion_fixture(self):
        """Five fixture-only registrations in an order distinct from the primary order."""
        providers = ('nvme', 'iwlwifi_1', 'acpitz', 'k10temp', 'amdgpu')
        for n, provider in enumerate(providers):
            if n < len(self.targets):
                target = self.targets[n]
            else:
                target = self.base / f'devices/controller{n}/hwmon/hwmon{n}'
                target.mkdir(parents=True)
                (target / 'temp1_input').write_text('42000')
                (self.root / target.name).symlink_to(target)
                self.targets.append(target)
            (target / 'name').write_text(provider)
        self.regs = enumerate_registrations(self.root)
        return self.regs

    def test_completion_plan_order_counts_caps_and_enumeration_independence(self):
        selected = self.completion_fixture()
        cases = harness.plan(selected, completion_block=True)
        self.assertEqual(cases, harness.plan(list(reversed(selected)), completion_block=True))
        self.assertEqual((len(cases), sum(c['count'] for c in cases)), (13, 2173))
        self.assertEqual(sum(c['output_limit_bytes'] for c in cases) + harness.RESERVE_BYTES,
                         126 * harness.MIB)
        smoke, primary = cases[:6], cases[6:]
        self.assertEqual([c['role'] for c in smoke], ['full_smoke'] + ['isolated_smoke'] * 5)
        self.assertEqual([c['token'] for c in smoke], [None] + [r['token'] for r in selected])
        by_token = {r['token']: r['provider'] for r in selected}
        self.assertEqual([by_token.get(c['token']) for c in primary],
                         [None, 'amdgpu', 'k10temp', 'iwlwifi_1', 'acpitz', 'nvme', None])
        self.assertEqual([c['role'] for c in primary],
                         ['full_before'] + ['isolated'] * 5 + ['full_after'])
        self.assertTrue(all(c['block'] == 3 for c in primary))
        self.assertTrue(all(c['interval_ms'] == 100 and c['format'] == 'json-lines' for c in cases))
        self.assertTrue(all(c['count'] == 11 and c['warmup_seconds'] == 0
                            and c['timeout_seconds'] == 5 and c['output_limit_bytes'] == harness.MIB
                            for c in smoke))
        self.assertTrue(all(c['count'] == 301 and c['warmup_seconds'] == 10
                            and c['timeout_seconds'] == 40 and c['output_limit_bytes'] == 8 * harness.MIB
                            for c in primary))
        self.assertEqual(sum((c['count'] - 1) * c['interval_ms'] / 1000 for c in cases)
                         + (len(cases) - 1) * harness.SETTLE_SECONDS, 396)
        self.assertEqual(sum(c['timeout_seconds'] for c in cases)
                         + (len(cases) - 1) * harness.SETTLE_SECONDS, 490)

    def test_completion_rejects_missing_duplicate_unknown_or_unreadable_provider(self):
        selected = self.completion_fixture()
        invalid = [selected[:-1], selected + [dict(selected[0], token='r999')]]
        for provider in ('nvme', 'different', None):
            invalid.append(selected[:-1] + [dict(selected[-1], provider=provider)])
        invalid.append(selected[:-1] + [dict(selected[-1], token=selected[0]['token'])])
        for registrations in invalid:
            with self.subTest(registrations=registrations), self.assertRaises(ValueError):
                harness.plan(registrations, completion_block=True)
        with self.assertRaises(ValueError):
            harness.plan(selected, completion_block=True, smoke_only=True)

    def test_completion_cli_rejects_smoke_only_and_accepts_explicit_mode(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            self.args('--completion-block', '--smoke-only')
        args = self.args('--completion-block')
        self.assertTrue(args.completion_block)
        self.assertFalse(args.smoke_only)

    def test_completion_execution_uses_local_bounds_and_exactly_one_block(self):
        self.completion_fixture()
        # 200 MiB satisfies the smaller mode's disk admission but not the default's.
        code, document = self.execute('--completion-block', extra=patch.object(
            harness.shutil, 'disk_usage', return_value=SimpleNamespace(free=200 * harness.MIB)))
        self.assertEqual(code, 0)
        self.assertEqual(document['status'], 'complete')
        self.assertEqual(document['configuration']['suite_timeout_seconds'], 600)
        self.assertEqual(document['configuration']['suite_limit_bytes'], 128 * harness.MIB)
        self.assertEqual((harness.SUITE_SECONDS, harness.SUITE_BYTES), (1500, 256 * harness.MIB))
        self.assertEqual(len(self.calls), 13)
        self.assertEqual(len(document['runs']), 13)
        self.assertEqual(sum(r['validity']['actual_attempts'] for r in document['runs']), 2173)
        self.assertTrue(all(r['validity']['complete'] for r in document['runs']))
        self.assertFalse(document['not_started'])
        self.assertEqual(len(document['identity_comparisons']), 5)
        self.assertTrue(all(c['compatible'] for c in document['identity_comparisons']))
        self.assertEqual([call[1]['timeout_s'] for call in self.calls], [5] * 6 + [40] * 7)
        self.assertEqual([call[1]['output_limit_bytes'] for call in self.calls],
                         [harness.MIB] * 6 + [8 * harness.MIB] * 7)
        self.assertEqual((self.output / 'report.md').read_text(), harness.render_report(document))

    def test_completion_admission_enforces_600_second_budget(self):
        self.completion_fixture()
        clock = itertools.chain([100.0], itertools.repeat(692.0))
        code, document = self.execute('--completion-block', extra=patch.object(
            harness.time, 'monotonic', side_effect=lambda: next(clock)))
        self.assertEqual(code, 1)
        self.assertEqual(document['status'], 'suite_timeout')
        self.assertFalse(self.calls)
        self.assertEqual(len(document['not_started']), 13)

    def test_completion_slow_checkpoint_cannot_shorten_child_timeout(self):
        self.completion_fixture()
        clock = {'now': 100.0}
        real_save = harness.save
        checkpoints = []
        def slow_checkpoint(document, output):
            real_save(document, output)
            if document['in_progress'] is not None and not checkpoints:
                checkpoints.append(document['in_progress'])
                # Only two seconds remain after the ten-second finalization reserve.
                # The five-second smoke must not launch with a shortened timeout.
                clock['now'] = 688.0
        with patch.object(harness, 'save', slow_checkpoint):
            code, document = self.execute('--completion-block', extra=patch.object(
                harness.time, 'monotonic', side_effect=lambda: clock['now']))
        self.assertEqual(code, 1)
        self.assertEqual(len(checkpoints), 1)
        self.assertEqual(document['status'], 'suite_timeout')
        self.assertFalse(self.calls)
        self.assertFalse(document['runs'])
        self.assertIsNone(document['in_progress'])
        self.assertEqual(document['not_started'],
                         [case['id'] for case in document['configuration']['cases']])
        self.assertEqual(len(document['not_started']), 13)
        self.assertEqual(document['alias_cleanup'], 'complete')
        self.assertFalse((self.output / 'aliases').exists())

    def test_completion_primary_failure_stops_without_retry(self):
        self.completion_fixture()
        def failed_primary(argv, output, **kwargs):
            resource = self.child(argv, output, **kwargs)
            if argv[argv.index('--count') + 1] == '301':
                resource.update(termination='timeout', exit_code=None)
            return resource
        code, document = self.execute('--completion-block', child=failed_primary)
        self.assertEqual(code, 1)
        self.assertEqual(document['status'], 'incomplete_case')
        self.assertEqual(len(self.calls), 7)
        self.assertEqual(len(document['runs']), 7)
        self.assertEqual(len(document['not_started']), 6)
        self.assertIsNone(document['runs'][-1]['validity']['actual_attempts'])
        self.assertEqual(document['runs'][-1]['resource']['termination'], 'timeout')

    def test_finalization_keeps_handlers_during_cleanup_and_save(self):
        previous = {sig: signal.getsignal(sig) for sig in (signal.SIGINT, signal.SIGTERM)}
        real_rmdir, real_save = Path.rmdir, harness.save
        observed = []
        def assert_handlers():
            for sig, before in previous.items():
                self.assertIsNot(signal.getsignal(sig), before)
                self.assertTrue(callable(signal.getsignal(sig)))
        def checked_rmdir(path):
            if path == self.output / 'aliases':
                assert_handlers()
                observed.append('cleanup')
                signal.getsignal(signal.SIGTERM)(signal.SIGTERM, None)
            return real_rmdir(path)
        def checked_save(document, output):
            if 'alias_cleanup' in document:
                assert_handlers()
                observed.append('save')
            return real_save(document, output)
        with patch.object(Path, 'rmdir', checked_rmdir), patch.object(harness, 'save', checked_save):
            code, document = self.execute('--smoke-only')
        self.assertEqual(code, 1)
        self.assertEqual(document['status'], 'interrupted')
        self.assertEqual(observed, ['cleanup', 'save'])
        self.assertTrue(self.event.is_set())
        self.assertEqual(document['alias_cleanup'], 'complete')
        self.assertFalse((self.output / 'aliases').exists())
        for sig, before in previous.items():
            self.assertIs(signal.getsignal(sig), before)

    def test_sigterm_during_final_save_persists_interrupted_status(self):
        previous = {sig: signal.getsignal(sig) for sig in (signal.SIGINT, signal.SIGTERM)}
        real_save = harness.save
        final_statuses = []
        def signalled_save(document, output):
            final = 'alias_cleanup' in document
            if final:
                for sig, before in previous.items():
                    self.assertIsNot(signal.getsignal(sig), before)
                final_statuses.append(document['status'])
            real_save(document, output)
            if final and len(final_statuses) == 1:
                signal.getsignal(signal.SIGTERM)(signal.SIGTERM, None)
        with patch.object(harness, 'save', signalled_save):
            code, document = self.execute('--smoke-only')
        self.assertEqual(code, 1)
        self.assertEqual(final_statuses, ['smoke_complete', 'interrupted'])
        self.assertEqual(document['status'], 'interrupted')
        self.assertEqual(document['alias_cleanup'], 'complete')
        self.assertTrue(self.event.is_set())
        for sig, before in previous.items():
            self.assertIs(signal.getsignal(sig), before)

    def test_completion_final_save_crossing_deadline_persists_timeout(self):
        self.completion_fixture()
        clock = {'now': 100.0}
        real_rmdir, real_save = Path.rmdir, harness.save
        final_statuses = []
        def cleanup_near_deadline(path):
            result = real_rmdir(path)
            if path == self.output / 'aliases':
                clock['now'] = 699.0
            return result
        def slow_final_save(document, output):
            final = 'alias_cleanup' in document
            if final:
                final_statuses.append(document['status'])
                if len(final_statuses) == 1:
                    self.assertEqual(document['actual_suite_elapsed_seconds'], 599.0)
            real_save(document, output)
            if final:
                clock['now'] = 701.0
        with patch.object(Path, 'rmdir', cleanup_near_deadline), \
                patch.object(harness, 'save', slow_final_save):
            code, document = self.execute('--completion-block', extra=patch.object(
                harness.time, 'monotonic', side_effect=lambda: clock['now']))
        self.assertEqual(code, 1)
        self.assertEqual(final_statuses, ['complete', 'suite_timeout'])
        self.assertEqual(document['status'], 'suite_timeout')
        self.assertGreaterEqual(document['actual_suite_elapsed_seconds'], 600)
        self.assertEqual(len(document['runs']), 13)
        self.assertTrue(all(r['validity']['complete'] for r in document['runs']))
        self.assertEqual(document['alias_cleanup'], 'complete')

    def test_final_announcement_cannot_hide_timeout_or_signal(self):
        self.completion_fixture()
        for reason in ('suite_timeout', 'interrupted'):
            with self.subTest(reason=reason):
                self.output = self.base / f'final-announcement-{reason}'
                self.event = Event()
                self.calls.clear()
                clock = {'now': 100.0}
                announcements = []
                previous = {sig: signal.getsignal(sig)
                            for sig in (signal.SIGINT, signal.SIGTERM)}
                def delayed_or_signalled_print(*args, **_kwargs):
                    if args and str(args[0]).startswith('Finalizing '):
                        announcements.append(str(args[0]))
                        if reason == 'suite_timeout':
                            clock['now'] = 701.0
                        else:
                            self.assertIsNot(signal.getsignal(signal.SIGTERM),
                                             previous[signal.SIGTERM])
                            signal.getsignal(signal.SIGTERM)(signal.SIGTERM, None)
                with patch('builtins.print', side_effect=delayed_or_signalled_print):
                    code, document = self.execute('--completion-block', extra=patch.object(
                        harness.time, 'monotonic', side_effect=lambda: clock['now']))
                self.assertEqual(code, 1)
                self.assertEqual(len(announcements), 1)
                self.assertEqual(document['status'], reason)
                self.assertEqual(len(document['runs']), 13)
                self.assertTrue(all(r['validity']['complete'] for r in document['runs']))
                self.assertEqual(document['alias_cleanup'], 'complete')
                if reason == 'suite_timeout':
                    self.assertGreaterEqual(document['actual_suite_elapsed_seconds'], 600)
                else:
                    self.assertTrue(self.event.is_set())
                for sig, before in previous.items():
                    self.assertIs(signal.getsignal(sig), before)

    def test_finalization_handlers_restore_when_final_save_fails(self):
        previous = {sig: signal.getsignal(sig) for sig in (signal.SIGINT, signal.SIGTERM)}
        real_save = harness.save
        observed = []
        def failed_save(document, output):
            if 'alias_cleanup' in document:
                observed.append('save')
                for sig, before in previous.items():
                    self.assertIsNot(signal.getsignal(sig), before)
                    self.assertTrue(callable(signal.getsignal(sig)))
                raise OSError('fixture finalization write failed')
            return real_save(document, output)
        with self.assertRaises(OSError), patch.object(harness, 'save', failed_save):
            self.execute('--smoke-only')
        self.assertEqual(observed, ['save'])
        self.assertFalse((self.output / 'aliases').exists())
        for sig, before in previous.items():
            self.assertIs(signal.getsignal(sig), before)

    def test_matrix_bounds_order_and_bracketing(self):
        selected = [{'token': f'r{n:03d}'} for n in range(1, 6)]
        cases = harness.plan(selected)
        self.assertEqual((len(cases), sum(c['count'] for c in cases)), (27, 6387))
        self.assertEqual(sum(c['output_limit_bytes'] for c in cases) + harness.RESERVE_BYTES, 238 * harness.MIB)
        primary = [c for c in cases if c['phase'] == 'primary']
        for block in range(1, 4):
            group = [c for c in primary if c['block'] == block]
            self.assertEqual((group[0]['role'], group[-1]['role']), ('full_before', 'full_after'))
            self.assertEqual(len(group), 7)
            self.assertEqual(group[1]['token'], f'r{block:03d}')
        self.assertEqual(cases, harness.plan(selected))

    def test_invalid_plan_and_arguments_rejected(self):
        for selected in ([], [{'token': 'same'}] * 2, [{'token': str(n)} for n in range(6)]):
            with self.assertRaises(ValueError):
                harness.plan(selected)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            harness.arguments(['--run'])

    def test_complete_protocol_smoke_gate_regeneration_and_privacy(self):
        code, document = self.execute()
        self.assertEqual(code, 0)
        self.assertEqual(document['status'], 'complete')
        self.assertEqual(len(document['runs']), 15)  # three smoke + 3*(full,2 isolated,full)
        self.assertEqual(len(document['identity_comparisons']), 2)
        self.assertTrue(all(c['compatible'] for c in document['identity_comparisons']))
        self.assertTrue(all(r['validity']['complete'] for r in document['runs']))
        self.assertFalse(document['not_started'])
        self.assertEqual(document['alias_cleanup'], 'complete')
        self.assertEqual((self.output / 'report.md').read_text(), harness.render_report(document))
        self.assertNotIn(str(self.base), json.dumps(document))
        self.assertEqual((self.output / 'private-manifest.json').stat().st_mode & 0o777, 0o600)
        self.assertTrue(all(not call[1]['observe_rss'] for call in self.calls))

    def test_smoke_failure_retains_prefix_and_blocks_primary(self):
        def partial(*args, **kwargs):
            resource = self.child(*args, **kwargs)
            path = args[1] / 'stdout.raw'
            path.write_bytes(path.read_bytes().splitlines(keepends=True)[0] + b'{')
            resource['stdout_bytes'] = path.stat().st_size
            return resource
        code, document = self.execute(child=partial)
        self.assertEqual(code, 1)
        self.assertEqual(len(self.calls), 1)
        self.assertEqual(document['runs'][0]['parsed']['emitted_count'], 1)
        self.assertIsNone(document['runs'][0]['validity']['actual_attempts'])
        self.assertEqual(document['status'], 'incomplete_case')

    def test_all_unreadable_is_not_valid_acquisition(self):
        def unreadable(*args, **kwargs):
            resource = self.child(*args, **kwargs)
            path = args[1] / 'stdout.raw'
            rows = [json.loads(line) for line in path.read_text().splitlines()]
            for row in rows:
                for sensor in row['snapshot']['sensors']:
                    sensor['status'] = 'permission_denied'
            path.write_text(''.join(json.dumps(row) + '\n' for row in rows))
            resource['stdout_bytes'] = path.stat().st_size
            return resource
        code, document = self.execute(child=unreadable)
        self.assertEqual(code, 1)
        self.assertFalse(document['runs'][0]['coverage']['usable'])
        self.assertEqual(len(self.calls), 1)

    def test_timeout_interruption_and_output_limit_stop_with_unknown_counts(self):
        for reason in ('timeout', 'interrupted', 'output_limit'):
            self.output = self.base / reason
            self.calls.clear()
            self.event = Event()
            def failed(*args, **kwargs):
                resource = self.child(*args, **kwargs)
                resource.update(termination=reason, exit_code=None)
                if reason == 'interrupted':
                    self.event.set()
                return resource
            code, document = self.execute(child=failed)
            self.assertEqual(code, 1)
            self.assertEqual(len(self.calls), 1)
            self.assertEqual(document['runs'][0]['resource']['termination'], reason)
            self.assertIsNone(document['runs'][0]['validity']['actual_attempts'])

    def test_post_alias_failure_preserves_known_accounting(self):
        real_alias = harness.alias_root
        @contextlib.contextmanager
        def changed(*args):
            with real_alias(*args) as root:
                yield root
            raise SelectionError('registration_changed')
        code, document = self.execute(extra=patch.object(harness, 'alias_root', changed))
        self.assertEqual(code, 1)
        self.assertEqual(len(self.calls), 2)
        self.assertEqual(document['runs'][-1]['alias_error'], 'registration_changed')
        self.assertEqual(document['retained_raw_bytes'], sum(r['resource']['stdout_bytes'] for r in document['runs']))
        self.assertFalse(document['runs'][-1]['validity']['complete'])

    def test_output_cannot_be_nested_in_external_hardware_target(self):
        self.output = self.targets[0] / 'results'
        with self.assertRaises(ValueError):
            harness.execute(self.args())
        self.assertFalse(self.output.exists())

    def test_budget_guards_prevent_launch_and_keep_unstarted(self):
        code, document = self.execute(extra=patch.object(harness, 'SUITE_SECONDS', 2))
        self.assertEqual(code, 1)
        self.assertFalse(self.calls)
        self.assertEqual(document['status'], 'suite_timeout')
        self.assertTrue(document['not_started'])
        self.output = self.base / 'disk'
        code, document = self.execute(extra=patch.object(harness.shutil, 'disk_usage', return_value=SimpleNamespace(free=1)))
        self.assertEqual(code, 1)
        self.assertFalse(self.calls)
        self.assertEqual(document['status'], 'disk_budget')

    def test_metadata_limit_and_missing_measurements(self):
        self.output.mkdir()
        with patch.object(harness, 'RESULT_BYTES', 10), self.assertRaises(ValueError):
            harness.save({'status': 'not_started', 'runs': []}, self.output)
        self.assertFalse((self.output / 'results.json').exists())
        with self.assertRaises(ValueError):
            harness.load_result(self.binary)

    def test_real_sigterm_records_interruption_reaps_child_and_cleans_alias(self):
        """A real signal reaches the CLI; a bounded synthetic child ignores TERM."""
        ready = self.base / 'child-ready.json'
        first_capture = ''.join(json.dumps(record) + '\n'
                                for record in samples(self.regs, 11, root=str(self.root)))
        self.binary.write_text(f'''#!{sys.executable}
import json, os, signal, sys
from pathlib import Path
root = Path(sys.argv[-1])
if root == Path({str(self.root)!r}):
    sys.stdout.write({first_capture!r})
else:
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    signal.alarm(5)  # A test failure cannot leave this fixture running indefinitely.
    Path({str(ready)!r}).write_text(json.dumps({{'pid': os.getpid(), 'root': str(root)}}))
    while True:
        signal.pause()
''')
        self.binary.chmod(0o700)
        scripts = Path(__file__).resolve().parents[1]
        wrapper = f'''
import sys
from types import SimpleNamespace
sys.path.insert(0, {str(scripts)!r})
import measure_provider_latency as harness
harness.collect_environment = lambda *args: {{'fixture': True}}
harness.os.getloadavg = lambda: (0, 0, 0)
harness.shutil.disk_usage = lambda path: SimpleNamespace(free=10 * 1024**3)
harness.SETTLE_SECONDS = 0
raise SystemExit(harness.main(sys.argv[1:]))
'''
        process = subprocess.Popen([sys.executable, '-B', '-c', wrapper, '--run',
            '--binary', str(self.binary), '--hwmon-root', str(self.root),
            '--output-dir', str(self.output), '--select', 'r001', '--smoke-only'],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, start_new_session=True)
        try:
            marker = None
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline and process.poll() is None:
                try:
                    marker = json.loads(ready.read_text())
                    break
                except (OSError, json.JSONDecodeError):
                    time.sleep(0.01)
            self.assertIsNotNone(marker, 'synthetic isolated child did not become ready')
            alias = Path(marker['root'])
            self.assertTrue(alias.is_dir())
            process.send_signal(signal.SIGTERM)
            stdout, stderr = process.communicate(timeout=6)
            self.assertEqual(process.returncode, 1, (stdout, stderr))
            document = harness.load_result(self.output)
            self.assertEqual(document['status'], 'interrupted')
            self.assertEqual(len(document['runs']), 2)
            child = document['runs'][-1]
            self.assertEqual(child['resource']['termination'], 'interrupted')
            self.assertTrue(child['resource']['reaped'])
            self.assertEqual(child['resource']['signal'], signal.SIGKILL)
            self.assertIsNone(child['validity']['actual_attempts'])
            self.assertFalse(child['validity']['complete'])
            self.assertIsNone(document['in_progress'])
            self.assertEqual(document['alias_cleanup'], 'complete')
            self.assertFalse(alias.exists())
            self.assertFalse((self.output / 'aliases').exists())
            with self.assertRaises(ProcessLookupError):
                os.kill(marker['pid'], 0)
            self.assertEqual((self.targets[0] / 'temp1_input').read_text(), '42000')
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=6)


if __name__ == '__main__':
    unittest.main()
