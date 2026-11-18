//! Narrow Linux socket boundary; never creates, chmods, unlinks, or reads sensors.
use std::fs;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::os::unix::net::UnixStream;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

fn denied(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
pub(super) fn user() -> io::Result<u32> {
    // SAFETY: no pointers, only process credentials are returned.
    let (real, effective) = unsafe { (libc::getuid(), libc::geteuid()) };
    if real == 0 || real != effective {
        Err(denied("API requires matching non-root real/effective UIDs"))
    } else {
        Ok(effective)
    }
}
pub(super) fn endpoint(root: &Path, uid: u32) -> io::Result<PathBuf> {
    if !root.is_absolute()
        || root
            .as_os_str()
            .as_bytes()
            .split(|b| *b == b'/')
            .any(|p| p == b"." || p == b"..")
    {
        return Err(denied(
            "runtime directory must be absolute without traversal",
        ));
    }
    let mut path = PathBuf::new();
    let mut after_shared_tmp = false;
    for part in root.components() {
        match part {
            Component::RootDir | Component::Normal(_) => path.push(part),
            _ => return Err(denied("invalid runtime path component")),
        }
        let m = fs::symlink_metadata(&path)?;
        let mode = m.mode() & 0o7777;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(denied("runtime ancestors must be real directories"));
        }
        if after_shared_tmp && (m.uid() != uid || mode != 0o700) {
            return Err(denied("directory beneath /tmp must be private and owned"));
        }
        if m.uid() != 0 && m.uid() != uid {
            return Err(denied("untrusted runtime ancestor owner"));
        }
        after_shared_tmp = path == Path::new("/tmp") && m.uid() == 0 && mode == 0o1777;
        if mode & 0o022 != 0 && !after_shared_tmp {
            return Err(denied("runtime ancestor writable by another user"));
        }
        if path == root && (m.uid() != uid || mode != 0o700) {
            return Err(denied("runtime root must be owned with mode 0700"));
        }
    }
    let child = root.join("siliconwatch");
    let m = fs::symlink_metadata(&child)?;
    if !m.is_dir() || m.file_type().is_symlink() || m.uid() != uid || m.mode() & 0o7777 != 0o700 {
        return Err(denied("API directory must be real, owned, and mode 0700"));
    }
    let socket = child.join("api.sock");
    let m = fs::symlink_metadata(&socket)?;
    if !m.file_type().is_socket() || m.uid() != uid || m.mode() & 0o7777 != 0o600 {
        return Err(denied("API endpoint must be an owned mode-0600 socket"));
    }
    let bytes = socket.as_os_str().as_bytes();
    if bytes.len() >= 108 || bytes.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "API socket pathname exceeds Linux limit",
        ));
    }
    Ok(socket)
}
pub(super) fn check(deadline: Instant, stop: &AtomicBool) -> io::Result<()> {
    if stop.load(Ordering::Acquire) {
        Err(io::Error::new(io::ErrorKind::Interrupted, "client stopped"))
    } else if Instant::now() >= deadline {
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "API transaction deadline expired",
        ))
    } else {
        Ok(())
    }
}
pub(super) fn wait(
    fd: libc::c_int,
    events: libc::c_short,
    deadline: Instant,
    stop: &AtomicBool,
) -> io::Result<()> {
    loop {
        check(deadline, stop)?;
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .min(Duration::from_millis(50));
        let millis = remaining.as_millis().max(1) as libc::c_int;
        let mut p = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        // SAFETY: one initialized pollfd, mutable for this call only.
        let rc = unsafe { libc::poll(&mut p, 1, millis) };
        if rc > 0 {
            if p.revents & libc::POLLNVAL != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "invalid API descriptor",
                ));
            }
            return Ok(()); // caller reads/writes or checks SO_ERROR, including HUP/ERR.
        }
        if rc < 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        }
    }
}
fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    for _ in 0..8 {
        let mut cred = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: valid live fd, correctly sized initialized output buffers.
        let rc = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut len,
            )
        };
        if rc == 0 {
            if len as usize != std::mem::size_of::<libc::ucred>() {
                return Err(denied("invalid peer credential size"));
            }
            return Ok(cred.uid);
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
    Err(denied("peer credential query interrupted"))
}
pub(super) fn check_peer(stream: &UnixStream, uid: u32) -> io::Result<()> {
    if peer_uid(stream)? == uid {
        Ok(())
    } else {
        Err(denied("API server peer UID mismatch"))
    }
}
pub(super) fn connect(
    path: &Path,
    uid: u32,
    deadline: Instant,
    stop: &AtomicBool,
) -> io::Result<UnixStream> {
    let bytes = path.as_os_str().as_bytes();
    if bytes.len() >= 108 || bytes.contains(&0) {
        return Err(denied("invalid API socket path"));
    }
    loop {
        check(deadline, stop)?;
        // SAFETY: socket has no pointer arguments; owned fd is immediately wrapped.
        let fd = unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                0,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: newly created fd is uniquely owned and is a Unix stream socket.
        let stream = unsafe { UnixStream::from_raw_fd(fd) };
        // SAFETY: zero is a valid initial sockaddr_un; sun_path stays NUL-terminated.
        let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
        addr.sun_family = libc::AF_UNIX as libc::sa_family_t;
        for (slot, byte) in addr.sun_path.iter_mut().zip(bytes) {
            *slot = *byte as libc::c_char;
        }
        let size = (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1)
            as libc::socklen_t;
        // SAFETY: correctly sized sockaddr_un lives through this synchronous call.
        let rc = unsafe { libc::connect(fd, (&addr as *const libc::sockaddr_un).cast(), size) };
        if rc != 0 {
            let e = io::Error::last_os_error();
            match e.raw_os_error() {
                // AF_UNIX backlog pressure requires a NEW connect attempt; unlike
                // TCP, writable readiness on this failed socket is not success.
                Some(libc::EAGAIN) | Some(libc::EINTR) => {
                    drop(stream);
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    std::thread::sleep(remaining.min(Duration::from_millis(50)));
                    continue;
                }
                Some(libc::EINPROGRESS) | Some(libc::EALREADY) => {
                    wait(fd, libc::POLLOUT, deadline, stop)?;
                    if let Some(error) = stream.take_error()? {
                        return Err(error);
                    }
                    stream.peer_addr()?;
                }
                _ => return Err(e),
            }
        }
        check(deadline, stop)?;
        check_peer(&stream, uid)?;
        return Ok(stream);
    }
}
