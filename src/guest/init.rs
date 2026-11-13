//! Guest PID 1: mount the virtio-fs root and connect to the host agent.
//!
//! This is the guest half of a full boot. The kernel runs it as `/init` from
//! the initramfs; it switches from the initramfs root to the real root served
//! by virtio-fs, then speaks the agent protocol to the host over `AF_VSOCK`.
//! See [`super::run_guest_init`].

use std::ffi::CString;
use std::io;
use std::time::Duration;

use crate::error::{Error, Result};
use crate::hypervisor::linux::vsock::{self, VsockStream};

use super::reference;

/// virtio-fs tag the host gives the container rootfs share.
const ROOTFS_TAG: &str = "rootfs";
/// `VMADDR_CID_HOST`: the cid a guest uses to reach the host.
const CID_HOST: u32 = 2;
/// Where the initramfs stages the real root before switching to it.
const NEW_ROOT: &str = "/newroot";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

const PROC: &str = "/proc";
const SYS: &str = "/sys";
const DEV: &str = "/dev";
const TMP: &str = "/tmp";

/// Mount the rootfs, switch to it, then serve the agent protocol.
pub(super) async fn serve_as_init() -> Result<()> {
    mount_root()?;
    let stream = connect_host_agent().await?;
    reference::serve(stream).await
}

fn mount_root() -> Result<()> {
    std::fs::create_dir_all(NEW_ROOT).map_err(Error::Io)?;
    mount(Some(ROOTFS_TAG), NEW_ROOT, "virtiofs")?;
    // Pseudo filesystems the guest init and its containers rely on.
    mount(None, &format!("{NEW_ROOT}{PROC}"), "proc")?;
    mount(None, &format!("{NEW_ROOT}{SYS}"), "sysfs")?;
    mount(None, &format!("{NEW_ROOT}{DEV}"), "devtmpfs")?;
    mount(None, &format!("{NEW_ROOT}{TMP}"), "tmpfs")?;
    enter_new_root(NEW_ROOT)
}

/// `mount(source, target, fstype)`. `source` is `None` for pseudo filesystems.
fn mount(source: Option<&str>, target: &str, fstype: &str) -> Result<()> {
    std::fs::create_dir_all(target).map_err(Error::Io)?;
    let c_source = cstring(source.unwrap_or("none"))?;
    let c_target = cstring(target)?;
    let c_fstype = cstring(fstype)?;
    // SAFETY: every pointer is NUL-terminated and valid for the call, and no
    // data argument is used for these filesystems.
    let rc = unsafe {
        libc::mount(
            c_source.as_ptr(),
            c_target.as_ptr(),
            c_fstype.as_ptr(),
            0,
            std::ptr::null(),
        )
    };
    if rc != 0 {
        return Err(Error::Hypervisor(format!(
            "guest init: mount {fstype} at {target}: {}",
            io::Error::last_os_error()
        )));
    }
    Ok(())
}

/// Chdir into `root`, make it the process root, and move to `/`.
fn enter_new_root(root: &str) -> Result<()> {
    let c_root = cstring(root)?;
    let c_dot = cstring(".")?;
    let c_slash = cstring("/")?;
    // SAFETY: the CStrings outlive each call and are NUL-terminated.
    unsafe {
        if libc::chdir(c_root.as_ptr()) != 0 {
            return Err(io_err("chdir newroot"));
        }
        if libc::chroot(c_dot.as_ptr()) != 0 {
            return Err(io_err("chroot"));
        }
        if libc::chdir(c_slash.as_ptr()) != 0 {
            return Err(io_err("chdir /"));
        }
    }
    Ok(())
}

/// Dial the host agent over `AF_VSOCK`. The host's virtio-vsock device turns
/// this into a connection to the per-VM proxy socket.
async fn connect_host_agent() -> Result<VsockStream> {
    let port = crate::hypervisor::GUEST_AGENT_PORT;
    tokio::task::spawn_blocking(move || vsock::connect_blocking(CID_HOST, port, CONNECT_TIMEOUT))
        .await
        .map_err(|err| Error::Hypervisor(format!("guest init: vsock connect task: {err}")))?
}

fn cstring(value: &str) -> Result<CString> {
    CString::new(value)
        .map_err(|_| Error::Hypervisor(format!("guest init: path contains a NUL byte: {value}")))
}

fn io_err(what: &str) -> Error {
    Error::Hypervisor(format!(
        "guest init: {what}: {}",
        io::Error::last_os_error()
    ))
}
