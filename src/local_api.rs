//! Owner-only local telemetry transport. All responses are encoded before I/O.

mod os;
pub(crate) mod wire;

use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use wire::{ApiError, Budget, Method};

const ACCEPT_WAIT: Duration = Duration::from_millis(50);

/// The stop flag and registered socket share admission synchronization. The
/// coordinator never needs a cache guard to interrupt a client's socket I/O.
pub(crate) struct Stop {
    stopped: AtomicBool,
    active: Mutex<Option<UnixStream>>,
    wake: Condvar,
}

impl Stop {
    pub(crate) fn new() -> Self {
        Self {
            stopped: AtomicBool::new(false),
            active: Mutex::new(None),
            wake: Condvar::new(),
        }
    }

    pub(crate) fn flag(&self) -> &AtomicBool {
        &self.stopped
    }

    pub(crate) fn request(&self) -> io::Result<()> {
        self.stopped.store(true, Ordering::Release);
        // Recover only to cancel and release a socket, never to resume service.
        let (socket, poisoned) = match self.active.lock() {
            Ok(mut active) => (active.take(), false),
            Err(error) => (error.into_inner().take(), true),
        };
        self.wake.notify_all();
        let result = socket.map_or(Ok(()), |socket| match socket.shutdown(Shutdown::Both) {
            Err(error) if error.kind() == io::ErrorKind::NotConnected => Ok(()),
            result => result,
        });
        if poisoned {
            Err(io::Error::other("local API cancellation lock is poisoned"))
        } else {
            result
        }
    }

    fn register(&self, stream: &UnixStream) -> io::Result<bool> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| io::Error::other("local API cancellation lock is poisoned"))?;
        if self.stopped.load(Ordering::Acquire) {
            return Ok(false);
        }
        if active.is_some() {
            return Err(io::Error::other("local API already has an active client"));
        }
        *active = Some(stream.try_clone()?);
        Ok(true)
    }

    fn clear(&self) -> io::Result<()> {
        self.active
            .lock()
            .map_err(|_| io::Error::other("local API cancellation lock is poisoned"))?
            .take();
        Ok(())
    }

    fn idle(&self) -> io::Result<()> {
        let active = self
            .active
            .lock()
            .map_err(|_| io::Error::other("local API cancellation lock is poisoned"))?;
        let _state = self
            .wake
            .wait_timeout_while(active, ACCEPT_WAIT, |_| {
                !self.stopped.load(Ordering::Acquire)
            })
            .map_err(|_| io::Error::other("local API cancellation lock is poisoned"))?;
        Ok(())
    }
}

/// Owns only this invocation's endpoint. Dropping before a worker is spawned,
/// or while unwinding, still attempts ownership-checked cleanup.
pub(crate) struct Prepared {
    listener: Option<UnixListener>,
    endpoint: os::Endpoint,
    uid: u32,
    instance_id: String,
    monitoring: Option<wire::Monitoring>,
}

impl Prepared {
    pub(crate) fn new(runtime_root: &Path) -> io::Result<Self> {
        let uid = os::current_user()?;
        os::validate_runtime_root(runtime_root, uid)?;
        let instance_id = os::instance_id()?;
        let (listener, endpoint) = os::Endpoint::bind(runtime_root, uid)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener: Some(listener),
            endpoint,
            uid,
            instance_id,
            monitoring: None,
        })
    }

    pub(crate) fn with_monitoring(mut self, interval_ms: u64, stale_after_ms: u64) -> Self {
        self.monitoring = Some(wire::Monitoring {
            schema_version: 1,
            interval_ms,
            stale_after_ms,
        });
        self
    }

    pub(crate) fn run(
        mut self,
        stop: Arc<Stop>,
        snapshot: impl Fn(&str, &Budget<'_>) -> Result<Vec<u8>, ApiError>,
    ) -> io::Result<()> {
        let result = self.serve(&stop, &snapshot);
        self.listener.take();
        let cleanup = self.endpoint.cleanup();
        match (result, cleanup) {
            (Err(error), Err(cleanup)) => Err(io::Error::other(format!(
                "{error}; local API endpoint cleanup also failed: {cleanup}"
            ))),
            (Err(error), _) | (_, Err(error)) => Err(error),
            _ => Ok(()),
        }
    }

    fn serve(
        &self,
        stop: &Stop,
        snapshot: &impl Fn(&str, &Budget<'_>) -> Result<Vec<u8>, ApiError>,
    ) -> io::Result<()> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| io::Error::other("local API listener is unavailable"))?;
        while !stop.flag().load(Ordering::Acquire) {
            let stream = match listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    stop.idle()?;
                    continue;
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            let accepted_at = Instant::now();
            if !os::same_user(&stream, self.uid).unwrap_or(false) {
                continue;
            }
            stream.set_nonblocking(false)?;
            if !stop.register(&stream)? {
                break;
            }
            let result = self.client(&stream, accepted_at, stop.flag(), snapshot);
            stop.clear()?;
            result?;
        }
        Ok(())
    }

    fn client(
        &self,
        stream: &UnixStream,
        accepted_at: Instant,
        stopped: &AtomicBool,
        snapshot: &impl Fn(&str, &Budget<'_>) -> Result<Vec<u8>, ApiError>,
    ) -> io::Result<()> {
        let read_deadline = deadline(accepted_at, wire::REQUEST_DEADLINE_MS)?;
        let request = read_frame(stream, &Budget::new(read_deadline, stopped));
        let response_deadline = deadline(Instant::now(), wire::RESPONSE_DEADLINE_MS)?;
        let budget = Budget::new(response_deadline, stopped);
        let response = match request {
            Ok(frame) => match wire::parse_request(&frame) {
                Ok(Method::GetInfo) => wire::info(&self.instance_id, &budget, self.monitoring),
                Ok(Method::GetSnapshot) => snapshot(&self.instance_id, &budget),
                Err(error) => Err(error),
            },
            Err(ReadFailure::Protocol(error)) => Err(error),
            Err(ReadFailure::Disconnected) => return Ok(()),
        };
        let fatal = matches!(response, Err(ApiError::ServiceUnavailable));
        let response = match response {
            Err(ApiError::Cancelled | ApiError::Deadline) => return Ok(()),
            Err(error) => wire::error(&self.instance_id, error),
            Ok(bytes) => bytes,
        };
        if response.len() > wire::RESPONSE_BYTES || response.last() != Some(&b'\n') {
            return Err(io::Error::other(
                "local API encoder violated response framing",
            ));
        }
        // A peer that stops reading or disconnects affects this request only.
        // The callback's borrowed cache/lifecycle state has already been dropped.
        let _ = write_response(stream, &response, &budget);
        if fatal {
            Err(io::Error::other("local API service state is unavailable"))
        } else {
            Ok(())
        }
    }
}

impl Drop for Prepared {
    fn drop(&mut self) {
        self.listener.take();
        let _ = self.endpoint.cleanup();
    }
}

fn deadline(start: Instant, milliseconds: u64) -> io::Result<Instant> {
    start
        .checked_add(Duration::from_millis(milliseconds))
        .ok_or_else(|| io::Error::other("local API deadline overflow"))
}

#[derive(Debug, PartialEq, Eq)]
enum ReadFailure {
    Protocol(ApiError),
    Disconnected,
}

fn remaining(budget: &Budget<'_>) -> Result<Duration, ApiError> {
    budget.check()?;
    budget
        .deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or(ApiError::Deadline)
}

fn read_frame(stream: &UnixStream, budget: &Budget<'_>) -> Result<Vec<u8>, ReadFailure> {
    let mut stream = stream;
    read_frame_with(
        |bytes, remaining| {
            stream.set_read_timeout(Some(remaining))?;
            stream.read(bytes)
        },
        || remaining(budget),
    )
}

// The two closures let tests advance time exactly at an I/O boundary without
// sleeping or installing signal handlers in the shared test process.
fn read_frame_with(
    mut receive: impl FnMut(&mut [u8], Duration) -> io::Result<usize>,
    mut time_left: impl FnMut() -> Result<Duration, ApiError>,
) -> Result<Vec<u8>, ReadFailure> {
    let mut frame = Vec::with_capacity(wire::REQUEST_BYTES);
    let request_error = |error| {
        ReadFailure::Protocol(match error {
            ApiError::Deadline => ApiError::RequestTimeout,
            error => error,
        })
    };
    loop {
        let remaining = time_left().map_err(request_error)?;
        let mut bytes = [0_u8; 512];
        let limit = bytes.len().min(wire::REQUEST_BYTES - frame.len());
        let received = receive(&mut bytes[..limit], remaining);
        // A syscall's timeout does not account for a scheduling delay before
        // this thread resumes. Never admit a late frame or a cancelled request.
        time_left().map_err(request_error)?;
        match received {
            Ok(0) => {
                return Err(if frame.is_empty() {
                    ReadFailure::Disconnected
                } else {
                    ReadFailure::Protocol(ApiError::InvalidRequest)
                });
            }
            Ok(count) => {
                if let Some(end) = bytes[..count].iter().position(|byte| *byte == b'\n') {
                    frame.extend_from_slice(&bytes[..=end]);
                    return Ok(frame);
                }
                frame.extend_from_slice(&bytes[..count]);
                if frame.len() == wire::REQUEST_BYTES {
                    return Err(ReadFailure::Protocol(ApiError::RequestTooLarge));
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return Err(ReadFailure::Disconnected),
        }
    }
}

fn write_response(stream: &UnixStream, bytes: &[u8], budget: &Budget<'_>) -> io::Result<()> {
    let mut stream = stream;
    let mut written = 0;
    while written < bytes.len() {
        let timeout = remaining(budget)
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "local API response expired"))?;
        stream.set_write_timeout(Some(timeout))?;
        match stream.write(&bytes[written..]) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
            Ok(count) => written += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error),
        }
        remaining(budget)
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "local API response expired"))?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn write_response_for_test(
    stream: &UnixStream,
    bytes: &[u8],
    budget: &Budget<'_>,
) -> io::Result<()> {
    write_response(stream, bytes, budget)
}

#[cfg(test)]
mod tests;
