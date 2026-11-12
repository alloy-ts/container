//! Windows backend: WHPX (via the Windows Hypervisor Platform) + HNS/slirp
//! + AF_HYPERV sockets.
//!
//! TODO(windows): this is a compiling stub. The intended path is:
//!   * `WHvCreatePartition` / `WHvCreateVirtualProcessor` for the microVM
//!   * virtio devices backed by Cloud Hypervisor's Windows device model, or
//!     a small in-tree virtio implementation
//!   * HNS or slirp for the dedicated container IP
//!   * an AF_HYPERV socket (or a named pipe) for the guest agent
//! None of those APIs are available when this crate is built on Linux.

use async_trait::async_trait;

use super::{BootedVm, Hypervisor, VmConfig};
use crate::error::{Error, Result};

#[allow(dead_code)] // selected by `hypervisor::platform` on Windows
pub(crate) struct WindowsHypervisor;

#[allow(dead_code)]
impl WindowsHypervisor {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Hypervisor for WindowsHypervisor {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        let _ = cfg;
        Err(Error::Hypervisor(
            "TODO: Windows backend is not implemented (WHPX + HNS/slirp + AF_HYPERV)".into(),
        ))
    }

    fn pause(&mut self) -> Result<()> {
        Err(Error::Hypervisor(
            "TODO: Windows WHv vCPU pause is not implemented".into(),
        ))
    }

    fn resume(&mut self) -> Result<()> {
        Err(Error::Hypervisor(
            "TODO: Windows WHv vCPU resume is not implemented".into(),
        ))
    }

    fn shutdown_blocking(&mut self) {}

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Non-tier-1 hosts (for example FreeBSD, which napi lists as a target).
#[allow(dead_code)] // selected by `hypervisor::platform` on other operating systems
pub(crate) struct UnsupportedHypervisor;

#[async_trait]
impl Hypervisor for UnsupportedHypervisor {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        let _ = cfg;
        Err(Error::Hypervisor(
            "TODO: no hypervisor backend for this operating system".into(),
        ))
    }

    fn pause(&mut self) -> Result<()> {
        Err(Error::Hypervisor("no hypervisor backend".into()))
    }

    fn resume(&mut self) -> Result<()> {
        Err(Error::Hypervisor("no hypervisor backend".into()))
    }

    fn shutdown_blocking(&mut self) {}

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}
