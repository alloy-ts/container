//! macOS backend: Virtualization.framework + vmnet + VZVirtioSocket.
//!
//! TODO(macos): this is a compiling stub. The intended path is:
//!   * `VZVirtualMachineConfiguration` with the kernel, memory, and vCPU count
//!   * a `VZVirtioFileSystemDeviceConfiguration` per virtio-fs share
//!   * `VZVirtioSocketDeviceConfiguration` for the guest agent
//!   * vmnet (or `VZFileHandleNetworkDeviceAttachment`) for the dedicated IP
//! The Objective-C / Swift bindings are not linked here, and this crate is
//! developed on Linux where those frameworks are not present.

use async_trait::async_trait;

use super::{BootedVm, Hypervisor, VmConfig};
use crate::error::{Error, Result};

#[allow(dead_code)] // selected by `hypervisor::platform` on macOS
pub(crate) struct MacosHypervisor;

#[allow(dead_code)]
impl MacosHypervisor {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Hypervisor for MacosHypervisor {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        let _ = cfg;
        Err(Error::Hypervisor(
            "TODO: macOS backend is not implemented (Virtualization.framework + vmnet + VZVirtioSocket)"
                .into(),
        ))
    }

    fn pause(&mut self) -> Result<()> {
        Err(Error::Hypervisor(
            "TODO: macOS VZVirtualMachine pause is not implemented".into(),
        ))
    }

    fn resume(&mut self) -> Result<()> {
        Err(Error::Hypervisor(
            "TODO: macOS VZVirtualMachine resume is not implemented".into(),
        ))
    }

    fn shutdown_blocking(&mut self) {}

    async fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}
