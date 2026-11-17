use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

struct Fixture {
    base: std::path::PathBuf,
    root: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!(
            "siliconwatch-daemon-test-{}-{}-{nanos}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        let root = base.join("hwmon");
        fs::create_dir(&root).unwrap();
        Self { base, root }
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

const TEST_DEADLINE: Duration = Duration::from_secs(10);

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

#[derive(Default)]
struct FakeTiming {
    time: Rc<Cell<Duration>>,
    observations: VecDeque<io::Result<Duration>>,
    wake_after: VecDeque<Duration>,
    waits: Vec<Duration>,
    fail_wait: bool,
}

impl Timing for FakeTiming {
    fn now(&mut self) -> io::Result<Duration> {
        if let Some(value) = self.observations.pop_front() {
            self.time.set(value?);
        }
        Ok(self.time.get())
    }

    fn wait<'a>(
        &mut self,
        _control: &Control,
        state: MutexGuard<'a, State>,
        remaining: Duration,
    ) -> io::Result<MutexGuard<'a, State>> {
        assert_eq!(state.phase, Phase::Running);
        assert!(!state.admitted);
        self.waits.push(remaining);
        if self.fail_wait {
            return Err(io::Error::other("injected wait failure"));
        }
        self.time
            .set(self.time.get() + self.wake_after.pop_front().unwrap_or(remaining));
        Ok(state)
    }
}

fn cache(fixture: &Fixture) -> (CacheOwner, CacheReader) {
    snapshot_cache::new(&fixture.root, Duration::from_secs(3600)).unwrap()
}

fn assert_inactive(reader: &CacheReader) {
    let mut view = reader.read().unwrap();
    assert_eq!(
        view.assess().unwrap().producer,
        snapshot_cache::ProducerState::Stopped
    );
}

#[test]
fn option_bounds_and_lifecycle_transitions_are_explicit() {
    for interval in [100, 60_000] {
        for stale_after in [1, u64::MAX] {
            let options = Options::new(interval, stale_after).unwrap();
            assert_eq!(options.interval, ms(interval));
            assert_eq!(options.stale_after, ms(stale_after));
        }
    }
    for interval in [0, 99, 60_001, u64::MAX] {
        assert!(Options::new(interval, 1).is_err());
    }
    assert!(Options::new(100, 0).is_err());
    let control = Control::new();
    assert!(control.stopped().is_err());
    assert!(control.start().unwrap());
    assert!(control.start().is_err());
    control.stop().unwrap();
    control.stop().unwrap();
    assert!(!control.start().unwrap());
    control.stopped().unwrap();
    control.stop().unwrap();
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
}

#[test]
fn stop_before_start_prevents_announcement_and_acquisition() {
    let control = Control::new();
    control.stop().unwrap();
    acquire_loop(
        &control,
        ms(100),
        &mut FakeTiming::default(),
        || panic!("acquisition after stop"),
        || panic!("startup notice after stop"),
    )
    .unwrap();
    assert!(!control.lock().unwrap().admitted);
}

#[test]
fn announcement_precedes_sequential_failure_and_recovery_at_normal_interval() {
    let control = Control::new();
    let mut timing = FakeTiming::default();
    let time = Rc::clone(&timing.time);
    let events = RefCell::new(Vec::new());
    let mut starts = Vec::new();
    let mut active = false;
    acquire_loop(
        &control,
        ms(100),
        &mut timing,
        || {
            assert!(!active);
            active = true;
            let state = control
                .state
                .try_lock()
                .expect("control lock held during scan");
            assert!(state.admitted);
            assert_eq!(state.phase, Phase::Running);
            drop(state);
            events.borrow_mut().push("start");
            starts.push(time.get());
            time.set(time.get() + ms(20));
            events.borrow_mut().push("finish");
            active = false;
            let sequence = starts.len() as u64;
            if sequence == 3 {
                control.stop().unwrap();
            }
            Ok(if sequence == 2 {
                Acquisition::Failed(sequence)
            } else {
                Acquisition::Succeeded(sequence)
            })
        },
        || {
            let state = control
                .state
                .try_lock()
                .expect("control lock held during output");
            assert_eq!(state.phase, Phase::Running);
            assert!(!state.admitted);
            events.borrow_mut().push("announce");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(starts, [ms(0), ms(100), ms(200)]);
    assert_eq!(timing.waits, [ms(80), ms(80)]);
    assert_eq!(
        *events.borrow(),
        [
            "announce", "start", "finish", "start", "finish", "start", "finish"
        ]
    );
    assert!(!control.lock().unwrap().admitted);
}

#[test]
fn slow_acquisition_reanchors_without_catchup_bursts() {
    let control = Control::new();
    let mut timing = FakeTiming::default();
    let time = Rc::clone(&timing.time);
    let mut starts = Vec::new();
    acquire_loop(
        &control,
        ms(100),
        &mut timing,
        || {
            starts.push(time.get());
            time.set(time.get() + if starts.len() == 1 { ms(250) } else { ms(20) });
            if starts.len() == 3 {
                control.stop().unwrap();
            }
            Ok(Acquisition::Succeeded(starts.len() as u64))
        },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(starts, [ms(0), ms(250), ms(350)]);
    assert_eq!(timing.waits, [ms(80)]);
}

#[test]
fn spurious_early_and_delayed_wakes_recheck_remaining_time() {
    let control = Control::new();
    let mut timing = FakeTiming {
        wake_after: VecDeque::from([ms(0), ms(50), ms(70)]),
        ..FakeTiming::default()
    };
    let time = Rc::clone(&timing.time);
    let mut starts = Vec::new();
    acquire_loop(
        &control,
        ms(100),
        &mut timing,
        || {
            starts.push(time.get());
            if starts.len() == 3 {
                control.stop().unwrap();
            }
            Ok(Acquisition::Succeeded(starts.len() as u64))
        },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(starts, [ms(0), ms(120), ms(220)]);
    assert_eq!(timing.waits, [ms(100), ms(100), ms(50), ms(100)]);
}

#[test]
fn stop_between_admission_and_dispatch_permits_only_that_attempt() {
    struct StopAtDispatch<'a> {
        control: &'a Control,
        observations: usize,
    }
    impl Timing for StopAtDispatch<'_> {
        fn now(&mut self) -> io::Result<Duration> {
            self.observations += 1;
            if self.observations == 2 {
                assert!(self.control.lock()?.admitted);
                self.control.stop()?;
            }
            Ok(Duration::ZERO)
        }
        fn wait<'a>(
            &mut self,
            _: &Control,
            _: MutexGuard<'a, State>,
            _: Duration,
        ) -> io::Result<MutexGuard<'a, State>> {
            panic!("unexpected wait")
        }
    }
    let control = Control::new();
    let mut timing = StopAtDispatch {
        control: &control,
        observations: 0,
    };
    let mut calls = 0;
    acquire_loop(
        &control,
        ms(100),
        &mut timing,
        || {
            calls += 1;
            let state = control.lock().unwrap();
            assert_eq!(state.phase, Phase::Stopping);
            assert!(state.admitted);
            Ok(Acquisition::Succeeded(1))
        },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert!(!control.lock().unwrap().admitted);
}

#[test]
fn invalid_clock_wait_and_deadline_arithmetic_fail_without_extra_acquisitions() {
    let cases = [
        (
            vec![Err(io::Error::other("clock unavailable"))],
            false,
            0,
            "clock unavailable",
        ),
        (vec![Ok(ms(1)), Ok(ms(0))], false, 0, "moved backwards"),
        (
            vec![Ok(Duration::MAX), Ok(Duration::MAX)],
            false,
            1,
            "deadline overflow",
        ),
        (vec![], true, 1, "wait failure"),
    ];
    for (observations, fail_wait, expected_calls, message) in cases {
        let control = Control::new();
        let mut timing = FakeTiming {
            observations: observations.into(),
            fail_wait,
            ..FakeTiming::default()
        };
        let mut calls = 0;
        let error = acquire_loop(
            &control,
            ms(100),
            &mut timing,
            || {
                calls += 1;
                assert!(calls <= 1);
                Ok(Acquisition::Succeeded(1))
            },
            || Ok(()),
        )
        .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        assert_eq!(calls, expected_calls);
    }
}

#[test]
fn registration_and_spawn_failures_do_not_start_acquisition() {
    let fixture = Fixture::new();
    let called = Cell::new(false);
    let error = run_registered(&fixture.root, Options::new(100, 100).unwrap(), || {
        called.set(true);
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "registration failed",
        ))
    })
    .unwrap_err();
    assert!(called.get());
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(error.to_string(), "registration failed");
    let control = Control::new();
    let error = spawn_or_stop(&control, || {
        assert_eq!(control.lock().unwrap().phase, Phase::Starting);
        Err(io::Error::other("spawn failed"))
    })
    .unwrap_err();
    assert_eq!(error.to_string(), "spawn failed");
    let state = control.lock().unwrap();
    assert_eq!(state.phase, Phase::Stopped);
    assert!(!state.admitted);
}

#[test]
fn startup_write_error_and_unwind_both_close_and_invalidate_owner() {
    for panic in [false, true] {
        let fixture = Fixture::new();
        let (owner, reader) = cache(&fixture);
        let control = Control::new();
        let closed = Cell::new(0);
        let result = catch_unwind(AssertUnwindSafe(|| {
            worker(
                &control,
                Options::new(100, 100).unwrap(),
                owner,
                || closed.set(closed.get() + 1),
                || {
                    assert_eq!(control.lock().unwrap().phase, Phase::Running);
                    if panic {
                        panic!("injected startup panic");
                    }
                    Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "startup output failed",
                    ))
                },
            )
        }));
        if panic {
            assert!(result.is_err());
        } else {
            let error = result.unwrap().unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
            assert_eq!(control.lock().unwrap().phase, Phase::Stopping);
        }
        assert_eq!(closed.get(), 1);
        assert_inactive(&reader);
        assert_eq!(inspect(&reader, &control).unwrap().latest, "none");
    }
}

#[test]
fn actual_fixture_failure_retains_history_and_recovery_replaces_it() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "fixture");
    fixture.write("hwmon0/temp1_input", "42000\n");
    fixture.write("hwmon0/temp1_label", "Core\n");
    let original = serde_json::to_value(crate::hwmon::scan(&fixture.root).unwrap()).unwrap();
    let (mut owner, reader) = cache(&fixture);
    let control = Control::new();
    assert_eq!(inspect(&reader, &control).unwrap().latest, "none");
    let missing = fixture.root.with_extension("missing");
    let mut attempt = 0;
    acquire_loop(
        &control,
        ms(100),
        &mut FakeTiming::default(),
        || {
            attempt += 1;
            let result = owner.acquire_once()?;
            let status = inspect(&reader, &control).unwrap();
            assert_eq!(status.phase, Phase::Running);
            {
                let mut view = reader.read().unwrap();
                let assessment = view.assess().unwrap();
                let observation = assessment.observation.unwrap();
                match attempt {
                    1 => {
                        assert_eq!(result, Acquisition::Succeeded(1));
                        assert!(status.current);
                        assert_eq!(
                            serde_json::to_value(observation.snapshot).unwrap(),
                            original
                        );
                    }
                    2 => {
                        assert_eq!(result, Acquisition::Failed(2));
                        assert_eq!(status.latest, "failed");
                        assert_eq!(status.observation, "historical");
                        assert!(!status.current);
                        assert_eq!(
                            observation.role,
                            snapshot_cache::ObservationRole::HistoricalAfterFailure
                        );
                        assert_eq!(observation.attempt.sequence, 1);
                        assert_eq!(
                            serde_json::to_value(observation.snapshot).unwrap(),
                            original
                        );
                    }
                    3 => {
                        assert_eq!(result, Acquisition::Succeeded(3));
                        assert!(status.current);
                        assert_eq!(observation.attempt.sequence, 3);
                        assert_eq!(observation.snapshot.sensors[0].raw_value, Some(43000));
                        assert_eq!(
                            serde_json::to_value(&observation.snapshot.sensors[0].identity)
                                .unwrap(),
                            original["sensors"][0]["identity"]
                        );
                    }
                    _ => panic!("extra acquisition"),
                }
            }
            if attempt == 1 {
                fs::rename(&fixture.root, &missing).unwrap();
            }
            if attempt == 2 {
                fs::rename(&missing, &fixture.root).unwrap();
                fixture.write("hwmon0/temp1_input", "43000\n");
            }
            if attempt == 3 {
                control.stop().unwrap();
            }
            Ok(result)
        },
        || Ok(()),
    )
    .unwrap();
    assert_eq!(attempt, 3);
}

#[test]
fn empty_and_malformed_fixture_snapshots_keep_original_diagnostics() {
    let fixture = Fixture::new();
    let (mut owner, reader) = cache(&fixture);
    let control = Control::new();
    control.start().unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    let empty = inspect(&reader, &control).unwrap();
    assert_eq!(empty.latest, "succeeded");
    assert_eq!(empty.observation, "empty");
    assert!(!empty.current);
    fixture.write("hwmon0/name", "fixture");
    fixture.write("hwmon0/temp1_input", "malformed\n");
    let expected = serde_json::to_value(crate::hwmon::scan(&fixture.root).unwrap()).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(2));
    let mut view = reader.read().unwrap();
    let observation = view.assess().unwrap().observation.unwrap();
    assert_eq!(observation.snapshot.sensors[0].raw_value, None);
    assert_eq!(
        serde_json::to_value(observation.snapshot).unwrap(),
        expected
    );
}

#[test]
fn freshness_role_health_and_lifecycle_all_constrain_currentness() {
    use snapshot_cache::{
        AgeBounds, Assessment, AttemptMetadata, Freshness, LatestAttempt, Observation,
        ObservationRole, ProducerState,
    };
    let snapshot = crate::hwmon::Snapshot {
        schema_version: 2,
        root: "/fixture".into(),
        sensors: vec![],
        warnings: vec![],
    };
    let attempt = AttemptMetadata {
        sequence: 1,
        started: ms(0),
        finished: ms(1),
        published: ms(2),
        wall_start_unix_ms: None,
        wall_time_error: Some("wall clock unavailable".into()),
    };
    let error = snapshot_cache::AcquisitionError {
        kind: io::ErrorKind::NotFound,
        raw_os_error: None,
        message: "missing".into(),
    };
    let age = AgeBounds {
        lower: ms(1),
        upper: ms(2),
    };
    for phase in [
        Phase::Starting,
        Phase::Running,
        Phase::Stopping,
        Phase::Stopped,
    ] {
        for producer in [ProducerState::Active, ProducerState::Stopped] {
            for role in [
                ObservationRole::LatestAttempt,
                ObservationRole::HistoricalAfterFailure,
                ObservationRole::HistoricalAfterStop,
            ] {
                for freshness in [
                    Freshness::WithinBudget(age),
                    Freshness::Stale(age),
                    Freshness::NoSensorObservations,
                ] {
                    for succeeded in [true, false] {
                        let status = classify(
                            Assessment {
                                assessed_at: ms(3),
                                producer,
                                latest: if succeeded {
                                    LatestAttempt::Succeeded(&attempt)
                                } else {
                                    LatestAttempt::Failed {
                                        attempt: &attempt,
                                        error: &error,
                                    }
                                },
                                observation: Some(Observation {
                                    snapshot: &snapshot,
                                    attempt: &attempt,
                                    role,
                                    freshness,
                                }),
                                physical_age_known: false,
                            },
                            phase,
                        );
                        assert_eq!(
                            status.current,
                            phase == Phase::Running
                                && producer == ProducerState::Active
                                && succeeded
                                && role == ObservationRole::LatestAttempt
                                && matches!(freshness, Freshness::WithinBudget(_))
                        );
                        assert_eq!(
                            status.latest,
                            if succeeded { "succeeded" } else { "failed" }
                        );
                        if phase != Phase::Running || role != ObservationRole::LatestAttempt {
                            assert_eq!(status.observation, "historical");
                        } else {
                            assert_eq!(
                                status.observation,
                                match freshness {
                                    Freshness::WithinBudget(_) => "within-budget",
                                    Freshness::Stale(_) => "stale",
                                    Freshness::NoSensorObservations => "empty",
                                }
                            );
                        }
                    }
                }
            }
        }
    }
    let empty = classify(
        Assessment {
            assessed_at: ms(0),
            producer: ProducerState::Active,
            latest: LatestAttempt::Empty,
            observation: None,
            physical_age_known: false,
        },
        Phase::Running,
    );
    assert!(!empty.current);
    assert_eq!(empty.latest, "none");
    assert_eq!(empty.observation, "none");
}

#[test]
fn admitted_late_publication_is_historical_until_join_completes() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "fixture");
    fixture.write("hwmon0/temp1_input", "42000\n");
    let (mut owner, reader) = cache(&fixture);
    let control = Arc::new(Control::new());
    let worker_control = Arc::clone(&control);
    let (admitted_tx, admitted_rx) = mpsc::channel();
    let (acquire_tx, acquire_rx) = mpsc::channel();
    let (published_tx, published_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let result = acquire_loop(
            &worker_control,
            ms(100),
            &mut FakeTiming::default(),
            || {
                admitted_tx.send(()).unwrap();
                acquire_rx.recv_timeout(TEST_DEADLINE).unwrap();
                let result = owner.acquire_once();
                published_tx.send(()).unwrap();
                release_rx.recv_timeout(TEST_DEADLINE).unwrap();
                result
            },
            || Ok(()),
        );
        owner.stop();
        result
    });
    admitted_rx.recv_timeout(TEST_DEADLINE).unwrap();
    control.stop().unwrap();
    assert_eq!(control.lock().unwrap().phase, Phase::Stopping);
    assert!(control.lock().unwrap().admitted);
    assert!(!inspect(&reader, &control).unwrap().current);
    acquire_tx.send(()).unwrap();
    published_rx.recv_timeout(TEST_DEADLINE).unwrap();
    let status = inspect(&reader, &control).unwrap();
    assert_eq!(status.phase, Phase::Stopping);
    assert_eq!(status.latest, "succeeded");
    assert_eq!(status.observation, "historical");
    assert!(!status.current);
    {
        let mut view = reader.read().unwrap();
        let assessment = view.assess().unwrap();
        assert_eq!(assessment.producer, snapshot_cache::ProducerState::Active);
        assert_eq!(
            assessment.observation.unwrap().snapshot.sensors[0].raw_value,
            Some(42000)
        );
    }
    assert!(!handle.is_finished());
    release_tx.send(()).unwrap();
    let mut output = Vec::new();
    finish(&control, &reader, handle, Ok(()), &mut output).unwrap();
    assert_inactive(&reader);
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("outcome: graceful"));
    assert!(output.contains("service-current: false"));
}

#[test]
fn stop_wakes_a_real_idle_condvar_without_lost_notification() {
    struct Waiting {
        entered: mpsc::Sender<()>,
    }
    impl Timing for Waiting {
        fn now(&mut self) -> io::Result<Duration> {
            Ok(Duration::ZERO)
        }
        fn wait<'a>(
            &mut self,
            control: &Control,
            state: MutexGuard<'a, State>,
            _: Duration,
        ) -> io::Result<MutexGuard<'a, State>> {
            self.entered.send(()).unwrap();
            let (state, timeout) = control.wake.wait_timeout(state, TEST_DEADLINE).unwrap();
            assert!(!timeout.timed_out(), "stop notification was lost");
            Ok(state)
        }
    }
    let control = Arc::new(Control::new());
    let worker_control = Arc::clone(&control);
    let (entered, waiting) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut count = 0;
        let result = acquire_loop(
            &worker_control,
            ms(60_000),
            &mut Waiting { entered },
            || {
                count += 1;
                Ok(Acquisition::Succeeded(count))
            },
            || Ok(()),
        );
        done_tx.send((result, count)).unwrap();
    });
    waiting.recv_timeout(TEST_DEADLINE).unwrap();
    control.stop().unwrap();
    control.stop().unwrap();
    let (result, count) = done_rx.recv_timeout(TEST_DEADLINE).unwrap();
    handle.join().unwrap();
    result.unwrap();
    assert_eq!(count, 1);
}

#[test]
fn fatal_cache_error_is_preserved_after_a_concurrent_stop_request() {
    let control = Control::new();
    let mut count = 0;
    let error = acquire_loop(
        &control,
        ms(100),
        &mut FakeTiming::default(),
        || {
            count += 1;
            control.stop().unwrap();
            Err(CacheError::InvalidClock)
        },
        || Ok(()),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), CacheError::InvalidClock.to_string());
    assert_eq!(count, 1);
    assert!(!control.lock().unwrap().admitted);
}

struct CheckedOutput<'a> {
    joined: &'a AtomicBool,
    control: &'a Control,
    fail_write: bool,
    fail_flush: bool,
    bytes: Vec<u8>,
}

impl Write for CheckedOutput<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        assert!(self.joined.load(Ordering::Acquire));
        assert_eq!(self.control.lock().unwrap().phase, Phase::Stopped);
        if self.fail_write {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "final write failed",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            Err(io::Error::other("final flush failed"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn final_output_failures_happen_after_join_and_preserve_original_fatal_cause() {
    for (fatal, fail_write, fail_flush) in [
        (false, true, false),
        (false, false, true),
        (true, true, false),
    ] {
        let fixture = Fixture::new();
        let (mut owner, reader) = cache(&fixture);
        let control = Control::new();
        let joined = Arc::new(AtomicBool::new(false));
        let done = Arc::clone(&joined);
        let handle = thread::spawn(move || {
            owner.stop();
            done.store(true, Ordering::Release);
            if fatal {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "original fatal cause",
                ))
            } else {
                Ok(())
            }
        });
        let mut output = CheckedOutput {
            joined: &joined,
            control: &control,
            fail_write,
            fail_flush,
            bytes: vec![],
        };
        let coordinator = if fatal {
            Err(io::Error::other("coordinator failure"))
        } else {
            Ok(())
        };
        let error = finish(&control, &reader, handle, coordinator, &mut output).unwrap_err();
        assert_inactive(&reader);
        if fatal {
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(
                error
                    .to_string()
                    .starts_with("original fatal cause; additionally: coordinator failure")
            );
        }
        assert!(error.to_string().contains(if fail_write {
            "final write failed"
        } else {
            "final flush failed"
        }));
    }
}

#[test]
fn poisoned_control_still_closes_worker_and_joins_without_claiming_stopped() {
    let fixture = Fixture::new();
    let (owner, reader) = cache(&fixture);
    let control = Arc::new(Control::new());
    let poison = Arc::clone(&control);
    assert!(
        thread::spawn(move || {
            let _guard = poison.state.lock().unwrap();
            panic!("injected lifecycle poison");
        })
        .join()
        .is_err()
    );
    assert!(control.lock().is_err());
    assert!(control.stop().is_err());
    let closed = Arc::new(AtomicBool::new(false));
    let worker_closed = Arc::clone(&closed);
    let worker_control = Arc::clone(&control);
    let handle = thread::spawn(move || {
        worker(
            &worker_control,
            Options::new(100, 100).unwrap(),
            owner,
            || worker_closed.store(true, Ordering::Release),
            || panic!("startup must not run with poisoned control"),
        )
    });
    let mut output = Vec::new();
    let error = finish(&control, &reader, handle, Ok(()), &mut output).unwrap_err();
    assert!(error.to_string().contains("poisoned"));
    assert!(closed.load(Ordering::Acquire));
    assert_inactive(&reader);
    assert!(inspect(&reader, &control).is_err());
    assert_eq!(
        String::from_utf8(output).unwrap(),
        "SiliconWatch daemon lifecycle unavailable; worker joined; outcome: failed\n"
    );
}

#[test]
fn many_recoverable_attempts_do_not_accumulate_output_or_a_catchup_queue() {
    let control = Control::new();
    let mut timing = FakeTiming::default();
    let mut count = 0;
    let announcements = Cell::new(0);
    acquire_loop(
        &control,
        ms(100),
        &mut timing,
        || {
            count += 1;
            if count == 10_000 {
                control.stop().unwrap();
            }
            Ok(Acquisition::Failed(count))
        },
        || {
            announcements.set(announcements.get() + 1);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(count, 10_000);
    assert_eq!(announcements.get(), 1);
    assert_eq!(timing.time.get(), ms(999_900));
    assert_eq!(timing.waits.len(), 9_999);
    assert!(!control.lock().unwrap().admitted);
}

#[test]
fn worker_panic_closes_signal_iterator_in_isolated_process() {
    const CHILD: &str = "SILICONWATCH_DAEMON_PANIC_CHILD";
    if std::env::var_os(CHILD).as_deref() == Some(std::ffi::OsStr::new("1")) {
        let fixture = Fixture::new();
        let (owner, reader) = cache(&fixture);
        let control = Arc::new(Control::new());
        let worker_control = Arc::clone(&control);
        let mut signals = Signals::new([SIGINT, SIGTERM]).unwrap();
        let close = signals.handle();
        let handle = thread::spawn(move || {
            worker(
                &worker_control,
                Options::new(100, 100).unwrap(),
                owner,
                || close.close(),
                || panic!("injected panic before first admission"),
            )
        });
        assert_eq!(signals.forever().next(), None);
        let mut output = Vec::new();
        let error = finish(&control, &reader, handle, Ok(()), &mut output).unwrap_err();
        assert!(error.to_string().contains("worker panicked"));
        assert_inactive(&reader);
        assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
        assert!(
            String::from_utf8(output)
                .unwrap()
                .contains("outcome: failed")
        );
        return;
    }
    // Install process-global handlers only in this child. A deadline is a test
    // deadlock guard, not a promise about production shutdown latency.
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Child(
        Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "daemon::tests::worker_panic_closes_signal_iterator_in_isolated_process",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + TEST_DEADLINE;
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(
                status.success(),
                "isolated signal cleanup test failed: {status}"
            );
            break;
        }
        assert!(
            Instant::now() < deadline,
            "isolated signal cleanup child hung"
        );
        thread::sleep(ms(5));
    }
}

#[test]
fn held_cache_view_delays_publication_but_not_stop_acknowledgement() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "fixture");
    fixture.write("hwmon0/temp1_input", "42000\n");
    let (mut owner, reader) = cache(&fixture);
    owner.acquire_once().unwrap();
    fixture.write("hwmon0/temp1_input", "43000\n");
    let mut held = reader.read().unwrap();
    let control = Arc::new(Control::new());
    let worker_control = Arc::clone(&control);
    let (admitted_tx, admitted_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let result = acquire_loop(
            &worker_control,
            ms(100),
            &mut FakeTiming::default(),
            || {
                admitted_tx.send(()).unwrap();
                let result = owner.acquire_once();
                worker_control.stop().unwrap();
                result
            },
            || Ok(()),
        );
        owner.stop();
        result
    });
    admitted_rx.recv_timeout(TEST_DEADLINE).unwrap();
    control.stop().unwrap();
    assert_eq!(control.lock().unwrap().phase, Phase::Stopping);
    assert!(control.lock().unwrap().admitted);
    let previous = held.assess().unwrap();
    let observation = previous.observation.unwrap();
    assert_eq!(observation.attempt.sequence, 1);
    assert_eq!(observation.snapshot.sensors[0].raw_value, Some(42000));
    assert!(!handle.is_finished());
    // Never inspect recursively or join with this read guard still held.
    drop(held);
    let mut output = Vec::new();
    finish(&control, &reader, handle, Ok(()), &mut output).unwrap();
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
    let mut view = reader.read().unwrap();
    let assessment = view.assess().unwrap();
    let observation = assessment.observation.unwrap();
    assert_eq!(observation.attempt.sequence, 2);
    assert_eq!(observation.snapshot.sensors[0].raw_value, Some(43000));
    assert_eq!(
        observation.role,
        snapshot_cache::ObservationRole::HistoricalAfterStop
    );
}

#[test]
fn coordinator_wait_panic_closes_then_joins_before_reporting_error() {
    let fixture = Fixture::new();
    let (mut owner, reader) = cache(&fixture);
    let control = Arc::new(Control::new());
    control.start().unwrap();
    let worker_control = Arc::clone(&control);
    let (release_tx, release_rx) = mpsc::channel();
    let joined = Arc::new(AtomicBool::new(false));
    let worker_joined = Arc::clone(&joined);
    let handle = thread::spawn(move || {
        release_rx.recv_timeout(TEST_DEADLINE).unwrap();
        assert_eq!(worker_control.lock().unwrap().phase, Phase::Stopping);
        owner.stop();
        worker_joined.store(true, Ordering::Release);
        Ok(())
    });
    let closed = Cell::new(false);
    let mut output = CheckedOutput {
        joined: &joined,
        control: &control,
        fail_write: false,
        fail_flush: false,
        bytes: vec![],
    };
    let error = coordinate(
        &control,
        &reader,
        handle,
        || panic!("injected signal wait failure"),
        || {
            closed.set(true);
            release_tx.send(()).unwrap();
        },
        &mut output,
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "daemon signal wait panicked");
    assert!(closed.get());
    assert!(joined.load(Ordering::Acquire));
    assert_inactive(&reader);
    assert_eq!(control.lock().unwrap().phase, Phase::Stopped);
    assert!(
        String::from_utf8(output.bytes)
            .unwrap()
            .contains("outcome: failed")
    );
}
