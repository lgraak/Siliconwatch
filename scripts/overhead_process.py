"""Bounded Linux child-process measurements; no sensor or system configuration writes.

The executable is spawned directly, without a Python/shell launcher. Output goes
through bounded pipes into private files so the combined on-disk capture cannot
exceed its byte budget. This transport is part of the measured workload: it is
not equivalent to giving the child a regular-file descriptor. Only wait4 reaps
the child. Kernel peak RSS may still include launch history and is never treated
as the same quantity as periodically observed RSS.
"""

from __future__ import annotations

import math
import os
from pathlib import Path
import selectors
import signal
import time


def _start_time(pid: int, proc_root: Path = Path("/proc")) -> str:
    # comm may contain spaces or ')'; the final ')' precedes state (field 3).
    fields = (proc_root / str(pid) / "stat").read_text().rsplit(")", 1)[1].split()
    return fields[19]  # starttime is field 22.


def read_rss(pid: int, identity: str, method: str | None = None,
             proc_root: Path = Path("/proc")) -> tuple[int | None, str | None]:
    """Return KiB and a fixed method; null on exit, denial, or identity change.

    Prefer smaps_rollup. Once selected, a method is not silently changed on a
    later failure. This permits callers to reject comparisons of unlike methods.
    """
    methods = [method] if method else ["smaps_rollup", "status_vmrss_approximate"]
    try:
        if _start_time(pid, proc_root) != identity:
            return None, method
        for candidate in methods:
            name, key = ("smaps_rollup", "Rss:") if candidate == "smaps_rollup" else ("status", "VmRSS:")
            try:
                text = (proc_root / str(pid) / name).read_text()
                row = next(line.split() for line in text.splitlines() if line.startswith(key))
                if len(row) != 3 or row[2] != "kB":
                    raise ValueError("unknown procfs unit")
                value = int(row[1])
                if value < 0 or _start_time(pid, proc_root) != identity:
                    return None, method
                return value, candidate
            except (OSError, ValueError, StopIteration):
                continue
    except (OSError, ValueError, IndexError):
        pass
    return None, method


def _signal_group(pid: int, sig: int) -> None:
    """Only call while this direct child has not been reaped."""
    try:
        os.killpg(pid, sig)
    except ProcessLookupError:
        pass


def run_process(argv, output_dir, *, timeout_s, output_limit_bytes,
                rss_interval_s=1.0, observe_rss=True, sink="file",
                interrupt_event=None, grace_s=0.5, reap_timeout_s=3.0):
    """Run one executable, returning JSON-safe accounting and bounded samples.

    ``termination == 'completed'`` means a natural process exit, not successful
    sensor data or a verified record count. Caller must independently parse the
    capture and inspect exit status. A null-sink stdout has unknown byte/count
    totals. ``reaped == False`` means cancellation could not reap a blocked child;
    the caller must stop the suite, not start another case.
    """
    if not argv or not all(isinstance(arg, str) and "\0" not in arg for arg in argv) or not argv[0]:
        raise ValueError("argv must contain nonempty executable and string arguments")
    if not all(math.isfinite(v) and v > 0 for v in (timeout_s, rss_interval_s, grace_s, reap_timeout_s)):
        raise ValueError("time limits and RSS interval must be finite and positive")
    if not isinstance(output_limit_bytes, int) or isinstance(output_limit_bytes, bool) or output_limit_bytes < 1:
        raise ValueError("output limit must be a positive integer")
    if sink not in ("file", "null"):
        raise ValueError("sink must be file or null")
    if observe_rss and math.ceil(timeout_s / rss_interval_s) > 20000:
        raise ValueError("RSS sample budget exceeds 20000 observations")

    directory = Path(output_dir)
    directory.mkdir(mode=0o700, parents=True, exist_ok=False)
    result = {
        "termination": "setup_error", "exit_code": None, "signal": None,
        "reaped": True, "elapsed_seconds": 0.0,
        "user_cpu_seconds": None, "system_cpu_seconds": None,
        "total_cpu_seconds": None, "cpu_percent": None, "peak_rss_kib": None,
        "peak_rss_scope": "kernel_child_lifetime_including_inherited_launch_peak",
        "rss_method": None, "rss_samples": [], "sampled_read_failures": 0,
        "stdout_bytes": 0 if sink == "file" else None, "stderr_bytes": 0,
        "stdout_truncated": False, "stderr_truncated": False,
        "runner_cpu_seconds": 0.0, "setup_error": None,
        "output_transport": "pipe_capture_to_file" if sink == "file" else "direct_dev_null_stdout",
        "rss_interval_seconds": rss_interval_s if observe_rss else None,
        "poll_interval_seconds": 0.05,
    }
    cpu_start = time.process_time()
    start = time.monotonic()
    pid = None
    reaped = False
    fds = set()
    captures = {}
    selector = selectors.DefaultSelector()
    usage = None
    wait_status = None
    try:
        file_actions = []
        input_fd = os.open(os.devnull, os.O_RDONLY)
        fds.add(input_fd)
        file_actions.append((os.POSIX_SPAWN_DUP2, input_fd, 0))
        for name, child_fd in (("stdout", 1), ("stderr", 2)):
            if name == "stdout" and sink == "null":
                null_fd = os.open(os.devnull, os.O_WRONLY)
                fds.add(null_fd)
                file_actions.append((os.POSIX_SPAWN_DUP2, null_fd, child_fd))
                continue
            read_fd, write_fd = os.pipe2(os.O_CLOEXEC)
            fds.update((read_fd, write_fd))
            os.set_blocking(read_fd, False)
            output_fd = os.open(directory / f"{name}.raw", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            fds.add(output_fd)
            captures[read_fd] = (name, output_fd)
            selector.register(read_fd, selectors.EVENT_READ)
            file_actions.append((os.POSIX_SPAWN_DUP2, write_fd, child_fd))
        # Newly created descriptors are CLOEXEC; their dup2 stdio copies survive.
        if interrupt_event is not None and interrupt_event.is_set():
            result["termination"] = "interrupted"
            return result
        pid = os.posix_spawn(argv[0], argv, os.environ.copy(),
                             file_actions=file_actions, setpgroup=0,
                             setsigdef=(signal.SIGPIPE, signal.SIGINT, signal.SIGTERM))
        for fd in list(fds):
            if fd not in captures and fd not in [item[1] for item in captures.values()]:
                os.close(fd)
                fds.remove(fd)
        result["termination"] = "completed"
        try:
            identity = _start_time(pid)
        except (OSError, IndexError, ValueError):
            identity = None
        next_rss = start + rss_interval_s
        stop_at = None
        killed_at = None
        stored_bytes = 0
        child_exit_time = None
        while True:
            try:
                now = time.monotonic()
                if not reaped:
                    waited, status, child_usage = os.wait4(pid, os.WNOHANG)
                    if waited:
                        reaped, wait_status, usage = True, status, child_usage
                        child_exit_time = now
                if not reaped and stop_at is None:
                    if interrupt_event is not None and interrupt_event.is_set():
                        result["termination"] = "interrupted"
                        stop_at = now
                    elif now - start >= timeout_s:
                        result["termination"] = "timeout"
                        stop_at = now
                    if stop_at is not None:
                        _signal_group(pid, signal.SIGTERM)
                if not reaped and stop_at is not None:
                    if killed_at is None and now - stop_at >= grace_s:
                        _signal_group(pid, signal.SIGKILL)
                        killed_at = now
                    if killed_at is not None and now - killed_at >= reap_timeout_s:
                        result["reaped"] = False
                        break
                if observe_rss and not reaped and stop_at is None and now >= next_rss:
                    read_start = time.monotonic()
                    value, method = read_rss(pid, identity, result["rss_method"]) if identity is not None else (None, None)
                    result["rss_method"] = method
                    result["rss_samples"].append({
                        "offset_seconds": read_start - start, "rss_kib": value,
                        "method": method, "read_seconds": time.monotonic() - read_start,
                    })
                    result["sampled_read_failures"] += value is None
                    next_rss = time.monotonic() + rss_interval_s
                if reaped and not selector.get_map():
                    break
                # A descendant retaining a pipe must not make a completed leader
                # keep the harness alive forever. SiliconWatch launches no children.
                if reaped and child_exit_time is not None and now - child_exit_time >= 0.5:
                    result["termination"] = "pipe_not_closed"
                    break
                delay = 0.05
                if not reaped and stop_at is None:
                    delay = min(delay, max(0.0, start + timeout_s - now))
                    if observe_rss:
                        delay = min(delay, max(0.0, next_rss - now))
                for key, _ in selector.select(delay):
                    fd = key.fd
                    try:
                        chunk = os.read(fd, 65536)
                    except BlockingIOError:
                        continue
                    if not chunk:
                        selector.unregister(fd)
                        os.close(fd)
                        fds.remove(fd)
                        continue
                    name, output_fd = captures[fd]
                    remaining = max(0, output_limit_bytes - stored_bytes)
                    accepted = chunk[:remaining]
                    while accepted:
                        written = os.write(output_fd, accepted)
                        if written == 0:
                            raise OSError("zero-byte capture write")
                        accepted = accepted[written:]
                        result[f"{name}_bytes"] += written
                        stored_bytes += written
                    if len(chunk) > remaining:
                        result[f"{name}_truncated"] = True
                        if stop_at is None:
                            result["termination"] = "output_limit"
                            stop_at = time.monotonic()
                            if not reaped:
                                _signal_group(pid, signal.SIGTERM)
            except KeyboardInterrupt:
                if stop_at is None:
                    result["termination"] = "interrupted"
                    stop_at = time.monotonic()
                    if not reaped:
                        _signal_group(pid, signal.SIGTERM)
    except OSError as exc:
        result["termination"] = "setup_error" if pid is None else "capture_error"
        # Avoid publishing executable/output paths through exception messages.
        result["setup_error"] = {"type": type(exc).__name__, "errno": exc.errno}
    finally:
        if pid is not None and not reaped and result["reaped"]:
            _signal_group(pid, signal.SIGKILL)
            until = time.monotonic() + reap_timeout_s
            while time.monotonic() < until:
                waited, status, child_usage = os.wait4(pid, os.WNOHANG)
                if waited:
                    reaped, wait_status, usage = True, status, child_usage
                    break
                time.sleep(0.01)
            result["reaped"] = reaped
        selector.close()
        for fd in fds:
            os.close(fd)
        elapsed = time.monotonic() - start
        result["elapsed_seconds"] = elapsed
        result["runner_cpu_seconds"] = time.process_time() - cpu_start
        if usage is not None:
            result["user_cpu_seconds"] = usage.ru_utime
            result["system_cpu_seconds"] = usage.ru_stime
            result["total_cpu_seconds"] = usage.ru_utime + usage.ru_stime
            result["cpu_percent"] = 100 * result["total_cpu_seconds"] / elapsed if elapsed > 0 else None
            result["peak_rss_kib"] = usage.ru_maxrss if usage.ru_maxrss > 0 else None
        if wait_status is not None:
            if os.WIFEXITED(wait_status):
                result["exit_code"] = os.WEXITSTATUS(wait_status)
            elif os.WIFSIGNALED(wait_status):
                result["signal"] = os.WTERMSIG(wait_status)
    return result
