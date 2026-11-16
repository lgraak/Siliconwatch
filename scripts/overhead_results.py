"""Bounded, privacy-preserving parsing and reporting for Prototype 004.

Parsers deliberately target Prototype 003's experimental formats. They retain
timing/count scalars, never source paths, readings, labels, or error messages.
"""

import json
import math
import re
from collections import Counter
from pathlib import Path


MAX_RECORDS = 10_000
STATUSES = {"ok", "unavailable", "permission_denied", "unsupported", "invalid", "error", "fault"}
IDENTITIES = {"best_effort", "insufficient_evidence", "unreadable_evidence", "conflicting_evidence", "ambiguous_channel", "collision"}
TIMING_FIELDS = ("target_start_ns", "started_ns", "finished_ns", "prepared_ns", "scan_duration_ns", "lateness_ns")


class ParseError(ValueError):
    """A sanitized structural error, containing no telemetry text."""


def summarize(values):
    """Ordinary median; nearest-rank tails only above declared N thresholds."""
    values = list(values)
    if any(type(v) not in (int, float) or not math.isfinite(v) for v in values):
        raise ValueError("statistics require finite numeric observations")
    values.sort()
    n = len(values)
    return {
        "n": n,
        "median": (values[(n - 1) // 2] + values[n // 2]) / 2 if n else None,
        "min": values[0] if n else None,
        "max": values[-1] if n else None,
        "p95": values[(95 * n + 99) // 100 - 1] if n >= 200 else None,
        "p99": values[(99 * n + 99) // 100 - 1] if n >= 1000 else None,
    }


def _integer(value):
    if type(value) is not int or not 0 <= value <= (1 << 64) - 1:
        raise ParseError("expected an unsigned integer")
    return value


def _lines(path, max_line_bytes):
    with Path(path).open("rb") as source:
        while line := source.readline(max_line_bytes + 1):
            if len(line) > max_line_bytes:
                raise ParseError("line exceeds parser byte limit")
            if not line.endswith(b"\n"):
                raise ParseError("unterminated final line")
            try:
                yield line[:-1].decode("utf-8")
            except UnicodeDecodeError:
                raise ParseError("invalid UTF-8") from None


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ParseError("duplicate JSON object key")
        result[key] = value
    return result


def _nonfinite(_value):
    raise ParseError("nonfinite JSON constant")


def _json_records(lines):
    for line in lines:
        try:
            obj = json.loads(line, object_pairs_hook=_object, parse_constant=_nonfinite)
            if obj["record_type"] != "siliconwatch.sample" or type(obj["sampling_schema_version"]) is not int or obj["sampling_schema_version"] != 1:
                raise ParseError("unsupported sampling schema")
            timing = {key: _integer(obj["timing"][key]) for key in TIMING_FIELDS}
            wall = obj["timing"]["wall_start_unix_ms"]
            if wall is not None:
                _integer(wall)
            if (wall is None) != isinstance(obj["timing"]["wall_time_error"], str):
                raise ParseError("inconsistent wall-clock fields")
            snapshot, error = obj["snapshot"], obj["error"]
            if (snapshot is None) == (error is None):
                raise ParseError("expected exactly one snapshot or error")
            if error is not None and (not isinstance(error["kind"], str) or not isinstance(error["message"], str)):
                raise ParseError("malformed attempt error")
            counts = Counter()
            sensor_count = warning_count = None
            if snapshot is not None:
                if type(snapshot["schema_version"]) is not int or snapshot["schema_version"] != 2:
                    raise ParseError("unsupported snapshot schema")
                sensors, warnings = snapshot["sensors"], snapshot["warnings"]
                if not isinstance(sensors, list) or not isinstance(warnings, list):
                    raise ParseError("invalid snapshot collections")
                sensor_count, warning_count = len(sensors), len(warnings)
                for sensor in sensors:
                    status = sensor["status"]
                    if status not in STATUSES or type(sensor["identity"]["schema_version"]) is not int or sensor["identity"]["schema_version"] != 1:
                        raise ParseError("unsupported sensor status or identity schema")
                    counts[status] += 1
            fresh = obj["freshness"]
            if fresh["physical_age_known"] is not False:
                raise ParseError("unexpected physical-age claim")
            yield {
                "sequence": _integer(obj["sequence"]),
                "requested_count": _integer(obj["requested_count"]),
                "interval_ms": _integer(obj["interval_ms"]),
                **timing,
                "wall_start_unix_ms": wall,
                "failed": error is not None,
                "sensor_count": sensor_count,
                "sensor_status_counts": dict(sorted(counts.items())),
                "warning_count": warning_count,
                "basis": fresh["basis"],
                "lower_bound_ns": fresh["lower_bound_ns"],
                "upper_bound_ns": fresh["upper_bound_ns"],
                "window_exceeds_interval": fresh["observation_window_exceeds_interval"],
            }
        except (KeyError, TypeError, AttributeError, json.JSONDecodeError, RecursionError):
            raise ParseError("malformed JSON sample") from None


def _match(pattern, text):
    match = re.fullmatch(pattern, text)
    if not match:
        raise ParseError("unexpected table structure")
    return match.groups()


def _table_records(lines):
    lines = iter(lines)

    def next_line():
        try:
            return next(lines)
        except StopIteration:
            raise ParseError("incomplete table record") from None

    for line in lines:
        seq, count, interval = map(int, _match(r"Sample (\d+)/(\d+); interval (\d+) ms", line))
        target, start, finish, prepared = map(int, _match(r"Monotonic offsets \(ns\): target (\d+), start (\d+), finish (\d+), prepared (\d+)", next_line()))
        duration, lateness, wall = _match(r"Scan duration (\d+) ns; start lateness (\d+) ns; wall start (\d+|unavailable) Unix ms", next_line())
        line = next_line()
        if wall == "unavailable":
            if not line.startswith("Wall clock: "):
                raise ParseError("missing wall-clock diagnostic")
            line = next_line()
        basis, = _match(r"Observation basis: (current_attempt_observation_window|no_sensor_observations|no_snapshot)", line)
        lower = upper = exceeds = None
        if basis == "current_attempt_observation_window":
            lower, upper, exceeds = _match(r"Observation-age bounds at preparation: (\d+)\.\.(\d+) ns; window exceeds interval: (true|false)", next_line())
            lower, upper, exceeds = int(lower), int(upper), exceeds == "true"
        if next_line() != "Physical sensor age is unknown; preparation precedes serialization and output.":
            raise ParseError("missing freshness disclaimer")
        counts = Counter()
        sensor_count = warning_count = None
        if basis == "no_snapshot":
            _match(r"Attempt error \([^\n]+\): .*", next_line())
            if next_line() != "":
                raise ParseError("missing table record terminator")
        else:
            if next_line() != "SiliconWatch - Hardware Sensors" or next_line() != "":
                raise ParseError("missing sensor table heading")
            _match(r"DEVICE +SENSOR +VALUE +STATUS +IDENTITY", next_line())
            _match(r"-+", next_line())
            while (line := next_line()) != "":
                # Parse the two fixed enum columns from the right; labels may contain spaces.
                parts = line.rsplit(None, 2)
                if len(parts) != 3 or parts[-2] not in STATUSES or parts[-1] not in IDENTITIES:
                    raise ParseError("invalid table sensor row")
                counts[parts[-2]] += 1
            sensor_count, ok_count = map(int, _match(r"(\d+) sensors discovered; (\d+) with status ok\.", next_line()))
            if sensor_count != sum(counts.values()) or ok_count != counts["ok"]:
                raise ParseError("inconsistent sensor table counts")
            if next_line() != "Values are unvalidated hardware reports; 'ok' means read and parsed.":
                raise ParseError("missing sensor disclaimer")
            warning_count = identity_count = 0
            while (line := next_line()) != "":
                if line.startswith("Warning: "):
                    warning_count += 1
                elif line.startswith("Identity v1 "):
                    identity_count += 1
                elif not (line.startswith("Identity v1 ") or line.startswith("  Identity: ") or ": " in line):
                    raise ParseError("unexpected sensor diagnostic structure")
                if re.match(r"Sample \d+/", line):
                    raise ParseError("missing table terminator")
            if identity_count != sensor_count:
                raise ParseError("incomplete sensor identity diagnostics")
        yield {
            "sequence": seq, "requested_count": count, "interval_ms": interval,
            "target_start_ns": target, "started_ns": start, "finished_ns": finish,
            "prepared_ns": prepared, "scan_duration_ns": int(duration), "lateness_ns": int(lateness),
            "wall_start_unix_ms": None if wall == "unavailable" else int(wall),
            "failed": basis == "no_snapshot", "sensor_count": sensor_count,
            "sensor_status_counts": dict(sorted(counts.items())), "warning_count": warning_count,
            "basis": basis, "lower_bound_ns": lower, "upper_bound_ns": upper,
            "window_exceeds_interval": exceeds,
        }


def _validate(record, previous, requested_count, interval_ms):
    for key in TIMING_FIELDS + ("sequence", "requested_count", "interval_ms"):
        _integer(record[key])
    if record["sequence"] != (previous["sequence"] + 1 if previous else 1):
        raise ParseError("nonconsecutive sequence")
    if record["requested_count"] != requested_count or record["interval_ms"] != interval_ms:
        raise ParseError("sample configuration mismatch")
    target, start, finish, prepared, duration, lateness = (record[k] for k in TIMING_FIELDS)
    if not target <= start <= finish <= prepared or duration != finish - start or lateness != start - target:
        raise ParseError("inconsistent timing arithmetic")
    expected_target = previous["started_ns"] + interval_ms * 1_000_000 if previous else 0
    if target != expected_target or (previous and start < previous["prepared_ns"]):
        raise ParseError("inconsistent sequential scheduling")
    basis = "no_snapshot" if record["failed"] else "current_attempt_observation_window" if record["sensor_count"] else "no_sensor_observations"
    if record["basis"] != basis:
        raise ParseError("inconsistent observation basis")
    if basis == "current_attempt_observation_window":
        if (_integer(record["lower_bound_ns"]) != prepared - finish or
                _integer(record["upper_bound_ns"]) != prepared - start or
                type(record["window_exceeds_interval"]) is not bool or
                record["window_exceeds_interval"] != (prepared - start > interval_ms * 1_000_000)):
            raise ParseError("inconsistent freshness bounds")
    elif any(record[key] is not None for key in ("lower_bound_ns", "upper_bound_ns", "window_exceeds_interval")):
        raise ParseError("unexpected freshness bounds")
    record["start_gap_ns"] = start - previous["started_ns"] if previous else None
    record["interval_deviation_ns"] = record["start_gap_ns"] - interval_ms * 1_000_000 if previous else None


def _record_summary(records):
    statuses = Counter()
    for record in records:
        statuses.update(record["sensor_status_counts"])
    return {
        "record_count": len(records),
        "failed_attempts": sum(r["failed"] for r in records),
        "window_exceeds_interval_count": sum(r["window_exceeds_interval"] is True for r in records),
        "sensor_status_observations": dict(sorted(statuses.items())),
        "scan_duration_ns": summarize(r["scan_duration_ns"] for r in records),
        "snapshot_scan_duration_ns": summarize(r["scan_duration_ns"] for r in records if not r["failed"]),
        "failed_scan_duration_ns": summarize(r["scan_duration_ns"] for r in records if r["failed"]),
        "lateness_ns": summarize(r["lateness_ns"] for r in records if r["sequence"] > 1),
        "start_gap_ns": summarize(r["start_gap_ns"] for r in records if r["start_gap_ns"] is not None),
        "interval_deviation_ns": summarize(r["interval_deviation_ns"] for r in records if r["interval_deviation_ns"] is not None),
    }


def parse_output(path, output_format, requested_count, interval_ms, warmup_seconds=10.0, max_line_bytes=4 * 1024 * 1024):
    """Parse at most 10,000 records and one bounded line at a time.

    `complete` validates output completeness, not process success or physical
    sensor accuracy. A bad suffix retains only the structurally valid prefix.
    """
    if not 1 <= requested_count <= MAX_RECORDS or not 100 <= interval_ms <= 60_000:
        raise ValueError("invalid sampling configuration")
    if not math.isfinite(warmup_seconds) or warmup_seconds < 0 or max_line_bytes < 1:
        raise ValueError("invalid parser bound")
    if output_format not in ("table", "json-lines"):
        raise ValueError("unsupported output format")
    records, errors = [], []
    try:
        lines = _lines(path, max_line_bytes)
        parser = _table_records if output_format == "table" else _json_records
        for record in parser(lines):
            if len(records) >= requested_count:
                raise ParseError("too many records")
            _validate(record, records[-1] if records else None, requested_count, interval_ms)
            records.append(record)
    except ParseError as error:
        errors.append(str(error))
    except (ValueError, OverflowError):
        errors.append("unrepresentable numeric input")
    except OSError:
        errors.append("output could not be read")
    valid = not errors
    complete = valid and len(records) == requested_count
    if not complete and valid:
        errors.append("fewer complete records than requested")
    return {
        "parser_schema_version": 1, "sampling_schema_version": 1,
        "valid": valid, "complete": complete, "errors": errors,
        "emitted_count": len(records), "failed_attempts": sum(r["failed"] for r in records),
        "warmup_seconds": warmup_seconds,
        "first_start_lateness_ns": records[0]["lateness_ns"] if records else None,
        "records": records, "whole_run": _record_summary(records),
        "post_warmup": _record_summary([r for r in records if r["started_ns"] >= warmup_seconds * 1_000_000_000]),
    }


def _display(value, scale=1):
    if value is None:
        return "N/A"
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (int, float)):
        return f"{value / scale:.6g}"
    return str(value).replace("|", "\\|").replace("\n", " ").replace("\r", " ")


def rss_summary(samples, warmup_seconds, elapsed_seconds=None):
    """Sampled RSS windows stay distinct from the kernel lifetime peak."""
    available = [s for s in samples if s.get("rss_kib") is not None]
    post = [s for s in available if s["offset_seconds"] >= warmup_seconds]
    midpoint = (warmup_seconds + elapsed_seconds) / 2 if elapsed_seconds is not None else ((post[0]["offset_seconds"] + post[-1]["offset_seconds"]) / 2) if post else 0
    return {
        "whole_run": summarize(s["rss_kib"] for s in available),
        "post_warmup": summarize(s["rss_kib"] for s in post),
        "early_post_warmup": summarize(s["rss_kib"] for s in post if s["offset_seconds"] < midpoint),
        "late_post_warmup": summarize(s["rss_kib"] for s in post if s["offset_seconds"] >= midpoint),
        "read_duration_seconds": summarize(s["read_seconds"] for s in samples if s.get("read_seconds") is not None),
        "missing_observations": sum(s.get("rss_kib") is None for s in samples),
        "methods": sorted({s["method"] for s in available if s.get("method")}),
    }


def render_report(document):
    """Deterministic Markdown from sanitized results; no wall-clock access."""
    if document.get("schema_version") != 1:
        raise ValueError("unsupported benchmark result schema")
    lines = [
        "# SiliconWatch foreground monitoring overhead measurements", "",
        "These are measurements of the recorded release binary and conditions, not performance guarantees.", "",
        f"Suite status: **{_display(document.get('status'))}**. Retained runs: {len(document.get('runs', []))}; not started/unfinished: {len(document.get('not_started', []))}. In-progress case: {_display(document.get('in_progress'))}.", "",
        "## Provenance", "", "```json",
        json.dumps(document.get("environment", {}), indent=2, sort_keys=True, ensure_ascii=True), "```", "",
        "## Per-run resource measurements", "",
        "CPU 100% represents one logical CPU. Kernel peak RSS and sampled RSS are different metrics. N/A means unavailable, not zero.", "",
        "**Peak-RSS attribution warning:** kernel child-lifetime peak includes inherited launch high-water accounting on the tested platform, even with posix_spawn. It is not an isolated SiliconWatch application peak. Sampled RSS describes the running child and can miss transient peaks; no guessed launch constant is subtracted.", "",
        "| Case | Interval ms | Format | Requested | Emitted | Failed attempts | State | Elapsed s | CPU s | CPU % | Peak RSS KiB | Stdout bytes |",
        "| --- | ---: | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for run in document.get("runs", []):
        case, resource, parsed = run.get("case", {}), run.get("resource", {}), run.get("parsed") or {}
        cells = (case.get("id"), case.get("interval_ms"), case.get("format"), case.get("count"), parsed.get("emitted_count"), parsed.get("failed_attempts"), resource.get("termination"), resource.get("elapsed_seconds"), resource.get("total_cpu_seconds"), resource.get("cpu_percent"), resource.get("peak_rss_kib"), resource.get("stdout_bytes"))
        lines.append("| " + " | ".join(map(_display, cells)) + " |")
    lines.extend(["", "## CPU attribution and completion", "",
                  "CPU per attempt requires validated complete output, exit 0, and normal termination. Failed attempts remain in that denominator. Resource-only null-output runs cannot establish completion counts.", "",
                  "| Case | Exit | Output complete | User CPU s | System CPU s | CPU ms/attempt | Runner CPU s | Stderr bytes |",
                  "| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: |"])
    for run in document.get("runs", []):
        case, resource, parsed = run.get("case", {}), run.get("resource", {}), run.get("parsed") or {}
        qualified = parsed.get("complete") is True and resource.get("exit_code") == 0 and resource.get("termination") == "completed"
        per_attempt = resource["total_cpu_seconds"] * 1000 / parsed["emitted_count"] if qualified and parsed.get("emitted_count") and resource.get("total_cpu_seconds") is not None else None
        cells = (case.get("id"), resource.get("exit_code"), parsed.get("complete"), resource.get("user_cpu_seconds"), resource.get("system_cpu_seconds"), per_attempt, resource.get("runner_cpu_seconds"), resource.get("stderr_bytes"))
        lines.append("| " + " | ".join(map(_display, cells)) + " |")
    lines.extend(["", "## Sampled process memory", "",
                  "Each series retains its RSS method; approximate VmRSS is not interchangeable with smaps_rollup. The post-warmup window is an operational observation window, not a guaranteed steady state. Early/late medians expose sustained drift.", "",
                  "| Case | Method | Window | N | Median KiB | p95 KiB | p99 KiB | Max KiB |",
                  "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |"])
    for run in document.get("runs", []):
        resource, case = run.get("resource", {}), run.get("case", {})
        memory = rss_summary(resource.get("rss_samples", []), case.get("warmup_seconds", 10), resource.get("elapsed_seconds"))
        for window in ("whole_run", "post_warmup", "early_post_warmup", "late_post_warmup"):
            stats = memory[window]
            cells = [_display(case.get("id")), _display(resource.get("rss_method")), window, str(stats["n"])]
            cells.extend(_display(stats[k]) for k in ("median", "p95", "p99", "max"))
            lines.append("| " + " | ".join(cells) + " |")
    lines.extend(["", "## Per-run timing distributions", "",
                  "Times are milliseconds; N is the number of observations. First-start lateness is excluded from interval lateness. Snapshot and failed-attempt scan paths are separate cohorts.", "",
                  "p95 requires N ≥ 200; p99 requires N ≥ 1,000. These reporting thresholds are not statistical confidence guarantees. N/A tails remain unavailable; runs are not pooled.", "",
                  "| Case | Window | Metric | N | Median ms | p95 ms | p99 ms | Max ms |",
                  "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |"])
    for run in document.get("runs", []):
        parsed = run.get("parsed") or {}
        for window in ("whole_run", "post_warmup"):
            for name in ("snapshot_scan_duration_ns", "failed_scan_duration_ns", "lateness_ns", "interval_deviation_ns"):
                stats = parsed.get(window, {}).get(name, summarize([]))
                cells = [_display(run.get("case", {}).get("id")), window, name, str(stats["n"])]
                cells.extend(_display(stats[k], 1_000_000) for k in ("median", "p95", "p99", "max"))
                lines.append("| " + " | ".join(cells) + " |")
    groups = {}
    for run in document.get("runs", []):
        case = run.get("case", {})
        key = (case.get("interval_ms"), case.get("format"), case.get("duration_seconds"), case.get("sink"), case.get("observe_rss"), run.get("resource", {}).get("rss_method"))
        groups.setdefault(key, []).append(run)
    lines.extend(["", "## Repetition comparisons", "",
                  "Each row summarizes per-process values within one condition and RSS method, including only runs explicitly marked comparable. Incomplete, failed-attempt, or changed-inventory runs remain visible above but are excluded here. Null-output controls qualify for resource comparisons only; output-derived metrics remain unavailable. N varies when metrics are unavailable. Reported ranges expose run-to-run variability; no p95/p99 is inferred from three repetitions and unlike conditions are not pooled.", "",
                  "| Interval ms | Format | Duration s | Sink | RSS observer | RSS method | Metric | N | Median | Min | Max |",
                  "| ---: | --- | ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: |"])
    for key, runs in sorted(groups.items(), key=lambda item: str(item[0])):
        for metric in ("cpu_percent", "peak_rss_kib", "post_warmup_scan_median_ms"):
            values = []
            for run in runs:
                if run.get("validity", {}).get("comparable") is not True:
                    continue
                if metric == "post_warmup_scan_median_ms":
                    value = (run.get("parsed") or {}).get("post_warmup", {}).get("snapshot_scan_duration_ns", {}).get("median")
                    value = value / 1_000_000 if value is not None else None
                else:
                    value = run.get("resource", {}).get(metric)
                if value is not None:
                    values.append(value)
            stats = summarize(values)
            cells = [_display(v) for v in key] + [metric, str(stats["n"])] + [_display(stats[k]) for k in ("median", "min", "max")]
            lines.append("| " + " | ".join(cells) + " |")
    lines.extend(["", "## Integrity and limitations", "",
                  "Requested counts and exit status never substitute for independently validated emitted records. A valid prefix does not establish all attempted scans after interruption. CPU per attempt is meaningful only for complete validated output and successful process completion.", "",
                  "CPU includes startup, discovery, formatting, and output writes, excluding the Python parent where child accounting permits. Kernel peak RSS may include launch/exec history; sampled RSS can miss transient peaks. The result manifest identifies RSS method and observer cadence.", "",
                  "Scan duration covers sequential userspace observation, not physical sensor freshness. File/null comparisons cannot exactly isolate acquisition CPU or terminal rendering. Thermal/frequency drift, background workload, page caches, provider availability, and correlated samples limit generalization.", ""])
    lines.extend(["| Case | Comparable | Inventory unchanged | RSS read failures | Output complete |",
                  "| --- | --- | --- | ---: | --- |"])
    for run in document.get("runs", []):
        stable = run["inventory_before"] == run["inventory_after"] if "inventory_before" in run and "inventory_after" in run else None
        cells = (run.get("case", {}).get("id"), run.get("validity", {}).get("comparable"), stable, run.get("resource", {}).get("sampled_read_failures"), (run.get("parsed") or {}).get("complete"))
        lines.append("| " + " | ".join(map(_display, cells)) + " |")
    lines.append("")
    for run in document.get("runs", []):
        parsed = run.get("parsed") or {}
        problems = parsed.get("errors", [])
        if problems:
            lines.append(f"- {_display(run.get('case', {}).get('id'))}: " + "; ".join(_display(e) for e in problems))
    return "\n".join(lines).rstrip() + "\n"
