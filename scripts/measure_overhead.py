#!/usr/bin/env python3
"""Bounded, sequential measurements of the unchanged SiliconWatch executable."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import signal
import sys
import threading
import time

from overhead_process import run_process
from overhead_provenance import collect_environment, inventory
from overhead_results import parse_output, render_report, rss_summary as summarize_rss

MIB = 1024 * 1024
MAX_RESULT_BYTES = 64 * MIB
MAX_ENCODED_RESULT_BYTES = MAX_RESULT_BYTES // 2 - MIB


def case(interval, count, fmt, repetition=1, *, sink="file", observe=True, warmup=10):
    if not 100 <= interval <= 60000 or not 1 <= count <= 10000:
        raise ValueError("interval/count outside application bounds")
    if fmt not in ("table", "json-lines") or sink not in ("file", "null"):
        raise ValueError("unknown format or sink")
    return {"interval_ms": interval, "count": count, "format": fmt,
            "repetition": repetition, "duration_seconds": (count - 1) * interval / 1000,
            "sink": sink, "observe_rss": observe, "warmup_seconds": warmup}


def generate_matrix(name, repeats=3):
    """Published deterministic alternating order; no concurrent measurements."""
    if not 1 <= repeats <= 3:
        raise ValueError("repeats must be 1..3")
    if name == "baseline":
        configs = [case(i, d * 1000 // i + 1, f)
                   for d in (30, 300) for i in (100, 1000)
                   for f in ("table", "json-lines")]
    elif name == "controls":
        configs = [case(100, 301, f, sink=s, observe=False)
                   for s in ("file", "null") for f in ("table", "json-lines")]
    elif name == "smoke":
        configs = [case(i, 11 if i == 100 else 3, f, warmup=0)
                   for i in (100, 1000) for f in ("table", "json-lines")]
    else:
        raise ValueError("unknown matrix")
    result = []
    for rep in range(1, repeats + 1):
        order = configs if rep % 2 else list(reversed(configs))
        offset = (rep - 1) % len(order)
        for config in order[offset:] + order[:offset]:
            result.append(dict(config, repetition=rep))
    return number_cases(result)


def number_cases(cases):
    return [dict(c, id=f"{n:03d}-{c['format']}-{c['interval_ms']}ms-"
                 f"{c['count']}n-{c['sink']}-r{c['repetition']}")
            for n, c in enumerate(cases, 1)]


def child_args(binary, config, root=None):
    args = [str(binary), "sample", "--count", str(config["count"]),
            "--interval-ms", str(config["interval_ms"])]
    if config["format"] == "json-lines":
        args.append("--json-lines")
    if root is not None:
        args.extend(["--hwmon-root", str(root)])
    return args


def load_result(path):
    if path.is_dir():
        path = path / "results.json"
    if path.stat().st_size > MAX_RESULT_BYTES:
        raise ValueError("result file exceeds bounded reader limit")
    with path.open() as f:
        value = json.load(f)
    if value.get("schema_version") != 1:
        raise ValueError("unsupported benchmark result schema")
    return value


def binary_hash(path):
    with path.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def projection(cases, smoke, digest, settle_seconds):
    if smoke.get("binary_sha256") != digest or smoke.get("status") != "complete":
        raise ValueError("preflight requires complete smoke results for the exact binary")
    observations = {}
    largest_record = 0
    for run in smoke["runs"]:
        c, r, p = run["case"], run["resource"], run["parsed"]
        if not run["validity"]["complete"] or not p.get("complete"):
            continue
        largest_record = max([largest_record] + [len(json.dumps(record, separators=(",", ":")))
                                                for record in p.get("records", [])])
        observations.setdefault(c["format"], []).append((r["stdout_bytes"] / c["count"],
            max(0, r["elapsed_seconds"] - c["duration_seconds"]),
            r["elapsed_seconds"] / c["duration_seconds"] if c["duration_seconds"] else 1))
    if set(observations) != {"table", "json-lines"}:
        raise ValueError("preflight requires verified table and JSON Lines smoke runs")
    # Twofold output allowance, metadata reserve, and conservative startup/delay allowance.
    disk = MAX_RESULT_BYTES
    elapsed = settle_seconds * max(0, len(cases) - 1)
    metadata = 128 * 1024
    for c in cases:
        rows = observations[c["format"]]
        disk += (2 * max(v[0] for v in rows) * c["count"] if c["sink"] == "file" else 0)
        metadata += (largest_record + 256) * c["count"] + 32768
        metadata += math.ceil(c["duration_seconds"] + max(10, c["duration_seconds"] * .2)) * 256
        case_estimate = c["duration_seconds"] * max(1, max(v[2] for v in rows)) + 2 * max(v[1] for v in rows)
        if case_estimate > c["duration_seconds"] + max(10, c["duration_seconds"] * .2):
            raise ValueError("projected case runtime exceeds timeout; matrix not started")
        elapsed += case_estimate
    if metadata > MAX_ENCODED_RESULT_BYTES:
        raise ValueError("projected metadata exceeds result budget; matrix not started")
    return {"estimated_bytes_including_reserve": math.ceil(disk),
            "estimated_metadata_bytes": metadata,
            "estimated_seconds_including_settling": elapsed,
            "basis": "smoke extrapolation; estimate, not a guarantee"}


def save(document, output):
    encoded = (json.dumps(document, separators=(",", ":"), sort_keys=True, allow_nan=False) + "\n").encode()
    if len(encoded) > MAX_ENCODED_RESULT_BYTES:
        raise ValueError("result metadata reserve exceeded")
    pending = output / "results.pending"
    pending.write_bytes(encoded)
    pending.replace(output / "results.json")
    report = render_report(document)
    if len(report.encode()) > MIB:
        raise ValueError("report exceeds metadata reserve")
    (output / "report.md").write_text(report)


def rss_summary(resource, warmup):
    return summarize_rss(resource.get("rss_samples", []), warmup, resource["elapsed_seconds"])


def observed_validity(resource, parsed):
    process_ok = resource.get("termination") == "completed" and resource.get("exit_code") == 0
    complete = process_ok and parsed.get("complete", False)
    return {"process_completed": process_ok, "complete": complete,
            "actual_attempts": parsed["emitted_count"] if complete else None,
            "cpu_ms_per_attempt": (1000 * resource["total_cpu_seconds"] / parsed["emitted_count"]
                                   if complete and resource.get("total_cpu_seconds") is not None else None)}


def execute(args, cases):
    binary = args.binary.resolve(strict=True)
    output = args.output_dir
    if output.exists():
        raise ValueError("output directory must be new; previous evidence is never overwritten")
    if not hasattr(os, "wait4") or not sys.platform.startswith("linux"):
        raise ValueError("Linux with wait4 is required")
    digest = binary_hash(binary)
    projected = None
    root = args.hwmon_root or Path("/sys/class/hwmon")
    root_kind = "system" if args.hwmon_root is None else "custom (path omitted)"
    initial_inventory = inventory(root)
    if args.matrix in ("baseline", "controls"):
        if args.preflight_smoke is None:
            raise ValueError("baseline/controls require --preflight-smoke from a successful smoke suite")
        smoke = load_result(args.preflight_smoke)
        if (smoke.get("configuration", {}).get("hwmon_root") != root_kind or
                smoke.get("environment", {}).get("provider_inventory") != initial_inventory or
                any(r.get("inventory_before") != initial_inventory or r.get("inventory_after") != initial_inventory
                    for r in smoke.get("runs", []))):
            raise ValueError("smoke inventory/root cohort differs; matrix not started")
        projected = projection(cases, smoke, digest, args.settle_seconds)
        if projected["estimated_bytes_including_reserve"] > args.suite_limit_mib * MIB:
            raise ValueError("projected disk exceeds suite budget; full matrix not started")
        if projected["estimated_seconds_including_settling"] > args.suite_timeout_seconds:
            raise ValueError("projected runtime exceeds suite budget; full matrix not started")
    output.mkdir(parents=True, mode=0o700)
    output.chmod(0o700)
    if shutil.disk_usage(output).free < args.suite_limit_mib * MIB + 128 * MIB:
        raise ValueError("insufficient free disk for suite budget plus safety margin")
    repo = Path(__file__).resolve().parents[1]
    environment = collect_environment(binary, repo, output, root)
    document = {"schema_version": 1, "status": "running", "binary_sha256": digest,
                "harness_sha256": {p.name: binary_hash(p) for p in sorted(Path(__file__).parent.glob("*.py"))},
                "environment": environment, "projection": projected,
                "configuration": {"matrix": args.matrix or "individual", "cases": cases,
                    "settle_seconds": args.settle_seconds, "suite_limit_bytes": args.suite_limit_mib * MIB,
                    "per_run_limit_bytes": args.output_limit_mib * MIB,
                    "suite_timeout_seconds": args.suite_timeout_seconds,
                    "timeout_override_seconds": args.timeout_seconds,
                    "timeout_contract": "override or nominal duration + max(10s, 20%); clamped to remaining suite time; own child only",
                    "notes": args.notes, "hwmon_root": root_kind,
                    "operator_declared_build_command": args.build_command},
                "runs": [], "not_started": [c["id"] for c in cases]}
    save(document, output)
    interrupt = threading.Event()
    previous_handlers = {}
    def stop(_signum, _frame):
        interrupt.set()
    for sig in (signal.SIGINT, signal.SIGTERM):
        previous_handlers[sig] = signal.signal(sig, stop)
    begin = time.monotonic()
    raw_bytes = 0
    try:
        for index, config in enumerate(cases):
            if index and interrupt.wait(args.settle_seconds):
                document["status"] = "interrupted"
                break
            elapsed = time.monotonic() - begin
            if interrupt.is_set():
                document["status"] = "interrupted"
                break
            if elapsed + config["duration_seconds"] + 10 > args.suite_timeout_seconds:
                document["status"] = "suite_timeout"
                break
            if binary_hash(binary) != digest:
                document["status"] = "binary_changed"
                break
            before = inventory(root)
            if before != initial_inventory:
                document["status"] = "inventory_changed"
                break
            load_before = list(os.getloadavg())
            if args.matrix in ("baseline", "controls") and load_before[0] > 0.8 * (os.cpu_count() or 1):
                document["status"] = "high_background_load"
                break
            remaining = args.suite_limit_mib * MIB - MAX_RESULT_BYTES - raw_bytes
            if remaining < args.output_limit_mib * MIB or shutil.disk_usage(output).free < remaining + 128 * MIB:
                document["status"] = "disk_budget"
                break
            timeout = args.timeout_seconds or config["duration_seconds"] + max(10, config["duration_seconds"] * .2)
            timeout = min(timeout, args.suite_timeout_seconds - elapsed)
            print(f"START {index + 1}/{len(cases)} {config['id']} timeout={timeout:.1f}s", flush=True)
            run_dir = output / config["id"]
            document["in_progress"] = config["id"]
            save(document, output)
            resource = run_process(child_args(binary, config, args.hwmon_root), run_dir,
                timeout_s=timeout, output_limit_bytes=args.output_limit_mib * MIB,
                observe_rss=config["observe_rss"], sink=config["sink"], interrupt_event=interrupt)
            raw_bytes += sum(resource.get(k) or 0 for k in ("stdout_bytes", "stderr_bytes"))
            parsed = (parse_output(run_dir / "stdout.raw", config["format"], config["count"],
                                   config["interval_ms"], config["warmup_seconds"])
                      if config["sink"] == "file" and (run_dir / "stdout.raw").exists() else
                      {"valid": None, "complete": False, "emitted_count": None,
                       "failed_attempts": None, "records": [], "errors": [],
                       "unavailable_reason": "stdout discarded or unavailable; exit is not a count"})
            after = inventory(root)
            run = {"case": config, "effective_timeout_seconds": timeout,
                   "resource": resource, "parsed": parsed,
                   "validity": observed_validity(resource, parsed),
                   "rss_summary": rss_summary(resource, config["warmup_seconds"]),
                   "inventory_before": before, "inventory_after": after,
                   "load_before": load_before, "load_after": list(os.getloadavg())}
            run["validity"]["comparable"] = (run["validity"]["process_completed"] and before == after and
                (config["sink"] == "null" or (parsed["complete"] and not parsed.get("failed_attempts"))))
            document["runs"].append(run)
            document["in_progress"] = None
            document["not_started"].remove(config["id"])
            document["actual_suite_elapsed_seconds"] = time.monotonic() - begin
            document["retained_raw_bytes"] = raw_bytes
            print(f"END {config['id']} {resource['termination']} exit={resource.get('exit_code')} "
                  f"records={parsed.get('emitted_count')} elapsed={resource['elapsed_seconds']:.3f}s", flush=True)
            if not run["validity"]["process_completed"] or (config["sink"] == "file" and not parsed["complete"]):
                document["status"] = "stopped_after_incomplete_run"
            elif before != after:
                document["status"] = "inventory_changed"
            elif parsed.get("failed_attempts"):
                document["status"] = "stopped_after_recoverable_errors"
            save(document, output)
            if document["status"] != "running":
                break
        else:
            document["status"] = "complete"
    finally:
        for sig, handler in previous_handlers.items():
            signal.signal(sig, handler)
        if document["status"] == "running":
            document["status"] = "harness_error"
        document["actual_suite_elapsed_seconds"] = time.monotonic() - begin
        save(document, output)
    print(f"Suite {document['status']}; {len(document['runs'])}/{len(cases)} cases retained", flush=True)
    return 0 if document["status"] == "complete" else 1


def arguments(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--binary", type=Path, default=Path("target/overhead/release/siliconwatch"))
    p.add_argument("--matrix", choices=("baseline", "controls", "smoke"))
    p.add_argument("--count", type=int)
    p.add_argument("--interval-ms", type=int, default=100)
    p.add_argument("--format", choices=("table", "json-lines"), default="json-lines")
    p.add_argument("--repeats", type=int, default=3)
    p.add_argument("--sink", choices=("file", "null"), default="file")
    p.add_argument("--no-rss", action="store_true")
    p.add_argument("--warmup-seconds", type=float, default=10)
    p.add_argument("--settle-seconds", type=float, default=15)
    p.add_argument("--timeout-seconds", type=float)
    p.add_argument("--suite-timeout-seconds", type=float, default=5400)
    p.add_argument("--output-limit-mib", type=int, default=64)
    p.add_argument("--suite-limit-mib", type=int, default=1024)
    p.add_argument("--output-dir", type=Path)
    p.add_argument("--preflight-smoke", type=Path)
    p.add_argument("--hwmon-root", type=Path)
    p.add_argument("--notes", default="Uncontrolled interactive workstation; background workload may vary.")
    p.add_argument("--build-command", default=None, help="Actual build command, recorded as operator-supplied provenance")
    p.add_argument("--summarize", type=Path, help="Regenerate report from sanitized results without running a child")
    p.add_argument("--plan", action="store_true", help="Print configurations only")
    args = p.parse_args(argv)
    if args.summarize:
        return args, []
    if bool(args.matrix) == (args.count is not None):
        p.error("choose exactly one of --matrix or --count")
    if not 1 <= args.repeats <= 3:
        p.error("repeats must be 1..3")
    for name in ("settle_seconds", "warmup_seconds", "suite_timeout_seconds"):
        if not math.isfinite(getattr(args, name)) or getattr(args, name) < 0:
            p.error(f"invalid {name}")
    if args.timeout_seconds is not None and (not math.isfinite(args.timeout_seconds) or args.timeout_seconds <= 0):
        p.error("timeout must be positive and finite")
    if not 1 <= args.output_limit_mib <= 128 or not 128 <= args.suite_limit_mib <= 1024:
        p.error("output limit must be 1..128 MiB; suite limit 128..1024 MiB")
    if args.matrix in ("baseline", "controls") and args.settle_seconds < 15:
        p.error("baseline and controls require at least 15 seconds settling")
    if not args.plan and args.output_dir is None:
        p.error("--output-dir is required")
    try:
        cases = (generate_matrix(args.matrix, args.repeats) if args.matrix else
                 number_cases([case(args.interval_ms, args.count, args.format, r,
                                   sink=args.sink, observe=not args.no_rss, warmup=args.warmup_seconds)
                               for r in range(1, args.repeats + 1)]))
    except ValueError as e:
        p.error(str(e))
    return args, cases


def main(argv=None):
    args, cases = arguments(argv)
    try:
        if args.summarize:
            print(render_report(load_result(args.summarize)), end="")
            return 0
        if args.plan:
            print(json.dumps(cases, indent=2))
            return 0
        return execute(args, cases)
    except (OSError, ValueError) as e:
        # Avoid filesystem paths/child output in exported console diagnostics.
        print(f"measurement error: {type(e).__name__}: {e if isinstance(e, ValueError) else 'operating-system operation failed'}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
