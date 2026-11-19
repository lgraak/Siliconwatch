#!/usr/bin/env python3
"""Run installed software as an ordinary user with a synthetic hwmon and private X11."""
from contextlib import contextmanager
import json
import os
from pathlib import Path
import socket
import stat
import subprocess
import tempfile
import time


def wait_for(check, process, label):
    deadline = time.monotonic() + 25
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"{label}: process exited {process.returncode}")
        if check():
            return
        time.sleep(0.15)
    raise TimeoutError(label)


@contextmanager
def child(*args):
    process = subprocess.Popen(args)
    try:
        yield process
    finally:
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=8)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                raise


def method(path, name):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as conn:
        conn.settimeout(3)
        conn.connect(str(path))
        conn.sendall(json.dumps({"api_version": 1, "method": name}).encode() + b"\n")
        with conn.makefile("rb") as stream:
            response = json.loads(stream.readline(1_048_577))
        assert response["ok"] and response["api_version"] == 1
        return response["result"]


def main():
    if os.getuid() == 0:
        raise RuntimeError("Installed telemetry smoke tests require an ordinary user")
    with tempfile.TemporaryDirectory() as temp:
        root = Path(temp)
        hwmon = root / "hwmon"
        device = hwmon / "hwmon0"
        device.mkdir(parents=True)
        (device / "name").write_text("package_fixture\n")
        (device / "temp1_input").write_text("42000\n")
        os.environ.update(XDG_RUNTIME_DIR=temp, XDG_CONFIG_HOME=str(root / "config"), DISPLAY=":77",
                          SLINT_BACKEND="winit-femtovg", WINIT_UNIX_BACKEND="x11", LIBGL_ALWAYS_SOFTWARE="1")
        cli = json.loads(subprocess.check_output(["siliconwatch", "sensors", "--json", "--hwmon-root", hwmon]))
        sensors = cli["sensors"]
        assert len(sensors) == 1 and sensors[0]["raw_value"] == 42000 and sensors[0]["value"] == 42
        endpoint = root / "siliconwatch/api.sock"
        with child("siliconwatch", "daemon", "--hwmon-root", hwmon, "--local-api",
                   "--interval-ms", "100", "--stale-after-ms", "3000") as daemon:
            wait_for(endpoint.is_socket, daemon, "daemon endpoint")
            for file, mode in [(endpoint, 0o600), (endpoint.parent, 0o700)]:
                assert file.stat().st_uid == os.getuid() and stat.S_IMODE(file.stat().st_mode) == mode
            info = method(endpoint, "get_info")
            assert info["read_only"] and info["monitoring"]["interval_ms"] == 100
            def matches_fixture():
                observation = method(endpoint, "get_snapshot")["observation"]
                return observation is not None and observation["snapshot"] == cli
            wait_for(matches_fixture, daemon, "cached fixture measurement")
            with child("Xvfb", ":77", "-ac", "-screen", "0", "1280x800x24", "-nolisten", "tcp") as display:
                def windows():
                    return subprocess.run(["xwininfo", "-root", "-tree"], capture_output=True).stdout
                wait_for(windows, display, "X11 screen")
                with child("siliconwatch-gui") as gui:
                    wait_for(lambda: b"Sensors" in windows(), gui, "visible installed GUI")
            assert daemon.poll() is None
        assert daemon.returncode == 0 and not endpoint.exists()
    print("Installed CLI, daemon, read-only socket, desktop window and shutdown passed")


if __name__ == "__main__":
    main()
