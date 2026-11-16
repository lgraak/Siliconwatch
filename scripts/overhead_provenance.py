"""Read-only, allowlisted provenance for local overhead measurements.

No hostnames, serial numbers, source paths, full environment, or command errors
are returned. Inventory counts describe exposed filenames, not successful reads.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
from datetime import datetime, timezone


_PROVIDER = re.compile(r"[A-Za-z0-9_.+-]{1,80}\Z")
_INPUT = re.compile(r"([a-z]+)[0-9]+_input\Z")
_CATEGORIES = {
    "temp": "temperature", "in": "voltage", "fan": "fan",
    "freq": "frequency", "power": "power", "curr": "current",
    "energy": "energy", "humidity": "humidity",
}


def _read(path: Path, limit: int = 65536) -> str | None:
    try:
        with path.open("r", encoding="utf-8", errors="strict") as stream:
            value = stream.read(limit + 1)
        return value if len(value) <= limit else None
    except (OSError, UnicodeError):
        return None


def _text(value: str | None, limit: int = 200) -> str | None:
    if value is None:
        return None
    value = value.strip()
    if not value or len(value) > limit or not value.isprintable():
        return None
    return value


def _error(error: OSError) -> str:
    if isinstance(error, FileNotFoundError):
        return "missing"
    if isinstance(error, PermissionError):
        return "permission_denied"
    return "io_error"


def inventory(root: Path) -> list[dict]:
    """Describe every registered provider, including zero-input providers.

    A root enumeration failure is represented by one null-provider entry. Device
    paths and volatile hwmon numbering are intentionally absent. Duplicate
    provider entries are preserved; sorting makes comparisons deterministic.
    """
    try:
        devices = sorted(p for p in root.iterdir() if re.fullmatch(r"hwmon[0-9]+", p.name))
    except OSError as error:
        return [{"provider": None, "name_status": "unavailable",
                 "discovery_status": _error(error), "input_categories": None,
                 "exposed_input_count": None}]
    result = []
    for device in devices:
        name = _read(device / "name", 256)
        provider = name.strip() if name is not None else None
        valid = provider is not None and _PROVIDER.fullmatch(provider) is not None
        entry = {"provider": provider if valid else None,
                 "name_status": "ok" if valid else "unavailable_or_invalid",
                 "discovery_status": "ok", "input_categories": {},
                 "exposed_input_count": 0}
        try:
            for attribute in device.iterdir():
                match = _INPUT.fullmatch(attribute.name)
                if match:
                    category = _CATEGORIES.get(match.group(1), "other")
                    counts = entry["input_categories"]
                    counts[category] = counts.get(category, 0) + 1
                    entry["exposed_input_count"] += 1
        except OSError as error:
            entry.update(discovery_status=_error(error), input_categories=None,
                         exposed_input_count=None)
        result.append(entry)
    return sorted(result, key=lambda item: json.dumps(item, sort_keys=True))


def _fields(path: Path, separator: str = ":") -> dict[str, str]:
    text = _read(path)
    if text is None:
        return {}
    fields = {}
    for line in text.splitlines():
        if separator in line:
            key, value = line.split(separator, 1)
            fields.setdefault(key.strip(), value.strip())
    return fields


def _integer(value: str | None) -> int | None:
    if value is None or not re.fullmatch(r"[0-9]+", value.strip()):
        return None
    return int(value.strip())


def _command(argv: list[str], cwd: Path | None = None) -> str | None:
    try:
        result = subprocess.run(argv, cwd=cwd, stdin=subprocess.DEVNULL,
                                stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                text=True, timeout=10, check=False)
        return _text(result.stdout) if result.returncode == 0 else None
    except (OSError, subprocess.TimeoutExpired, UnicodeError):
        return None


def file_sha256(path: Path) -> str | None:
    try:
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(65536), b""):
                digest.update(chunk)
        return digest.hexdigest()
    except OSError:
        return None


def _filesystem(path: Path, mountinfo: Path = Path("/proc/self/mountinfo")) -> str | None:
    text = _read(mountinfo, 1024 * 1024)
    if text is None:
        return None
    target = path.resolve()
    matches = []
    for line in text.splitlines():
        parts = line.split()
        if "-" not in parts or len(parts) < 7:
            continue
        marker = parts.index("-")
        if marker + 1 >= len(parts):
            continue
        mount = Path(re.sub(r"\\([0-7]{3})", lambda m: chr(int(m.group(1), 8)), parts[4]))
        if target == mount or mount in target.parents:
            matches.append((len(mount.parts), _text(parts[marker + 1], 40)))
    return max(matches, default=(0, None), key=lambda item: item[0])[1]


def _power(root: Path = Path("/sys/class/power_supply")) -> list[dict]:
    supplies = []
    try:
        paths = list(root.iterdir())
    except OSError:
        return []
    for path in paths:
        kind = _text(_read(path / "type"), 40)
        if kind is None:
            continue
        supplies.append({"type": kind, "online": _integer(_read(path / "online")),
                         "status": _text(_read(path / "status"), 40)})
    return sorted(supplies, key=lambda item: json.dumps(item, sort_keys=True))


def _cpu_quota() -> dict:
    # Read only a unified cgroup path contained within its mount. Never export it.
    text = _read(Path("/proc/self/cgroup")) or ""
    relative = next((line[3:] for line in text.splitlines() if line.startswith("0::")), None)
    if relative is None or ".." in Path(relative).parts:
        return {"quota_us": None, "period_us": None, "status": "unavailable"}
    value = _read(Path("/sys/fs/cgroup") / relative.lstrip("/") / "cpu.max")
    parts = value.split() if value is not None else []
    if len(parts) != 2 or (parts[0] != "max" and _integer(parts[0]) is None):
        return {"quota_us": None, "period_us": None, "status": "unavailable"}
    return {"quota_us": None if parts[0] == "max" else int(parts[0]),
            "period_us": _integer(parts[1]),
            "status": "unlimited_at_current_cgroup" if parts[0] == "max" else "limited_at_current_cgroup"}


def collect_environment(binary: Path, repo: Path, output_dir: Path,
                        hwmon_root: Path = Path("/sys/class/hwmon")) -> dict:
    """Collect an explicit allowlist; null means unavailable, never zero.

    Compiler strings describe the installed requested toolchain; the runner must
    separately record build commands and cannot infer a binary's compiler here.
    This snapshot runs outside the measured child interval.
    """
    os_release = _fields(Path("/etc/os-release"), "=")
    cpu = _fields(Path("/proc/cpuinfo"))
    memory = _fields(Path("/proc/meminfo"))
    memory_total = re.fullmatch(r"([0-9]+) kB", memory.get("MemTotal", ""))
    revision = _command(["git", "rev-parse", "HEAD"], repo)
    if revision is not None and not re.fullmatch(r"[a-f0-9]{40,64}", revision):
        revision = None
    try:
        free_bytes = os.statvfs(output_dir).f_bavail * os.statvfs(output_dir).f_frsize
    except OSError:
        free_bytes = None
    try:
        affinity_count = len(os.sched_getaffinity(0))
    except (AttributeError, OSError):
        affinity_count = None
    try:
        load = list(os.getloadavg())
    except (AttributeError, OSError):
        load = None
    governors = sorted({value for path in Path("/sys/devices/system/cpu").glob(
        "cpufreq/policy*/scaling_governor") if (value := _text(_read(path), 40)) is not None})
    return {
        "provenance_schema_version": 1,
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "os": {"id": _text(os_release.get("ID", "").strip('"')),
               "version_id": _text(os_release.get("VERSION_ID", "").strip('"')),
               "pretty_name": _text(os_release.get("PRETTY_NAME", "").strip('"')),
               "kernel_release": _text(platform.release()),
               "architecture": _text(platform.machine())},
        "cpu": {"model": _text(cpu.get("model name", cpu.get("Processor"))),
                "logical_count": os.cpu_count(), "affinity_cpu_count": affinity_count,
                "observed_governors": governors, "cgroup_quota": _cpu_quota()},
        "memory_total_kib": int(memory_total.group(1)) if memory_total else None,
        "python_version": platform.python_version(),
        "installed_rust_1_85": _command(["rustc", "+1.85.0", "--version"]),
        "installed_cargo_1_85": _command(["cargo", "+1.85.0", "--version"]),
        "git_revision": revision,
        "binary_sha256": file_sha256(binary),
        "provider_inventory": inventory(hwmon_root),
        "output_filesystem": _filesystem(output_dir),
        "output_free_bytes": free_bytes,
        "power_supplies": _power(),
        "load_average_1_5_15_minutes": load,
        "background_workload": "uncontrolled; load averages are observations, not process attribution",
        "limitations": ["No physical sensor-read validation in inventory",
                        "Current cgroup quota may omit stricter ancestor limits",
                        "Installed compiler version does not prove binary build provenance",
                        "No hostname, serial, hardware source path, or environment dump collected"],
    }
