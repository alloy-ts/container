//! AF_VSOCK helpers for the host side of the guest agent connection.
//!
//! Binding `VMADDR_CID_ANY` works on this host. Connecting to a guest CID
//! requires a virtio-vsock device inside the VM, which is not attached yet
//! (see the Linux hypervisor TODO). The connect path is still real: nonblocking
//! `connect` plus `poll`.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::error::{Error, Result};

const AF_VSOCK: libc::c_int = 40;
const VMADDR_CID_ANY: u32 = 0xFFFF_FFFF;
const VMADDR_PORT_ANY: u32 = 0xFFFF_FFFF;

#[repr(C)]
struct SockaddrVm {
    svm_family: libc::sa_family_t,
    svm_reserved1: u16,
    svm_port: u32,
    svm_cid: u32,
    svm_zero: [u8; 4],
}

pub(crate) struct VsockListener {
    fd: OwnedFd,
    port: u32,
}

#[allow(dead_code)] // exercised by tests; virtio-vsock will bind or connect
impl VsockListener {
    /// Bind an ephemeral port on `VMADDR_CID_ANY` and listen.
    pub(crate) fn bind_ephemeral() -> Result<Self> {
        let fd = vsock_socket()?;
        let addr = SockaddrVm {
            svm_family: AF_VSOCK as libc::sa_family_t,
            svm_reserved1: 0,
            svm_port: VMADDR_PORT_ANY,
            svm_cid: VMADDR_CID_ANY,
            svm_zero: [0; 4],
        };
        let rc = unsafe {
            libc::bind(
                fd.as_raw_fd(),
                &addr as *const SockaddrVm as *const libc::sockaddr,
                std::mem::size_of::<SockaddrVm>() as libc::socklen_t,
            )
        };
        if rc < 0 {
            return Err(Error::Hypervisor(format!(
                "vsock bind: {}",
                io::Error::last_os_error()
            )));
        }
        let rc = unsafe { libc::listen(fd.as_raw_fd(), 16) };
        if rc < 0 {
            return Err(Error::Hypervisor(format!(
                "vsock listen: {}",
                io::Error::last_os_error()
            )));
        }
        let mut got = SockaddrVm {
            svm_family: 0,
            svm_reserved1: 0,
            svm_port: 0,
            svm_cid: 0,
            svm_zero: [0; 4],
        };
        let mut len = std::mem::size_of::<SockaddrVm>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockname(
                fd.as_raw_fd(),
                &mut got as *mut SockaddrVm as *mut libc::sockaddr,
                &mut len,
            )
        };
        if rc < 0 {
            return Err(Error::Hypervisor(format!(
                "vsock getsockname: {}",
                io::Error::last_os_error()
            )));
        }
        Ok(Self {
            fd,
            port: got.svm_port,
        })
    }

    pub(crate) fn port(&self) -> u32 {
        // Touch the fd so the listener socket stays obviously owned.
        let _ = self.fd.as_raw_fd();
        self.port
    }
}

/// Connect to `cid:port`. Used once a guest virtio-vsock device exists.
#[allow(dead_code)]
pub(crate) fn connect_blocking(cid: u32, port: u32, timeout: Duration) -> Result<VsockStream> {
    let fd = vsock_socket()?;
    set_nonblocking(fd.as_raw_fd())?;
    let addr = SockaddrVm {
        svm_family: AF_VSOCK as libc::sa_family_t,
        svm_reserved1: 0,
        svm_port: port,
        svm_cid: cid,
        svm_zero: [0; 4],
    };
    let rc = unsafe {
        libc::connect(
            fd.as_raw_fd(),
            &addr as *const SockaddrVm as *const libc::sockaddr,
            std::mem::size_of::<SockaddrVm>() as libc::socklen_t,
        )
    };
    if rc < 0 {
        let err = io::Error::last_os_error();
        if err.kind() != io::ErrorKind::WouldBlock && err.raw_os_error() != Some(libc::EINPROGRESS)
        {
            return Err(Error::Hypervisor(format!("vsock connect: {err}")));
        }
        let mut pollfd = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLOUT,
            revents: 0,
        };
        let ms = timeout.as_millis().min(i32::MAX as u128) as libc::c_int;
        let pr = unsafe { libc::poll(&mut pollfd, 1, ms) };
        if pr == 0 {
            return Err(Error::Hypervisor(format!(
                "vsock connect to cid {cid} port {port} timed out"
            )));
        }
        if pr < 0 {
            return Err(Error::Hypervisor(format!(
                "vsock poll: {}",
                io::Error::last_os_error()
            )));
        }
        let mut so_error: libc::c_int = 0;
        let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
        let gr = unsafe {
            libc::getsockopt(
                fd.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                &mut so_error as *mut libc::c_int as *mut libc::c_void,
                &mut len,
            )
        };
        if gr < 0 {
            return Err(Error::Hypervisor(format!(
                "vsock SO_ERROR: {}",
                io::Error::last_os_error()
            )));
        }
        if so_error != 0 {
            return Err(Error::Hypervisor(format!(
                "vsock connect: {}",
                io::Error::from_raw_os_error(so_error)
            )));
        }
    }
    Ok(VsockStream { fd: tokio_fd(fd)? })
}

fn vsock_socket() -> Result<OwnedFd> {
    let fd = unsafe { libc::socket(AF_VSOCK, libc::SOCK_STREAM | libc::SOCK_CLOEXEC, 0) };
    if fd < 0 {
        return Err(Error::Hypervisor(format!(
            "socket(AF_VSOCK): {}",
            io::Error::last_os_error()
        )));
    }
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn set_nonblocking(fd: i32) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(Error::Io(io::Error::last_os_error()));
    }
    let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if rc < 0 {
        return Err(Error::Io(io::Error::last_os_error()));
    }
    Ok(())
}

fn tokio_fd(fd: OwnedFd) -> Result<tokio::io::unix::AsyncFd<OwnedFd>> {
    set_nonblocking(fd.as_raw_fd())?;
    tokio::io::unix::AsyncFd::new(fd).map_err(Error::Io)
}

/// Nonblocking AF_VSOCK stream. `connect_blocking` returns one; nothing in
/// `boot` connects yet, so the type is ready for the virtio-vsock TODO.
#[allow(dead_code)]
pub(crate) struct VsockStream {
    fd: tokio::io::unix::AsyncFd<OwnedFd>,
}

impl AsyncRead for VsockStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            let mut guard = std::task::ready!(this.fd.poll_read_ready(cx))?;
            let unfilled = buf.initialize_unfilled();
            let n = unsafe {
                libc::read(
                    guard.get_inner().as_raw_fd(),
                    unfilled.as_mut_ptr() as *mut libc::c_void,
                    unfilled.len(),
                )
            };
            match n {
                0 => return Poll::Ready(Ok(())),
                n if n > 0 => {
                    let n = n as usize;
                    buf.advance(n);
                    return Poll::Ready(Ok(()));
                }
                _ => {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        guard.clear_ready();
                        continue;
                    }
                    return Poll::Ready(Err(err));
                }
            }
        }
    }
}

impl AsyncWrite for VsockStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        loop {
            let mut guard = match this.fd.poll_write_ready(cx) {
                Poll::Ready(Ok(guard)) => guard,
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => return Poll::Pending,
            };
            let n = unsafe {
                libc::write(
                    guard.get_inner().as_raw_fd(),
                    buf.as_ptr() as *const libc::c_void,
                    buf.len(),
                )
            };
            match n {
                n if n >= 0 => return Poll::Ready(Ok(n as usize)),
                _ => {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        guard.clear_ready();
                        continue;
                    }
                    return Poll::Ready(Err(err));
                }
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let fd = self.fd.get_ref().as_raw_fd();
        let rc = unsafe { libc::shutdown(fd, libc::SHUT_WR) };
        if rc < 0 {
            Poll::Ready(Err(io::Error::last_os_error()))
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_ephemeral_port() {
        match VsockListener::bind_ephemeral() {
            Ok(listener) => {
                assert_ne!(listener.port(), 0);
                assert_ne!(listener.port(), VMADDR_PORT_ANY);
            }
            Err(err) => {
                let message = err.to_string();
                assert!(
                    message.contains("Operation not permitted")
                        || message.contains("Permission denied")
                        || message.contains("Address family not supported")
                        || message.contains("No such device"),
                    "{message}"
                );
            }
        }
    }
}
