#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt, symlink};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// Test deadlines detect hangs, not a product latency or shutdown guarantee.
const DEADLINE: Duration = Duration::from_secs(10);
const RESPONSE_LIMIT: usize = 1024 * 1024;
const STARTUP: &str = "SiliconWatch daemon running";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    base: PathBuf,
    runtime: PathBuf,
    hwmon: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // Keep the complete Unix pathname below Linux's sun_path limit.
        let base = PathBuf::from("/tmp").join(format!(
            "swapi-{}-{}-{nonce:x}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        private_dir(&base);
        let runtime = base.join("run");
        let hwmon = base.join("hwmon");
        private_dir(&runtime);
        private_dir(&hwmon);
        assert_ne!(
            fs::metadata(&base).unwrap().uid(),
            0,
            "same-UID API integration fixtures require a non-root test user"
        );
        Self {
            base,
            runtime,
            hwmon,
        }
    }

    fn socket(&self) -> PathBuf {
        self.runtime.join("siliconwatch/api.sock")
    }

    fn write(&self, name: &str, contents: &str) {
        let path = self.hwmon.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn command(&self, api: bool, interval: u64, stale: u64) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_siliconwatch"));
        command
            .args(["daemon", "--interval-ms"])
            .arg(interval.to_string())
            .arg("--stale-after-ms")
            .arg(stale.to_string())
            .arg("--hwmon-root")
            .arg(&self.hwmon)
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .stdin(Stdio::null());
        if api {
            command.arg("--local-api");
        }
        command
    }

    fn spawn(&self, command: &mut Command, label: &str) -> Running {
        let stdout = self.base.join(format!("{label}.stdout"));
        let stderr = self.base.join(format!("{label}.stderr"));
        let child = GuardedChild(
            command
                .stdout(Stdio::from(File::create(&stdout).unwrap()))
                .stderr(Stdio::from(File::create(&stderr).unwrap()))
                .spawn()
                .unwrap(),
        );
        Running {
            child,
            stdout,
            stderr,
        }
    }

    fn start(&self, interval: u64, stale: u64) -> Running {
        let mut running = self.spawn(&mut self.command(true, interval, stale), "daemon");
        running.ready();
        running
    }

    fn connect(&self) -> UnixStream {
        let deadline = Instant::now() + DEADLINE;
        loop {
            match UnixStream::connect(self.socket()) {
                Ok(stream) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(4)))
                        .unwrap();
                    stream
                        .set_write_timeout(Some(Duration::from_secs(4)))
                        .unwrap();
                    return stream;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                    ) =>
                {
                    assert!(
                        Instant::now() < deadline,
                        "API never became connectable: {error}"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("could not connect to fixture API: {error}"),
            }
        }
    }

    fn request(&self, bytes: &[u8]) -> Value {
        let mut stream = self.connect();
        stream.write_all(bytes).unwrap();
        complete_response(&mut stream).expect("server returned no complete response")
    }

    fn method(&self, method: &str) -> Value {
        let frame = format!("{{\"api_version\":1,\"method\":\"{method}\"}}\n");
        self.request(frame.as_bytes())
    }

    fn snapshot_when(&self, predicate: impl Fn(&Value) -> bool) -> Value {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let response = self.method("get_snapshot");
            if response["ok"] == true && predicate(&response["result"]) {
                return response;
            }
            if response["ok"] != true {
                assert_eq!(response["error"]["code"], "cache_busy", "{response}");
            }
            assert!(
                Instant::now() < deadline,
                "snapshot condition not reached: {response}"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn private_dir(path: &Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

struct GuardedChild(Child);

impl GuardedChild {
    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "fixture child did not terminate");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn signal(&mut self, signal: &str) {
        // Only this unreaped child can own this positive PID until the next wait.
        assert!(
            self.0.try_wait().unwrap().is_none(),
            "fixture child exited early"
        );
        assert!(
            Command::new("kill")
                .args(["-s", signal, "--", &self.0.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
    }
}

impl Drop for GuardedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}

struct Running {
    child: GuardedChild,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Running {
    fn ready(&mut self) {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let stderr = capture(&self.stderr);
            if stderr.lines().any(|line| line == STARTUP) {
                return;
            }
            assert!(
                self.child.0.try_wait().unwrap().is_none(),
                "startup failed: {stderr}"
            );
            assert!(
                Instant::now() < deadline,
                "no startup acknowledgement: {stderr}"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn stopped(&mut self) {
        let status = self.child.wait();
        assert_eq!(status.code(), Some(0), "{}", capture(&self.stderr));
        assert!(capture(&self.stdout).is_empty());
        assert!(
            capture(&self.stderr)
                .lines()
                .any(|line| line.starts_with("SiliconWatch daemon stopped;"))
        );
    }

    fn stop(&mut self) {
        self.child.signal("TERM");
        self.stopped();
    }
}

fn capture(path: &Path) -> String {
    assert!(fs::metadata(path).unwrap().len() <= 16 * 1024);
    fs::read_to_string(path).unwrap()
}

fn complete_response(stream: &mut UnixStream) -> Option<Value> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                assert!(bytes.len() <= RESPONSE_LIMIT, "response exceeded cap");
            }
            // Closing a request with unread extra input can produce a reset.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe
                ) =>
            {
                break;
            }
            Err(error) => panic!("response did not finish: {error}"),
        }
    }
    if bytes.is_empty() {
        return None;
    }
    assert_eq!(bytes.last(), Some(&b'\n'), "partial response is invalid");
    assert_eq!(
        bytes.iter().filter(|byte| **byte == b'\n').count(),
        1,
        "multiple response frames"
    );
    let response: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(response["api_version"], 1);
    let id = response["instance_id"].as_str().unwrap();
    assert_eq!(id.len(), 32);
    assert!(
        id.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
    assert_ne!(
        response.get("result").is_some(),
        response.get("error").is_some()
    );
    Some(response)
}

fn assert_error(response: &Value, code: &str) {
    assert_eq!(response["ok"], false, "{response}");
    assert_eq!(response["error"]["code"], code, "{response}");
    assert_eq!(
        response["error"]["retryable"],
        matches!(code, "cache_busy" | "request_timeout")
    );
}

#[test]
fn api_is_opt_in_and_duplicate_flag_is_rejected() {
    let f = Fixture::new();
    let mut command = f.command(false, 60_000, 3_000);
    command.env_remove("XDG_RUNTIME_DIR");
    let mut running = f.spawn(&mut command, "disabled");
    running.ready();
    assert!(!f.runtime.join("siliconwatch").exists());
    running.stop();

    let mut duplicate = f.command(true, 60_000, 3_000);
    duplicate.arg("--local-api");
    let mut running = f.spawn(&mut duplicate, "duplicate");
    assert_eq!(running.child.wait().code(), Some(2));
    assert!(!f.runtime.join("siliconwatch").exists());
}

#[test]
fn same_uid_methods_preserve_complete_fixture_snapshot_and_permissions() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture\n");
    f.write("hwmon0/temp1_input", "42000\n");
    f.write("hwmon0/temp1_label", "CPU fixture\n");
    f.write("hwmon0/temp2_input", "malformed\n");
    f.write("hwmon0/fan1_input", "9223372036854775807\n");
    f.write("hwmon0/power1_input", "123456\n");
    let expected = serde_json::to_value(siliconwatch::hwmon::scan(&f.hwmon).unwrap()).unwrap();
    let mut running = f.start(60_000, 60_000);
    let info = f.method("get_info");
    assert_eq!(info["ok"], true);
    assert_eq!(
        info["result"],
        json!({
            "supported_versions": [1], "methods": ["get_info", "get_snapshot"],
            "snapshot_schema_version": 2, "identity_schema_version": 1, "read_only": true,
            "limits": { "request_bytes": 4096, "response_bytes": 1048576,
                "active_clients": 1, "request_deadline_ms": 1000, "response_deadline_ms": 2000,
                "max_sensors": 4096, "max_warnings": 1024, "max_identity_issues_per_sensor": 64,
                "max_text_bytes": 16384, "max_total_text_bytes": 262144 }
        })
    );
    let response = f.snapshot_when(|value| value["latest"]["state"] == "succeeded");
    assert_eq!(response["instance_id"], info["instance_id"]);
    let result = &response["result"];
    assert_eq!(result["observation"]["snapshot"], expected);
    assert_eq!(result["latest"]["attempt"]["sequence"], "1");
    assert_eq!(result["observation"]["role"], "latest_attempt");
    assert_eq!(result["physical_age_known"], false);
    assert_eq!(result["service_current"], true);
    for key in ["sequence", "started_ns", "finished_ns", "published_ns"] {
        assert!(
            result["latest"]["attempt"][key]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .is_ok()
        );
    }
    assert!(
        result["assessed_at_ns"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .is_ok()
    );
    assert_eq!(result["stale_after_ns"], "60000000000");
    for (path, mode) in [(f.runtime.join("siliconwatch"), 0o700), (f.socket(), 0o600)] {
        let metadata = fs::symlink_metadata(path).unwrap();
        assert_eq!(metadata.mode() & 0o7777, mode);
        assert_eq!(metadata.uid(), fs::metadata(&f.runtime).unwrap().uid());
    }
    assert!(
        fs::symlink_metadata(f.socket())
            .unwrap()
            .file_type()
            .is_socket()
    );
    running.stop();
    assert!(!f.runtime.join("siliconwatch").exists());
}

#[test]
fn failed_acquisition_history_and_recovery_are_distinct_from_api_failure() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    f.write("hwmon0/temp1_input", "42000");
    let mut running = f.start(100, 10_000);
    let success = f.snapshot_when(|value| value["latest"]["state"] == "succeeded");
    let moved = f.base.join("saved-hwmon");
    fs::rename(&f.hwmon, &moved).unwrap();
    let failure = f.snapshot_when(|value| value["latest"]["state"] == "failed");
    assert_eq!(failure["ok"], true);
    assert_eq!(
        failure["result"]["observation"]["snapshot"],
        success["result"]["observation"]["snapshot"]
    );
    assert_eq!(
        failure["result"]["observation"]["role"],
        "historical_after_failure"
    );
    assert_eq!(failure["result"]["service_current"], false);
    assert!(failure["result"]["latest"]["error"]["message"].is_string());
    assert_ne!(
        failure["result"]["latest"]["attempt"]["sequence"],
        failure["result"]["observation"]["attempt"]["sequence"]
    );
    fs::rename(&moved, &f.hwmon).unwrap();
    let recovered = f.snapshot_when(|value| value["latest"]["state"] == "succeeded");
    assert_eq!(recovered["result"]["latest"]["error"], Value::Null);
    assert_eq!(recovered["result"]["observation"]["role"], "latest_attempt");
    running.stop();
}

#[test]
fn stale_empty_and_initial_failure_snapshots_are_not_current() {
    let f = Fixture::new();
    let mut empty = f.start(60_000, 1);
    let response = f.snapshot_when(|value| value["latest"]["state"] == "succeeded");
    assert_eq!(
        response["result"]["observation"]["freshness"]["state"],
        "no_sensor_observations"
    );
    assert_eq!(response["result"]["service_current"], false);
    empty.stop();

    f.write("hwmon0/name", "fixture");
    f.write("hwmon0/temp1_input", "42000");
    let mut stale = f.start(60_000, 1);
    let response = f.snapshot_when(|value| value["observation"]["freshness"]["state"] == "stale");
    assert_eq!(response["result"]["service_current"], false);
    assert_eq!(response["result"]["observation"]["role"], "latest_attempt");
    stale.stop();

    fs::remove_dir_all(&f.hwmon).unwrap();
    let mut failed = f.start(60_000, 1);
    let response = f.snapshot_when(|value| value["latest"]["state"] == "failed");
    assert_eq!(response["result"]["observation"], Value::Null);
    assert_eq!(response["result"]["service_current"], false);
    failed.stop();
}

#[test]
fn strict_requests_size_boundaries_and_first_frame_only() {
    let f = Fixture::new();
    let mut running = f.start(60_000, 3_000);
    for bytes in [
        b"{}\n".as_slice(),
        b"[]\n",
        b"not-json\n",
        b"\xff\n",
        b"{\"api_version\":1,\"method\":\"get_info\",\"extra\":0}\n",
        b"{\"api_version\":1,\"api_version\":1,\"method\":\"get_info\"}\n",
        b"{\"api_version\":1,\"method\":\"get_info\",\"method\":\"get_info\"}\n",
        b"{\"api_version\":1.0,\"method\":\"get_info\"}\n",
        b"{\"api_version\":1,\"method\":\"get_info\"} {}\n",
    ] {
        assert_error(&f.request(bytes), "invalid_request");
    }
    let version = f.request(b"{\"api_version\":99,\"method\":\"get_info\"}\n");
    assert_error(&version, "unsupported_version");
    assert_eq!(version["error"]["supported_versions"], json!([1]));
    assert_error(&f.method("refresh"), "unknown_method");
    let mut unterminated = f.connect();
    unterminated
        .write_all(b"{\"api_version\":1,\"method\":\"get_info\"}")
        .unwrap();
    unterminated.shutdown(Shutdown::Write).unwrap();
    assert_error(
        &complete_response(&mut unterminated).unwrap(),
        "invalid_request",
    );
    let request = b"{\"api_version\":1,\"method\":\"get_info\"}";
    for length in [4095, 4096] {
        let mut frame = request.to_vec();
        frame.resize(length - 1, b' ');
        frame.push(b'\n');
        assert_eq!(f.request(&frame)["ok"], true);
    }
    let mut oversized = request.to_vec();
    oversized.resize(4096, b' ');
    oversized.push(b'\n');
    assert_error(&f.request(&oversized), "request_too_large");
    let response = f.request(b"{\"api_version\":1,\"method\":\"get_info\"}\n{\"api_version\":1,\"method\":\"get_snapshot\"}\n");
    assert_eq!(response["result"]["read_only"], true);
    running.stop();
}

#[test]
fn idle_partial_clients_time_out_and_disconnections_do_not_accumulate_workers() {
    let f = Fixture::new();
    let mut running = f.start(60_000, 3_000);
    let mut slow = f.connect();
    slow.write_all(b"{").unwrap();
    let began = Instant::now();
    if let Some(response) = complete_response(&mut slow) {
        assert_error(&response, "request_timeout");
    }
    assert!(began.elapsed() < Duration::from_secs(4));
    for _ in 0..16 {
        let stream = f.connect();
        stream.shutdown(Shutdown::Both).unwrap();
    }
    assert_eq!(f.method("get_info")["ok"], true);
    let tasks = fs::read_dir(format!("/proc/{}/task", running.child.0.id()))
        .unwrap()
        .count();
    assert_eq!(
        tasks, 3,
        "daemon must retain only coordinator, acquisition and API worker"
    );
    running.stop();
}

#[test]
fn concurrent_clients_receive_serial_complete_responses() {
    let f = Fixture::new();
    let mut running = f.start(60_000, 3_000);
    let barrier = Arc::new(Barrier::new(9));
    let mut clients = Vec::new();
    for _ in 0..8 {
        let mut stream = f.connect();
        let barrier = Arc::clone(&barrier);
        clients.push(thread::spawn(move || {
            barrier.wait();
            stream
                .write_all(b"{\"api_version\":1,\"method\":\"get_info\"}\n")
                .unwrap();
            complete_response(&mut stream).unwrap()
        }));
    }
    barrier.wait();
    let responses: Vec<_> = clients
        .into_iter()
        .map(|client| client.join().unwrap())
        .collect();
    for response in &responses {
        assert_eq!(response["ok"], true);
        assert_eq!(response["instance_id"], responses[0]["instance_id"]);
    }
    assert_eq!(f.method("get_info")["ok"], true);
    running.stop();
}

#[test]
fn invalid_runtime_paths_fail_before_any_fifo_acquisition() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    let fifo = f.hwmon.join("hwmon0/temp1_input");
    make_fifo(&fifo);
    let mut absent = f.command(true, 60_000, 3_000);
    absent.env_remove("XDG_RUNTIME_DIR");
    let mut child = f.spawn(&mut absent, "absent");
    assert_eq!(child.child.wait().code(), Some(1));
    assert!(!capture(&child.stderr).contains(STARTUP));
    let linked = f.base.join("linked");
    symlink(&f.runtime, &linked).unwrap();
    let linked_parent = f.base.join("parent-link");
    symlink(&f.base, &linked_parent).unwrap();
    let shared = f.base.join("shared");
    private_dir(&shared);
    fs::set_permissions(&shared, fs::Permissions::from_mode(0o777)).unwrap();
    let shared_runtime = shared.join("run");
    private_dir(&shared_runtime);
    let long = f.base.join("x".repeat(70));
    private_dir(&long);
    for (i, path) in [
        PathBuf::from("relative"),
        f.base.join("missing"),
        linked,
        linked_parent.join("run"),
        shared_runtime,
        long,
    ]
    .iter()
    .enumerate()
    {
        let mut command = f.command(true, 60_000, 3_000);
        command.env("XDG_RUNTIME_DIR", path);
        let mut child = f.spawn(&mut command, &format!("invalid-{i}"));
        assert_eq!(
            child.child.wait().code(),
            Some(1),
            "{}",
            capture(&child.stderr)
        );
        assert!(!capture(&child.stderr).contains(STARTUP));
        assert!(!f.socket().exists());
    }
    fs::set_permissions(&f.runtime, fs::Permissions::from_mode(0o755)).unwrap();
    let mut child = f.spawn(&mut f.command(true, 60_000, 3_000), "unsafe-mode");
    assert_eq!(child.child.wait().code(), Some(1));
    assert_eq!(fs::metadata(&f.runtime).unwrap().mode() & 0o777, 0o755);
    assert!(fs::symlink_metadata(fifo).unwrap().file_type().is_fifo());
}

#[test]
fn existing_namespace_objects_and_second_instance_are_never_taken_over() {
    for kind in ["directory", "file", "symlink", "stale-socket"] {
        let f = Fixture::new();
        let namespace = f.runtime.join("siliconwatch");
        match kind {
            "directory" => private_dir(&namespace),
            "file" => fs::write(&namespace, "preserve").unwrap(),
            "symlink" => symlink(&f.hwmon, &namespace).unwrap(),
            "stale-socket" => {
                private_dir(&namespace);
                drop(UnixListener::bind(f.socket()).unwrap());
            }
            _ => unreachable!(),
        }
        let before = fs::symlink_metadata(&namespace).unwrap();
        let mut running = f.spawn(&mut f.command(true, 60_000, 3_000), "existing");
        assert_eq!(running.child.wait().code(), Some(1));
        let after = fs::symlink_metadata(&namespace).unwrap();
        assert_eq!(
            (before.dev(), before.ino(), before.mode()),
            (after.dev(), after.ino(), after.mode())
        );
        if kind == "stale-socket" {
            assert!(
                fs::symlink_metadata(f.socket())
                    .unwrap()
                    .file_type()
                    .is_socket()
            );
        }
    }
    let f = Fixture::new();
    let mut first = f.start(60_000, 3_000);
    let identity = fs::symlink_metadata(f.socket()).unwrap();
    let mut second = f.spawn(&mut f.command(true, 60_000, 3_000), "second");
    assert_eq!(second.child.wait().code(), Some(1));
    assert_eq!(
        fs::symlink_metadata(f.socket()).unwrap().ino(),
        identity.ino()
    );
    assert_eq!(f.method("get_info")["ok"], true);
    first.stop();
}

#[test]
fn shutdown_does_not_remove_a_replaced_socket_or_namespace() {
    let f = Fixture::new();
    let mut running = f.start(60_000, 3_000);
    fs::remove_file(f.socket()).unwrap();
    fs::write(f.socket(), "replacement must survive").unwrap();
    let before = fs::metadata(f.socket()).unwrap();
    running.child.signal("TERM");
    assert_eq!(running.child.wait().code(), Some(1));
    assert_eq!(
        fs::read_to_string(f.socket()).unwrap(),
        "replacement must survive"
    );
    assert_eq!(fs::metadata(f.socket()).unwrap().ino(), before.ino());

    let f = Fixture::new();
    let mut running = f.start(60_000, 3_000);
    let namespace = f.runtime.join("siliconwatch");
    let moved = f.runtime.join("original");
    fs::rename(&namespace, &moved).unwrap();
    private_dir(&namespace);
    fs::write(namespace.join("sentinel"), "replacement directory").unwrap();
    running.child.signal("TERM");
    assert_eq!(running.child.wait().code(), Some(1));
    assert_eq!(
        fs::read_to_string(namespace.join("sentinel")).unwrap(),
        "replacement directory"
    );
    assert!(moved.exists());
}

fn make_fifo(path: &Path) {
    assert!(Command::new("mkfifo").arg(path).status().unwrap().success());
}

fn holding_writer(f: &Fixture, fifo: &Path) -> GuardedChild {
    let ready = f.base.join("writer-ready");
    let mut child = GuardedChild(Command::new("sh")
        .args(["-c", "exec 3>\"$1\"; printf 'ready\\n'; IFS= read -r release || exit 3; printf '42000\\n' >&3; exec 3>&-", "api-fixture-writer"])
        .arg(fifo)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(File::create(&ready).unwrap()))
        .stderr(Stdio::null())
        .spawn().unwrap());
    let deadline = Instant::now() + DEADLINE;
    while capture(&ready) != "ready\n" {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "FIFO writer exited early"
        );
        assert!(
            Instant::now() < deadline,
            "acquisition never opened fixture FIFO"
        );
        thread::sleep(Duration::from_millis(5));
    }
    child
}

fn release_writer(writer: &mut GuardedChild) {
    writer
        .0
        .stdin
        .take()
        .unwrap()
        .write_all(b"release\n")
        .unwrap();
    assert_eq!(writer.wait().code(), Some(0));
}

#[test]
fn api_reads_never_start_another_acquisition() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    let fifo = f.hwmon.join("hwmon0/temp1_input");
    make_fifo(&fifo);
    let mut running = f.start(60_000, 60_000);
    let mut writer = holding_writer(&f, &fifo);
    for _ in 0..8 {
        assert_eq!(f.method("get_info")["ok"], true);
        let response = f.snapshot_when(|_| true);
        assert_eq!(response["result"]["latest"]["state"], "empty");
        assert_eq!(response["result"]["observation"], Value::Null);
        assert_eq!(response["result"]["service_current"], false);
    }
    release_writer(&mut writer);
    let initial = f.snapshot_when(|value| value["latest"]["state"] == "succeeded");
    fs::remove_file(&fifo).unwrap();
    fs::write(&fifo, "99000").unwrap();
    for _ in 0..8 {
        let response = f.snapshot_when(|_| true);
        assert_eq!(
            response["result"]["latest"]["attempt"],
            initial["result"]["latest"]["attempt"]
        );
        assert_eq!(
            response["result"]["observation"]["snapshot"],
            initial["result"]["observation"]["snapshot"]
        );
    }
    running.stop();
}

#[test]
fn signals_cancel_api_io_and_remove_endpoint_before_blocked_acquisition_finishes() {
    for signal in ["INT", "TERM"] {
        let f = Fixture::new();
        f.write("hwmon0/name", "fixture");
        let fifo = f.hwmon.join("hwmon0/temp1_input");
        make_fifo(&fifo);
        let mut running = f.start(60_000, 3_000);
        let mut writer = holding_writer(&f, &fifo);
        let before_client = fs::read_dir(format!("/proc/{}/fd", running.child.0.id()))
            .unwrap()
            .count();
        let mut client = f.connect();
        client.write_all(b"{").unwrap();
        // The FIFO handshake has stopped acquisition with its files open. Two
        // extra server descriptors acknowledge the accepted stream and its
        // cancellation clone; the signal therefore races active API handling.
        let accepted_deadline = Instant::now() + DEADLINE;
        while fs::read_dir(format!("/proc/{}/fd", running.child.0.id()))
            .unwrap()
            .count()
            < before_client + 2
        {
            assert!(running.child.0.try_wait().unwrap().is_none());
            assert!(
                Instant::now() < accepted_deadline,
                "API did not register active stream"
            );
            thread::sleep(Duration::from_millis(5));
        }
        running.child.signal(signal);
        let deadline = Instant::now() + DEADLINE;
        while f.runtime.join("siliconwatch").exists() {
            assert!(running.child.0.try_wait().unwrap().is_none());
            assert!(
                Instant::now() < deadline,
                "API endpoint was held behind hardware join"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(running.child.0.try_wait().unwrap().is_none());
        assert!(complete_response(&mut client).is_none());
        release_writer(&mut writer);
        running.stopped();
    }
}

#[test]
fn failed_startup_reporting_cleans_api_endpoint_without_waiting_for_fifo() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    make_fifo(&f.hwmon.join("hwmon0/temp1_input"));
    let mut child = GuardedChild(
        f.command(true, 60_000, 3_000)
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                OpenOptions::new().write(true).open("/dev/full").unwrap(),
            ))
            .spawn()
            .unwrap(),
    );
    assert_eq!(child.wait().code(), Some(1));
    assert!(!f.runtime.join("siliconwatch").exists());
}

#[test]
fn oversized_encoded_snapshot_is_rejected_whole_and_service_remains_available() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    // Each label is within the text bound and the aggregate remains below
    // 256KiB, but JSON escaping expands these controls past the 1MiB wire cap.
    let label = "\u{1}".repeat(16_384);
    for index in 1..=14 {
        f.write(&format!("hwmon0/temp{index}_input"), "42000");
        f.write(&format!("hwmon0/temp{index}_label"), &label);
    }
    let snapshot = siliconwatch::hwmon::scan(&f.hwmon).unwrap();
    assert!(serde_json::to_vec(&snapshot).unwrap().len() > RESPONSE_LIMIT);
    let mut running = f.start(60_000, 3_000);
    let deadline = Instant::now() + DEADLINE;
    loop {
        let response = f.method("get_snapshot");
        if response["ok"] == false && response["error"]["code"] != "cache_busy" {
            assert_error(&response, "response_too_large");
            break;
        }
        if response["ok"] == true {
            assert_eq!(response["result"]["latest"]["state"], "empty");
        }
        assert!(
            Instant::now() < deadline,
            "oversized cache was not published"
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(f.method("get_info")["ok"], true);
    running.stop();
}
