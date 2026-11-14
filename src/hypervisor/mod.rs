//! Internal VMM interface.
//!
//! Linux and macOS boot through libkrun (KVM or Hypervisor.framework). Windows
//! stays a stub: libkrun has no Windows port.

mod windows;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod libkrun;

#[cfg(test)]
pub(crate) mod loopback;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Result;
use crate::guest::ExitWait;
use crate::resource::{NetworkPlan, VirtioFsShare};

pub(crate) const GUEST_AGENT_PORT: u32 = 1024;

/// Byte stream the agent client can own. Implemented for any tokio duplex,
/// socket, or vsock stream.
pub(crate) trait BytePipe: AsyncRead + AsyncWrite + Send + Unpin {}
impl<T> BytePipe for T where T: AsyncRead + AsyncWrite + Send + Unpin {}

pub(crate) enum AgentLink {
    /// Already connected to the guest init (test hypervisor, or a guest that
    /// dialed the vsock port libkrun bridges to a unix socket).
    Connected(Box<dyn BytePipe>),
    /// libkrunfw booted and libkrun's init started the entrypoint. There is no
    /// in-guest agent; stdio is the virtio console.
    Direct {
        stdout: Option<Box<dyn AsyncRead + Unpin + Send>>,
        stderr: Option<Box<dyn AsyncRead + Unpin + Send>>,
        exit: ExitWait,
    },
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
    /// `(host_port, guest_port)` TSI forwards. Empty keeps a dedicated TAP on Linux.
    pub ports: Vec<(u16, u16)>,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub workdir: PathBuf,
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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Box::new(libkrun::LibkrunHypervisor::new())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Box::new(windows::WindowsHypervisor::new())
    }
}

/// Guest CIDs 0, 1, and 2 are reserved by AF_VSOCK (hypervisor, local, host).
pub(crate) fn alloc_guest_cid() -> u32 {
    static NEXT: AtomicU32 = AtomicU32::new(3);
    NEXT.fetch_add(1, Ordering::Relaxed).max(3)
}
