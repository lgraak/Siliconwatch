//! One bounded, cancellable read-only local API worker. No acquisition imports.
use crate::protocol::{self, Attempt, ProtocolError, SnapshotResponse};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
#[path = "client/os.rs"]
mod os;

#[derive(Clone, Debug, PartialEq)]
pub struct TimedSnapshot {
    pub snapshot: SnapshotResponse,
    pub request_started: Instant,
    pub response_received: Instant,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Disconnected,
    Incompatible,
    PermissionDenied,
    ServiceUnavailable,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ClientEvent {
    Snapshot(Box<TimedSnapshot>),
    State {
        kind: ConnectionState,
        message: String,
    },
}
struct Shared {
    stop: AtomicBool,
    reconnect: AtomicBool,
    mailbox: Mutex<Option<ClientEvent>>,
    wake: Condvar,
    wait_lock: Mutex<()>,
    notify: Box<dyn Fn() + Send + Sync>,
}
impl Shared {
    fn publish(&self, event: ClientEvent) {
        let notify = {
            let mut slot = self.mailbox.lock().unwrap_or_else(|e| e.into_inner());
            let empty = slot.is_none();
            *slot = Some(event);
            empty
        };
        // Only the transition from no pending event schedules a notification.
        // The consumer clears the slot before processing it on its UI thread.
        if notify {
            (self.notify)();
        }
    }
    fn state(&self, kind: ConnectionState, message: impl Into<String>) {
        self.publish(ClientEvent::State {
            kind,
            message: message.into(),
        });
    }
    fn until(&self, due: Instant) -> bool {
        let mut guard = self.wait_lock.lock().unwrap_or_else(|e| e.into_inner());
        while !self.stop.load(Ordering::Acquire) {
            let remaining = due.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return true;
            }
            let waited = self
                .wake
                .wait_timeout(guard, remaining.min(Duration::from_millis(50)))
                .unwrap_or_else(|e| e.into_inner());
            guard = waited.0;
        }
        false
    }
    fn command(&self) -> bool {
        let mut guard = self.wait_lock.lock().unwrap_or_else(|e| e.into_inner());
        while !self.stop.load(Ordering::Acquire) && !self.reconnect.load(Ordering::Acquire) {
            let waited = self
                .wake
                .wait_timeout(guard, Duration::from_millis(50))
                .unwrap_or_else(|e| e.into_inner());
            guard = waited.0;
        }
        !self.stop.load(Ordering::Acquire)
    }
}
#[derive(Clone, Copy)]
struct Policy {
    refresh: Duration,
    busy: Duration,
    transport: Duration,
    transaction: Duration,
}
impl Policy {
    fn normal(refresh_ms: u64) -> io::Result<Self> {
        if !(1000..=60000).contains(&refresh_ms) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refresh must be 1000..60000 ms",
            ));
        }
        Ok(Self {
            refresh: Duration::from_millis(refresh_ms),
            busy: Duration::from_secs(2),
            transport: Duration::from_secs(5),
            transaction: Duration::from_secs(5),
        })
    }
}
/// Call stop from the close callback; join after the GUI event loop ends.
/// Drop also cancels and joins, so the worker can never be accidentally detached.
pub struct ClientHandle {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}
impl ClientHandle {
    pub fn spawn(refresh_ms: u64, notify: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let policy = Policy::normal(refresh_ms)?;
        let root = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
        Self::start(root, policy, notify)
    }
    fn start(
        root: Option<PathBuf>,
        policy: Policy,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            reconnect: AtomicBool::new(false),
            mailbox: Mutex::new(None),
            wake: Condvar::new(),
            wait_lock: Mutex::new(()),
            notify: Box::new(notify),
        });
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("siliconwatch-api-client".into())
            .spawn(move || run(root, policy, worker_shared))?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }
    pub fn take_update(&self) -> Option<ClientEvent> {
        self.shared
            .mailbox
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
    pub fn reconnect(&self) {
        self.shared.reconnect.store(true, Ordering::Release);
        self.shared.wake.notify_all();
    }
    pub fn stop(&self) {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.wake.notify_all();
    }
    pub fn join(mut self) -> thread::Result<()> {
        self.stop();
        self.worker.take().map_or(Ok(()), JoinHandle::join)
    }
}
impl Drop for ClientHandle {
    fn drop(&mut self) {
        self.stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[derive(Debug)]
enum Failure {
    Io(io::Error),
    Protocol(ProtocolError),
}
impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<ProtocolError> for Failure {
    fn from(e: ProtocolError) -> Self {
        Self::Protocol(e)
    }
}
struct Frame {
    bytes: Vec<u8>,
    started: Instant,
    received: Instant,
    deadline: Instant,
}
fn request(
    root: &Path,
    method: &str,
    budget: Duration,
    stop: &AtomicBool,
) -> Result<Frame, Failure> {
    let started = Instant::now();
    let deadline = started + budget;
    let uid = os::user()?;
    let endpoint = os::endpoint(root, uid)?;
    let mut stream = os::connect(&endpoint, uid, deadline, stop)?;
    let request = match method {
        "get_info" => b"{\"api_version\":1,\"method\":\"get_info\"}\n".as_slice(),
        "get_snapshot" => b"{\"api_version\":1,\"method\":\"get_snapshot\"}\n".as_slice(),
        _ => {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "unknown client read method").into(),
            );
        }
    };
    let mut offset = 0;
    while offset < request.len() {
        os::check(deadline, stop)?;
        match stream.write(&request[offset..]) {
            Ok(0) => {
                return Err(
                    io::Error::new(io::ErrorKind::WriteZero, "API write made no progress").into(),
                );
            }
            Ok(n) => offset += n,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                os::wait(stream.as_raw_fd(), libc::POLLOUT, deadline, stop)?
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    let mut bytes = Vec::with_capacity(8192);
    let mut chunk = [0u8; 8192];
    loop {
        os::check(deadline, stop)?;
        // Read at most one byte beyond the response cap; no unbounded frame buffer.
        let capacity = (protocol::RESPONSE_BYTES + 1 - bytes.len()).min(chunk.len());
        match stream.read(&mut chunk[..capacity]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "API closed before a complete LF response",
                )
                .into());
            }
            Ok(n) => {
                if let Some(end) = chunk[..n].iter().position(|b| *b == b'\n') {
                    if end + 1 != n {
                        return Err(ProtocolError::Invalid(
                            "extra buffered bytes after first frame".into(),
                        )
                        .into());
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if bytes.len() > protocol::RESPONSE_BYTES {
                        return Err(ProtocolError::Invalid("response exceeds 1 MiB".into()).into());
                    }
                    let received = Instant::now();
                    os::check(deadline, stop)?;
                    return Ok(Frame {
                        bytes,
                        started,
                        received,
                        deadline,
                    });
                }
                bytes.extend_from_slice(&chunk[..n]);
                if bytes.len() >= protocol::RESPONSE_BYTES {
                    return Err(ProtocolError::Invalid(
                        "response exceeds 1 MiB or has no terminator".into(),
                    )
                    .into());
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                os::wait(stream.as_raw_fd(), libc::POLLIN, deadline, stop)?
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
}
#[derive(Default)]
struct SequenceGuard {
    instance: Option<String>,
    assessed: Option<u128>,
    latest: Option<Attempt>,
    observation: Option<Attempt>,
}
impl SequenceGuard {
    fn accept(&mut self, s: &SnapshotResponse) -> Result<(), ProtocolError> {
        if self.instance.as_deref() != Some(&s.instance_id) {
            *self = Self::default();
        }
        let a = &s.assessment;
        if self.assessed.is_some_and(|v| a.assessed_at_ns < v) {
            return Err(ProtocolError::Invalid(
                "same-instance assessment time regressed".into(),
            ));
        }
        for (old, new) in [
            (&self.latest, &a.latest.attempt),
            (
                &self.observation,
                &a.observation.as_ref().map(|o| o.attempt.clone()),
            ),
        ] {
            if let Some(old) = old {
                let Some(new) = new else {
                    return Err(ProtocolError::Invalid(
                        "same-instance observation disappeared".into(),
                    ));
                };
                if new.sequence < old.sequence || (new.sequence == old.sequence && new != old) {
                    return Err(ProtocolError::Invalid(
                        "same-instance publication regressed or changed metadata".into(),
                    ));
                }
            }
        }
        self.instance = Some(s.instance_id.clone());
        self.assessed = Some(a.assessed_at_ns);
        self.latest = a.latest.attempt.clone();
        self.observation = a.observation.as_ref().map(|o| o.attempt.clone());
        Ok(())
    }
}
fn run(root: Option<PathBuf>, policy: Policy, shared: Arc<Shared>) {
    let mut expected: Option<String> = None;
    let mut blocked_instance: Option<String> = None;
    let mut require_explicit = false;
    let mut due = Instant::now();
    let mut sequences = SequenceGuard::default();
    shared.state(
        ConnectionState::Connecting,
        "Connecting to the current-user read-only API",
    );
    while shared.until(due) {
        if require_explicit && !shared.command() {
            break;
        }
        if shared.reconnect.swap(false, Ordering::AcqRel) {
            expected = None;
            blocked_instance = None;
            require_explicit = false;
            shared.state(
                ConnectionState::Connecting,
                "Reconnecting to the read-only API",
            );
        }
        if shared.stop.load(Ordering::Acquire) {
            break;
        }
        let result = (|| -> Result<Option<TimedSnapshot>, Failure> {
            let root = root.as_deref().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "XDG_RUNTIME_DIR is missing; start from the correct desktop user session",
                )
            })?;
            if expected.is_none() {
                let f = request(root, "get_info", policy.transaction, &shared.stop)?;
                let info = protocol::parse_info(&f.bytes)?;
                os::check(f.deadline, &shared.stop)?;
                if blocked_instance.as_deref() == Some(&info.instance_id) {
                    return Ok(None);
                }
                blocked_instance = None;
                expected = Some(info.instance_id);
            }
            let f = request(root, "get_snapshot", policy.transaction, &shared.stop)?;
            let snapshot = protocol::parse_snapshot(&f.bytes)?;
            os::check(f.deadline, &shared.stop)?;
            if expected.as_deref() != Some(&snapshot.instance_id) {
                expected = None;
                shared.state(
                    ConnectionState::Connecting,
                    "Daemon instance changed; validating a new handshake",
                );
                return Ok(None);
            }
            sequences.accept(&snapshot)?;
            Ok(Some(TimedSnapshot {
                snapshot,
                request_started: f.started,
                response_received: f.received,
            }))
        })();
        if shared.stop.load(Ordering::Acquire) {
            break;
        }
        let wait = match result {
            Ok(Some(snapshot)) => {
                shared.publish(ClientEvent::Snapshot(Box::new(snapshot)));
                policy.refresh
            }
            Ok(None) => policy.transport.max(policy.refresh),
            Err(Failure::Protocol(ProtocolError::Remote { instance_id, error })) => {
                if expected.as_deref() != Some(&instance_id) {
                    expected = None;
                }
                if error.code == "service_unavailable" {
                    blocked_instance = Some(instance_id);
                    expected = None;
                    shared.state(ConnectionState::ServiceUnavailable,"Daemon service unavailable; waiting for a different instance or explicit Reconnect");
                    policy.transport.max(policy.refresh)
                } else if error.retryable {
                    shared.state(
                        ConnectionState::Disconnected,
                        format!("API {}; retrying after backoff", error.code),
                    );
                    policy.busy.max(policy.refresh)
                } else {
                    require_explicit = true;
                    shared.state(
                        ConnectionState::Incompatible,
                        format!("API {}; correct the problem, then Reconnect", error.code),
                    );
                    policy.refresh
                }
            }
            Err(Failure::Protocol(error)) => {
                require_explicit = true;
                expected = None;
                shared.state(ConnectionState::Incompatible, error.to_string());
                policy.refresh
            }
            Err(Failure::Io(error)) => {
                expected = None;
                let permanent = matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::InvalidInput
                );
                require_explicit = permanent;
                let kind = if permanent {
                    ConnectionState::PermissionDenied
                } else {
                    ConnectionState::Disconnected
                };
                // OS error text is bounded and includes no telemetry or private socket path.
                shared.state(
                    kind,
                    format!(
                        "API connection: {}",
                        error.to_string().chars().take(240).collect::<String>()
                    ),
                );
                policy.transport.max(policy.refresh)
            }
        };
        due = Instant::now() + wait;
    }
}

#[cfg(test)]
#[path = "client/tests.rs"]
mod tests;
