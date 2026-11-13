//! Linux microVM host: KVM + virtio-mmio + TAP.
//!
//! What `boot` does on x86_64:
//!   * open `/dev/kvm`, create a VM, map guest RAM, create vCPUs, install CPUID
//!   * parse a bzImage, load it, build an identity map and 64-bit boot state
//!   * create the in-kernel LAPIC/IOAPIC/PIT and route a GSI per device
//!   * place a virtio-vsock device on an MMIO window and append the
//!     `virtio_mmio.device=` property to the kernel command line
//!   * spawn a vCPU thread that dispatches MMIO/port-I/O to the device bus
//!   * wait for the guest init to connect into the vsock proxy and hand that
//!     stream to the agent client
//!
//! virtio-fs, virtio-net, and virtio-vsock devices are served over virtio-mmio,
//! and the loader places an initramfs (the `guest-init` PID-1 binary) in guest
//! RAM for the guest to run. `boot` still needs a bootable image — a kernel with
//! the virtio drivers built in plus that initramfs — so without one it runs the
//! machine and reports the handshake failure rather than pretending to succeed.
//! arm64 Linux is a stub: the kernel loader is x86_64 only.

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
#[cfg(target_arch = "x86_64")]
mod vmm;

use async_trait::async_trait;

use super::{BootedVm, Hypervisor, VmConfig};
use crate::error::{Error, Result};

#[cfg(target_arch = "x86_64")]
use std::sync::{Arc, Mutex};
#[cfg(target_arch = "x86_64")]
use std::time::Duration;

#[cfg(target_arch = "x86_64")]
use super::AgentLink;
#[cfg(target_arch = "x86_64")]
use bus::{DeviceBus, MmioDevice};
#[cfg(target_arch = "x86_64")]
use kvm::KvmMachine;
#[cfg(target_arch = "x86_64")]
use tap::TapDevice;
#[cfg(target_arch = "x86_64")]
use virtio::fs::{FsConfig, VirtioFs};
#[cfg(target_arch = "x86_64")]
use virtio::mmio::VirtioMmioTransport;
#[cfg(target_arch = "x86_64")]
use virtio::net::{NetConfig, VirtioNet};
#[cfg(target_arch = "x86_64")]
use virtio::vsock::{VirtioVsock, VsockConfig};
#[cfg(target_arch = "x86_64")]
use virtio::{MmioLayout, MMIO_SIZE};

#[cfg(target_arch = "x86_64")]
const SERIAL_PORT: u16 = 0x3f8;
/// How long `boot` waits for the guest init to reach the vsock proxy.
#[cfg(target_arch = "x86_64")]
const BOOT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct LinuxHypervisor {
    #[cfg(target_arch = "x86_64")]
    vmm: Option<vmm::Vmm>,
    #[cfg(target_arch = "x86_64")]
    tap: Option<TapDevice>,
    paused: bool,
}

impl LinuxHypervisor {
    pub(crate) fn new() -> Self {
        Self {
            #[cfg(target_arch = "x86_64")]
            vmm: None,
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

        let initramfs = match &cfg.initramfs_path {
            Some(path) => std::fs::read(path).map_err(|err| {
                Error::Hypervisor(format!(
                    "failed to read initramfs {}: {err}",
                    path.display()
                ))
            })?,
            None => Vec::new(),
        };

        let mut machine = KvmMachine::create(cfg.vcpus, cfg.memory_mb)?;

        // Place the virtio-mmio devices and extend the kernel command line so
        // the guest probes each one without ACPI or PCI enumeration. virtio-net
        // gets a window only when the config asks for a NIC; every rootfs or
        // bind-mount share gets its own virtio-fs window.
        let mut layout = MmioLayout::new();
        let vsock_slot = layout.place();
        let net_slot = cfg.network.as_ref().map(|_| layout.place());
        let fs_slots: Vec<(u64, u32)> = cfg.shares.iter().map(|_| layout.place()).collect();
        let mut placements = vec![vsock_slot];
        placements.extend(net_slot);
        placements.extend(fs_slots.iter().copied());
        let cmdline = virtio::append_device_args(&cfg.cmdline, &placements);
        machine.load_kernel(&kernel, &initramfs, &cmdline)?;

        // In-kernel LAPIC/IOAPIC/PIT, plus a routable line per device irqfd.
        let lines: Vec<u32> = placements.iter().map(|(_, irq)| *irq).collect();
        irq::install(machine.vm(), &lines)?;
        let vsock_gsi = irq::Gsi::register(machine.vm(), vsock_slot.1)?;

        // Device bus: the 16550 serial (boot logs) and the virtio devices.
        let mut bus = DeviceBus::new();
        bus.register_pio(SERIAL_PORT, 8, vmm::serial_device())?;
        let mut devices: Vec<MmioDevice> = Vec::new();

        // The vsock device dials this Unix socket when the guest connects; the
        // accepted host end is what the agent client speaks over.
        let proxy_path = proxy_socket_path(&cfg.id);
        let _ = std::fs::remove_file(&proxy_path);
        let listener = std::os::unix::net::UnixListener::bind(&proxy_path).map_err(|err| {
            Error::Hypervisor(format!("bind vsock proxy {}: {err}", proxy_path.display()))
        })?;

        let vsock = VirtioVsock::new(VsockConfig {
            guest_cid: u64::from(cfg.vsock_cid),
            port: cfg.vsock_port,
            proxy_path: proxy_path.clone(),
        });
        let transport = VirtioMmioTransport::new(
            Box::new(vsock),
            machine.memory().clone(),
            virtio::Interrupt::new(vsock_gsi),
        )?;
        let device: MmioDevice = Arc::new(Mutex::new(transport));
        bus.register_mmio(vsock_slot.0, MMIO_SIZE, device.clone())?;
        devices.push(device);

        // virtio-net bridges the NIC virtqueues to the host tap. The tap is
        // opened here and cloned for the device worker; the original is kept so
        // the interface outlives the device.
        if let (Some(network), Some(slot)) = (&cfg.network, net_slot) {
            let tap = TapDevice::open(network)?;
            let tap_for_device = tap
                .try_clone()
                .map_err(|err| Error::Hypervisor(format!("clone tap for virtio-net: {err}")))?;
            let net_gsi = irq::Gsi::register(machine.vm(), slot.1)?;
            let net = VirtioNet::new(
                NetConfig {
                    mac: network.mac,
                    link_up: true,
                },
                Box::new(tap_for_device),
            );
            let transport = VirtioMmioTransport::new(
                Box::new(net),
                machine.memory().clone(),
                virtio::Interrupt::new(net_gsi),
            )?;
            let device: MmioDevice = Arc::new(Mutex::new(transport));
            bus.register_mmio(slot.0, MMIO_SIZE, device.clone())?;
            devices.push(device);
            self.tap = Some(tap);
        }

        // Each share becomes a virtio-fs device the guest mounts by its tag.
        for (share, slot) in cfg.shares.iter().zip(fs_slots.iter()) {
            let fs_gsi = irq::Gsi::register(machine.vm(), slot.1)?;
            let fs = VirtioFs::new(FsConfig {
                tag: share.tag.clone(),
                root_dir: share.path.clone(),
            })?;
            let transport = VirtioMmioTransport::new(
                Box::new(fs),
                machine.memory().clone(),
                virtio::Interrupt::new(fs_gsi),
            )?;
            let device: MmioDevice = Arc::new(Mutex::new(transport));
            bus.register_mmio(slot.0, MMIO_SIZE, device.clone())?;
            devices.push(device);
        }

        let running = vmm::Vmm::start(machine, Arc::new(Mutex::new(bus)), devices)?;
        self.vmm = Some(running);

        // Wait for the guest init to reach the proxy, then hand the stream to
        // the agent client.
        let incoming =
            tokio::task::spawn_blocking(move || accept_agent(&listener, BOOT_CONNECT_TIMEOUT))
                .await
                .map_err(|err| Error::Hypervisor(format!("vsock accept task: {err}")))??;
        let _ = std::fs::remove_file(&proxy_path);
        let stream = tokio::net::UnixStream::from_std(incoming)
            .map_err(|err| Error::Hypervisor(format!("adopt vsock proxy stream: {err}")))?;

        Ok(BootedVm {
            link: AgentLink::Connected(Box::new(stream)),
            host_pid: None,
        })
    }
}

/// A per-VM vsock proxy socket path under the temp directory.
#[cfg(target_arch = "x86_64")]
fn proxy_socket_path(id: &str) -> std::path::PathBuf {
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    std::env::temp_dir().join(format!(
        "container-native-vsock-{}-{safe}.sock",
        std::process::id()
    ))
}

/// Accept the guest-init connection into the proxy, or time out.
///
/// Runs on a blocking thread: the guest may take seconds to reach the init, and
/// we must not block the async runtime while waiting.
#[cfg(target_arch = "x86_64")]
fn accept_agent(
    listener: &std::os::unix::net::UnixListener,
    timeout: Duration,
) -> Result<std::os::unix::net::UnixStream> {
    use std::time::Instant;
    listener
        .set_nonblocking(true)
        .map_err(|err| Error::Hypervisor(format!("vsock proxy nonblocking: {err}")))?;
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(true)
                    .map_err(|err| Error::Hypervisor(format!("vsock proxy stream: {err}")))?;
                return Ok(stream);
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(Error::Hypervisor(
                        "guest init did not connect to the vsock proxy before the boot deadline \
                         (the guest could not mount its rootfs, or the init binary is missing)"
                            .into(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => {
                return Err(Error::Hypervisor(format!("vsock proxy accept: {err}")));
            }
        }
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
        match &self.vmm {
            Some(running) => running.pause(),
            None => {
                return Err(Error::Hypervisor(
                    "cannot pause a VM that is not running".into(),
                ))
            }
        }
        // The vCPU thread checks the flag between exits. A `KVM_RUN` already in
        // progress is not interrupted, so pause takes effect at the next exit.
        self.paused = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        if !self.paused {
            return Err(Error::Hypervisor("VM is not paused".into()));
        }
        #[cfg(target_arch = "x86_64")]
        if let Some(running) = &self.vmm {
            running.resume();
        }
        self.paused = false;
        Ok(())
    }

    fn shutdown_blocking(&mut self) {
        #[cfg(target_arch = "x86_64")]
        {
            // Dropping the VMM joins the vCPU threads and stops device workers
            // before the guest RAM mapping is released.
            self.vmm.take();
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
            initramfs_path: None,
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
    async fn parsed_kernel_reaches_kvm_or_the_boot_handshake() {
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
            Ok(_) => panic!("without a bootable guest init the handshake must not succeed"),
        };
        // Either KVM is unavailable (the common case in CI) or the guest never
        // reached the proxy. Both are the expected outcomes here.
        let msg = err.to_string();
        assert!(
            msg.contains("kvm")
                || msg.contains("KVM")
                || msg.contains("guest init")
                || msg.contains("vsock"),
            "{msg}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
