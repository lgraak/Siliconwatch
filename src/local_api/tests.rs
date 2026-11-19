use super::*;
use std::fs;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt, symlink};
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::mpsc;
use std::thread;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = PathBuf::from(format!(
            "/tmp/sw-api-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }

    fn socket(&self) -> PathBuf {
        self.0.join("siliconwatch/api.sock")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // This uniquely created fixture tree contains only this test's artifacts.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn budget(flag: &AtomicBool) -> Budget<'_> {
    Budget::new(Instant::now() + Duration::from_secs(2), flag)
}

fn receive(stream: &UnixStream) -> serde_json::Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response).unwrap();
    assert!(response.ends_with('\n'));
    serde_json::from_str(&response).unwrap()
}

#[test]
fn socket_and_directory_are_private_and_drop_cleans_only_owned_paths() {
    let fixture = Fixture::new();
    let prepared = Prepared::new(&fixture.0).unwrap();
    let uid = os::current_user().unwrap();
    for (path, mode) in [
        (fixture.0.join("siliconwatch"), 0o700),
        (fixture.socket(), 0o600),
    ] {
        let metadata = fs::symlink_metadata(path).unwrap();
        assert_eq!(metadata.mode() & 0o7777, mode);
        assert_eq!(metadata.uid(), uid);
    }
    assert_eq!(prepared.instance_id.len(), 32);
    assert!(
        prepared
            .instance_id
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    );
    drop(prepared);
    assert!(!fixture.0.join("siliconwatch").exists());
    assert!(fixture.0.exists());
}

#[test]
fn runtime_validation_rejects_untrusted_modes_components_and_owners() {
    let fixture = Fixture::new();
    let uid = os::current_user().unwrap();
    os::validate_runtime_root(&fixture.0, uid).unwrap();
    assert!(os::validate_runtime_root(Path::new("relative"), uid).is_err());
    assert!(os::validate_runtime_root(&fixture.0.join(".."), uid).is_err());
    assert!(os::validate_runtime_root(&fixture.0, uid.wrapping_add(1)).is_err());
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(os::validate_runtime_root(&fixture.0, uid).is_err());
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o700)).unwrap();
    let unsafe_parent = fixture.0.join("shared");
    fs::create_dir(&unsafe_parent).unwrap();
    fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o777)).unwrap();
    let nested = unsafe_parent.join("private");
    fs::DirBuilder::new().mode(0o700).create(&nested).unwrap();
    assert!(os::validate_runtime_root(&nested, uid).is_err());
    let link = fixture.0.join("link");
    symlink(&nested, &link).unwrap();
    assert!(os::validate_runtime_root(&link, uid).is_err());
    assert!(os::validate_runtime_root(&link.join("child"), uid).is_err());
}

#[test]
fn preexisting_namespace_is_never_replaced_or_chmodded() {
    let fixture = Fixture::new();
    let child = fixture.0.join("siliconwatch");
    fs::write(&child, b"preserve existing file").unwrap();
    assert!(Prepared::new(&fixture.0).is_err());
    assert_eq!(fs::read(&child).unwrap(), b"preserve existing file");
    fs::remove_file(&child).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&child).unwrap();
    fs::write(child.join("evidence"), b"preserve stale directory").unwrap();
    assert!(Prepared::new(&fixture.0).is_err());
    assert_eq!(
        fs::read(child.join("evidence")).unwrap(),
        b"preserve stale directory"
    );
}

#[test]
fn second_listener_is_rejected_without_disrupting_first() {
    let fixture = Fixture::new();
    let first = Prepared::new(&fixture.0).unwrap();
    let before = fs::symlink_metadata(fixture.socket()).unwrap();
    assert!(Prepared::new(&fixture.0).is_err());
    let after = fs::symlink_metadata(fixture.socket()).unwrap();
    assert_eq!((before.dev(), before.ino()), (after.dev(), after.ino()));
    UnixStream::connect(fixture.socket()).unwrap();
    drop(first);
    assert!(!fixture.socket().exists());
}

#[test]
fn replacement_socket_is_left_untouched_and_cleanup_failure_is_reported() {
    let fixture = Fixture::new();
    let mut prepared = Prepared::new(&fixture.0).unwrap();
    fs::remove_file(fixture.socket()).unwrap();
    fs::write(fixture.socket(), b"replacement").unwrap();
    prepared.listener.take();
    assert!(prepared.endpoint.cleanup().is_err());
    drop(prepared);
    assert_eq!(fs::read(fixture.socket()).unwrap(), b"replacement");
}

#[test]
fn replaced_directory_is_not_traversed_during_cleanup() {
    let fixture = Fixture::new();
    let prepared = Prepared::new(&fixture.0).unwrap();
    let child = fixture.0.join("siliconwatch");
    let moved = fixture.0.join("original");
    fs::rename(&child, &moved).unwrap();
    fs::DirBuilder::new().mode(0o700).create(&child).unwrap();
    fs::write(child.join("api.sock"), b"replacement").unwrap();
    drop(prepared);
    assert_eq!(fs::read(child.join("api.sock")).unwrap(), b"replacement");
    assert!(moved.join("api.sock").exists());
}

#[test]
fn overlong_socket_path_fails_without_creating_endpoint() {
    let fixture = Fixture::new();
    let root = fixture.0.join("long".repeat(25));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    assert_eq!(
        Prepared::new(&root).err().unwrap().kind(),
        io::ErrorKind::InvalidInput
    );
    assert!(!root.join("siliconwatch").exists());
}

#[test]
fn kernel_credentials_accept_same_uid_and_reject_a_different_expected_uid() {
    let (left, _) = UnixStream::pair().unwrap();
    let uid = os::current_user().unwrap();
    assert!(os::same_user(&left, uid).unwrap());
    assert!(!os::same_user(&left, uid.wrapping_add(1)).unwrap());
}

#[test]
fn framing_accepts_limit_and_only_first_frame_and_rejects_overflow() {
    let flag = AtomicBool::new(false);
    let (server, mut client) = UnixStream::pair().unwrap();
    let mut request = vec![b' '; wire::REQUEST_BYTES - 1];
    request.push(b'\n');
    request.extend_from_slice(b"second frame\n");
    client.write_all(&request).unwrap();
    let frame = read_frame(&server, &budget(&flag)).unwrap();
    assert_eq!(frame.len(), wire::REQUEST_BYTES);
    assert_eq!(frame.last(), Some(&b'\n'));
    let (server, mut client) = UnixStream::pair().unwrap();
    client.write_all(&vec![b'x'; wire::REQUEST_BYTES]).unwrap();
    assert_eq!(
        read_frame(&server, &budget(&flag)),
        Err(ReadFailure::Protocol(ApiError::RequestTooLarge))
    );
}

#[test]
fn partial_frame_eof_is_invalid_but_empty_disconnect_is_not_a_request() {
    let flag = AtomicBool::new(false);
    let (server, mut client) = UnixStream::pair().unwrap();
    client.write_all(b"{}").unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    assert_eq!(
        read_frame(&server, &budget(&flag)),
        Err(ReadFailure::Protocol(ApiError::InvalidRequest))
    );
    let (server, client) = UnixStream::pair().unwrap();
    drop(client);
    assert_eq!(
        read_frame(&server, &budget(&flag)),
        Err(ReadFailure::Disconnected)
    );
}

#[test]
fn expired_and_cancelled_budgets_do_not_do_socket_io() {
    let (server, _) = UnixStream::pair().unwrap();
    let flag = AtomicBool::new(false);
    let expired = Budget::new(Instant::now(), &flag);
    assert_eq!(
        read_frame(&server, &expired),
        Err(ReadFailure::Protocol(ApiError::RequestTimeout))
    );
    assert!(write_response(&server, b"{}\n", &expired).is_err());
    flag.store(true, Ordering::Release);
    assert_eq!(
        read_frame(&server, &budget(&flag)),
        Err(ReadFailure::Protocol(ApiError::Cancelled))
    );
}

#[test]
fn late_or_cancelled_complete_read_is_not_admitted() {
    for terminal in [ApiError::Deadline, ApiError::Cancelled] {
        let mut checks = 0;
        let result = read_frame_with(
            |bytes, _| {
                bytes[..3].copy_from_slice(b"{}\n");
                Ok(3)
            },
            || {
                checks += 1;
                if checks == 1 {
                    Ok(Duration::from_secs(1))
                } else {
                    Err(terminal)
                }
            },
        );
        let expected = if terminal == ApiError::Deadline {
            ApiError::RequestTimeout
        } else {
            terminal
        };
        assert_eq!(result, Err(ReadFailure::Protocol(expected)));
        assert_eq!(checks, 2);
    }
}

#[test]
fn partial_reads_and_interruptions_use_only_the_remaining_budget() {
    let mut checks = 0;
    let mut reads = 0;
    let mut timeouts = Vec::new();
    let result = read_frame_with(
        |bytes, timeout| {
            reads += 1;
            timeouts.push(timeout.as_millis());
            if reads == 2 {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            bytes[0] = b'x';
            Ok(1)
        },
        || {
            checks += 1;
            if checks < 6 {
                Ok(Duration::from_millis(60 - checks * 10))
            } else {
                Err(ApiError::Deadline)
            }
        },
    );
    assert_eq!(result, Err(ReadFailure::Protocol(ApiError::RequestTimeout)));
    assert_eq!(timeouts, [50, 30, 10]);
    assert_eq!(reads, 3);
}

#[test]
fn silent_and_slow_clients_expire_without_growing_buffers() {
    let flag = AtomicBool::new(false);
    let (server, mut client) = UnixStream::pair().unwrap();
    client.write_all(b"partial frame").unwrap();
    let deadline = Instant::now() + Duration::from_millis(20);
    assert_eq!(
        read_frame(&server, &Budget::new(deadline, &flag)),
        Err(ReadFailure::Protocol(ApiError::RequestTimeout))
    );
    let (server, client) = UnixStream::pair().unwrap();
    let bytes = vec![b'x'; wire::RESPONSE_BYTES];
    let deadline = Instant::now() + Duration::from_millis(20);
    assert_eq!(
        write_response(&server, &bytes, &Budget::new(deadline, &flag))
            .unwrap_err()
            .kind(),
        io::ErrorKind::TimedOut
    );
    client.set_nonblocking(true).unwrap();
    assert!((&client).read(&mut [0; 512]).unwrap() > 0);
}

#[test]
fn registered_stream_is_cancelled_and_registration_after_stop_is_closed() {
    let stop = Arc::new(Stop::new());
    let (server, _client) = UnixStream::pair().unwrap();
    assert!(stop.register(&server).unwrap());
    let (ready_tx, ready_rx) = mpsc::channel();
    let worker_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || {
        ready_tx.send(()).unwrap();
        read_frame(&server, &budget(worker_stop.flag()))
    });
    ready_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    stop.request().unwrap();
    let result = worker.join().unwrap();
    assert!(matches!(
        result,
        Err(ReadFailure::Disconnected | ReadFailure::Protocol(ApiError::Cancelled))
    ));
    let (stream, _) = UnixStream::pair().unwrap();
    assert!(!stop.register(&stream).unwrap());
    stop.request().unwrap();
}

#[test]
fn active_socket_is_cancelled_even_when_registration_mutex_is_poisoned() {
    let stop = Arc::new(Stop::new());
    let (server, _) = UnixStream::pair().unwrap();
    stop.register(&server).unwrap();
    let poison = Arc::clone(&stop);
    assert!(
        thread::spawn(move || {
            let _guard = poison.active.lock().unwrap();
            panic!("injected API cancellation poison");
        })
        .join()
        .is_err()
    );
    assert!(stop.request().is_err());
    assert!(stop.flag().load(Ordering::Acquire));
    assert!(stop.active.lock().unwrap_err().into_inner().is_none());
}

#[test]
fn serial_service_handles_info_errors_and_snapshot_without_a_second_worker() {
    let fixture = Fixture::new();
    let prepared = Prepared::new(&fixture.0).unwrap();
    let stop = Arc::new(Stop::new());
    let worker_stop = Arc::clone(&stop);
    let (calls_tx, calls_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let worker = thread::spawn(move || {
        prepared.run(worker_stop, |instance, budget| {
            calls_tx.send(()).unwrap();
            release_rx
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            wire::info(instance, budget, None)
        })
    });
    let mut first = UnixStream::connect(fixture.socket()).unwrap();
    first
        .write_all(b"{\"api_version\":1,\"method\":\"get_snapshot\"}\n")
        .unwrap();
    calls_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let mut second = UnixStream::connect(fixture.socket()).unwrap();
    second
        .write_all(b"{\"api_version\":1,\"method\":\"get_info\"}\n")
        .unwrap();
    second.set_nonblocking(true).unwrap();
    assert_eq!(
        (&second).read(&mut [0]).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    second.set_nonblocking(false).unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(receive(&first)["ok"], true);
    assert_eq!(receive(&second)["result"]["read_only"], true);
    let mut malformed = UnixStream::connect(fixture.socket()).unwrap();
    malformed.write_all(b"not-json\n").unwrap();
    assert_eq!(receive(&malformed)["error"]["code"], "invalid_request");
    assert!(calls_rx.try_recv().is_err());
    stop.request().unwrap();
    worker.join().unwrap().unwrap();
    assert!(!fixture.0.join("siliconwatch").exists());
}

#[test]
fn service_failure_returns_fatal_after_cleanup_and_disconnected_clients_do_not() {
    let fixture = Fixture::new();
    let prepared = Prepared::new(&fixture.0).unwrap();
    let stop = Arc::new(Stop::new());
    let worker_stop = Arc::clone(&stop);
    let worker =
        thread::spawn(move || prepared.run(worker_stop, |_, _| Err(ApiError::ServiceUnavailable)));
    drop(UnixStream::connect(fixture.socket()).unwrap());
    let mut request = UnixStream::connect(fixture.socket()).unwrap();
    request
        .write_all(b"{\"api_version\":1,\"method\":\"get_snapshot\"}\n")
        .unwrap();
    assert_eq!(receive(&request)["error"]["code"], "service_unavailable");
    assert!(worker.join().unwrap().is_err());
    assert!(!fixture.0.join("siliconwatch").exists());
}
