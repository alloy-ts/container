//! Linux microVM host: KVM + TAP + AF_VSOCK.
//!
//! What is implemented on x86_64:
//!   * open `/dev/kvm`, create a VM, map guest RAM, create vCPUs, install CPUID
//!   * parse a bzImage, load it, build an identity map and 64-bit boot state
//!   * open a TAP device and assign its IPv4 address
//!   * bind an AF_VSOCK port on the host
//!
//! What is not (each call site says TODO):
//!   * virtio-net, virtio-fs / virtio-blk, and virtio-vsock device emulation
//!   * driving `KVM_RUN` as the steady state of `Container::create`
//!
//! `boot` therefore builds the machine and then rolls it back with an explicit
//! error instead of returning a container whose guest can never reach the
//! agent. arm64 Linux is a stub: the kernel loader is x86_64 only.

mod tap;
pub(crate) mod vsock;

#[cfg(target_arch = "x86_64")]
mod boot;
#[cfg(target_arch = "x86_64")]
pub(crate) mod bus;
#[cfg(target_arch = "x86_64")]
mod irq;
#[cfg(target_arch = "x86_64")]
mod kvm;
#[cfg(target_arch = "x86_64")]
pub(crate) mod virtio;

use async_trait::async_trait;

use super::{BootedVm, Hypervisor, VmConfig};
use crate::error::{Error, Result};

#[cfg(target_arch = "x86_64")]
use kvm::KvmMachine;
#[cfg(target_arch = "x86_64")]
use tap::TapDevice;

pub(crate) struct LinuxHypervisor {
    #[cfg(target_arch = "x86_64")]
    machine: Option<KvmMachine>,
    #[cfg(target_arch = "x86_64")]
    tap: Option<TapDevice>,
    paused: bool,
}

impl LinuxHypervisor {
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(target_arch = "x86_64")]
            machine: None,
            #[cfg(target_arch = "x86_64")]
            tap: None,
            paused: false,
        }
    }

    #[cfg(target_arch = "x86_64")]
    async fn boot_x86(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        let kernel = std::fs::read(&cfg.kernel_path).map_err(|err| {
            Error::Hypervisor(format!(
                "failed to read kernel {}: {err}",
                cfg.kernel_path.display()
            ))
        })?;
        // Fail before allocating guest RAM when the image is not a bzImage.
        boot::validate_header(&kernel)?;

        let mut machine = KvmMachine::create(cfg.vcpus, cfg.memory_mb)?;
        let loaded = machine.load_kernel(&kernel, &cfg.cmdline)?;
        if let Some(network) = &cfg.network {
            self.tap = Some(TapDevice::open(network)?);
        }
        // Shares are retained on the config the device model will consume.
        let _shares = &cfg.shares;
        let _loaded = loaded;
        self.machine = Some(machine);

        // TODO(linux): attach virtio-blk/virtio-fs for `cfg.shares`, virtio-net
        // for the TAP, and a virtio-vsock (or vhost-vsock) device bound to
        // `cfg.vsock_cid`, then run the vCPU thread (`kvm::run_vcpu`) until
        // the guest init connects. Until that device model exists the guest
        // cannot speak the agent protocol, so we do not report success.
        Err(Error::Hypervisor(format!(
            "TODO: virtio-vsock/virtio-fs/virtio-net device model is not implemented; \
             KVM machine for '{}' was created (cid {}, port {}) and rolled back",
            cfg.id, cfg.vsock_cid, cfg.vsock_port
        )))
    }
}

#[async_trait]
impl Hypervisor for LinuxHypervisor {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        #[cfg(target_arch = "x86_64")]
        {
            let result = self.boot_x86(cfg).await;
            if result.is_err() {
                self.shutdown_blocking();
            }
            result
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = cfg;
            Err(Error::Hypervisor(
                "TODO: the Linux hypervisor backend is implemented for x86_64 only \
                 (arm64 Image + FDT boot is not done)"
                    .into(),
            ))
        }
    }

    fn pause(&mut self) -> Result<()> {
        #[cfg(target_arch = "x86_64")]
        if self.machine.is_none() {
            return Err(Error::Hypervisor(
                "cannot pause a VM that is not running".into(),
            ));
        }
        // The vCPU thread (see `kvm::run_vcpu`) checks this flag between exits.
        // TODO(linux): interrupt an in-progress KVM_RUN so pause takes effect
        // before the next VM exit.
        self.paused = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        if !self.paused {
            return Err(Error::Hypervisor("VM is not paused".into()));
        }
        self.paused = false;
        Ok(())
    }

    fn shutdown_blocking(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            self.machine.take();
            self.tap.take();
        }
        self.paused = false;
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.shutdown_blocking();
        Ok(())
    }
}

impl Drop for LinuxHypervisor {
    fn drop(&mut self) {
        self.shutdown_blocking();
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
mod tests {
    use super::*;
    use crate::hypervisor::VmConfig;
    use std::path::PathBuf;

    fn cfg(kernel: PathBuf) -> VmConfig {
        VmConfig {
            id: "kvm-test".into(),
            vcpus: 1,
            memory_mb: 64,
            kernel_path: kernel,
            cmdline: "console=ttyS0".into(),
            vsock_cid: 3,
            vsock_port: 1024,
            network: None,
            shares: Vec::new(),
        }
    }

    #[tokio::test]
    async fn missing_kernel_fails_before_kvm() {
        let mut hv = LinuxHypervisor::new();
        let err = match hv
            .boot(cfg(PathBuf::from("/no/such/container-native-kernel")))
            .await
        {
            Err(err) => err,
            Ok(_) => panic!("missing kernel should not boot"),
        };
        let msg = err.to_string();
        assert!(msg.contains("kernel"), "{msg}");
    }

    #[tokio::test]
    async fn parsed_kernel_reaches_kvm_or_the_virtio_todo() {
        let dir = std::env::temp_dir().join(format!(
            "container-native-kernel-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vmlinuz");
        std::fs::write(&path, boot::synthesize_bzimage(b"\x90\xf4")).unwrap();
        let mut hv = LinuxHypervisor::new();
        let err = match hv.boot(cfg(path)).await {
            Err(err) => err,
            Ok(_) => panic!("virtio device model is not implemented; boot should fail"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("kvm") || msg.contains("KVM") || msg.contains("TODO"),
            "{msg}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
