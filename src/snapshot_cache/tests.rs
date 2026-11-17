use super::*;
use crate::hwmon::{Sensor, Status, Warning};
use crate::identity::SourceIdentity;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::AtomicUsize;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

const WAIT: Duration = Duration::from_secs(5);

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

struct FakeClock {
    times: VecDeque<Result<Duration, CacheError>>,
    wall: Result<u64, String>,
}

impl FakeClock {
    fn at(times: [u64; 3]) -> Self {
        Self {
            times: times.into_iter().map(|value| Ok(ms(value))).collect(),
            wall: Ok(1234),
        }
    }
}

impl Clock for FakeClock {
    fn now(&mut self) -> Result<Duration, CacheError> {
        self.times
            .pop_front()
            .expect("unexpected clock observation")
    }

    fn wall_time(&mut self) -> Result<u64, String> {
        self.wall.clone()
    }
}

fn snapshot(raw: i64) -> Snapshot {
    Snapshot {
        schema_version: 2,
        root: PathBuf::from("fixture-root-never-opened"),
        sensors: vec![Sensor {
            identity: SourceIdentity::unresolved(),
            device: "fixture".into(),
            device_path: PathBuf::from("fixture-device"),
            attribute: "temp1_input".into(),
            label: Some("Fixture temperature".into()),
            source_path: PathBuf::from("fixture-root-never-opened/hwmon0/temp1_input"),
            raw_text: Some(raw.to_string()),
            raw_value: Some(raw),
            value: Some(raw as f64 / 1000.0),
            unit: Some("°C"),
            status: Status::Ok,
            error: None,
        }],
        warnings: vec![Warning {
            path: PathBuf::from("fixture-warning"),
            message: "preserve this fixture warning".into(),
        }],
    }
}

fn cache(threshold: Duration) -> (CacheOwner, CacheReader) {
    new("fixture-root-never-opened", threshold).unwrap()
}

fn publish(owner: &mut CacheOwner, times: [u64; 3], raw: i64) {
    assert!(matches!(
        owner.acquire_with(&mut FakeClock::at(times), || Ok(snapshot(raw))),
        Ok(Acquisition::Succeeded(_))
    ));
}

#[test]
fn constructor_and_readers_never_acquire() {
    assert!(matches!(
        new("fixture-root-never-opened", Duration::ZERO),
        Err(CacheError::InvalidThreshold)
    ));
    let (mut owner, reader) = cache(ms(10));
    let scans = AtomicUsize::new(0);
    for clone in [reader.clone(), reader.clone()] {
        let mut view = clone.read().unwrap();
        let assessment = view.assess_at(Duration::ZERO).unwrap();
        assert!(matches!(assessment.latest, LatestAttempt::Empty));
        assert!(assessment.observation.is_none());
        assert_eq!(assessment.producer, ProducerState::Active);
        assert!(!assessment.physical_age_known);
    }
    assert_eq!(scans.load(Ordering::Relaxed), 0);
    assert_eq!(owner.root, PathBuf::from("fixture-root-never-opened"));
    assert_eq!(
        owner.acquire_with(&mut FakeClock::at([1, 2, 3]), || {
            scans.fetch_add(1, Ordering::Relaxed);
            Ok(snapshot(42_000))
        }),
        Ok(Acquisition::Succeeded(1))
    );
    for _ in 0..10 {
        let clone = reader.clone();
        let mut view = clone.read().unwrap();
        assert!(view.assess_at(ms(4)).unwrap().observation.is_some());
    }
    assert_eq!(scans.load(Ordering::Relaxed), 1);
}

#[test]
fn successful_empty_inventory_and_first_failure_are_distinct() {
    let (mut owner, reader) = cache(ms(10));
    let mut empty = snapshot(1);
    empty.sensors.clear();
    assert_eq!(
        owner.acquire_with(&mut FakeClock::at([1, 2, 3]), || Ok(empty)),
        Ok(Acquisition::Succeeded(1))
    );
    {
        let mut view = reader.read().unwrap();
        let assessment = view.assess_at(ms(50)).unwrap();
        assert!(matches!(assessment.latest, LatestAttempt::Succeeded(_)));
        let observed = assessment.observation.unwrap();
        assert!(observed.snapshot.sensors.is_empty());
        assert_eq!(observed.freshness, Freshness::NoSensorObservations);
        assert_eq!(observed.role, ObservationRole::LatestAttempt);
        assert_eq!(observed.attempt.published, ms(3));
    }
    let (mut owner, reader) = cache(ms(10));
    assert_eq!(
        owner.acquire_with(&mut FakeClock::at([1, 2, 3]), || {
            Err(io::Error::from_raw_os_error(13))
        }),
        Ok(Acquisition::Failed(1))
    );
    let mut view = reader.read().unwrap();
    let assessment = view.assess_at(ms(4)).unwrap();
    let LatestAttempt::Failed { attempt, error } = assessment.latest else {
        panic!("scan failure must be explicit");
    };
    assert_eq!(attempt.sequence, 1);
    assert_eq!(error.kind, io::ErrorKind::PermissionDenied);
    assert_eq!(error.raw_os_error, Some(13));
    assert!(assessment.observation.is_none());
}

#[test]
fn fault_and_permission_denied_rows_survive_publication_unchanged() {
    let (mut owner, reader) = cache(ms(10));
    let mut incoming = snapshot(42_000);
    incoming.sensors[0].status = Status::Fault;
    incoming.sensors[0].error = Some("fixture sensor reports a fault".into());
    let mut denied = snapshot(0).sensors.remove(0);
    denied.attribute = "temp2_input".into();
    denied.source_path = PathBuf::from("fixture-root-never-opened/hwmon0/temp2_input");
    denied.raw_text = None;
    denied.raw_value = None;
    denied.value = None;
    denied.status = Status::PermissionDenied;
    denied.error = Some("fixture permission denied".into());
    incoming.sensors.push(denied);
    let expected = serde_json::to_value(&incoming).unwrap();
    assert_eq!(
        owner.acquire_with(&mut FakeClock::at([1, 2, 3]), || Ok(incoming)),
        Ok(Acquisition::Succeeded(1))
    );
    let mut view = reader.read().unwrap();
    let assessment = view.assess_at(ms(4)).unwrap();
    assert!(matches!(assessment.latest, LatestAttempt::Succeeded(_)));
    let observed = assessment.observation.unwrap();
    assert_eq!(serde_json::to_value(observed.snapshot).unwrap(), expected);
    assert_eq!(observed.snapshot.sensors[0].status, Status::Fault);
    assert_eq!(
        observed.snapshot.sensors[1].status,
        Status::PermissionDenied
    );
    assert_eq!(observed.role, ObservationRole::LatestAttempt);
    assert_eq!(
        observed.freshness,
        Freshness::WithinBudget(AgeBounds {
            lower: ms(2),
            upper: ms(3)
        })
    );
}

#[test]
fn freshness_boundaries_reassess_the_held_view() {
    let (mut owner, reader) = cache(ms(10));
    publish(&mut owner, [10, 12, 15], 42_000);
    let mut view = reader.read().unwrap();
    for (now, lower, upper, stale) in [(19, 7, 9, false), (20, 8, 10, false), (21, 9, 11, true)] {
        let assessment = view.assess_at(ms(now)).unwrap();
        let bounds = AgeBounds {
            lower: ms(lower),
            upper: ms(upper),
        };
        assert_eq!(assessment.assessed_at, ms(now));
        assert_eq!(assessment.producer, ProducerState::Active);
        assert!(!assessment.physical_age_known);
        let observation = assessment.observation.unwrap();
        assert_eq!(observation.role, ObservationRole::LatestAttempt);
        assert_eq!(
            observation.freshness,
            if stale {
                Freshness::Stale(bounds)
            } else {
                Freshness::WithinBudget(bounds)
            }
        );
    }
}

#[test]
fn backward_assessment_cannot_make_stale_data_fresh() {
    let (mut owner, reader) = cache(ms(10));
    publish(&mut owner, [1, 2, 3], 42_000);
    let mut view = reader.read().unwrap();
    assert!(matches!(
        view.assess_at(ms(20))
            .unwrap()
            .observation
            .unwrap()
            .freshness,
        Freshness::Stale(_)
    ));
    // Both rejected observations still exceed the publication timestamp.
    assert_eq!(view.assess_at(ms(4)).unwrap_err(), CacheError::InvalidClock);
    assert_eq!(
        view.assess_at(ms(19)).unwrap_err(),
        CacheError::InvalidClock
    );
    assert!(matches!(
        view.assess_at(ms(20))
            .unwrap()
            .observation
            .unwrap()
            .freshness,
        Freshness::Stale(_)
    ));
    drop(view);
    let mut fresh_view = reader.read().unwrap();
    assert_eq!(
        fresh_view.assess_at(ms(2)).unwrap_err(),
        CacheError::InvalidClock
    );
}

#[test]
fn wall_jumps_and_unavailable_wall_time_do_not_change_age() {
    let (mut owner, reader) = cache(ms(10));
    for (index, wall) in [Ok(50_000), Ok(10), Err("fixture wall unavailable".into())]
        .into_iter()
        .enumerate()
    {
        let start = 1 + 4 * index as u64;
        let mut clock = FakeClock::at([start, start + 1, start + 2]);
        clock.wall = wall.clone();
        assert_eq!(
            owner.acquire_with(&mut clock, || Ok(snapshot(42_000))),
            Ok(Acquisition::Succeeded(index as u64 + 1))
        );
        let mut view = reader.read().unwrap();
        let observation = view.assess_at(ms(start + 3)).unwrap().observation.unwrap();
        assert_eq!(
            observation.attempt.wall_start_unix_ms,
            wall.as_ref().ok().copied()
        );
        assert_eq!(
            observation.attempt.wall_time_error.as_deref(),
            wall.as_ref().err().map(String::as_str)
        );
        assert_eq!(
            observation.freshness,
            Freshness::WithinBudget(AgeBounds {
                lower: ms(2),
                upper: ms(3)
            })
        );
    }
}

#[test]
fn invalid_owner_clock_stops_without_replacing_prior_success() {
    let invalid = [
        vec![Ok(ms(2))], // A new start cannot precede prior publication at 3.
        vec![Ok(ms(4)), Ok(ms(3))],
        vec![Ok(ms(4)), Ok(ms(5)), Ok(ms(4))],
        vec![Err(CacheError::InvalidClock)],
        vec![Ok(ms(4)), Err(CacheError::InvalidClock)],
        vec![Ok(ms(4)), Ok(ms(5)), Err(CacheError::InvalidClock)],
    ];
    for times in invalid {
        let (mut owner, reader) = cache(ms(10));
        publish(&mut owner, [1, 2, 3], 42_000);
        let mut clock = FakeClock {
            times: times.into(),
            wall: Ok(999),
        };
        assert_eq!(
            owner.acquire_with(&mut clock, || Ok(snapshot(99_000))),
            Err(CacheError::InvalidClock)
        );
        assert_eq!(owner.sequence, 1);
        assert_eq!(owner.last_published, ms(3));
        assert!(!owner.shared.state.is_poisoned());
        let mut view = reader.read().unwrap();
        let assessment = view.assess_at(ms(6)).unwrap();
        assert_eq!(assessment.producer, ProducerState::Stopped);
        let observation = assessment.observation.unwrap();
        assert_eq!(observation.snapshot.sensors[0].raw_value, Some(42_000));
        assert_eq!(observation.attempt.sequence, 1);
        assert_eq!(observation.role, ObservationRole::HistoricalAfterStop);
    }
}

#[test]
fn public_assessment_propagates_invalid_clock() {
    let (owner, mut reader) = cache(ms(10));
    drop(owner);
    // A controlled future origin makes the production clock fail, without sleeps.
    Arc::get_mut(&mut reader.shared).unwrap().origin = Instant::now()
        .checked_add(Duration::from_secs(86_400))
        .unwrap();
    let mut view = reader.read().unwrap();
    assert_eq!(view.assess().unwrap_err(), CacheError::InvalidClock);
}

#[test]
fn duration_maximum_does_not_require_u64_nanosecond_conversion() {
    let (mut owner, reader) = cache(ms(2));
    let mut clock = FakeClock {
        times: [Duration::MAX - ms(2), Duration::MAX - ms(1), Duration::MAX]
            .into_iter()
            .map(Ok)
            .collect(),
        wall: Ok(0),
    };
    assert_eq!(
        owner.acquire_with(&mut clock, || Ok(snapshot(1))),
        Ok(Acquisition::Succeeded(1))
    );
    let mut view = reader.read().unwrap();
    let observation = view.assess_at(Duration::MAX).unwrap().observation.unwrap();
    assert_eq!(observation.attempt.published, Duration::MAX);
    assert_eq!(
        observation.freshness,
        Freshness::WithinBudget(AgeBounds {
            lower: ms(1),
            upper: ms(2)
        })
    );
}

#[test]
fn sequence_exhaustion_never_scans_or_relabels_previous_data() {
    let (mut owner, reader) = cache(ms(10));
    publish(&mut owner, [1, 2, 3], 42_000);
    owner.sequence = u64::MAX;
    let mut clock = FakeClock {
        times: VecDeque::new(),
        wall: Ok(0),
    };
    assert_eq!(
        owner.acquire_with(&mut clock, || panic!("exhausted owner must not scan")),
        Err(CacheError::SequenceExhausted)
    );
    assert_eq!(
        owner.acquire_with(&mut clock, || panic!("stopped owner must not scan")),
        Err(CacheError::Stopped)
    );
    let mut view = reader.read().unwrap();
    let assessment = view.assess_at(ms(4)).unwrap();
    assert_eq!(assessment.producer, ProducerState::Stopped);
    assert_eq!(assessment.observation.unwrap().attempt.sequence, 1);
}

#[test]
fn repeated_failures_retain_one_original_payload_and_recovery_replaces_it() {
    let (mut owner, reader) = cache(ms(1000));
    publish(&mut owner, [1, 2, 3], 42_000);
    let original = {
        let view = reader.read().unwrap();
        let PublishedState::Succeeded(observed) = &*view.guard else {
            panic!("success expected")
        };
        observed.snapshot.sensors.as_ptr()
    };
    for sequence in 2..=128_u64 {
        let start = sequence * 4;
        assert_eq!(
            owner.acquire_with(&mut FakeClock::at([start, start + 1, start + 2]), || {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("fixture failure {sequence}"),
                ))
            }),
            Ok(Acquisition::Failed(sequence))
        );
        let mut view = reader.read().unwrap();
        let assessment = view.assess_at(ms(start + 3)).unwrap();
        let LatestAttempt::Failed { attempt, error } = assessment.latest else {
            panic!("failure expected")
        };
        assert_eq!(attempt.sequence, sequence);
        assert_eq!(error.message, format!("fixture failure {sequence}"));
        let observation = assessment.observation.unwrap();
        assert_eq!(observation.role, ObservationRole::HistoricalAfterFailure);
        assert_eq!(observation.attempt.sequence, 1);
        assert_eq!(observation.snapshot.sensors.as_ptr(), original);
        assert_eq!(observation.snapshot.sensors[0].raw_value, Some(42_000));
        assert_eq!(
            observation.freshness,
            Freshness::WithinBudget(AgeBounds {
                lower: ms(start + 1),
                upper: ms(start + 2)
            })
        );
    }
    publish(&mut owner, [520, 521, 522], 99_000);
    let mut view = reader.read().unwrap();
    let assessment = view.assess_at(ms(523)).unwrap();
    assert!(
        matches!(assessment.latest, LatestAttempt::Succeeded(attempt) if attempt.sequence == 129)
    );
    let observation = assessment.observation.unwrap();
    assert_eq!(observation.role, ObservationRole::LatestAttempt);
    assert_eq!(observation.snapshot.sensors[0].raw_value, Some(99_000));
    assert_eq!(observation.attempt.sequence, 129);
}

struct PublicationClock {
    calls: usize,
    finished: mpsc::Sender<()>,
    proceed: mpsc::Receiver<()>,
}

impl Clock for PublicationClock {
    fn now(&mut self) -> Result<Duration, CacheError> {
        self.calls += 1;
        match self.calls {
            1 => Ok(ms(3)),
            2 => {
                self.finished.send(()).unwrap();
                self.proceed.recv_timeout(WAIT).unwrap();
                Ok(ms(4))
            }
            3 => Ok(ms(100)),
            _ => panic!("unexpected clock observation"),
        }
    }

    fn wall_time(&mut self) -> Result<u64, String> {
        Ok(0)
    }
}

#[test]
fn simultaneous_readers_hold_a_coherent_generation_until_publication() {
    for failure in [false, true] {
        let (mut owner, reader) = cache(ms(10));
        publish(&mut owner, [0, 1, 2], 42_000);
        let (ready_tx, ready_rx) = mpsc::channel();
        let mut releases = Vec::new();
        let mut readers = Vec::new();
        for _ in 0..2 {
            let reader = reader.clone();
            let ready = ready_tx.clone();
            let (release_tx, release_rx) = mpsc::channel();
            releases.push(release_tx);
            readers.push(thread::spawn(move || {
                let mut view = reader.read().unwrap();
                {
                    let assessment = view.assess_at(ms(3)).unwrap();
                    assert!(matches!(assessment.latest, LatestAttempt::Succeeded(attempt) if attempt.sequence == 1));
                    let observed = assessment.observation.unwrap();
                    assert_eq!(observed.attempt.sequence, 1);
                    assert_eq!(observed.snapshot.sensors[0].raw_value, Some(42_000));
                }
                ready.send(()).unwrap();
                release_rx.recv_timeout(WAIT).unwrap();
                let assessment = view.assess_at(ms(5)).unwrap();
                assert!(matches!(assessment.latest, LatestAttempt::Succeeded(attempt) if attempt.sequence == 1));
                let observed = assessment.observation.unwrap();
                assert_eq!(observed.role, ObservationRole::LatestAttempt);
                assert_eq!(observed.attempt.sequence, 1);
                assert_eq!(observed.snapshot.sensors[0].raw_value, Some(42_000));
            }));
        }
        ready_rx.recv_timeout(WAIT).unwrap();
        ready_rx.recv_timeout(WAIT).unwrap();
        let shared = Arc::clone(&owner.shared);
        let (finished_tx, finished_rx) = mpsc::channel();
        let (proceed_tx, proceed_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let writer = thread::spawn(move || {
            let mut clock = PublicationClock {
                calls: 0,
                finished: finished_tx,
                proceed: proceed_rx,
            };
            let outcome = owner.acquire_with(&mut clock, || {
                assert!(
                    shared.state.try_read().is_ok(),
                    "scan must run outside write lock"
                );
                if failure {
                    Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "fixture provider missing",
                    ))
                } else {
                    Ok(snapshot(99_000))
                }
            });
            assert!(done_tx.send((owner, outcome)).is_ok());
        });
        finished_rx.recv_timeout(WAIT).unwrap();
        assert!(matches!(
            reader.shared.state.try_write(),
            Err(std::sync::TryLockError::WouldBlock)
        ));
        proceed_tx.send(()).unwrap();
        assert!(matches!(done_rx.try_recv(), Err(TryRecvError::Empty)));
        for release in releases {
            release.send(()).unwrap();
        }
        for reader in readers {
            reader.join().unwrap();
        }
        let (_owner, outcome) = done_rx.recv_timeout(WAIT).unwrap();
        assert_eq!(
            outcome,
            Ok(if failure {
                Acquisition::Failed(2)
            } else {
                Acquisition::Succeeded(2)
            })
        );
        writer.join().unwrap();
        let mut view = reader.read().unwrap();
        let assessment = view.assess_at(ms(100)).unwrap();
        let latest = match assessment.latest {
            LatestAttempt::Succeeded(attempt) if !failure => attempt,
            LatestAttempt::Failed { attempt, error } if failure => {
                assert_eq!(error.kind, io::ErrorKind::NotFound);
                assert_eq!(error.message, "fixture provider missing");
                attempt
            }
            _ => panic!("unexpected published outcome"),
        };
        assert_eq!(latest.sequence, 2);
        assert_eq!(
            (latest.started, latest.finished, latest.published),
            (ms(3), ms(4), ms(100))
        );
        let observed = assessment.observation.unwrap();
        if failure {
            assert_eq!(observed.snapshot.sensors[0].raw_value, Some(42_000));
            assert_eq!(observed.attempt.sequence, 1);
            assert_eq!(observed.role, ObservationRole::HistoricalAfterFailure);
            assert_eq!(
                (
                    observed.attempt.started,
                    observed.attempt.finished,
                    observed.attempt.published
                ),
                (ms(0), ms(1), ms(2))
            );
            assert_eq!(
                observed.freshness,
                Freshness::Stale(AgeBounds {
                    lower: ms(99),
                    upper: ms(100)
                })
            );
        } else {
            assert_eq!(observed.snapshot.sensors[0].raw_value, Some(99_000));
            assert_eq!(observed.attempt.sequence, 2);
            assert_eq!(observed.role, ObservationRole::LatestAttempt);
            assert_eq!(
                observed.freshness,
                Freshness::Stale(AgeBounds {
                    lower: ms(96),
                    upper: ms(97)
                })
            );
        }
    }
}

#[test]
fn stop_and_drop_are_visible_through_an_existing_read_guard() {
    for explicit_stop in [true, false] {
        let (mut owner, reader) = cache(ms(10));
        publish(&mut owner, [1, 2, 3], 42_000);
        let mut view = reader.read().unwrap();
        assert_eq!(
            view.assess_at(ms(4)).unwrap().producer,
            ProducerState::Active
        );
        if explicit_stop {
            owner.stop();
            assert_eq!(
                owner.acquire_with(&mut FakeClock::at([5, 6, 7]), || panic!(
                    "stopped owner scanned"
                )),
                Err(CacheError::Stopped)
            );
        }
        drop(owner); // Must not wait for the read guard held in this same thread.
        let assessment = view.assess_at(ms(5)).unwrap();
        assert_eq!(assessment.producer, ProducerState::Stopped);
        assert_eq!(
            assessment.observation.unwrap().role,
            ObservationRole::HistoricalAfterStop
        );
    }
}

struct PanicClock {
    call: usize,
    panic_at: usize,
}

impl Clock for PanicClock {
    fn now(&mut self) -> Result<Duration, CacheError> {
        self.call += 1;
        assert_ne!(self.call, self.panic_at, "fixture clock panic");
        Ok(ms(self.call as u64 + 3))
    }
    fn wall_time(&mut self) -> Result<u64, String> {
        assert_ne!(self.panic_at, 4, "fixture wall-clock panic");
        Ok(0)
    }
}

#[test]
fn caught_scan_and_prepublication_clock_panics_stop_with_a_held_view() {
    for phase in [0, 1, 2, 4] {
        let (mut owner, reader) = cache(ms(10));
        publish(&mut owner, [1, 2, 3], 42_000);
        let mut view = reader.read().unwrap();
        let panic = catch_unwind(AssertUnwindSafe(|| {
            owner.acquire_with(
                &mut PanicClock {
                    call: 0,
                    panic_at: phase,
                },
                || {
                    assert_ne!(phase, 0, "fixture scan panic");
                    Ok(snapshot(99_000))
                },
            )
        }));
        assert!(panic.is_err());
        assert!(!owner.shared.state.is_poisoned());
        let assessment = view.assess_at(ms(6)).unwrap();
        assert_eq!(assessment.producer, ProducerState::Stopped);
        let observed = assessment.observation.unwrap();
        assert_eq!(observed.role, ObservationRole::HistoricalAfterStop);
        assert_eq!(observed.snapshot.sensors[0].raw_value, Some(42_000));
        assert_eq!(
            owner.acquire_with(&mut FakeClock::at([7, 8, 9]), || panic!(
                "stopped owner scanned"
            )),
            Err(CacheError::Stopped)
        );
    }
}

#[test]
fn publication_clock_panic_poison_is_never_silently_recovered() {
    let (mut owner, reader) = cache(ms(10));
    publish(&mut owner, [1, 2, 3], 42_000);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            owner.acquire_with(
                &mut PanicClock {
                    call: 0,
                    panic_at: 3,
                },
                || Ok(snapshot(99_000)),
            )
        }))
        .is_err()
    );
    assert!(!owner.shared.producer_live.load(Ordering::Acquire));
    assert_eq!(reader.read().err(), Some(CacheError::Poisoned));
    // A separate poisoned live state exercises the owner poison branch directly.
    let (mut owner, reader) = cache(ms(10));
    let shared = Arc::clone(&owner.shared);
    assert!(
        thread::spawn(move || {
            let _guard = shared.state.write().unwrap();
            panic!("fixture writer panic");
        })
        .join()
        .is_err()
    );
    assert_eq!(reader.read().err(), Some(CacheError::Poisoned));
    assert_eq!(
        owner.acquire_with(&mut FakeClock::at([1, 2, 3]), || panic!(
            "poisoned state scanned"
        )),
        Err(CacheError::Poisoned)
    );
    assert!(!owner.shared.producer_live.load(Ordering::Acquire));
}

#[test]
fn shared_state_is_released_after_the_last_handle_and_view() {
    let (mut owner, reader) = cache(ms(10));
    publish(&mut owner, [1, 2, 3], 42_000);
    let weak = Arc::downgrade(&owner.shared);
    let clone = reader.clone();
    let view = clone.read().unwrap();
    drop(owner);
    drop(reader);
    assert!(weak.upgrade().is_some());
    drop(view);
    drop(clone);
    assert!(weak.upgrade().is_none());
}
