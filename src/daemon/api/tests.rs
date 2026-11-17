use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

const WAIT: Duration = Duration::from_secs(5);
const INSTANCE: &str = "0123456789abcdef0123456789abcdef";

fn cache() -> (CacheOwner, CacheReader) {
    // Construction and these read-only tests perform no filesystem acquisition.
    snapshot_cache::new("never-opened-api-fixture", Duration::from_secs(3)).unwrap()
}

fn api_worker(control: Arc<Control>) -> (ApiWorker, mpsc::Receiver<Phase>) {
    let stop = Arc::new(local_api::Stop::new());
    let thread_stop = Arc::clone(&stop);
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + WAIT;
        while !thread_stop.flag().load(Ordering::Acquire) {
            assert!(Instant::now() < deadline, "coordinator did not cancel API");
            thread::yield_now();
        }
        tx.send(control.lock().unwrap().phase).unwrap();
        Ok(())
    });
    (ApiWorker { stop, handle }, rx)
}

#[test]
fn api_options_are_explicit_and_disabled_by_default() {
    let options = Options::new(100, 300).unwrap();
    assert!(!options.local_api);
    assert!(options.with_local_api(true).local_api);
    assert!(!options.with_local_api(true).with_local_api(false).local_api);
}

#[test]
fn repeated_service_reads_do_not_create_acquisition_attempts() {
    let (_owner, reader) = cache();
    let control = Control::new();
    control.start().unwrap();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(Instant::now() + WAIT, &cancelled);
    for _ in 0..16 {
        let bytes = snapshot(&reader, &control, INSTANCE, Duration::from_secs(3), &budget).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["result"]["latest"]["state"], "empty");
        assert_eq!(value["result"]["observation"], serde_json::Value::Null);
        assert_eq!(value["result"]["service_current"], false);
        assert!(
            control.state.try_lock().is_ok(),
            "no lifecycle guard escaped"
        );
    }
    assert!(matches!(
        reader.read().unwrap().assess().unwrap().latest,
        snapshot_cache::LatestAttempt::Empty
    ));
}

#[test]
fn lifecycle_contention_is_busy_and_poison_is_unavailable() {
    let (_owner, reader) = cache();
    let control = Control::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(Instant::now() + WAIT, &cancelled);
    let held = control.lock().unwrap();
    assert!(matches!(
        snapshot(&reader, &control, INSTANCE, Duration::from_secs(3), &budget),
        Err(ApiError::CacheBusy)
    ));
    drop(held);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _held = control.lock().unwrap();
        panic!("injected lifecycle poison");
    }));
    assert!(matches!(
        snapshot(&reader, &control, INSTANCE, Duration::from_secs(3), &budget),
        Err(ApiError::ServiceUnavailable)
    ));
}

#[test]
fn cancelled_or_expired_service_reads_return_no_snapshot() {
    let (_owner, reader) = cache();
    let control = Control::new();
    let cancelled = AtomicBool::new(true);
    let budget = Budget::new(Instant::now() + WAIT, &cancelled);
    assert!(matches!(
        snapshot(&reader, &control, INSTANCE, Duration::from_secs(3), &budget),
        Err(ApiError::Cancelled)
    ));
    cancelled.store(false, Ordering::Release);
    let budget = Budget::new(Instant::now(), &cancelled);
    assert!(matches!(
        snapshot(&reader, &control, INSTANCE, Duration::from_secs(3), &budget),
        Err(ApiError::Deadline)
    ));
}

#[test]
fn acquisition_spawn_failure_joins_api_before_stopped_without_waiting() {
    let (owner, reader) = cache();
    drop(owner); // A failed spawn drops its acquisition closure/owner.
    let control = Arc::new(Control::new());
    let (api, exit_phase) = api_worker(Arc::clone(&control));
    let closed = AtomicBool::new(false);
    let mut output = Vec::new();
    let error = coordinate(
        &control,
        &reader,
        api,
        Err(io::Error::other("injected acquisition spawn failure")),
        || panic!("must not enter signal wait after spawn failure"),
        || closed.store(true, Ordering::Release),
        &mut output,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("injected acquisition spawn failure")
    );
    assert_eq!(exit_phase.recv_timeout(WAIT).unwrap(), Phase::Stopping);
    assert!(closed.load(Ordering::Acquire));
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("outcome: failed")
    );
}

#[test]
fn api_is_joined_before_waiting_for_an_admitted_acquisition() {
    let (owner, reader) = cache();
    let control = Arc::new(Control::new());
    control.start().unwrap();
    let (api, api_exited) = api_worker(Arc::clone(&control));
    let (acquired, released) = mpsc::channel();
    let acquisition = thread::spawn(move || {
        assert_eq!(api_exited.recv_timeout(WAIT).unwrap(), Phase::Stopping);
        drop(owner);
        acquired.send(()).unwrap();
        Ok(())
    });
    let mut output = Vec::new();
    coordinate(
        &control,
        &reader,
        api,
        Ok(acquisition),
        || {},
        || {},
        &mut output,
    )
    .unwrap();
    released.recv_timeout(WAIT).unwrap();
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("outcome: graceful")
    );
}

#[test]
fn api_and_wait_panics_still_join_acquisition_and_report_failure() {
    let (owner, reader) = cache();
    let control = Arc::new(Control::new());
    let stop = Arc::new(local_api::Stop::new());
    let api = ApiWorker {
        stop,
        handle: thread::spawn(|| -> io::Result<()> { panic!("injected API panic") }),
    };
    let (done, completed) = mpsc::channel();
    let acquisition = thread::spawn(move || {
        drop(owner);
        done.send(()).unwrap();
        Ok(())
    });
    let mut output = Vec::new();
    let error = coordinate(
        &control,
        &reader,
        api,
        Ok(acquisition),
        || panic!("injected coordinator wait panic"),
        || {},
        &mut output,
    )
    .unwrap_err();
    completed.recv_timeout(WAIT).unwrap();
    assert!(error.to_string().contains("signal wait panicked"));
    assert!(error.to_string().contains("API worker panicked"));
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("outcome: failed")
    );
}

#[test]
fn guarded_api_exit_wakes_real_signal_wait_in_isolated_process() {
    use std::fs;
    use std::io::Read;
    use std::os::unix::fs::DirBuilderExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    const CHILD: &str = "SILICONWATCH_API_EXIT_CHILD";
    fn child_case(mode: &'static str) {
        struct Runtime(PathBuf);
        impl Drop for Runtime {
            fn drop(&mut self) {
                // Only this exclusively created fixture tree is removed.
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let runtime = Runtime(PathBuf::from(format!(
            "/tmp/sw-api-exit-{}-{mode}",
            std::process::id()
        )));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&runtime.0)
            .unwrap();
        let endpoint = runtime.0.join("siliconwatch");
        let socket = endpoint.join("api.sock");
        let prepared = local_api::Prepared::new(&runtime.0).unwrap();
        let (owner, reader) = cache();
        let control = Arc::new(Control::new());
        let stop = Arc::new(local_api::Stop::new());
        let mut signals = Signals::new([SIGINT, SIGTERM]).unwrap();
        let close = signals.handle();
        let api_control = Arc::clone(&control);
        let api_stop = Arc::clone(&stop);
        let api_close = close.clone();
        let api_handle = thread::spawn(move || {
            guarded_api_worker(
                &api_control,
                || api_close.close(),
                || {
                    prepared.run(api_stop, |_, _| match mode {
                        "fatal" => Err(ApiError::ServiceUnavailable),
                        "panic" => panic!("injected API callback panic"),
                        _ => panic!("normal exit must not need a request"),
                    })
                },
            )
        });

        let (trigger, triggered) = mpsc::channel();
        let trigger_stop = Arc::clone(&stop);
        let client = thread::spawn(move || {
            triggered.recv_timeout(WAIT).unwrap();
            if mode == "normal" {
                trigger_stop.request().unwrap();
                return;
            }
            let mut stream = UnixStream::connect(socket).unwrap();
            stream.set_read_timeout(Some(WAIT)).unwrap();
            stream
                .write_all(b"{\"api_version\":1,\"method\":\"get_snapshot\"}\n")
                .unwrap();
            let mut response = Vec::new();
            stream.read_to_end(&mut response).unwrap();
            if mode == "fatal" {
                let value: serde_json::Value = serde_json::from_slice(&response).unwrap();
                assert_eq!(value["error"]["code"], "service_unavailable");
            } else {
                assert!(response.is_empty());
            }
        });

        let (release, released) = mpsc::channel();
        let (joined, joined_observed) = mpsc::channel();
        let acquisition_control = Arc::clone(&control);
        let acquisition_endpoint = endpoint.clone();
        let acquisition = thread::spawn(move || {
            // This worker cannot wake the signal iterator. It stays alive
            // until the production API guard alone has closed that wait.
            released.recv_timeout(WAIT).unwrap();
            assert!(!acquisition_endpoint.exists());
            assert_eq!(acquisition_control.lock().unwrap().phase, Phase::Stopping);
            drop(owner);
            joined.send(()).unwrap();
            Ok(())
        });
        let mut output = Vec::new();
        let result = coordinate(
            &control,
            &reader,
            ApiWorker {
                stop,
                handle: api_handle,
            },
            Ok(acquisition),
            || {
                trigger.send(()).unwrap();
                // No signal is sent and this closure does not inject an
                // error. Removing the production guard makes the child hang.
                assert_eq!(signals.forever().next(), None);
                release.send(()).unwrap();
            },
            || close.close(),
            &mut output,
        );
        client.join().unwrap();
        joined_observed.recv_timeout(WAIT).unwrap();
        assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
        assert_eq!(
            reader.read().unwrap().assess().unwrap().producer,
            snapshot_cache::ProducerState::Stopped
        );
        assert!(!endpoint.exists());
        let output = String::from_utf8(output).unwrap();
        match mode {
            "normal" => {
                result.unwrap();
                assert!(output.contains("outcome: graceful"));
            }
            "fatal" => {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("service state is unavailable")
                );
                assert!(output.contains("outcome: failed"));
            }
            "panic" => {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("API worker panicked")
                );
                assert!(output.contains("outcome: failed"));
            }
            _ => unreachable!(),
        }
    }

    if let Some(mode) = std::env::var_os(CHILD) {
        child_case(match mode.to_str().unwrap() {
            "normal" => "normal",
            "fatal" => "fatal",
            "panic" => "panic",
            _ => panic!("invalid isolated API test case"),
        });
        return;
    }
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    for mode in ["normal", "fatal", "panic"] {
        let mut child = Child(
            Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "daemon::api::tests::guarded_api_exit_wakes_real_signal_wait_in_isolated_process",
                    "--nocapture",
                ])
                .env(CHILD, mode)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + WAIT + WAIT;
        loop {
            if let Some(status) = child.0.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "isolated API {mode} case failed: {status}"
                );
                break;
            }
            assert!(Instant::now() < deadline, "isolated API {mode} case hung");
            // Polling is only the parent's deadlock guard. Child coordination
            // uses channels; no timing sleep causes the tested worker exits.
            thread::sleep(Duration::from_millis(5));
        }
    }
}
