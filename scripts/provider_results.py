"""Bounded provider evidence and deterministic reports; raw identities stay private.

The shared parser validates timing and schemas. A second bounded pass checks the
registration/channel cohort without copying readings, paths or identity values
into the published result. Smoke identity profiles are a separate private API.
"""

from collections import Counter
import json
from pathlib import Path
import re

from overhead_results import (IDENTITIES, ParseError, _lines, _nonfinite, _object,
                              parse_output, summarize)

MAX_LINE_BYTES = 4 * 1024 * 1024
MAX_PROFILE_BYTES = 1024 * 1024
TAIL_NS = 100_000_000
_CONTEXT_ISSUES = {
    "duplicate identity candidate; every source is retained",
    "multiple hwmon registrations share parent and provider; channel ownership is ambiguous",
}


def _objects(path):
    for line in _lines(path, MAX_LINE_BYTES):
        try:
            value = json.loads(line, object_pairs_hook=_object, parse_constant=_nonfinite)
            if not isinstance(value, dict):
                raise ParseError("malformed JSON sample")
            yield value
        except (json.JSONDecodeError, RecursionError):
            raise ParseError("malformed JSON sample") from None


def _registrations(registrations):
    """Build private lookup and expected ordered rows; reject uncertain matching."""
    lookup, ordered = {}, []
    for registration in sorted(registrations, key=lambda r: r["entry"]):
        token = registration["token"]
        if re.fullmatch(r"r[0-9]{3}", token) is None:
            raise ValueError("invalid registration token")
        if not registration.get("target"):
            continue
        key = (str(registration["target"]), registration["entry"])
        if key in lookup or any(r["token"] == token for r in lookup.values()):
            raise ValueError("duplicate registration evidence")
        lookup[key] = registration
        ordered.extend((token, channel) for channel in registration.get("channels", []))
    return lookup, ordered


def _rows(snapshot, lookup):
    rows = []
    for sensor in snapshot["sensors"]:
        key = (sensor["device_path"], Path(sensor["source_path"]).parent.name)
        registration = lookup.get(key)
        if registration is None:
            raise ParseError("unexpected registration in captured snapshot")
        channel = sensor["attribute"]
        if (not isinstance(channel, str) or channel not in registration["channels"] or
                Path(sensor["source_path"]) != Path(snapshot["root"]) / registration["entry"] / channel):
            raise ParseError("unexpected channel in captured snapshot")
        identity = sensor["identity"]
        if (type(identity["schema_version"]) is not int or identity["schema_version"] != 1 or identity["status"] not in IDENTITIES or
                not isinstance(identity["issues"], list) or
                any(not isinstance(v, str) for v in identity["issues"]) or
                not isinstance(identity["evidence"], dict) or
                (identity["candidate"] is not None and not isinstance(identity["candidate"], str))):
            raise ParseError("malformed identity evidence")
        rows.append((registration["token"], channel, sensor))
    return rows


def inspect_capture(path, config, registrations):
    """Return shareable parsed scalars, cohort coverage, and tail record indexes.

    ``coverage.valid`` means the expected rows were present in every emitted
    snapshot. It does not establish completion, success, or physical identity.
    The caller must combine it with process status and parsed completeness.
    """
    if not 1 <= config["count"] <= 301:
        raise ValueError("provider cases permit at most 301 attempts")
    parsed = parse_output(path, "json-lines", config["count"], config["interval_ms"],
                          config.get("warmup_seconds", 10))
    lookup, expected = _registrations(registrations)
    totals = {r["token"]: Counter() for r in registrations if r.get("channels")}
    identity_counts = {token: Counter() for token in totals}
    errors = []
    matched = snapshots = inspected = unusable_snapshots = 0
    missing = set()
    try:
        objects = _objects(path)
        for record in parsed["records"]:
            obj = next(objects)
            inspected += 1
            if obj["sequence"] != record["sequence"]:
                raise ParseError("capture changed between validation passes")
            if obj["snapshot"] is None:
                continue
            snapshots += 1
            rows = _rows(obj["snapshot"], lookup)
            actual = [(token, channel) for token, channel, _ in rows]
            if actual != expected:
                missing.update(token for token, channel in expected if (token, channel) not in actual)
                raise ParseError("missing, duplicate or reordered registration channels")
            matched += 1
            counts = Counter()
            for token, _channel, sensor in rows:
                totals[token][sensor["status"]] += 1
                identity_counts[token][sensor["identity"]["status"]] += 1
                counts[sensor["status"]] += 1
            if not counts["ok"]:
                unusable_snapshots += 1
    except (KeyError, TypeError, ValueError, StopIteration, OSError) as error:
        errors.append(str(error) if isinstance(error, ParseError) else "invalid or unavailable cohort evidence")
    aggregate = sum(totals.values(), Counter())
    failed = sum(count for status, count in aggregate.items() if status not in ("ok", "unsupported"))
    details = []
    for registration in sorted(registrations, key=lambda r: r["token"]):
        token = registration["token"]
        provider = registration.get("provider")
        if not isinstance(provider, str) or re.fullmatch(r"[A-Za-z0-9_.+-]{1,80}", provider) is None:
            provider = None
        details.append({"token": token, "provider": provider,
                        "expected_channel_count": len(registration.get("channels", [])),
                        "status_counts": dict(sorted(totals.get(token, {}).items())),
                        "identity_status_counts": dict(sorted(identity_counts.get(token, {}).items()))})
    valid = not errors and inspected == parsed["emitted_count"] and snapshots == matched
    sequences = [r["sequence"] for r in parsed["records"] if r["scan_duration_ns"] > TAIL_NS]
    available = {r["sequence"] for r in parsed["records"]}
    neighbors = sorted({n for seq in sequences for n in (seq - 1, seq, seq + 1) if n in available})
    return {"parsed": parsed,
            "coverage": {"valid": valid, "complete": valid and parsed["complete"], "expected_channel_count": len(expected),
                "inspected_records": inspected, "snapshot_count": snapshots,
                "matched_snapshots": matched, "unusable_snapshot_count": unusable_snapshots,
                "usable": valid and snapshots > 0 and unusable_snapshots == 0 and bool(expected),
                "missing_tokens": sorted(missing), "errors": errors,
                "successful_readings": aggregate["ok"], "failed_readings": failed,
                "unsupported_readings": aggregate["unsupported"],
                "status_counts": dict(sorted(aggregate.items())), "registrations": details},
            "tails": {"threshold_ns": TAIL_NS, "sequences": sequences,
                      "neighbor_sequences": neighbors}}


def identity_profile(path, registrations):
    """PRIVATE first-snapshot identity profile; never include this in results JSON.

    Live readings/labels are intentionally not compared across different times.
    Profiles retain identities and diagnostic source paths only in memory.
    """
    lookup, expected = _registrations(registrations)
    try:
        for index, obj in enumerate(_objects(path)):
            if index >= 301:
                raise ParseError("identity profile record limit exceeded")
            snapshot = obj["snapshot"]
            if snapshot is None:
                continue
            rows = _rows(snapshot, lookup)
            if [(token, channel) for token, channel, _ in rows] != expected:
                raise ParseError("identity profile cohort differs")
            profile = {"root": snapshot["root"], "registrations": {}}
            for token, channel, sensor in rows:
                profile["registrations"].setdefault(token, {})[channel] = {
                    "identity": sensor["identity"], "device_path": sensor["device_path"],
                    "source_path": sensor["source_path"],
                }
            if len(json.dumps(profile).encode()) > MAX_PROFILE_BYTES:
                raise ParseError("private identity profile exceeds bound")
            return profile
    except (KeyError, TypeError, OSError, ValueError) as error:
        if isinstance(error, ParseError):
            raise
        raise ParseError("identity profile unavailable or malformed") from None
    raise ParseError("identity profile has no snapshot")


def compare_identity_profiles(full, isolated, token):
    """Shareable comparison; expected collision-context differences stay explicit."""
    before = full.get("registrations", {}).get(token, {})
    after = isolated.get("registrations", {}).get(token, {})
    channels_equal = bool(before) and list(before) == list(after)
    problems, changes = [], []
    if not channels_equal:
        problems.append("identity profile channels missing or reordered")
    candidate_equal = evidence_equal = device_equal = True
    source_changes = 0
    for index, channel in enumerate(before):
        if channel not in after:
            continue
        a, b = before[channel], after[channel]
        ia, ib = a["identity"], b["identity"]
        candidate_equal &= ia["candidate"] == ib["candidate"]
        evidence_equal &= ia["evidence"] == ib["evidence"]
        device_equal &= a["device_path"] == b["device_path"]
        source_changes += a["source_path"] != b["source_path"]
        if ia["status"] != ib["status"] or ia["issues"] != ib["issues"]:
            removed = set(ia["issues"]) - set(ib["issues"])
            added = set(ib["issues"]) - set(ia["issues"])
            allowed = (bool(removed) and not added and removed <= _CONTEXT_ISSUES and
                       ia["status"] in ("collision", "ambiguous_channel") and
                       ib["status"] in ("best_effort", "collision", "ambiguous_channel"))
            changes.append({"channel_index": index, "from_status": ia["status"],
                            "to_status": ib["status"], "collision_context_only": allowed})
            if not allowed:
                problems.append("unexpected identity status or issue change")
    if not candidate_equal:
        problems.append("identity candidate changed under isolation")
    if not evidence_equal:
        problems.append("identity evidence changed under isolation")
    if not device_equal:
        problems.append("canonical device path changed under isolation")
    return {"token": token, "compatible": channels_equal and not problems,
            "channel_count": len(before), "candidate_equal": candidate_equal,
            "evidence_equal": evidence_equal, "canonical_device_equal": device_equal,
            "root_changed": full.get("root") != isolated.get("root"),
            "source_path_changes": source_changes, "changes": changes,
            "errors": sorted(set(problems))}


def _display(value, scale=1):
    if value is None:
        return "N/A"
    if type(value) in (int, float):
        return f"{value / scale:.6g}"
    return str(value).replace("|", "\\|").replace("\n", " ").replace("\r", " ")


def render_report(document):
    """Render only saved evidence; identical input produces identical Markdown."""
    if document.get("schema_version") != 1:
        raise ValueError("unsupported provider result schema")
    runs = document.get("runs", [])
    configuration = document.get("configuration", {})
    bounds = {key: configuration.get(key) for key in
              ("suite_timeout_seconds", "suite_limit_bytes", "metadata_reserve_bytes",
               "settle_seconds", "observe_rss")}
    effective_timeouts = {run.get("case", {}).get("id"): run.get("effective_timeout_seconds")
                          for run in runs}
    lines = ["# SiliconWatch provider-level scan latency", "",
             "Measured scan windows are observations, not performance guarantees or pure driver-read latency.", "",
             f"Suite status: **{_display(document.get('status'))}**; retained cases: {len(runs)}; unstarted: {len(document.get('not_started', []))}.", "",
             f"Actual suite elapsed seconds: {_display(document.get('actual_suite_elapsed_seconds'))}. Selected session tokens: {_display(', '.join(document.get('selected_tokens', [])))}.", "",
             "Configured bounds:", "", "```json",
             json.dumps(bounds, sort_keys=True, indent=2), "```", "",
             "## Case bounds", "",
             "Configured limits describe every planned case, including unstarted cases. Effective timeout is the saved value passed to the child; N/A means unrecorded or unstarted, not zero.", "",
             "| Case | Requested | Interval ms | Warmup s | Configured timeout s | Effective timeout s | Combined output cap bytes |",
             "| --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for case in configuration.get("cases", []):
        values = (case.get("id"), case.get("count"), case.get("interval_ms"),
                  case.get("warmup_seconds"), case.get("timeout_seconds"),
                  effective_timeouts.get(case.get("id")))
        cap = str(case["output_limit_bytes"]) if case.get("output_limit_bytes") is not None else "N/A"
        lines.append("| " + " | ".join(map(_display, values)) + " | " + cap + " |")
    lines += ["", "## Environment and registration inventory", "", "```json",
             json.dumps(document.get("environment", {}), sort_keys=True, indent=2, ensure_ascii=True), "```", "",
             "| Token | Provider | Channels | Availability |",
             "| --- | --- | ---: | --- |"]
    for item in document.get("inventory", []):
        lines.append("| " + " | ".join(map(_display, (item.get("token"), item.get("provider"),
                                                      item.get("channel_count"), item.get("status")))) + " |")
    lines += ["", "## Cases and resource accounting", "",
              "CPU 100% is one logical CPU. Runner CPU is separate from child CPU. Kernel child-lifetime peak RSS includes inherited launch high-water accounting; it is not an isolated application peak. Periodic RSS observation is disabled consistently; absent sampled RSS is unavailable, not zero.", "",
              "| Case | Phase / role | Block | Token | Requested | Interval ms | Verified attempts | Failed attempts | State | Elapsed s | Child CPU s | CPU % | Runner CPU s | Kernel peak KiB | Output bytes |",
              "| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for run in runs:
        c, r, p, v = (run.get(k, {}) for k in ("case", "resource", "parsed", "validity"))
        values = (c.get("id"), f"{c.get('phase')} / {c.get('role')}", c.get("block"), c.get("token"), c.get("count"), c.get("interval_ms"),
                  v.get("actual_attempts"), p.get("failed_attempts"), r.get("termination"),
                  r.get("elapsed_seconds"), r.get("total_cpu_seconds"), r.get("cpu_percent"),
                  r.get("runner_cpu_seconds"), r.get("peak_rss_kib"),
                  sum(r[k] for k in ("stdout_bytes", "stderr_bytes")) if all(r.get(k) is not None for k in ("stdout_bytes", "stderr_bytes")) else None)
        lines.append("| " + " | ".join(map(_display, values)) + " |")
    lines += ["", "## Reading coverage", "",
              "Counts describe emitted observations, not physical accuracy. Failed readings exclude unsupported channels, which remain separate. Valid output prefixes do not establish completed attempts after interruption.", "",
              "| Case | Expected channels | Matched snapshots | OK readings | Failed readings | Unsupported | Usable cohort | Complete |",
              "| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |"]
    for run in runs:
        c = run.get("coverage", {})
        values = (run.get("case", {}).get("id"), c.get("expected_channel_count"),
                  c.get("matched_snapshots"), c.get("successful_readings"), c.get("failed_readings"),
                  c.get("unsupported_readings"), c.get("usable"), run.get("validity", {}).get("complete"))
        lines.append("| " + " | ".join(map(_display, values)) + " |")
    lines += ["", "## Timing distributions", "",
              "Milliseconds. Each case/window is separate: no provider/repetition pooling. p95 requires N ≥200; p99 N ≥1,000. These reporting thresholds do not guarantee statistical confidence. First-start lateness is retained in scalar data but excluded from interval-lateness statistics.", "",
              "| Case | Window | Metric | N | Median ms | p95 ms | p99 ms | Max ms |",
              "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |"]
    for run in runs:
        for window in ("whole_run", "post_warmup"):
            for metric in ("snapshot_scan_duration_ns", "failed_scan_duration_ns", "lateness_ns", "interval_deviation_ns"):
                stats = run.get("parsed", {}).get(window, {}).get(metric, summarize([]))
                values = [_display(run.get("case", {}).get("id")), window, metric, str(stats["n"])]
                values += [_display(stats[k], 1_000_000) for k in ("median", "p95", "p99", "max")]
                lines.append("| " + " | ".join(values) + " |")
    lines += ["", "## Long-tail evidence", "",
              "A scan window >100 ms is a descriptive flag, not an alarm. All scalar records remain saved; tail/neighbor sequence indexes reference them without duplicating raw telemetry. Failed attempts are separately identified in each record.", "",
              "| Case | Tail count | Maximum sequence | Maximum ms | Neighbor records retained |",
              "| --- | ---: | ---: | ---: | ---: |"]
    for run in runs:
        records = run.get("parsed", {}).get("records", [])
        maximum = max(records, key=lambda r: r["scan_duration_ns"], default={})
        tail = run.get("tails", {})
        cells = [_display(run.get("case", {}).get("id")), str(len(tail.get("sequences", []))),
                 _display(maximum.get("sequence")), _display(maximum.get("scan_duration_ns"), 1_000_000),
                 str(len(tail.get("neighbor_sequences", [])))]
        lines.append("| " + " | ".join(cells) + " |")
    lines += ["", "## Identity context", "",
              "Temporary roots change diagnostic source paths. Removing peers may change collision status; this never establishes unique physical identity. Candidate/evidence changes invalidate the smoke comparison.", "", "```json",
              json.dumps(document.get("identity_comparisons", []), sort_keys=True, indent=2), "```", "",
              "## Interpretation and limitations", "",
              "Full-scan controls bracket isolated cases within each repetition block. Compare contemporary windows, never sum isolated durations or subtract them as exact full-scan contributions. Snapshot timing includes timestamp preamble, discovery, identity/collision processing, reads and descheduling. Serialization/capture happens afterward but may delay subsequent starts. Child/runner CPU and output bytes expose some overhead, not exact kernel/provider attribution.", "",
              "Thirty-second cases do not recreate the historical 300-second run containing the 503.365 ms maximum. Non-recurrence is inconclusive. No driver is established as its cause by these measurements. Background work, thermal/frequency drift, driver caches and identity-context changes limit inference; blocking I/O prevents a hard completion guarantee.", "",
              "Incomplete cases and missing measurements remain explicit. An entirely failed or incomplete comparison is not successful characterization; valid comparisons may still yield inconclusive attribution.", ""]
    for run in runs:
        errors = run.get("parsed", {}).get("errors", []) + run.get("coverage", {}).get("errors", [])
        if errors:
            lines.append(f"- {_display(run.get('case', {}).get('id'))}: " + "; ".join(map(_display, errors)))
    if document.get("prior_stop_reason"):
        lines += ["", f"Earlier stop reason: **{_display(document['prior_stop_reason'])}**."]
    if document.get("reporting_errors"):
        lines += ["", "## Reporting failures", "", "```json",
                  json.dumps(document["reporting_errors"], sort_keys=True, indent=2, ensure_ascii=True), "```"]
    return "\n".join(lines).rstrip() + "\n"
