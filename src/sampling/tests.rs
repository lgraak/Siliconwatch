use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fs;
use std::rc::Rc;

#[path = "../../tests/common/mod.rs"]
mod common;
use common::Fixture;

#[derive(Default)]
struct FakeClock {
    time: Rc<Cell<Duration>>,
    observations: VecDeque<Duration>,
    wake_after: VecDeque<Duration>,
    sleeps: Vec<Duration>,
    wall: VecDeque<Result<u64, String>>,
}

impl Clock for FakeClock {
    fn now(&mut self) -> io::Result<Duration> {
        if let Some(time) = self.observations.pop_front() {
            self.time.set(time);
        }
        Ok(self.time.get())
    }

    fn sleep(&mut self, duration: Duration) {
        self.sleeps.push(duration);
        self.time
            .set(self.time.get() + self.wake_after.pop_front().unwrap_or(duration));
    }

    fn wall_time(&mut self) -> Result<u64, String> {
        self.wall.pop_front().unwrap_or(Ok(1234))
    }
}

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn empty() -> Snapshot {
    Snapshot {
        schema_version: 2,
        root: "/fixture".into(),
        sensors: Vec::new(),
        warnings: Vec::new(),
    }
}

#[test]
fn option_bounds_include_both_endpoints() {
    for count in [1, 10_000] {
        for interval in [100, 60_000] {
            assert!(Options::new(count, interval).is_ok());
        }
    }
    for count in [0, 10_001, u32::MAX] {
        assert!(Options::new(count, 100).is_err());
    }
    for interval in [0, 99, 60_001, u64::MAX] {
        assert!(Options::new(1, interval).is_err());
    }
}

#[test]
fn scans_and_output_are_sequential_with_no_final_sleep() {
    let mut clock = FakeClock::default();
    let time = clock.time.clone();
    let events = RefCell::new(Vec::new());
    let mut records = Vec::new();
    run_with(
        Options::new(3, 100).unwrap(),
        &mut clock,
        || {
            events.borrow_mut().push("scan start");
            time.set(time.get() + ms(20));
            events.borrow_mut().push("scan end");
            Ok(empty())
        },
        |record| {
            events.borrow_mut().push("emit start");
            time.set(time.get() + ms(5));
            records.push(record);
            events.borrow_mut().push("emit end");
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        *events.borrow(),
        ["scan start", "scan end", "emit start", "emit end"].repeat(3)
    );
    assert_eq!(clock.sleeps, [ms(75), ms(75)]);
    assert_eq!(time.get(), ms(225));
    for (i, record) in records.iter().enumerate() {
        assert_eq!(record.sequence, i as u32 + 1);
        assert_eq!(record.timing.started_ns, i as u64 * 100_000_000);
        assert_eq!(record.timing.lateness_ns, 0);
        assert_eq!(record.timing.scan_duration_ns, 20_000_000);
    }
}

#[test]
fn slow_scan_and_output_reanchor_without_catch_up() {
    let mut clock = FakeClock::default();
    let time = clock.time.clone();
    let mut scans = 0;
    let mut records = Vec::new();
    run_with(
        Options::new(3, 100).unwrap(),
        &mut clock,
        || {
            scans += 1;
            time.set(time.get() + ms(if scans == 1 { 250 } else { 10 }));
            Ok(empty())
        },
        |record| {
            if record.sequence == 1 {
                time.set(time.get() + ms(50));
            }
            records.push(record);
            Ok(())
        },
    )
    .unwrap();
    let starts: Vec<_> = records.iter().map(|r| r.timing.started_ns).collect();
    assert_eq!(starts, [0, 300_000_000, 400_000_000]);
    assert_eq!(records[1].timing.target_start_ns, 100_000_000);
    assert_eq!(records[1].timing.lateness_ns, 200_000_000);
    assert_eq!(records[2].timing.lateness_ns, 0);
    assert_eq!(clock.sleeps, [ms(90)]);
}

#[test]
fn early_wakeup_rechecks_and_oversleep_is_explicit() {
    let mut clock = FakeClock {
        wake_after: [ms(50), ms(70)].into(),
        ..FakeClock::default()
    };
    let mut records = Vec::new();
    run_with(
        Options::new(3, 100).unwrap(),
        &mut clock,
        || Ok(empty()),
        |r| {
            records.push(r);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(clock.sleeps, [ms(100), ms(50), ms(100)]);
    assert_eq!(records[1].timing.started_ns, 120_000_000);
    assert_eq!(records[1].timing.lateness_ns, 20_000_000);
    assert_eq!(records[2].timing.started_ns, 220_000_000);
}

#[test]
fn changing_wall_time_does_not_change_monotonic_schedule() {
    let mut clock = FakeClock {
        wall: [
            Ok(10_000),
            Ok(9_000),
            Ok(999_999),
            Err("before epoch".into()),
        ]
        .into(),
        ..FakeClock::default()
    };
    let mut records = Vec::new();
    run_with(
        Options::new(4, 100).unwrap(),
        &mut clock,
        || Ok(empty()),
        |r| {
            records.push(r);
            Ok(())
        },
    )
    .unwrap();
    for (i, r) in records.iter().enumerate() {
        assert_eq!(r.timing.started_ns, i as u64 * 100_000_000);
        assert_eq!(r.timing.lateness_ns, 0);
    }
    assert_eq!(records[1].timing.wall_start_unix_ms, Some(9_000));
    assert_eq!(records[3].timing.wall_start_unix_ms, None);
    assert_eq!(
        records[3].timing.wall_time_error.as_deref(),
        Some("before epoch")
    );
    assert!(records[3].snapshot.is_some());
}

#[test]
fn freshness_bounds_are_observation_windows_not_physical_age() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "test");
    fixture.write("hwmon0/temp1_input", "42000");
    for (finished, prepared, exceeds) in [(120, 125, true), (100, 100, false), (40, 45, false)] {
        let mut clock = FakeClock {
            observations: [ms(0), ms(finished), ms(prepared)].into(),
            ..FakeClock::default()
        };
        run_with(
            Options::new(1, 100).unwrap(),
            &mut clock,
            || hwmon::scan(&fixture.root),
            |r| {
                assert_eq!(r.timing.scan_duration_ns, finished * 1_000_000);
                assert_eq!(
                    r.freshness.lower_bound_ns,
                    Some((prepared - finished) * 1_000_000)
                );
                assert_eq!(r.freshness.upper_bound_ns, Some(prepared * 1_000_000));
                assert_eq!(
                    r.freshness.observation_window_exceeds_interval,
                    Some(exceeds)
                );
                assert_eq!(r.freshness.basis, "current_attempt_observation_window");
                assert!(!r.freshness.physical_age_known);
                assert_eq!(r.snapshot.unwrap().sensors[0].raw_value, Some(42000));
                Ok(())
            },
        )
        .unwrap();
        assert!(clock.sleeps.is_empty());
    }
}

#[test]
fn missing_root_consumes_attempt_and_recovers_without_old_values() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "test");
    fixture.write("hwmon0/temp1_input", "42000");
    let mut attempt = 0;
    let mut records = Vec::new();
    run_with(
        Options::new(4, 100).unwrap(),
        &mut FakeClock::default(),
        || {
            attempt += 1;
            match attempt {
                2 => fs::remove_dir_all(&fixture.root).unwrap(),
                3 => {
                    fixture.write("hwmon0/name", "test");
                    fixture.write("hwmon0/temp1_input", "55000");
                }
                4 => fs::remove_file(fixture.root.join("hwmon0/temp1_input")).unwrap(),
                _ => {}
            }
            hwmon::scan(&fixture.root)
        },
        |r| {
            records.push(r);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(
        records[0].snapshot.as_ref().unwrap().sensors[0].value,
        Some(42.0)
    );
    assert!(records[1].snapshot.is_none());
    assert_eq!(records[1].error.as_ref().unwrap().kind, "NotFound");
    assert_eq!(records[1].freshness.basis, "no_snapshot");
    assert_eq!(records[1].freshness.upper_bound_ns, None);
    assert_eq!(
        records[2].snapshot.as_ref().unwrap().sensors[0].value,
        Some(55.0)
    );
    assert!(records[3].snapshot.as_ref().unwrap().sensors.is_empty());
    assert_eq!(records[3].freshness.basis, "no_sensor_observations");
    assert_eq!(
        records[3].freshness.observation_window_exceeds_interval,
        None
    );
}

#[test]
fn partial_sensor_failures_keep_statuses_and_raw_data() {
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "test");
    fixture.write("hwmon0/temp1_input", "invalid");
    fixture.write("hwmon0/power1_input", "123");
    run_with(
        Options::new(2, 100).unwrap(),
        &mut FakeClock::default(),
        || hwmon::scan(&fixture.root),
        |r| {
            assert!(r.error.is_none());
            let sensors = r.snapshot.unwrap().sensors;
            assert_eq!(sensors.len(), 2);
            let temp = sensors
                .iter()
                .find(|s| s.attribute == "temp1_input")
                .unwrap();
            assert_eq!(temp.status, hwmon::Status::Invalid);
            assert_eq!(temp.value, None);
            let power = sensors
                .iter()
                .find(|s| s.attribute == "power1_input")
                .unwrap();
            assert_eq!(power.status, hwmon::Status::Unsupported);
            assert_eq!(power.raw_value, Some(123));
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn backwards_monotonic_observations_and_overflow_stop_explicitly() {
    for observations in [
        vec![ms(10), ms(20), ms(15)],
        vec![ms(10), ms(20), ms(20), ms(15)],
        vec![Duration::MAX],
    ] {
        let mut clock = FakeClock {
            observations: observations.into(),
            ..FakeClock::default()
        };
        let mut attempts = 0;
        let error = run_with(
            Options::new(2, 100).unwrap(),
            &mut clock,
            || {
                attempts += 1;
                Ok(empty())
            },
            |_| Ok(()),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert!(attempts <= 1);
    }
}

#[derive(Default)]
struct Sink {
    bytes: Vec<u8>,
    flushes: usize,
    write_error: Option<io::ErrorKind>,
    fail_flush: bool,
}

impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(kind) = self.write_error {
            return Err(io::Error::from(kind));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.fail_flush {
            return Err(io::Error::other("injected flush failure"));
        }
        Ok(())
    }
}

#[test]
fn lines_are_flushed_before_next_scan_and_remain_independently_parseable() {
    let sink = RefCell::new(Sink::default());
    let mut attempts = 0;
    run_with(
        Options::new(3, 100).unwrap(),
        &mut FakeClock::default(),
        || {
            assert_eq!(sink.borrow().flushes, attempts);
            attempts += 1;
            if attempts == 2 {
                Err(io::ErrorKind::PermissionDenied.into())
            } else {
                Ok(empty())
            }
        },
        |r| write_json_line(r, &mut *sink.borrow_mut()),
    )
    .unwrap();
    let sink = sink.into_inner();
    assert_eq!(sink.flushes, 3);
    let text = String::from_utf8(sink.bytes).unwrap();
    assert_eq!(text.lines().count(), 3);
    for (i, line) in text.lines().enumerate() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["sampling_schema_version"], 1);
        assert_eq!(value["sequence"], i + 1);
        assert_ne!(value["snapshot"].is_null(), value["error"].is_null());
        if i != 1 {
            assert_eq!(value["snapshot"]["schema_version"], 2);
        }
    }
}

#[test]
fn serialization_failure_emits_only_error_then_recovers() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    let fixture = Fixture::new();
    fixture.write("hwmon0/name", "test");
    fixture.write("hwmon0/temp1_input", "42000");
    let mut attempt = 0;
    let mut sink = Sink::default();
    run_with(
        Options::new(2, 100).unwrap(),
        &mut FakeClock::default(),
        || {
            attempt += 1;
            let mut snapshot = hwmon::scan(&fixture.root)?;
            if attempt == 1 {
                snapshot.root = OsString::from_vec(vec![0xff]).into();
            }
            Ok(snapshot)
        },
        |r| write_json_line(r, &mut sink),
    )
    .unwrap();
    let lines: Vec<serde_json::Value> = String::from_utf8(sink.bytes)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0]["snapshot"].is_null());
    assert_eq!(lines[0]["error"]["kind"], "serialization_error");
    assert_eq!(lines[0]["freshness"]["basis"], "no_snapshot");
    assert!(lines[0]["freshness"]["upper_bound_ns"].is_null());
    assert!(lines[1]["error"].is_null());
    assert_eq!(lines[1]["snapshot"]["sensors"][0]["raw_value"], 42000);
    assert_eq!(sink.flushes, 2);
}

#[test]
fn write_and_flush_failures_stop_without_another_scan() {
    for (kind, fail_flush) in [
        (Some(io::ErrorKind::BrokenPipe), false),
        (Some(io::ErrorKind::WriteZero), false),
        (None, true),
    ] {
        let mut sink = Sink {
            write_error: kind,
            fail_flush,
            ..Sink::default()
        };
        let mut scans = 0;
        let result = run_with(
            Options::new(3, 100).unwrap(),
            &mut FakeClock::default(),
            || {
                scans += 1;
                Ok(empty())
            },
            |r| write_json_line(r, &mut sink),
        );
        assert!(result.is_err());
        assert_eq!(scans, 1);
    }
}

#[test]
fn maximum_count_streams_to_discarding_sink_without_history() {
    let mut scans = 0;
    let mut emitted = 0;
    run_with(
        Options::new(10_000, 100).unwrap(),
        &mut FakeClock::default(),
        || {
            scans += 1;
            Ok(empty())
        },
        |r| {
            emitted += 1;
            assert_eq!(r.sequence, emitted);
            write_json_line(r, &mut io::sink())
        },
    )
    .unwrap();
    assert_eq!(scans, 10_000);
    assert_eq!(emitted, 10_000);
}
