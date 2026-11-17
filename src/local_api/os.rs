//! Linux-only OS boundary. Peer and entropy operations never interpret client text.

use std::fs::{self, DirBuilder, Metadata};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Component, Path, PathBuf};

fn denied(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

pub(super) fn current_user() -> io::Result<u32> {
    // SAFETY: these calls take no pointers and only return process credentials.
    let (real, effective) = unsafe { (libc::getuid(), libc::geteuid()) };
    validate_user(real, effective)
}

fn validate_user(real: u32, effective: u32) -> io::Result<u32> {
    if real == 0 || real != effective {
        Err(denied(
            "local API requires matching non-root real and effective UIDs",
        ))
    } else {
        Ok(effective)
    }
}

pub(super) fn same_user(stream: &UnixStream, expected: u32) -> io::Result<bool> {
    peer_matches(expected, |credentials, length| {
        // SAFETY: the live stream owns this fd; both output pointers reference
        // initialized, correctly sized writable objects for this call only.
        let result = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (credentials as *mut libc::ucred).cast(),
                length,
            )
        };
        if result == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    })
}

fn peer_matches(
    expected: u32,
    mut query: impl FnMut(&mut libc::ucred, &mut libc::socklen_t) -> io::Result<()>,
) -> io::Result<bool> {
    for _ in 0..8 {
        let mut credentials = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        match query(&mut credentials, &mut length) {
            Ok(()) if length as usize != std::mem::size_of::<libc::ucred>() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid peer credential size",
                ));
            }
            Ok(()) => return Ok(credentials.uid == expected),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Interrupted,
        "peer credential lookup interrupted",
    ))
}

pub(super) fn instance_id() -> io::Result<String> {
    random_token(|buffer| {
        // SAFETY: the slice is a live writable buffer of the supplied size;
        // getrandom retains no pointer, and the nonblocking flag is valid.
        let count = unsafe {
            libc::getrandom(
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                libc::GRND_NONBLOCK,
            )
        };
        if count < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(count as usize)
        }
    })
}

fn random_token(mut fill: impl FnMut(&mut [u8]) -> io::Result<usize>) -> io::Result<String> {
    let mut bytes = [0_u8; 16];
    let mut offset = 0;
    for _ in 0..32 {
        match fill(&mut bytes[offset..]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "entropy source made no progress",
                ));
            }
            Ok(count) if count > bytes.len() - offset => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "entropy source returned invalid length",
                ));
            }
            Ok(count) => offset += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        }
        if offset == bytes.len() {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let mut token = String::with_capacity(32);
            for byte in bytes {
                token.push(HEX[usize::from(byte >> 4)] as char);
                token.push(HEX[usize::from(byte & 15)] as char);
            }
            return Ok(token);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Interrupted,
        "entropy source retry limit reached",
    ))
}

pub(super) fn validate_runtime_root(root: &Path, uid: u32) -> io::Result<()> {
    if !root.is_absolute() {
        return Err(denied("local API runtime directory must be absolute"));
    }
    let mut path = PathBuf::new();
    let mut after_shared_tmp = false;
    for component in root.components() {
        match component {
            Component::RootDir | Component::Normal(_) => path.push(component),
            _ => return Err(denied("local API runtime directory has invalid components")),
        }
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(denied(
                "local API runtime ancestors must be real directories",
            ));
        }
        let mode = metadata.mode() & 0o7777;
        if after_shared_tmp && (metadata.uid() != uid || mode != 0o700) {
            return Err(denied(
                "local API directory beneath /tmp must be private and owned",
            ));
        }
        if metadata.uid() != 0 && metadata.uid() != uid {
            return Err(denied("local API runtime ancestor has an untrusted owner"));
        }
        after_shared_tmp = path == Path::new("/tmp") && metadata.uid() == 0 && mode == 0o1777;
        if mode & 0o022 != 0 && !after_shared_tmp {
            return Err(denied(
                "local API runtime ancestor is writable by other users",
            ));
        }
        if path == root && (metadata.uid() != uid || mode != 0o700) {
            return Err(denied(
                "local API runtime directory must be owned with mode 0700",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Identity {
    device: u64,
    inode: u64,
    kind: u32,
    uid: u32,
}

impl Identity {
    fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            kind: metadata.mode() & libc::S_IFMT,
            uid: metadata.uid(),
        }
    }

    fn check(self, path: &Path) -> io::Result<()> {
        let actual = Self::of(&fs::symlink_metadata(path)?);
        if self.device != actual.device
            || self.inode != actual.inode
            || self.kind != actual.kind
            || self.uid != actual.uid
        {
            return Err(denied(
                "local API endpoint ownership changed; left untouched",
            ));
        }
        Ok(())
    }
}

pub(super) struct Endpoint {
    directory: PathBuf,
    directory_identity: Identity,
    socket: PathBuf,
    socket_identity: Option<Identity>,
    armed: bool,
}

impl Endpoint {
    pub(super) fn bind(root: &Path, uid: u32) -> io::Result<(UnixListener, Self)> {
        let directory = root.join("siliconwatch");
        let socket = directory.join("api.sock");
        // Linux sockaddr_un has 108 bytes, including the terminating NUL.
        if socket.as_os_str().as_bytes().len() >= 108 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "local API socket path is too long",
            ));
        }
        DirBuilder::new().mode(0o700).create(&directory)?;
        let metadata = fs::symlink_metadata(&directory)?;
        if !metadata.is_dir() || metadata.uid() != uid {
            return Err(denied(
                "local API directory identity could not be established",
            ));
        }
        let mut endpoint = Self {
            directory,
            directory_identity: Identity::of(&metadata),
            socket,
            socket_identity: None,
            armed: true,
        };
        if metadata.mode() & 0o7777 != 0o700 {
            return Err(denied("local API directory was not created with mode 0700"));
        }
        let listener = UnixListener::bind(&endpoint.socket)?;
        let metadata = fs::symlink_metadata(&endpoint.socket)?;
        if !metadata.file_type().is_socket() || metadata.uid() != uid {
            return Err(denied("local API socket identity could not be established"));
        }
        endpoint.socket_identity = Some(Identity::of(&metadata));
        endpoint.directory_identity.check(&endpoint.directory)?;
        fs::set_permissions(&endpoint.socket, fs::Permissions::from_mode(0o600))?;
        endpoint
            .socket_identity
            .expect("socket identity recorded above")
            .check(&endpoint.socket)?;
        if fs::symlink_metadata(&endpoint.socket)?.mode() & 0o7777 != 0o600 {
            return Err(denied("local API socket was not created with mode 0600"));
        }
        Ok((listener, endpoint))
    }

    pub(super) fn cleanup(&mut self) -> io::Result<()> {
        if !self.armed {
            return Ok(());
        }
        // Exactly one cleanup attempt: on uncertainty leave artifacts for review.
        self.armed = false;
        self.directory_identity.check(&self.directory)?;
        if let Some(identity) = self.socket_identity {
            identity.check(&self.socket)?;
            fs::remove_file(&self.socket)?;
        }
        self.directory_identity.check(&self.directory)?;
        fs::remove_dir(&self.directory)
    }
}

impl Drop for Endpoint {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    fn identity_policy_rejects_root_and_setid() {
        assert!(validate_user(0, 0).is_err());
        assert!(validate_user(7, 0).is_err());
        assert!(validate_user(7, 8).is_err());
        assert_eq!(validate_user(7, 7).unwrap(), 7);
    }

    #[test]
    fn malformed_or_failed_peer_lookup_never_authorizes_and_retry_is_bounded() {
        assert!(
            peer_matches(7, |credentials, _| {
                credentials.uid = 7;
                Ok(())
            })
            .unwrap()
        );
        assert!(
            !peer_matches(7, |credentials, _| {
                credentials.uid = 8;
                Ok(())
            })
            .unwrap()
        );
        assert!(
            peer_matches(7, |credentials, length| {
                credentials.uid = 7;
                *length = 0;
                Ok(())
            })
            .is_err()
        );
        assert!(
            peer_matches(7, |_, _| Err(io::Error::from(
                io::ErrorKind::PermissionDenied
            )))
            .is_err()
        );
        let mut calls = 0;
        assert!(
            peer_matches(7, |_, _| {
                calls += 1;
                Err(io::Error::from(io::ErrorKind::Interrupted))
            })
            .is_err()
        );
        assert_eq!(calls, 8);
    }

    #[test]
    fn entropy_is_bounded_and_has_no_fallback() {
        let mut calls = 0;
        let token = random_token(|bytes| {
            calls += 1;
            if calls == 1 {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            bytes[0] = 0xab;
            Ok(1)
        })
        .unwrap();
        assert_eq!(token, "ab".repeat(16));
        assert_eq!(calls, 17);
        for kind in [
            io::ErrorKind::WouldBlock,
            io::ErrorKind::Unsupported,
            io::ErrorKind::PermissionDenied,
        ] {
            assert_eq!(
                random_token(|_| Err(io::Error::from(kind)))
                    .unwrap_err()
                    .kind(),
                kind
            );
        }
        assert!(random_token(|_| Ok(0)).is_err());
        assert!(random_token(|bytes| Ok(bytes.len() + 1)).is_err());
        let mut interrupted = 0;
        assert!(
            random_token(|_| {
                interrupted += 1;
                Err(io::Error::from(io::ErrorKind::Interrupted))
            })
            .is_err()
        );
        assert_eq!(interrupted, 32);
    }
}
