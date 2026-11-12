//! Running the machine: vCPU threads and the exit-dispatch loop.
//!
//! [`KvmMachine`](super::kvm::KvmMachine) builds the VM; this module drives it.
//! One thread per vCPU runs `KVM_RUN` in a loop and turns each exit into a call
//! on the [`DeviceBus`](super::bus::DeviceBus): MMIO reads/writes reach the
//! virtio-mmio transports, port I/O reaches the 16550 serial and friends.
//!
//! Shutdown and pause are cooperative. The loops watch a shared
//! [`VmmControl`] between exits, and `shutdown` joins every thread before the
//! machine's guest RAM is released, so no worker can touch memory after free.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use kvm_ioctls::{VcpuExit, VcpuFd};
use vm_device::bus::PioAddress;
use vm_device::MutDevicePio;

use super::bus::{DeviceBus, MmioDevice, PioDevice};
use super::kvm::{KvmMachine, Serial};
use crate::error::{Error, Result};

/// How long a paused vCPU sleeps between checks.
const PAUSE_POLL: Duration = Duration::from_millis(1);

/// Stop/pause state shared with every run-loop thread.
pub(crate) struct VmmControl {
    stop: AtomicBool,
    paused: AtomicBool,
}

impl VmmControl {
    fn new() -> Self {
        Self {
            stop: AtomicBool::new(false),
            paused: AtomicBool::new(false),
        }
    }

    fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub(crate) fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub(crate) fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    fn should_stop(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }
}

/// The 16550 UART on I/O port `0x3f8`, as a bus device.
///
/// The guest writes `console=ttyS0` output here for `earlyprintk`/panic logs;
/// we capture it so a boot failure is diagnosable.
pub(crate) struct SerialPort {
    inner: Serial,
}

impl SerialPort {
    fn new() -> Self {
        Self {
            inner: Serial::default(),
        }
    }
}

impl MutDevicePio for SerialPort {
    fn pio_read(&mut self, base: PioAddress, offset: u16, data: &mut [u8]) {
        self.inner.io_in(base.0.wrapping_add(offset), data);
    }

    fn pio_write(&mut self, base: PioAddress, offset: u16, data: &[u8]) {
        self.inner.io_out(base.0.wrapping_add(offset), data);
    }
}

/// A running microVM: the machine plus the threads driving its vCPUs.
pub(crate) struct Vmm {
    // Drop order matters: the bus and devices (which own the device worker
    // threads) are declared first so they are torn down before `machine`
    // releases guest RAM. `Drop::drop` additionally joins the vCPU threads.
    bus: Arc<Mutex<DeviceBus>>,
    devices: Vec<MmioDevice>,
    control: Arc<VmmControl>,
    vcpu_threads: Vec<JoinHandle<()>>,
    machine: KvmMachine,
}

impl Vmm {
    /// Spawn a run-loop thread per vCPU.
    pub(crate) fn start(
        mut machine: KvmMachine,
        bus: Arc<Mutex<DeviceBus>>,
        devices: Vec<MmioDevice>,
    ) -> Result<Self> {
        let control = Arc::new(VmmControl::new());
        let vcpus = machine.take_vcpus();
        if vcpus.is_empty() {
            return Err(Error::Hypervisor("VM has no vCPU to run".into()));
        }
        let mut vcpu_threads = Vec::with_capacity(vcpus.len());
        for (id, vcpu) in vcpus.into_iter().enumerate() {
            let bus = bus.clone();
            let control = control.clone();
            let thread = std::thread::Builder::new()
                .name(format!("vcpu-{id}"))
                .spawn(move || run_vcpu_loop(vcpu, bus, control))
                .map_err(|err| Error::Hypervisor(format!("spawn vCPU {id} thread: {err}")))?;
            vcpu_threads.push(thread);
        }
        Ok(Self {
            bus,
            devices,
            control,
            vcpu_threads,
            machine,
        })
    }

    pub(crate) fn control(&self) -> &VmmControl {
        &self.control
    }

    /// Transactional execution is not implemented, so `pause` only freezes the
    /// vCPUs between exits.
    pub(crate) fn pause(&self) {
        self.control.pause();
    }

    pub(crate) fn resume(&self) {
        self.control.resume();
    }

    fn shutdown(&mut self) {
        self.control.stop();
        for thread in self.vcpu_threads.drain(..) {
            let _ = thread.join();
        }
        // Drop the devices (stopping their workers) before the machine's RAM.
        self.devices.clear();
        let mut bus = self.bus.lock().unwrap_or_else(|err| err.into_inner());
        *bus = DeviceBus::new();
    }
}

impl Drop for Vmm {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Serve exits from one vCPU until it halts, shuts down, or the VM stops.
fn run_vcpu_loop(mut vcpu: VcpuFd, bus: Arc<Mutex<DeviceBus>>, control: Arc<VmmControl>) {
    while !control.should_stop() {
        if control.is_paused() {
            std::thread::sleep(PAUSE_POLL);
            continue;
        }
        let exit = match vcpu.run() {
            Ok(exit) => exit,
            Err(err) if err.errno() == libc::EINTR || err.errno() == libc::EAGAIN => continue,
            Err(_) => break,
        };
        let bus = bus.lock().unwrap_or_else(|err| err.into_inner());
        match exit {
            VcpuExit::IoOut(port, data) => {
                let _ = bus.pio_write(port, data);
            }
            VcpuExit::IoIn(port, data) => {
                let _ = bus.pio_read(port, data);
            }
            VcpuExit::MmioRead(addr, data) => {
                let _ = bus.mmio_read(addr, data);
            }
            VcpuExit::MmioWrite(addr, data) => {
                let _ = bus.mmio_write(addr, data);
            }
            VcpuExit::X86Rdmsr(msr) => {
                *msr.data = 0;
                *msr.error = 0;
            }
            VcpuExit::X86Wrmsr(msr) => {
                *msr.error = 0;
            }
            VcpuExit::Intr => continue,
            VcpuExit::Hlt | VcpuExit::Shutdown | VcpuExit::SystemEvent(_, _) => break,
            _ => {}
        }
    }
}

/// A serial device handle for the bus.
pub(crate) fn serial_device() -> PioDevice {
    Arc::new(Mutex::new(SerialPort::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_routes_through_the_device_bus() {
        let mut bus = DeviceBus::new();
        bus.register_pio(0x3f8, 8, serial_device()).unwrap();
        bus.pio_write(0x3f8, b"ok").unwrap();
        let mut status = [0u8; 1];
        bus.pio_read(0x3fd, &mut status).unwrap();
        assert_eq!(status[0], 0x60);
    }

    #[test]
    fn control_toggles_pause_and_stop() {
        let control = VmmControl::new();
        assert!(!control.is_paused());
        control.pause();
        assert!(control.is_paused());
        control.resume();
        assert!(!control.is_paused());
        assert!(!control.should_stop());
        control.stop();
        assert!(control.should_stop());
    }
}
