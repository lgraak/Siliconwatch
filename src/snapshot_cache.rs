//! One acquisition owner and borrowed views of the latest published attempt.
//!
//! Read guards must be short lived: release them before acquisition, recursive
//! reads, blocking I/O, or waiting for the owner. A guard can delay publication
//! indefinitely; this module promises neither writer fairness nor cancellation.
//! Acquisition runs outside the state lock. No thread or scheduler is created.
//!
//! One observation is retained, plus one incoming replacement. This is a payload
//! retention bound, not a byte/RSS limit: scans, errors, and caller copies vary in
//! size. Hardware age is unknown. After known suspend, stop this cache and create
//! another; `Instant` does not promise suspend-aware elapsed time.

use crate::hwmon::Snapshot;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheError {
    InvalidThreshold,
    Stopped,
    Poisoned,
    InvalidClock,
    SequenceExhausted,
}

impl fmt::Display for CacheError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(match self {
            Self::InvalidThreshold => "cache freshness threshold must be positive",
            Self::Stopped => "cache acquisition owner is stopped",
            Self::Poisoned => "cache publication lock is poisoned",
            Self::InvalidClock => "invalid cache monotonic observation order",
            Self::SequenceExhausted => "cache attempt sequence exhausted",
        })
    }
}

impl std::error::Error for CacheError {}

/// Offsets share this cache's origin; they are not comparable across caches.
#[derive(Debug)]
pub struct AttemptMetadata {
    pub sequence: u64,
    pub started: Duration,
    pub finished: Duration,
    pub published: Duration,
    pub wall_start_unix_ms: Option<u64>,
    pub wall_time_error: Option<String>,
}

#[derive(Debug)]
pub struct AcquisitionError {
    pub kind: io::ErrorKind,
    pub raw_os_error: Option<i32>,
    pub message: String,
}

/// `Failed` is a published scan failure, not a fatal publication/control error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acquisition {
    Succeeded(u64),
    Failed(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerState {
    Active,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationRole {
    LatestAttempt,
    HistoricalAfterFailure,
    HistoricalAfterStop,
}

/// Userspace observation-age bounds, never physical sensor age.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgeBounds {
    pub lower: Duration,
    pub upper: Duration,
}

/// Numeric age does not override historical role or individual sensor status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    NoSensorObservations,
    WithinBudget(AgeBounds),
    Stale(AgeBounds),
}

#[derive(Debug)]
pub enum LatestAttempt<'a> {
    Empty,
    Succeeded(&'a AttemptMetadata),
    Failed {
        attempt: &'a AttemptMetadata,
        error: &'a AcquisitionError,
    },
}

#[derive(Debug)]
pub struct Observation<'a> {
    pub snapshot: &'a Snapshot,
    pub attempt: &'a AttemptMetadata,
    pub role: ObservationRole,
    pub freshness: Freshness,
}

/// A point-in-time assessment, not a continuing freshness/liveness guarantee.
/// Always interpret observation role, age and row diagnostics together.
#[derive(Debug)]
pub struct Assessment<'a> {
    pub assessed_at: Duration,
    pub producer: ProducerState,
    pub latest: LatestAttempt<'a>,
    pub observation: Option<Observation<'a>>,
    pub physical_age_known: bool,
}

struct ObservedSnapshot {
    attempt: AttemptMetadata,
    snapshot: Snapshot,
}

enum PublishedState {
    Empty,
    Succeeded(ObservedSnapshot),
    Failed {
        attempt: AttemptMetadata,
        error: AcquisitionError,
        previous: Option<ObservedSnapshot>,
    },
}

impl PublishedState {
    fn published(&self) -> Duration {
        match self {
            Self::Empty => Duration::ZERO,
            Self::Succeeded(observed) => observed.attempt.published,
            Self::Failed { attempt, .. } => attempt.published,
        }
    }

    fn take_observation(&mut self) -> Option<ObservedSnapshot> {
        match self {
            Self::Empty => None,
            Self::Failed { previous, .. } => previous.take(),
            Self::Succeeded(_) => match std::mem::replace(self, Self::Empty) {
                Self::Succeeded(observed) => Some(observed),
                _ => unreachable!(),
            },
        }
    }
}

struct Shared {
    state: RwLock<PublishedState>,
    origin: Instant,
    stale_after: Duration,
    producer_live: AtomicBool,
}

impl Shared {
    fn elapsed(&self) -> Result<Duration, CacheError> {
        Instant::now()
            .checked_duration_since(self.origin)
            .ok_or(CacheError::InvalidClock)
    }
}

/// Unique writer. All acquisition methods require exclusive access.
///
/// ```compile_fail
/// use siliconwatch::snapshot_cache;
/// use std::time::Duration;
/// let (owner, _) = snapshot_cache::new(".", Duration::from_secs(1)).unwrap();
/// let second_owner = owner.clone();
/// ```
pub struct CacheOwner {
    root: PathBuf,
    shared: Arc<Shared>,
    sequence: u64,
    last_published: Duration,
}

/// Cloneable read-only handle; cloning does not retain old snapshot generations.
#[derive(Clone)]
pub struct CacheReader {
    shared: Arc<Shared>,
}

/// Borrowed state; drop the view before any operation that could wait on a writer.
/// No snapshot reference can outlive this guard.
///
/// ```compile_fail
/// use siliconwatch::snapshot_cache;
/// use std::time::Duration;
/// let (_, reader) = snapshot_cache::new(".", Duration::from_secs(1)).unwrap();
/// let snapshot = {
///     let mut view = reader.read().unwrap();
///     view.assess().unwrap().observation.unwrap().snapshot
/// };
/// println!("{snapshot:?}");
/// ```
pub struct ReadView<'a> {
    guard: RwLockReadGuard<'a, PublishedState>,
    shared: &'a Shared,
    last_assessed: Duration,
}

/// Create an empty cache. Does not inspect the root or initiate hardware access.
pub fn new(
    root: impl Into<PathBuf>,
    stale_after: Duration,
) -> Result<(CacheOwner, CacheReader), CacheError> {
    if stale_after.is_zero() {
        return Err(CacheError::InvalidThreshold);
    }
    let shared = Arc::new(Shared {
        state: RwLock::new(PublishedState::Empty),
        origin: Instant::now(),
        stale_after,
        producer_live: AtomicBool::new(true),
    });
    Ok((
        CacheOwner {
            root: root.into(),
            shared: Arc::clone(&shared),
            sequence: 0,
            last_published: Duration::ZERO,
        },
        CacheReader { shared },
    ))
}

// Private timing/scan seam; production always uses the real reader and clock.
trait Clock {
    fn now(&mut self) -> Result<Duration, CacheError>;
    fn wall_time(&mut self) -> Result<u64, String>;
}

struct SystemClock {
    origin: Instant,
}

// Invalidate currentness even if a caller catches an acquisition panic and
// retains the owner. No lock is taken during unwinding.
struct AttemptGuard {
    shared: Arc<Shared>,
    published: bool,
}

impl Drop for AttemptGuard {
    fn drop(&mut self) {
        if !self.published {
            self.shared.producer_live.store(false, Ordering::Release);
        }
    }
}

impl Clock for SystemClock {
    fn now(&mut self) -> Result<Duration, CacheError> {
        Instant::now()
            .checked_duration_since(self.origin)
            .ok_or(CacheError::InvalidClock)
    }

    fn wall_time(&mut self) -> Result<u64, String> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| error.to_string())?;
        u64::try_from(elapsed.as_millis())
            .map_err(|_| "wall timestamp exceeds u64 milliseconds".into())
    }
}

fn observe(clock: &mut impl Clock, previous: &mut Duration) -> Result<Duration, CacheError> {
    let now = clock.now()?;
    now.checked_sub(*previous).ok_or(CacheError::InvalidClock)?;
    *previous = now;
    Ok(now)
}

impl CacheOwner {
    /// Scan once and publish. Completes only after obtaining the write lock.
    /// `Ok(Acquisition::Failed(_))` means a scan failure was published, never
    /// that acquisition succeeded. Fatal control errors stop the owner.
    /// An `Ok(Snapshot)` from the scanner retains every row/error, even if empty.
    /// Until publication, readers still see the preceding complete attempt.
    pub fn acquire_once(&mut self) -> Result<Acquisition, CacheError> {
        let root = self.root.clone();
        self.acquire_with(
            &mut SystemClock {
                origin: self.shared.origin,
            },
            || crate::scan(&root),
        )
    }

    fn acquire_with(
        &mut self,
        clock: &mut impl Clock,
        scan: impl FnOnce() -> io::Result<Snapshot>,
    ) -> Result<Acquisition, CacheError> {
        let mut guard = AttemptGuard {
            shared: Arc::clone(&self.shared),
            published: false,
        };
        let result = self.acquire_inner(clock, scan);
        guard.published = result.is_ok();
        result
    }

    fn acquire_inner(
        &mut self,
        clock: &mut impl Clock,
        scan: impl FnOnce() -> io::Result<Snapshot>,
    ) -> Result<Acquisition, CacheError> {
        if !self.shared.producer_live.load(Ordering::Acquire) {
            return Err(CacheError::Stopped);
        }
        if self.shared.state.is_poisoned() {
            return Err(CacheError::Poisoned);
        }
        let sequence = self
            .sequence
            .checked_add(1)
            .ok_or(CacheError::SequenceExhausted)?;
        let mut previous_time = self.last_published;
        let started = observe(clock, &mut previous_time)?;
        let (wall_start_unix_ms, wall_time_error) = match clock.wall_time() {
            Ok(time) => (Some(time), None),
            Err(error) => (None, Some(error)),
        };
        let result = scan().map_err(|error| AcquisitionError {
            kind: error.kind(),
            raw_os_error: error.raw_os_error(),
            message: error.to_string(),
        });
        let finished = observe(clock, &mut previous_time)?;
        let mut guard = self
            .shared
            .state
            .write()
            .map_err(|_| CacheError::Poisoned)?;
        let published = observe(clock, &mut previous_time)?;
        let attempt = AttemptMetadata {
            sequence,
            started,
            finished,
            published,
            wall_start_unix_ms,
            wall_time_error,
        };
        let mut retired = std::mem::replace(&mut *guard, PublishedState::Empty);
        let outcome = match result {
            Ok(snapshot) => {
                *guard = PublishedState::Succeeded(ObservedSnapshot { attempt, snapshot });
                Acquisition::Succeeded(sequence)
            }
            Err(error) => {
                *guard = PublishedState::Failed {
                    attempt,
                    error,
                    previous: retired.take_observation(),
                };
                Acquisition::Failed(sequence)
            }
        };
        drop(guard);
        drop(retired); // Retiring allocations must not extend the publication lock.
        self.sequence = sequence;
        self.last_published = published;
        Ok(outcome)
    }

    /// Terminal, nonblocking lifecycle change; cannot cancel a blocked scan.
    pub fn stop(&mut self) {
        self.shared.producer_live.store(false, Ordering::Release);
    }
}

impl Drop for CacheOwner {
    fn drop(&mut self) {
        self.stop();
    }
}

impl CacheReader {
    /// Nonblocking API read. None means contention; poison remains an error.
    /// Like read(), this never acquires sensors or retains an older generation.
    #[cfg(target_os = "linux")]
    pub(crate) fn try_read(&self) -> Result<Option<ReadView<'_>>, CacheError> {
        let guard = match self.shared.state.try_read() {
            Ok(guard) => guard,
            Err(std::sync::TryLockError::WouldBlock) => return Ok(None),
            Err(std::sync::TryLockError::Poisoned(_)) => return Err(CacheError::Poisoned),
        };
        let last_assessed = guard.published();
        Ok(Some(ReadView {
            guard,
            shared: &self.shared,
            last_assessed,
        }))
    }

    /// Borrow the complete latest publication. Never scans or hides poisoning.
    pub fn read(&self) -> Result<ReadView<'_>, CacheError> {
        let guard = self.shared.state.read().map_err(|_| CacheError::Poisoned)?;
        let last_assessed = guard.published();
        Ok(ReadView {
            guard,
            shared: &self.shared,
            last_assessed,
        })
    }
}

impl ReadView<'_> {
    /// Recheck age and owner liveness. The returned references borrow this view;
    /// inspection cannot modify shared data. Backward time never rejuvenates it.
    pub fn assess(&mut self) -> Result<Assessment<'_>, CacheError> {
        let now = self.shared.elapsed()?;
        self.assess_at(now)
    }

    fn assess_at(&mut self, now: Duration) -> Result<Assessment<'_>, CacheError> {
        now.checked_sub(self.last_assessed)
            .ok_or(CacheError::InvalidClock)?;
        self.last_assessed = now;
        let active = self.shared.producer_live.load(Ordering::Acquire);
        let (latest, observed, failed) = match &*self.guard {
            PublishedState::Empty => (LatestAttempt::Empty, None, false),
            PublishedState::Succeeded(observed) => (
                LatestAttempt::Succeeded(&observed.attempt),
                Some(observed),
                false,
            ),
            PublishedState::Failed {
                attempt,
                error,
                previous,
            } => (
                LatestAttempt::Failed { attempt, error },
                previous.as_ref(),
                true,
            ),
        };
        let observation = observed
            .map(|observed| {
                let role = if !active {
                    ObservationRole::HistoricalAfterStop
                } else if failed {
                    ObservationRole::HistoricalAfterFailure
                } else {
                    ObservationRole::LatestAttempt
                };
                let freshness = if observed.snapshot.sensors.is_empty() {
                    Freshness::NoSensorObservations
                } else {
                    let age = AgeBounds {
                        lower: now
                            .checked_sub(observed.attempt.finished)
                            .ok_or(CacheError::InvalidClock)?,
                        upper: now
                            .checked_sub(observed.attempt.started)
                            .ok_or(CacheError::InvalidClock)?,
                    };
                    if age.upper > self.shared.stale_after {
                        Freshness::Stale(age)
                    } else {
                        Freshness::WithinBudget(age)
                    }
                };
                Ok(Observation {
                    snapshot: &observed.snapshot,
                    attempt: &observed.attempt,
                    role,
                    freshness,
                })
            })
            .transpose()?;
        Ok(Assessment {
            assessed_at: now,
            producer: if active {
                ProducerState::Active
            } else {
                ProducerState::Stopped
            },
            latest,
            observation,
            physical_age_known: false,
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(all(test, target_os = "linux"))]
mod api_tests;
