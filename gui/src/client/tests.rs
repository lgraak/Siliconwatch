use super::*;
use crate::protocol::tests::{INSTANCE, SECOND, frame, info_value, sample_value};
use serde_json::{Value, json};
use std::fs::{self, DirBuilder};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::os::unix::net::UnixListener;
use std::sync::atomic::AtomicUsize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
enum Reply {
    Bytes(Vec<u8>),
    Close,
    Stall,
}
struct Server {
    root: PathBuf,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    requests: Arc<Mutex<Vec<String>>>,
}
impl Server {
    fn new(mut respond: impl FnMut(&str) -> Reply + Send + 'static) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = PathBuf::from(format!(
            "/tmp/sw-gui-test-{}-{}-{}",
            std::process::id(),
            stamp,
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        DirBuilder::new().mode(0o700).create(&root).unwrap();
        let dir = root.join("siliconwatch");
        DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let listener = UnixListener::bind(dir.join("api.sock")).unwrap();
        fs::set_permissions(dir.join("api.sock"), fs::Permissions::from_mode(0o600)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);
        let worker = thread::spawn(move || {
            while !stopping.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut s, _)) => {
                        s.set_read_timeout(Some(Duration::from_millis(500)))
                            .unwrap();
                        s.set_write_timeout(Some(Duration::from_millis(500)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut byte = [0];
                        while request.len() < 4096 {
                            match s.read(&mut byte) {
                                Ok(1) => {
                                    request.push(byte[0]);
                                    if byte[0] == b'\n' {
                                        break;
                                    }
                                }
                                _ => break,
                            }
                        }
                        let value: Value = serde_json::from_slice(&request).unwrap();
                        assert_eq!(value["api_version"], json!(1));
                        let method = value["method"].as_str().unwrap().to_owned();
                        recorded.lock().unwrap().push(method.clone());
                        match respond(&method) {
                            Reply::Bytes(b) => {
                                let _ = s.write_all(&b);
                            }
                            Reply::Close => {}
                            Reply::Stall => {
                                while !stopping.load(Ordering::Acquire) {
                                    thread::sleep(Duration::from_millis(5));
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            }
        });
        Self {
            root,
            stop,
            worker: Some(worker),
            requests,
        }
    }
    fn client(&self) -> ClientHandle {
        ClientHandle::start(Some(self.root.clone()), policy(), || {}).unwrap()
    }
    fn methods(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.worker.take().unwrap().join().unwrap();
        // Only this fixture's newly-created files; never a real daemon endpoint.
        fs::remove_file(self.root.join("siliconwatch/api.sock")).unwrap();
        fs::remove_dir(self.root.join("siliconwatch")).unwrap();
        fs::remove_dir(&self.root).unwrap();
    }
}
fn policy() -> Policy {
    Policy {
        refresh: Duration::from_millis(15),
        busy: Duration::from_millis(20),
        transport: Duration::from_millis(25),
        transaction: Duration::from_millis(150),
    }
}
fn until(mut f: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if f() {
            return;
        }
        thread::sleep(Duration::from_millis(3));
    }
    panic!("fixture condition not reached");
}
fn snapshot(client: &ClientHandle) -> TimedSnapshot {
    let mut result = None;
    until(|| {
        if let Some(ClientEvent::Snapshot(s)) = client.take_update() {
            result = Some(*s);
            true
        } else {
            false
        }
    });
    result.unwrap()
}
fn normal(method: &str) -> Reply {
    Reply::Bytes(frame(&if method == "get_info" {
        info_value()
    } else {
        sample_value(1)
    }))
}

#[test]
fn worker_handshakes_reads_only_and_leaves_endpoint_untouched() {
    let server = Server::new(normal);
    let socket = server.root.join("siliconwatch/api.sock");
    let before = fs::symlink_metadata(&socket).unwrap();
    let client = server.client();
    let s = snapshot(&client);
    assert_eq!(s.snapshot.instance_id, INSTANCE);
    assert!(s.request_started <= s.response_received);
    client.join().unwrap();
    let after = fs::symlink_metadata(&socket).unwrap();
    assert_eq!(before.ino(), after.ino());
    assert_eq!(before.mode(), after.mode());
    let methods = server.methods();
    assert_eq!(methods[0], "get_info");
    assert!(methods[1..].iter().all(|m| m == "get_snapshot"));
}
#[test]
fn cancel_and_drop_join_a_stalled_worker_without_waiting_for_transaction_deadline() {
    let server = Server::new(|_| Reply::Stall);
    let p = Policy {
        transaction: Duration::from_secs(5),
        ..policy()
    };
    let client = ClientHandle::start(Some(server.root.clone()), p, || {}).unwrap();
    until(|| !server.methods().is_empty());
    let start = Instant::now();
    drop(client);
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn saved_timeout_applies_to_the_next_request_on_the_existing_worker() {
    let (release_info, wait_for_update) = std::sync::mpsc::channel();
    let server = Server::new(move |method| {
        if method == "get_info" {
            wait_for_update
                .recv_timeout(Duration::from_secs(3))
                .unwrap();
            normal(method)
        } else {
            Reply::Stall
        }
    });
    let p = Policy {
        transaction: Duration::from_secs(30),
        transport: Duration::from_secs(30),
        ..policy()
    };
    let client = ClientHandle::start(Some(server.root.clone()), p, || {}).unwrap();
    until(|| server.methods() == ["get_info"]);
    // Gate the handshake reply so the snapshot definitely starts after the settings change.
    client.set_response_timeout(100);
    release_info.send(()).unwrap();
    until(|| {
        matches!(
            client.take_update(),
            Some(ClientEvent::State { kind: ConnectionState::Disconnected, message })
                if message.contains("deadline expired")
        )
    });
    assert_eq!(server.methods(), ["get_info", "get_snapshot"]);
    let start = Instant::now();
    client.join().unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[test]
fn incomplete_eof_and_deadline_do_not_produce_a_snapshot() {
    for response in [
        Reply::Close,
        Reply::Bytes(b"{\"api_version\":1".to_vec()),
        Reply::Stall,
    ] {
        let reply = response.clone();
        let server = Server::new(move |_| reply.clone());
        let stop = AtomicBool::new(false);
        let failure = request(&server.root, "get_info", Duration::from_millis(60), &stop)
            .err()
            .unwrap();
        assert!(matches!(failure, Failure::Io(_)));
    }
}
#[test]
fn transport_rejects_oversize_and_extra_buffered_frames() {
    let mut huge = vec![b' '; protocol::RESPONSE_BYTES + 1];
    huge.push(b'\n');
    for data in [huge, b"{}\n{}\n".to_vec()] {
        let server = Server::new(move |_| Reply::Bytes(data.clone()));
        assert!(matches!(
            request(
                &server.root,
                "get_info",
                Duration::from_secs(1),
                &AtomicBool::new(false)
            ),
            Err(Failure::Protocol(_))
        ));
    }
}
#[test]
fn peer_uid_and_endpoint_permissions_are_checked() {
    let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
    let uid = os::user().unwrap();
    assert!(os::check_peer(&a, uid).is_ok());
    assert_eq!(
        os::check_peer(&a, uid.wrapping_add(1)).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    let server = Server::new(normal);
    let path = server.root.join("siliconwatch/api.sock");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
    assert_eq!(
        os::endpoint(&server.root, uid).unwrap_err().kind(),
        io::ErrorKind::PermissionDenied
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let link = server.root.with_extension("symlink");
    std::os::unix::fs::symlink(&server.root, &link).unwrap();
    assert!(os::endpoint(&link, uid).is_err());
    fs::remove_file(link).unwrap();
    assert!(os::endpoint(Path::new("relative"), uid).is_err());
}
#[test]
fn missing_runtime_configuration_is_nonretryable_until_explicit_reconnect() {
    let client = ClientHandle::start(None, policy(), || {}).unwrap();
    until(|| {
        matches!(
            client.take_update(),
            Some(ClientEvent::State {
                kind: ConnectionState::PermissionDenied,
                ..
            })
        )
    });
    client.join().unwrap();
    assert!(ClientHandle::spawn(999, 5000, || {}).is_err());
    assert!(ClientHandle::spawn(60001, 5000, || {}).is_err());
}
#[test]
fn service_unavailable_same_instance_cannot_be_bypassed_by_automatic_rehandshake() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&calls);
    let server = Server::new(move |method| {
        if method == "get_info" {
            return Reply::Bytes(frame(&info_value()));
        }
        if counted.fetch_add(1, Ordering::SeqCst) == 0 {
            Reply::Bytes(frame(
                &json!({"api_version":1,"instance_id":INSTANCE,"ok":false,"error":{"code":"service_unavailable","message":"fixture stop","retryable":false}}),
            ))
        } else {
            Reply::Bytes(frame(&sample_value(2)))
        }
    });
    let client = server.client();
    until(|| server.methods().iter().filter(|x| *x == "get_info").count() >= 3);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.reconnect();
    assert_eq!(
        snapshot(&client)
            .snapshot
            .assessment
            .latest
            .attempt
            .unwrap()
            .sequence,
        2
    );
    client.join().unwrap();
}
#[test]
fn instance_change_between_info_and_snapshot_requires_new_handshake() {
    let switched = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&switched);
    let server = Server::new(move |method| {
        let mut value = if method == "get_info" {
            info_value()
        } else {
            flag.store(true, Ordering::SeqCst);
            sample_value(1)
        };
        if flag.load(Ordering::SeqCst) {
            value["instance_id"] = json!(SECOND);
        }
        Reply::Bytes(frame(&value))
    });
    let client = server.client();
    assert_eq!(snapshot(&client).snapshot.instance_id, SECOND);
    client.join().unwrap();
    assert!(server.methods().starts_with(&[
        "get_info".into(),
        "get_snapshot".into(),
        "get_info".into(),
        "get_snapshot".into()
    ]));
}
#[test]
fn malformed_protocol_waits_for_explicit_reconnect() {
    let first = Arc::new(AtomicBool::new(true));
    let first_request = Arc::clone(&first);
    let server = Server::new(move |m| {
        if first_request.swap(false, Ordering::SeqCst) {
            Reply::Bytes(b"{bad}\n".to_vec())
        } else {
            normal(m)
        }
    });
    let client = server.client();
    until(|| {
        matches!(
            client.take_update(),
            Some(ClientEvent::State {
                kind: ConnectionState::Incompatible,
                ..
            })
        )
    });
    thread::sleep(Duration::from_millis(80));
    assert_eq!(server.methods().len(), 1);
    client.reconnect();
    snapshot(&client);
    client.join().unwrap();
}
#[test]
fn transient_eof_reconnects_with_a_fresh_handshake() {
    let first = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&first);
    let server = Server::new(move |m| {
        if flag.swap(false, Ordering::SeqCst) {
            Reply::Close
        } else {
            normal(m)
        }
    });
    let client = server.client();
    snapshot(&client);
    client.join().unwrap();
    assert!(server.methods().starts_with(&[
        "get_info".into(),
        "get_info".into(),
        "get_snapshot".into()
    ]));
}
#[test]
fn notification_mailbox_coalesces_and_rearms_after_consumption() {
    let count = Arc::new(AtomicUsize::new(0));
    let notified = Arc::clone(&count);
    let shared = Shared {
        transaction_ms: AtomicU64::new(5000),
        stop: AtomicBool::new(false),
        reconnect: AtomicBool::new(false),
        mailbox: Mutex::new(None),
        wake: Condvar::new(),
        wait_lock: Mutex::new(()),
        notify: Box::new(move || {
            notified.fetch_add(1, Ordering::SeqCst);
        }),
    };
    for _ in 0..100 {
        shared.state(ConnectionState::Connecting, "fixture");
    }
    assert_eq!(count.load(Ordering::SeqCst), 1);
    shared.mailbox.lock().unwrap().take();
    shared.state(ConnectionState::Disconnected, "fixture");
    assert_eq!(count.load(Ordering::SeqCst), 2);
}
#[test]
fn sequence_guard_rejects_regression_but_accepts_unchanged_cache_and_new_instance() {
    let mut guard = SequenceGuard::default();
    let newer = protocol::parse_snapshot(&frame(&sample_value(2))).unwrap();
    guard.accept(&newer).unwrap();
    guard.accept(&newer).unwrap();
    let older = protocol::parse_snapshot(&frame(&sample_value(1))).unwrap();
    assert!(guard.accept(&older).is_err());
    let mut changed = newer.clone();
    changed.assessment.assessed_at_ns -= 1;
    assert!(guard.accept(&changed).is_err());
    let mut other = older;
    other.instance_id = SECOND.into();
    assert!(guard.accept(&other).is_ok());
}
