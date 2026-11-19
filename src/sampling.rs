//! Finite foreground scans. Timing describes userspace observation, not hardware age.

use crate::hwmon::Snapshot;
use serde::Serialize;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    count: u32,
    interval_ms: u64,
}

impl Options {
    pub fn new(count: u32, interval_ms: u64) -> Result<Self, &'static str> {
        if !(1..=10_000).contains(&count) {
            return Err("--count must be between 1 and 10000");
        }
        if !(100..=60_000).contains(&interval_ms) {
            return Err("--interval-ms must be between 100 and 60000");
        }
        Ok(Self { count, interval_ms })
    }
}

#[derive(Debug, Serialize)]
pub struct Timing {
    pub target_start_ns: u64,
    pub started_ns: u64,
    pub finished_ns: u64,
    pub prepared_ns: u64,
    pub scan_duration_ns: u64,
    pub lateness_ns: u64,
    pub wall_start_unix_ms: Option<u64>,
    pub wall_time_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Freshness {
    pub basis: &'static str,
    pub lower_bound_ns: Option<u64>,
    pub upper_bound_ns: Option<u64>,
    pub observation_window_exceeds_interval: Option<bool>,
    pub physical_age_known: bool,
}

impl Freshness {
    fn for_snapshot(snapshot: Option<&Snapshot>, timing: &Timing, interval: Duration) -> Self {
        let basis = match snapshot {
            Some(snapshot) if !snapshot.sensors.is_empty() => "current_attempt_observation_window",
            Some(_) => "no_sensor_observations",
            None => "no_snapshot",
        };
        let has_observations = basis == "current_attempt_observation_window";
        let upper = timing.prepared_ns - timing.started_ns;
        Self {
            basis,
            lower_bound_ns: has_observations.then_some(timing.prepared_ns - timing.finished_ns),
            upper_bound_ns: has_observations.then_some(upper),
            observation_window_exceeds_interval: has_observations
                .then_some(Duration::from_nanos(upper) > interval),
            physical_age_known: false,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SampleError {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct Record {
    pub record_type: &'static str,
    pub sampling_schema_version: u8,
    pub sequence: u32,
    pub requested_count: u32,
    pub interval_ms: u64,
    pub timing: Timing,
    pub freshness: Freshness,
    pub snapshot: Option<Snapshot>,
    pub error: Option<SampleError>,
}

// Private seam for deterministic tests; production has one clock and one serial loop.
trait Clock {
    fn now(&mut self) -> io::Result<Duration>;
    fn sleep(&mut self, duration: Duration);
    fn wall_time(&mut self) -> Result<u64, String>;
}

struct SystemClock {
    origin: Instant,
}

impl Clock for SystemClock {
    fn now(&mut self) -> io::Result<Duration> {
        Instant::now()
            .checked_duration_since(self.origin)
            .ok_or_else(|| io::Error::other("monotonic clock precedes sampling origin"))
    }

    fn sleep(&mut self, duration: Duration) {
        std::thread::sleep(duration);
    }

    fn wall_time(&mut self) -> Result<u64, String> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?;
        u64::try_from(elapsed.as_millis())
            .map_err(|_| "wall timestamp exceeds u64 milliseconds".into())
    }
}

fn observe(clock: &mut impl Clock, previous: &mut Duration) -> io::Result<Duration> {
    let now = clock.now()?;
    if now < *previous {
        return Err(io::Error::other("monotonic clock moved backwards"));
    }
    *previous = now;
    Ok(now)
}

fn nanos(duration: Duration) -> io::Result<u64> {
    u64::try_from(duration.as_nanos())
        .map_err(|_| io::Error::other("sampling timestamp exceeds u64 nanoseconds"))
}

/// Complete a bounded number of attempts, including recoverable scan failures.
/// The sink must consume each record synchronously; the CLI never keeps history.
/// Blocking reads/output can delay completion indefinitely. No deadline is promised.
pub fn run(
    root: &Path,
    options: Options,
    emit: impl FnMut(Record) -> io::Result<()>,
) -> io::Result<()> {
    run_with(
        options,
        &mut SystemClock {
            origin: Instant::now(),
        },
        || crate::scan(root),
        emit,
    )
}

fn run_with(
    options: Options,
    clock: &mut impl Clock,
    mut scan: impl FnMut() -> io::Result<Snapshot>,
    mut emit: impl FnMut(Record) -> io::Result<()>,
) -> io::Result<()> {
    let interval = Duration::from_millis(options.interval_ms);
    let mut last_observation = Duration::ZERO;
    let mut target = Duration::ZERO;
    for sequence in 1..=options.count {
        let mut started = observe(clock, &mut last_observation)?;
        while started < target {
            clock.sleep(target - started);
            started = observe(clock, &mut last_observation)?;
        }
        let started_ns = nanos(started)?;
        let target_start_ns = nanos(target)?;
        let (wall_start_unix_ms, wall_time_error) = match clock.wall_time() {
            Ok(timestamp) => (Some(timestamp), None),
            Err(error) => (None, Some(error)),
        };
        let result = scan();
        let finished = observe(clock, &mut last_observation)?;
        let (snapshot, error) = match result {
            Ok(snapshot) => (Some(snapshot), None),
            Err(error) => (
                None,
                Some(SampleError {
                    kind: format!("{:?}", error.kind()),
                    message: error.to_string(),
                }),
            ),
        };
        let prepared = observe(clock, &mut last_observation)?;
        let timing = Timing {
            target_start_ns,
            started_ns,
            finished_ns: nanos(finished)?,
            prepared_ns: nanos(prepared)?,
            scan_duration_ns: nanos(finished - started)?,
            lateness_ns: nanos(started - target)?,
            wall_start_unix_ms,
            wall_time_error,
        };
        let freshness = Freshness::for_snapshot(snapshot.as_ref(), &timing, interval);
        // Ownership passes to the sink; no previous snapshot survives in the loop.
        emit(Record {
            record_type: "siliconwatch.sample",
            sampling_schema_version: 1,
            sequence,
            requested_count: options.count,
            interval_ms: options.interval_ms,
            timing,
            freshness,
            snapshot,
            error,
        })?;
        if sequence < options.count {
            target = started
                .checked_add(interval)
                .ok_or_else(|| io::Error::other("sampling deadline overflow"))?;
        }
    }
    Ok(())
}

/// Buffer a single JSON line. Serialization failures become explicit attempt errors;
/// transport failures propagate and stop the loop, including broken pipes.
pub fn write_json_line(mut record: Record, out: &mut impl Write) -> io::Result<()> {
    let mut bytes = match serde_json::to_vec(&record) {
        Ok(bytes) => bytes,
        Err(error) => {
            record.snapshot = None;
            record.error = Some(SampleError {
                kind: "serialization_error".into(),
                message: error.to_string(),
            });
            record.freshness = Freshness::for_snapshot(
                None,
                &record.timing,
                Duration::from_millis(record.interval_ms),
            );
            serde_json::to_vec(&record)?
        }
    };
    bytes.push(b'\n');
    out.write_all(&bytes)?;
    out.flush()
}

#[cfg(test)]
mod tests;
