//! Internal VMM interface.
//!
//! Callers never see this module. [`crate::Container`] picks the host backend:
//! Linux (KVM), macOS (Virtualization.framework, stub), or Windows (WHPX, stub).

mod macos;
mod windows;

#[cfg(target_os = "linux")]
pub(crate) mod linux;

#[cfg(test)]
pub(crate) mod loopback;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Result;
use crate::resource::{NetworkPlan, VirtioFsShare};

pub(crate) const GUEST_AGENT_PORT: u32 = 1024;

/// Byte stream the agent client can own. Implemented for any tokio duplex,
/// socket, or vsock stream.
pub(crate) trait BytePipe: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> BytePipe for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

pub(crate) enum AgentLink {
    /// Already connected to the guest init (test hypervisor, or a backend
    /// that completed the handshake itself).
    #[allow(dead_code)]
    Connected(Box<dyn BytePipe>),
    /// Host should connect to this vsock address.
    ///
    /// Constructed once the Linux virtio-vsock device model lands; the connect
    /// path in `container::connect_link` already accepts it.
    #[allow(dead_code)]
    Vsock { cid: u32, port: u32 },
}

pub(crate) struct BootedVm {
    pub link: AgentLink,
    /// Host pid of an out-of-process VMM. `None` when the VMM is in-process.
    pub host_pid: Option<u32>,
}

pub(crate) struct VmConfig {
    pub id: String,
    pub vcpus: u32,
    pub memory_mb: u64,
    pub kernel_path: PathBuf,
    pub cmdline: String,
    pub vsock_cid: u32,
    pub vsock_port: u32,
    pub network: Option<NetworkPlan>,
    pub shares: Vec<VirtioFsShare>,
}

#[async_trait]
pub(crate) trait Hypervisor: Send {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm>;
    fn pause(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
    fn shutdown_blocking(&mut self);
    async fn shutdown(&mut self) -> Result<()>;
}

pub(crate) fn platform() -> Box<dyn Hypervisor> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxHypervisor::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosHypervisor::new())
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsHypervisor::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Box::new(windows::UnsupportedHypervisor)
    }
}

/// Guest CIDs 0, 1, and 2 are reserved by AF_VSOCK (hypervisor, local, host).
pub(crate) fn alloc_guest_cid() -> u32 {
    static NEXT: AtomicU32 = AtomicU32::new(3);
    NEXT.fetch_add(1, Ordering::Relaxed).max(3)
}
