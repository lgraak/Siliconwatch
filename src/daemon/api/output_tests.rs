use super::*;
use std::fs;
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const WAIT: Duration = Duration::from_secs(3);
const INSTANCE: &str = "0123456789abcdef0123456789abcdef";

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "sw-api-output-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("hwmon0")).unwrap();
        fs::write(path.join("hwmon0/name"), "fixture").unwrap();
        fs::write(path.join("hwmon0/temp1_input"), "42000").unwrap();
        fs::write(path.join("hwmon0/temp1_label"), "x".repeat(16_000)).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // This newly created subtree contains only this test's synthetic inputs.
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn publication_completes_while_encoded_api_response_is_socket_backpressured() {
    let fixture = Fixture::new();
    let (mut owner, reader) = snapshot_cache::new(&fixture.0, WAIT).unwrap();
    assert_eq!(owner.acquire_once().unwrap(), Acquisition::Succeeded(1));
    let control = Arc::new(Control::new());
    control.start().unwrap();
    let (mut server, client) = UnixStream::pair().unwrap();
    let monitor = server.try_clone().unwrap();
    let send_buffer: libc::c_int = 1024;
    // SAFETY: the live socket fd and correctly sized option value remain valid
    // for the call. This changes only a test-owned socket pair's send buffer.
    assert_eq!(
        unsafe {
            libc::setsockopt(
                server.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                (&send_buffer as *const libc::c_int).cast(),
                std::mem::size_of_val(&send_buffer) as libc::socklen_t,
            )
        },
        0
    );
    let (blocked_tx, blocked_rx) = mpsc::channel();
    let (written_tx, written_rx) = mpsc::channel();
    let writer = thread::spawn(move || {
        let cancelled = AtomicBool::new(false);
        let budget = Budget::new(Instant::now() + WAIT * 3, &cancelled);
        // Exercise the actual service view and encoder; only owned bytes may
        // survive this call into transport processing.
        let bytes = snapshot(&reader, &control, INSTANCE, WAIT, &budget).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["result"]["latest"]["attempt"]["sequence"], "1");
        assert!(control.state.try_lock().is_ok());
        server.set_nonblocking(true).unwrap();
        let mut written = 0;
        loop {
            assert!(
                written < bytes.len(),
                "fixture response must exceed the socket buffer"
            );
            match server.write(&bytes[written..]) {
                Ok(0) => panic!("fixture socket wrote zero bytes"),
                Ok(count) => written += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) => panic!("fixture socket failed: {error}"),
            }
        }
        assert!(written > 0);
        server.set_nonblocking(false).unwrap();
        blocked_tx.send(()).unwrap();
        let result = local_api::write_response_for_test(&server, &bytes[written..], &budget);
        written_tx.send(result.is_ok()).unwrap();
        result
    });
    // This is an observed WouldBlock on actual response bytes, not a delay
    // standing in for network backpressure. The client has read nothing.
    blocked_rx.recv_timeout(WAIT).unwrap();
    let mut readiness = libc::pollfd {
        fd: monitor.as_raw_fd(),
        events: libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: one initialized writable pollfd is supplied with a zero timeout.
    let poll_result = unsafe { libc::poll(&mut readiness, 1, 0) };

    fs::write(fixture.0.join("hwmon0/temp1_input"), "43000").unwrap();
    let (published_tx, published_rx) = mpsc::channel();
    let publisher = thread::spawn(move || {
        let result = owner.acquire_once();
        published_tx.send(result).unwrap();
        owner
    });
    let publication = published_rx.recv_timeout(WAIT);
    let output_pending = written_rx.try_recv().is_err();

    // Always release socket backpressure before asserting or joining. A guard
    // retention regression can then finish instead of hanging the test process.
    let _ = client.shutdown(Shutdown::Both);
    let writer_result = writer.join().unwrap();
    let mut owner = publisher.join().unwrap();
    owner.stop();
    assert_eq!(poll_result, 0, "full socket must not be writable");
    assert_eq!(readiness.revents & libc::POLLOUT, 0);
    assert_eq!(publication.unwrap(), Ok(Acquisition::Succeeded(2)));
    assert!(
        output_pending,
        "response must remain incomplete until cancellation"
    );
    assert!(
        writer_result.is_err(),
        "peer was cancelled without reading response"
    );
}
