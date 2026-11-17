#!/usr/bin/env python3
"""Bounded Prototype 005 comparisons using the unchanged read-only Rust binary."""

import argparse
from contextlib import nullcontext
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import threading
import time

from measure_overhead import binary_hash
from overhead_process import run_process
from overhead_results import ParseError
from overhead_provenance import collect_environment
from provider_selection import (SelectionError, alias_root, enumerate_registrations,
                                fingerprint, public_inventory, select_registrations)
from provider_results import (compare_identity_profiles, identity_profile,
                              inspect_capture, render_report)

MIB = 1024 * 1024
SUITE_SECONDS = 1500
SUITE_BYTES = 256 * MIB
RESERVE_BYTES = 64 * MIB
RESULT_BYTES = 30 * MIB
SETTLE_SECONDS = 15
CLEANUP_SECONDS = 4
FINAL_SAVE_SECONDS = 6
COMPLETION_SECONDS = 600
COMPLETION_BYTES = 128 * MIB
COMPLETION_PROVIDERS = ('amdgpu', 'k10temp', 'iwlwifi_1', 'acpitz', 'nvme')


class ProtocolError(ValueError):
    """A fixed, privacy-safe protocol diagnostic."""


def plan(selected, *, smoke_only=False, completion_block=False):
    """A full smoke precedes isolated smokes; every primary block is bracketed."""
    if not 1 <= len(selected) <= 5 or len({r['token'] for r in selected}) != len(selected):
        raise ProtocolError('plan requires one to five distinct registrations')
    if completion_block:
        if smoke_only or len(selected) != 5 or {r.get('provider') for r in selected} != set(COMPLETION_PROVIDERS):
            raise ProtocolError('completion block requires the five approved distinct providers')
        selected = sorted(selected, key=lambda r: r['entry'])
    cases = []

    def add(phase, role, token=None, block=0):
        smoke = phase == 'smoke'
        cases.append({'id': f'{len(cases)+1:03d}-{phase}-{role}-{token or "all"}-b{block}',
                      'phase': phase, 'role': role, 'token': token, 'block': block,
                      'count': 11 if smoke else 301, 'interval_ms': 100,
                      'format': 'json-lines', 'warmup_seconds': 0 if smoke else 10,
                      'timeout_seconds': 5 if smoke else 40,
                      'output_limit_bytes': (1 if smoke else 8) * MIB})
    add('smoke', 'full_smoke')
    for registration in selected:
        add('smoke', 'isolated_smoke', registration['token'])
    if not smoke_only:
        for block in ([3] if completion_block else range(1, 4)):
            add('primary', 'full_before', block=block)
            offset = (block - 1) % len(selected)
            ordered = ([next(r for r in selected if r['provider'] == name) for name in COMPLETION_PROVIDERS]
                       if completion_block else selected[offset:] + selected[:offset])
            for registration in ordered:
                add('primary', 'isolated', registration['token'], block)
            add('primary', 'full_after', block=block)
    assert len(cases) <= 27 and sum(c['count'] for c in cases) <= 6387
    assert sum(c['output_limit_bytes'] for c in cases) + RESERVE_BYTES <= SUITE_BYTES
    if completion_block:
        assert len(cases) == 13 and sum(c['count'] for c in cases) == 2173
        assert sum(c['output_limit_bytes'] for c in cases) + RESERVE_BYTES <= COMPLETION_BYTES
    return cases


def write_private(path, data):
    with open(path, 'wb', opener=lambda p, flags: os.open(p, flags, 0o600)) as stream:
        stream.write(data)


def save(document, output):
    data = (json.dumps(document, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n').encode()
    report = render_report(document).encode()
    if len(data) > RESULT_BYTES or len(report) > MIB:
        raise ProtocolError('result metadata limit exceeded')
    pending = output / 'results.pending'
    write_private(pending, data)
    pending.replace(output / 'results.json')
    write_private(output / 'report.md', report)


def load_result(path):
    path = path / 'results.json' if path.is_dir() else path
    with path.open('rb') as stream:
        data = stream.read(RESULT_BYTES + 1)
    if len(data) > RESULT_BYTES:
        raise ProtocolError('result reader limit exceeded')
    result = json.loads(data)
    if result.get('kind') != 'siliconwatch.provider-latency' or result.get('schema_version') != 1:
        raise ProtocolError('unsupported provider result schema')
    return result


def smoke_projection(runs, primary_cases, *, suite_bytes=SUITE_BYTES):
    """Project capture and observed start gaps; keep fixed startup overhead separate."""
    smoke = {r['case']['token']: r for r in runs if r['case']['phase'] == 'smoke'}
    estimated_bytes = RESERVE_BYTES + sum(r['resource']['stdout_bytes'] + r['resource']['stderr_bytes'] for r in runs)
    estimated_seconds = SETTLE_SECONDS * len(primary_cases)
    metadata_bytes = 512 * 1024
    for case in primary_cases:
        run = smoke[case['token']]
        records = run['parsed']['records']
        if not run['validity']['complete'] or not records:
            raise ProtocolError('smoke evidence incomplete')
        per_record = (run['resource']['stdout_bytes'] + run['resource']['stderr_bytes']) / len(records)
        projected_output = 2 * per_record * case['count']
        if projected_output > case['output_limit_bytes']:
            raise ProtocolError('smoke output projection exceeds per-case cap')
        # Extrapolate observed start gaps, not process startup as a periodic cost.
        gaps = [r['start_gap_ns'] / 1e9 for r in records if r['start_gap_ns'] is not None]
        period = max(.1, sum(gaps) / len(gaps)) if gaps else .1
        startup_and_final = max(0, run['resource']['elapsed_seconds'] - sum(gaps))
        projected_time = period * (case['count'] - 1) + 2 * startup_and_final + 1
        if projected_time > case['timeout_seconds']:
            raise ProtocolError('smoke timing projection exceeds per-case timeout')
        estimated_seconds += projected_time
        estimated_bytes += projected_output
        metadata_bytes += max(len(json.dumps(r)) for r in records) * case['count'] * 2 + 32768
    if estimated_bytes > suite_bytes or metadata_bytes > RESULT_BYTES:
        raise ProtocolError('smoke projection exceeds suite storage budget')
    return {'estimated_bytes_including_reserve': estimated_bytes,
            'estimated_remaining_seconds': estimated_seconds,
            'estimated_metadata_bytes': metadata_bytes,
            'basis': 'observed smoke start gaps and doubled capture/startup allowance; estimate only'}


def safe_output(output, root, registrations=()):
    output = output.resolve()
    source = root.resolve()
    if (any(output.is_relative_to(p) for p in ('/sys', '/proc', '/dev'))
            or output.is_relative_to(source) or source.is_relative_to(output)):
        raise ProtocolError('output must be separate from the hardware source tree')
    for registration in registrations:
        for key in ('target', 'device_link'):
            if registration.get(key):
                target = Path(registration[key])
                if output.is_relative_to(target) or target.is_relative_to(output):
                    raise ProtocolError('output must be separate from every hardware target')
                if key == 'target':
                    parent = target.parent.parent if target.parent.name == 'hwmon' else target.parent
                    if output.is_relative_to(parent):
                        raise ProtocolError('output must be separate from hardware parents')
    if output.exists():
        raise ProtocolError('output directory must be new; previous evidence is preserved')
    return output


def execute(args):
    begin = time.monotonic()  # Includes discovery, provenance, smoke and report work.
    suite_seconds = COMPLETION_SECONDS if args.completion_block else SUITE_SECONDS
    suite_bytes = COMPLETION_BYTES if args.completion_block else SUITE_BYTES
    final_reserve = CLEANUP_SECONDS + (FINAL_SAVE_SECONDS if args.completion_block else 0)
    binary = args.binary.resolve(strict=True)
    root = args.hwmon_root.resolve(strict=True)
    registrations = enumerate_registrations(root)
    output = safe_output(args.output_dir, root, registrations)
    if not sys.platform.startswith('linux') or not hasattr(os, 'wait4'):
        raise ProtocolError('Linux wait4 is required')
    output.mkdir(parents=True, mode=0o700)
    output.chmod(0o700)
    aliases = output / 'aliases'
    aliases.mkdir(mode=0o700)

    def announce(message):
        if args.completion_block:
            line = f'{time.monotonic() - begin:.6f}s {message}\n'.encode()
            # The finite protocol emits only short, fixed-vocabulary messages.
            fd = os.open(output / 'execution.log', os.O_WRONLY | os.O_CREAT | os.O_APPEND | os.O_NOFOLLOW, 0o600)
            with os.fdopen(fd, 'ab') as stream:
                if os.fstat(stream.fileno()).st_size + len(line) > 64 * 1024:
                    raise ProtocolError('execution log limit exceeded')
                stream.write(line)
        print(message, flush=True)

    document = {'schema_version': 1, 'kind': 'siliconwatch.provider-latency', 'status': 'setup',
                'configuration': {'cases': [], 'suite_timeout_seconds': suite_seconds,
                    'suite_limit_bytes': suite_bytes, 'metadata_reserve_bytes': RESERVE_BYTES,
                    'settle_seconds': SETTLE_SECONDS, 'observe_rss': False,
                    'hwmon_root': 'system' if root == Path('/sys/class/hwmon').resolve() else 'custom (omitted)',
                    'build_command': args.build_command, 'notes': args.notes},
                'environment': {}, 'inventory': [], 'selected_tokens': [], 'runs': [],
                'identity_comparisons': [], 'not_started': [], 'in_progress': None,
                'retained_raw_bytes': 0, 'actual_suite_elapsed_seconds': 0}
    interrupt = threading.Event()
    received_signals = set()

    def receive_signal(number, _frame):
        received_signals.add(number)
        interrupt.set()

    previous_handlers = {}
    for sig in (signal.SIGINT, signal.SIGTERM):
        previous_handlers[sig] = signal.signal(sig, receive_signal)
    try:
        document['inventory'] = public_inventory(registrations)
        selected = select_registrations(registrations, args.select)
        cases = plan(selected, smoke_only=args.smoke_only, completion_block=args.completion_block)
        document['configuration']['cases'] = cases
        document['selected_tokens'] = [r['token'] for r in selected]
        document['not_started'] = [c['id'] for c in cases]
        source_fingerprint = fingerprint(registrations)
        manifest = (json.dumps(registrations, sort_keys=True, indent=2) + '\n').encode()
        if len(manifest) > MIB:
            raise ProtocolError('private manifest limit exceeded')
        write_private(output / 'private-manifest.json', manifest)
        digest = binary_hash(binary)
        document['binary_sha256'] = digest
        document['harness_sha256'] = {p.name: binary_hash(p) for p in sorted(Path(__file__).parent.glob('*.py'))}
        document['environment'] = collect_environment(binary, Path(__file__).resolve().parents[1], output, root)
        document['status'] = 'running'
        save(document, output)
        full_profile = None
        by_token = {r['token']: r for r in selected}
        primary_started = False
        for index, case in enumerate(cases):
            if interrupt.is_set():
                document['status'] = 'interrupted'
                break
            if case['phase'] == 'primary' and not primary_started:
                projection = smoke_projection(document['runs'], [c for c in cases if c['phase'] == 'primary'],
                                              suite_bytes=suite_bytes)
                document['projection'] = projection
                if time.monotonic() - begin + projection['estimated_remaining_seconds'] + final_reserve > suite_seconds:
                    document['status'] = 'projection_exceeds_time_budget'
                    break
                primary_started = True
                announce('SMOKE PASSED; bounded primary protocol starting')
            wait_started = time.monotonic()
            if index:
                if wait_started - begin + SETTLE_SECONDS + case['timeout_seconds'] + final_reserve > suite_seconds:
                    document['status'] = 'suite_timeout'
                    break
                if interrupt.wait(SETTLE_SECONDS):
                    document['status'] = 'interrupted'
                    break
            settled = time.monotonic() - wait_started if index else 0
            elapsed = time.monotonic() - begin
            if elapsed + case['timeout_seconds'] + final_reserve > suite_seconds:
                document['status'] = 'suite_timeout'
                break
            if binary_hash(binary) != digest:
                document['status'] = 'binary_changed'
                break
            if fingerprint(enumerate_registrations(root)) != source_fingerprint:
                document['status'] = 'inventory_changed'
                break
            load = list(os.getloadavg())
            if load[0] > .8 * (os.cpu_count() or 1):
                document['status'] = 'high_background_load'
                break
            available = suite_bytes - RESERVE_BYTES - document['retained_raw_bytes']
            if available < case['output_limit_bytes'] or shutil.disk_usage(output).free < available + 128 * MIB:
                document['status'] = 'disk_budget'
                break
            registration = by_token.get(case['token'])
            context = alias_root(aliases, registration) if registration is not None else nullcontext(root)
            document['in_progress'] = case['id']
            document['actual_suite_elapsed_seconds'] = time.monotonic() - begin
            save(document, output)
            announce(f"START {index+1}/{len(cases)} {case['id']}")
            resource = None
            alias_error = None
            try:
                with context as scan_root:
                    timeout = min(case['timeout_seconds'], suite_seconds - (time.monotonic() - begin) - final_reserve)
                    if timeout <= 0 or (args.completion_block and timeout < case['timeout_seconds']):
                        document['status'] = 'suite_timeout'
                        document['in_progress'] = None  # This case never launched.
                        break
                    resource = run_process([str(binary), 'sample', '--count', str(case['count']),
                        '--interval-ms', '100', '--json-lines', '--hwmon-root', str(scan_root)],
                        output / case['id'], timeout_s=timeout, output_limit_bytes=case['output_limit_bytes'],
                        observe_rss=False, sink='file', interrupt_event=interrupt)
            except SelectionError as error:
                if resource is None:
                    raise
                alias_error = str(error)  # Safe code; keep completed child accounting despite changed targets.
            document['retained_raw_bytes'] += resource['stdout_bytes'] + resource['stderr_bytes']
            expected = [registration] if registration is not None else registrations
            captured = output / case['id'] / 'stdout.raw'
            inspected = inspect_capture(captured, case, expected)
            process_ok = resource['termination'] == 'completed' and resource['exit_code'] == 0 and resource['reaped']
            complete = (process_ok and inspected['parsed']['complete'] and inspected['coverage']['complete']
                        and inspected['coverage']['usable'] and alias_error is None)
            complete = complete and not inspected['parsed']['failed_attempts']
            actual = inspected['parsed']['emitted_count'] if process_ok and inspected['parsed']['complete'] else None
            run = {'case': case, 'resource': resource, **inspected,
                   'alias_error': alias_error,
                   'effective_timeout_seconds': timeout, 'actual_settle_seconds': settled,
                   'load_before': load, 'load_after': list(os.getloadavg()),
                   'validity': {'complete': complete, 'actual_attempts': actual,
                       'cpu_ms_per_attempt': 1000 * resource['total_cpu_seconds'] / actual
                           if actual and resource['total_cpu_seconds'] is not None else None},
                   'raw_sha256': {name: binary_hash(output / case['id'] / name)
                                  for name in ('stdout.raw', 'stderr.raw') if (output / case['id'] / name).exists()}}
            document['runs'].append(run)
            document['not_started'].remove(case['id'])
            document['in_progress'] = None
            if case['phase'] == 'smoke' and complete:
                profile = identity_profile(captured, expected)
                if case['token'] is None:
                    full_profile = profile
                else:
                    comparison = compare_identity_profiles(full_profile, profile, case['token'])
                    document['identity_comparisons'].append(comparison)
                    if not comparison['compatible']:
                        document['status'] = 'identity_context_changed'
            if not complete:
                document['status'] = 'incomplete_case'
            elif fingerprint(enumerate_registrations(root)) != source_fingerprint:
                document['status'] = 'inventory_changed'
            if interrupt.is_set():
                document['status'] = 'interrupted'
            document['actual_suite_elapsed_seconds'] = time.monotonic() - begin
            save(document, output)
            announce(f"END {case['id']} {resource['termination']} records={actual} valid={complete}")
            if document['status'] != 'running':
                break
        else:
            document['status'] = 'smoke_complete' if args.smoke_only else 'complete'
    except (OSError, ValueError, SelectionError) as error:
        document['status'] = 'stopped_error'
        document['error'] = {'type': type(error).__name__, 'errno': getattr(error, 'errno', None)}
        if isinstance(error, (SelectionError, ProtocolError, ParseError)):
            document['error']['code'] = str(error)
        # Unexpected exception messages may contain private source paths.
    finally:
        try:
            if document['status'] == 'running':
                document['status'] = 'harness_error'
            try:
                aliases.rmdir()  # Never recursive; unexpected entries are not followed.
            except OSError:
                document['alias_cleanup'] = 'not_empty_or_unavailable'
                if document['status'] in ('complete', 'smoke_complete'):
                    document['status'] = 'cleanup_incomplete'
            else:
                document['alias_cleanup'] = 'complete'
            announce(f"Finalizing {document['status']}; {len(document['runs'])} cases retained")
            if received_signals:
                announce('Received signals: ' + ', '.join(signal.Signals(s).name for s in sorted(received_signals)))
            if interrupt.is_set():
                document['status'] = 'interrupted'
            document['actual_suite_elapsed_seconds'] = time.monotonic() - begin
            if document['actual_suite_elapsed_seconds'] > suite_seconds:
                document['status'] = 'suite_timeout'
            save(document, output)
            # A signal during cleanup or the final write must not leave a success record.
            elapsed_after_save = time.monotonic() - begin
            if elapsed_after_save > suite_seconds:
                document['status'] = 'suite_timeout'
                document['actual_suite_elapsed_seconds'] = elapsed_after_save
                save(document, output)
            elif interrupt.is_set() and document['status'] not in ('interrupted', 'suite_timeout'):
                document['status'] = 'interrupted'
                document['actual_suite_elapsed_seconds'] = elapsed_after_save
                save(document, output)
        finally:
            for sig, previous in previous_handlers.items():
                signal.signal(sig, previous)
    return 0 if document['status'] in ('complete', 'smoke_complete') else 1


def arguments(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument('--plan', action='store_true')
    action.add_argument('--run', action='store_true')
    action.add_argument('--summarize', type=Path)
    parser.add_argument('--binary', type=Path, default=Path('target/provider-latency/release/siliconwatch'))
    parser.add_argument('--hwmon-root', type=Path, default=Path('/sys/class/hwmon'))
    parser.add_argument('--select', action='append', help='exact session token or unique provider name; at most five')
    parser.add_argument('--output-dir', type=Path)
    parser.add_argument('--smoke-only', action='store_true')
    parser.add_argument('--completion-block', action='store_true',
                        help='six smokes and one approved five-provider block; 600 seconds / 128 MiB')
    parser.add_argument('--build-command', default='not supplied; installed compiler is not build proof')
    parser.add_argument('--notes', default='interactive workstation; workload not controlled')
    args = parser.parse_args(argv)
    if args.run and args.output_dir is None:
        parser.error('--run requires a new --output-dir')
    if args.completion_block and (args.smoke_only or args.summarize is not None):
        parser.error('--completion-block cannot be combined with --smoke-only or --summarize')
    if len(args.build_command) > 4096 or len(args.notes) > 4096:
        parser.error('operator metadata exceeds 4096-character limit')
    return args


def main(argv=None):
    args = arguments(argv)
    try:
        if args.summarize is not None:
            sys.stdout.write(render_report(load_result(args.summarize)))
            return 0
        if args.plan:
            registrations = enumerate_registrations(args.hwmon_root)
            selected = select_registrations(registrations, args.select)
            print(json.dumps({'inventory': public_inventory(registrations),
                'cases': plan(selected, smoke_only=args.smoke_only, completion_block=args.completion_block)}, indent=2))
            return 0
        return execute(args)
    except (OSError, ValueError, SelectionError) as error:
        detail = str(error) if isinstance(error, (SelectionError, ProtocolError, ParseError)) else type(error).__name__
        print(f'Provider diagnostic stopped: {detail}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
