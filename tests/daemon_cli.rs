#![cfg(target_os = "linux")]

mod common;

use common::Fixture;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::OwnedFd;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// Deadlines guard against regressions hanging CI; they are not daemon latency guarantees.
const DEADLINE: Duration = Duration::from_secs(10);
const CAPTURE_LIMIT: u64 = 16 * 1024;
const STARTUP: &str = "SiliconWatch daemon running";

struct GuardedChild(Child);

impl GuardedChild {
    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + DEADLINE;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "child failed to terminate");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn signal(&mut self, signal: &str) {
        // Do not signal a reaped PID. If it exits after this check, it stays unreaped
        // until our next wait, so this positive PID cannot be reused in this window.
        assert!(self.0.try_wait().unwrap().is_none(), "child exited early");
        let status = Command::new("kill")
            .args(["-s", signal, "--", &self.0.id().to_string()])
            .status()
            .unwrap();
        assert!(status.success(), "could not send {signal} to fixture child");
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

fn capture(path: &Path) -> Stdio {
    Stdio::from(File::create(path).unwrap())
}

fn text(path: &Path) -> String {
    assert!(fs::metadata(path).unwrap().len() <= CAPTURE_LIMIT);
    fs::read_to_string(path).unwrap()
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_siliconwatch"))
}

fn daemon(root: &Path) -> Command {
    let mut command = cli();
    command
        .args([
            "daemon",
            "--interval-ms",
            "60000",
            "--stale-after-ms",
            "1",
            "--hwmon-root",
        ])
        .arg(root);
    command
}

struct RunningDaemon {
    child: GuardedChild,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl RunningDaemon {
    fn start(fixture: &Fixture, root: &Path) -> Self {
        let stdout = fixture.root.join("daemon-stdout");
        let stderr = fixture.root.join("daemon-stderr");
        let child = GuardedChild(
            daemon(root)
                .stdin(Stdio::null())
                .stdout(capture(&stdout))
                .stderr(capture(&stderr))
                .spawn()
                .unwrap(),
        );
        let mut running = Self {
            child,
            stdout,
            stderr,
        };
        let deadline = Instant::now() + DEADLINE;
        loop {
            let output = text(&running.stderr);
            if output.lines().any(|line| line == STARTUP) {
                return running;
            }
            assert!(
                running.child.0.try_wait().unwrap().is_none(),
                "daemon exited before readiness: {output}"
            );
            assert!(Instant::now() < deadline, "no startup acknowledgement");
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn assert_graceful(&mut self) {
        assert_eq!(self.child.wait().code(), Some(0));
        assert!(text(&self.stdout).is_empty());
        let output = text(&self.stderr);
        let lines: Vec<_> = output.lines().collect();
        assert_eq!(lines.len(), 2, "unexpected lifecycle output: {output}");
        assert_eq!(lines[0], STARTUP);
        assert!(lines[1].starts_with("SiliconWatch daemon stopped;"));
    }
}

#[test]
fn daemon_argument_errors_are_bounded_and_use_exit_two() {
    let f = Fixture::new();
    let mut invalid = vec![
        vec!["daemon"],
        vec!["daemon", "--interval-ms", "100"],
        vec!["daemon", "--stale-after-ms", "100"],
        vec!["daemon", "--interval-ms"],
        vec!["daemon", "--stale-after-ms"],
    ];
    for suffix in [
        vec!["--count", "1"],
        vec!["--json"],
        vec!["--json-lines"],
        vec!["--unknown"],
        vec!["--interval-ms", "200"],
        vec!["--stale-after-ms", "200"],
        vec!["--hwmon-root", "duplicate"],
        vec!["--hwmon-root", ""],
        vec!["--hwmon-root"],
    ] {
        let mut args = vec!["daemon", "--interval-ms", "100", "--stale-after-ms", "100"];
        args.extend(suffix);
        invalid.push(args);
    }
    for value in [
        "0",
        "99",
        "60001",
        "-100",
        "+100",
        "100.5",
        "",
        "1s",
        "１００",
        "18446744073709551616",
    ] {
        invalid.push(vec![
            "daemon",
            "--interval-ms",
            value,
            "--stale-after-ms",
            "100",
        ]);
    }
    for value in ["0", "-1", "+1", "1.5", "", "1s", "18446744073709551616"] {
        invalid.push(vec![
            "daemon",
            "--interval-ms",
            "100",
            "--stale-after-ms",
            value,
        ]);
    }
    for args in invalid {
        let stdout = f.root.join("invalid-stdout");
        let stderr = f.root.join("invalid-stderr");
        let mut child = GuardedChild(
            cli()
                .args(&args)
                .arg("--hwmon-root")
                .arg(&f.root)
                .stdin(Stdio::null())
                .stdout(capture(&stdout))
                .stderr(capture(&stderr))
                .spawn()
                .unwrap(),
        );
        assert_eq!(child.wait().code(), Some(2), "{args:?}");
        assert!(text(&stdout).is_empty());
        assert!(!text(&stderr).contains(STARTUP));
    }
}

#[test]
fn sigint_and_sigterm_stop_separate_fixture_processes_without_stdout() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    f.write("hwmon0/temp1_input", "42500");
    f.write("hwmon0/temp2_input", "malformed");
    for signal in ["INT", "TERM"] {
        let mut running = RunningDaemon::start(&f, &f.root);
        running.child.signal(signal);
        running.assert_graceful();
    }
    assert_eq!(
        fs::read_to_string(f.root.join("hwmon0/temp1_input")).unwrap(),
        "42500"
    );
    assert_eq!(
        fs::read_to_string(f.root.join("hwmon0/temp2_input")).unwrap(),
        "malformed"
    );
}

#[test]
fn missing_root_remains_running_until_requested_shutdown() {
    let f = Fixture::new();
    let mut running = RunningDaemon::start(&f, &f.root.join("missing"));
    running.child.signal("TERM");
    running.assert_graceful();
}

#[test]
fn repeated_signals_allow_an_admitted_fifo_read_to_finish_before_exit() {
    let f = Fixture::new();
    f.write("hwmon0/name", "fixture");
    let fifo = f.root.join("hwmon0/temp1_input");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let mut running = RunningDaemon::start(&f, &f.root);

    // The writer acknowledges that the scanner opened the FIFO, then keeps it
    // open until released. No timing assumption establishes scan admission.
    let writer_output = f.root.join("writer-ready");
    let mut writer = GuardedChild(
        Command::new("sh")
            .args([
                "-c",
                "exec 3>\"$1\"; printf 'writer-open\\n'; IFS= read -r release || exit 3; printf '42500\\n' >&3; exec 3>&-",
                "siliconwatch-fixture-writer",
            ])
            .arg(&fifo)
            .stdin(Stdio::piped())
            .stdout(capture(&writer_output))
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + DEADLINE;
    while text(&writer_output) != "writer-open\n" {
        assert!(
            writer.0.try_wait().unwrap().is_none(),
            "FIFO writer exited early"
        );
        assert!(running.child.0.try_wait().unwrap().is_none());
        assert!(
            Instant::now() < deadline,
            "scanner never opened fixture FIFO"
        );
        thread::sleep(Duration::from_millis(5));
    }
    for signal in ["INT", "TERM", "INT"] {
        running.child.signal(signal);
    }
    assert!(running.child.0.try_wait().unwrap().is_none());
    assert_eq!(text(&running.stderr).lines().count(), 1);
    let mut release = writer.0.stdin.take().unwrap();
    release.write_all(b"release\n").unwrap();
    drop(release);
    assert_eq!(writer.wait().code(), Some(0));
    running.assert_graceful();
    assert!(fs::symlink_metadata(fifo).unwrap().file_type().is_fifo());
}

#[test]
fn daemon_startup_write_failure_to_dev_full_returns_one() {
    let f = Fixture::new();
    let mut child = GuardedChild(
        daemon(&f.root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                OpenOptions::new().write(true).open("/dev/full").unwrap(),
            ))
            .spawn()
            .unwrap(),
    );
    assert_eq!(child.wait().code(), Some(1));
}

#[test]
fn daemon_broken_stderr_pipe_is_a_failure_not_one_shot_success() {
    let f = Fixture::new();
    let (writer, reader) = UnixStream::pair().unwrap();
    drop(reader);
    let fd: OwnedFd = writer.into();
    let mut child = GuardedChild(
        daemon(&f.root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(fd))
            .spawn()
            .unwrap(),
    );
    assert_eq!(child.wait().code(), Some(1));
}

#[test]
fn invalid_daemon_arguments_with_failed_stderr_do_not_panic() {
    let mut child = GuardedChild(
        cli()
            .arg("daemon")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                OpenOptions::new().write(true).open("/dev/full").unwrap(),
            ))
            .spawn()
            .unwrap(),
    );
    assert_eq!(child.wait().code(), Some(1));
}
