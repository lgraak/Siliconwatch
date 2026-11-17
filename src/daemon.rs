//! Foreground lifecycle only: one cache owner, cooperative shutdown, no read API.
//! A stop request closes admission; a previously admitted scan may still publish.
//! Blocking reads, borrowed cache views and output can delay joining indefinitely.

use crate::snapshot_cache::{self, Acquisition, CacheError, CacheOwner, CacheReader};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    interval: Duration,
    stale_after: Duration,
}

impl Options {
    pub fn new(interval_ms: u64, stale_after_ms: u64) -> Result<Self, &'static str> {
        if !(100..=60_000).contains(&interval_ms) {
            return Err("--interval-ms must be between 100 and 60000");
        }
        if stale_after_ms == 0 {
            return Err("--stale-after-ms must be positive");
        }
        Ok(Self {
            interval: Duration::from_millis(interval_ms),
            stale_after: Duration::from_millis(stale_after_ms),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Starting,
    Running,
    Stopping,
    Stopped,
}

struct State {
    phase: Phase,
    admitted: bool,
}

struct Control {
    state: Mutex<State>,
    wake: Condvar,
}

impl Control {
    fn new() -> Self {
        Self {
            state: Mutex::new(State {
                phase: Phase::Starting,
                admitted: false,
            }),
            wake: Condvar::new(),
        }
    }

    fn lock(&self) -> io::Result<MutexGuard<'_, State>> {
        self.state
            .lock()
            .map_err(|_| io::Error::other("daemon lifecycle lock is poisoned"))
    }

    fn start(&self) -> io::Result<bool> {
        let mut state = self.lock()?;
        match state.phase {
            Phase::Starting => {
                state.phase = Phase::Running;
                Ok(true)
            }
            Phase::Stopping => Ok(false),
            _ => Err(io::Error::other("invalid daemon startup state")),
        }
    }

    fn stop(&self) -> io::Result<()> {
        let result = self.lock().map(|mut state| {
            if state.phase != Phase::Stopped {
                state.phase = Phase::Stopping;
            }
        });
        // Even a poisoned lock must not leave a sleeping worker unwoken.
        self.wake.notify_all();
        result
    }

    // Only the coordinator calls this, after join (or when no worker was spawned).
    fn stopped(&self) -> io::Result<()> {
        let mut state = self.lock()?;
        if state.phase != Phase::Stopping {
            return Err(io::Error::other("daemon stopped before stopping"));
        }
        state.admitted = false;
        state.phase = Phase::Stopped;
        Ok(())
    }
}

// Private seam for deterministic scheduling tests, not a provider abstraction.
trait Timing {
    fn now(&mut self) -> io::Result<Duration>;
    fn wait<'a>(
        &mut self,
        control: &Control,
        state: MutexGuard<'a, State>,
        remaining: Duration,
    ) -> io::Result<MutexGuard<'a, State>>;
}

struct Monotonic {
    origin: Instant,
}

impl Timing for Monotonic {
    fn now(&mut self) -> io::Result<Duration> {
        Instant::now()
            .checked_duration_since(self.origin)
            .ok_or_else(|| io::Error::other("daemon clock precedes its origin"))
    }

    fn wait<'a>(
        &mut self,
        control: &Control,
        state: MutexGuard<'a, State>,
        remaining: Duration,
    ) -> io::Result<MutexGuard<'a, State>> {
        control
            .wake
            .wait_timeout(state, remaining)
            .map(|(state, _)| state)
            .map_err(|_| io::Error::other("daemon lifecycle lock is poisoned"))
    }
}

fn observe(timing: &mut impl Timing, previous: &mut Duration) -> io::Result<Duration> {
    let now = timing.now()?;
    now.checked_sub(*previous)
        .ok_or_else(|| io::Error::other("daemon monotonic clock moved backwards"))?;
    *previous = now;
    Ok(now)
}

fn acquire_loop(
    control: &Control,
    interval: Duration,
    timing: &mut impl Timing,
    mut acquire: impl FnMut() -> Result<Acquisition, CacheError>,
    announce: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    if !control.start()? {
        return Ok(());
    }
    // No coordinator readiness wait: main can already process signals. This line
    // records startup, not current health, and may race a later stop request.
    announce()?;
    let mut previous = Duration::ZERO;
    let mut deadline = Duration::ZERO;
    loop {
        let mut state = control.lock()?;
        loop {
            match state.phase {
                Phase::Stopping => return Ok(()),
                Phase::Running => (),
                _ => return Err(io::Error::other("invalid daemon acquisition state")),
            }
            let now = observe(timing, &mut previous)?;
            if now >= deadline {
                break;
            }
            state = timing.wait(control, state, deadline - now)?;
        }
        // Admission and stop acknowledgment share this mutex. Once admitted,
        // this attempt may begin/finish/publish after Stopping, by contract.
        state.admitted = true;
        drop(state);
        let dispatched = observe(timing, &mut previous)?;
        let acquired = acquire().map(|_| ()).map_err(io::Error::other);
        let cleared = control.lock().map(|mut state| state.admitted = false);
        let mut result = acquired;
        append_error(&mut result, cleared);
        result?;
        deadline = dispatched
            .checked_add(interval)
            .ok_or_else(|| io::Error::other("daemon scheduling deadline overflow"))?;
    }
}

// Fixed call sites, never per-attempt accumulation or an error history.
fn append_error(result: &mut io::Result<()>, next: io::Result<()>) {
    if let Err(next) = next {
        *result = Err(match result {
            Ok(()) => next,
            Err(first) => io::Error::new(first.kind(), format!("{first}; additionally: {next}")),
        });
    }
}

struct OnExit<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for OnExit<F> {
    fn drop(&mut self) {
        if let Some(close) = self.0.take() {
            close();
        }
    }
}

fn worker(
    control: &Control,
    options: Options,
    mut owner: CacheOwner,
    close: impl FnOnce(),
    announce: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    // Install before startup/output/clock operations. Neither closing nor owner
    // Drop needs the lifecycle mutex, including while unwinding after a panic.
    let _close = OnExit(Some(close));
    let mut result = acquire_loop(
        control,
        options.interval,
        &mut Monotonic {
            origin: Instant::now(),
        },
        || owner.acquire_once(),
        announce,
    );
    owner.stop();
    append_error(&mut result, control.stop());
    result
}

#[derive(Debug)]
struct Inspection {
    phase: Phase,
    latest: &'static str,
    observation: &'static str,
    current: bool,
}

fn inspect(reader: &CacheReader, control: &Control) -> io::Result<Inspection> {
    // Cache-first, then control; no path may take these in reverse order.
    let mut view = reader.read().map_err(io::Error::other)?;
    let state = control.lock()?;
    let assessment = view.assess().map_err(io::Error::other)?;
    Ok(classify(assessment, state.phase))
}

fn classify(assessment: snapshot_cache::Assessment<'_>, phase: Phase) -> Inspection {
    let latest = match assessment.latest {
        snapshot_cache::LatestAttempt::Empty => "none",
        snapshot_cache::LatestAttempt::Succeeded(_) => "succeeded",
        snapshot_cache::LatestAttempt::Failed { .. } => "failed",
    };
    let mut current = false;
    let observation = match assessment.observation {
        None => "none",
        Some(observation) => {
            current = phase == Phase::Running
                && assessment.producer == snapshot_cache::ProducerState::Active
                && latest == "succeeded"
                && observation.role == snapshot_cache::ObservationRole::LatestAttempt
                && matches!(
                    observation.freshness,
                    snapshot_cache::Freshness::WithinBudget(_)
                );
            if observation.role != snapshot_cache::ObservationRole::LatestAttempt
                || phase != Phase::Running
            {
                "historical"
            } else {
                match observation.freshness {
                    snapshot_cache::Freshness::NoSensorObservations => "empty",
                    snapshot_cache::Freshness::Stale(_) => "stale",
                    snapshot_cache::Freshness::WithinBudget(_) => "within-budget",
                }
            }
        }
    };
    Inspection {
        phase,
        latest,
        observation,
        current,
    }
}

fn finish(
    control: &Control,
    reader: &CacheReader,
    handle: JoinHandle<io::Result<()>>,
    coordinator: io::Result<()>,
    out: &mut impl Write,
) -> io::Result<()> {
    // No early return until the worker has been joined, even after poisoning.
    let stopped = control.stop();
    let mut result = handle
        .join()
        .unwrap_or_else(|_| Err(io::Error::other("daemon acquisition worker panicked")));
    // Keep the worker's original fatal cause ahead of a concurrent stop/output error.
    append_error(&mut result, coordinator);
    append_error(&mut result, stopped);
    let lifecycle = control.stopped();
    let state_available = lifecycle.is_ok();
    append_error(&mut result, lifecycle);
    let inspection = inspect(reader, control);
    if let Err(error) = &inspection {
        append_error(&mut result, Err(io::Error::other(error.to_string())));
    }
    let outcome = if result.is_ok() { "graceful" } else { "failed" };
    let written = if state_available {
        if let Ok(status) = inspection {
            debug_assert_eq!(status.phase, Phase::Stopped);
            writeln!(
                out,
                "SiliconWatch daemon stopped; outcome: {outcome}; latest attempt: {}; observation: {}; service-current: {}",
                status.latest, status.observation, status.current
            )
        } else {
            writeln!(
                out,
                "SiliconWatch daemon stopped; outcome: {outcome}; cache unavailable"
            )
        }
    } else {
        writeln!(
            out,
            "SiliconWatch daemon lifecycle unavailable; worker joined; outcome: failed"
        )
    };
    append_error(&mut result, written.and_then(|()| out.flush()));
    result
}

fn coordinate(
    control: &Control,
    reader: &CacheReader,
    handle: JoinHandle<io::Result<()>>,
    wait: impl FnOnce(),
    close: impl FnOnce(),
    out: &mut impl Write,
) -> io::Result<()> {
    // signal-hook's iterator can panic on a wait I/O error. Do not detach a live
    // worker in that case. The iterator is never resumed after this unwind.
    let mut coordinator = std::panic::catch_unwind(std::panic::AssertUnwindSafe(wait))
        .map_err(|_| io::Error::other("daemon signal wait panicked"));
    append_error(&mut coordinator, control.stop());
    close();
    finish(control, reader, handle, coordinator, out)
}

// Setup failure injection stays private; no signal/thread factory is public.
fn spawn_or_stop(
    control: &Control,
    spawn: impl FnOnce() -> io::Result<JoinHandle<io::Result<()>>>,
) -> io::Result<JoinHandle<io::Result<()>>> {
    match spawn() {
        Ok(handle) => Ok(handle),
        Err(error) => match control.stop().and_then(|()| control.stopped()) {
            Ok(()) => Err(error),
            Err(cleanup) => Err(io::Error::new(
                error.kind(),
                format!("{error}; additionally: {cleanup}"),
            )),
        },
    }
}

/// Run in the foreground until SIGINT/SIGTERM or fatal failure. Signal handlers
/// are process-global: call from the CLI, not concurrently or from a library host.
/// Reads only the supplied hwmon root. Never installs a service or controls hardware.
pub fn run(root: &Path, options: Options) -> io::Result<()> {
    run_registered(root, options, || Signals::new([SIGINT, SIGTERM]))
}

fn run_registered(
    root: &Path,
    options: Options,
    register: impl FnOnce() -> io::Result<Signals>,
) -> io::Result<()> {
    // Register before any worker exists. No application locks or output in handlers.
    let mut signals = register()?;
    let close = signals.handle();
    let (owner, reader) =
        snapshot_cache::new(root, options.stale_after).map_err(io::Error::other)?;
    let control = Arc::new(Control::new());
    let worker_control = Arc::clone(&control);
    let worker_close = close.clone();
    let handle = match spawn_or_stop(&control, || {
        thread::Builder::new()
            .name("siliconwatch-acquisition".into())
            .spawn(move || {
                worker(
                    &worker_control,
                    options,
                    owner,
                    || worker_close.close(),
                    || {
                        let mut out = io::stderr().lock();
                        writeln!(out, "SiliconWatch daemon running")?;
                        out.flush()
                    },
                )
            })
    }) {
        Ok(handle) => handle,
        Err(error) => {
            close.close();
            return Err(error);
        }
    };
    // The worker closes this wait on failure/return/unwind. No competing readiness
    // wait exists, and no stdout/cache/control lock is held while awaiting signals.
    // Keep `signals` (and hence registration) alive throughout join: later requests
    // remain idempotent. Close wakes the iterator, not a blocked acquisition.
    let result = coordinate(
        &control,
        &reader,
        handle,
        || {
            let _signal = signals.forever().next();
        },
        || close.close(),
        &mut io::stderr(),
    );
    drop(signals);
    result
}

#[cfg(test)]
mod tests;
