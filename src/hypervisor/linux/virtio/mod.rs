//! Virtio device model for the in-process Linux VMM.
//!
//! A virtio device has two halves that rust-vmm deliberately keeps separate:
//!
//!   * the **transport** ([`mmio::VirtioMmioTransport`]) — the register file the
//!     guest driver pokes over MMIO (feature negotiation, queue setup, status);
//!   * the **device** ([`VirtioDevice`]) — the queue-processing logic that moves
//!     data between guest memory and a host resource (AF_VSOCK, a rootfs, a TAP).
//!
//! The transport owns the descriptor rings (`virtio_queue::QueueSync`) and hands
//! the device a shared handle to them when the driver sets `DRIVER_OK`. From then
//! on the vCPU thread drives the transport while a device worker thread drains
//! the queues; they meet at the shared [`Interrupt`] line.

pub(crate) mod fs;
pub(crate) mod mmio;
pub(crate) mod net;
pub(crate) mod vsock;

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use vm_memory::GuestMemoryMmap;
use virtio_queue::QueueSync;

use super::irq::Gsi;

/// The interrupt line a virtio device raises when it wants the guest's
/// attention.
///
/// The MMIO transport reads the pending-bits register from here, and a device
/// worker sets bit 0 (`VIRTIO_MMIO_INT_VRING`) after it has added buffers to a
/// used ring. Both sides share the same `AtomicU8` and the same `irqfd`, so
/// signalling is lock-free and works across the vCPU and worker threads.
#[derive(Clone)]
pub(crate) struct Interrupt {
    status: Arc<AtomicU8>,
    line: u32,
    /// The `irqfd` to signal. `None` only in unit tests, which run without KVM.
    gsi: Option<Arc<Gsi>>,
}

/// Pending used-buffer notification (`VIRTIO_MMIO_INT_VRING`).
pub(crate) const INT_VRING: u8 = 1;
/// Pending config-change notification (`VIRTIO_MMIO_INT_CONFIG`).
pub(crate) const INT_CONFIG: u8 = 2;

impl Interrupt {
    pub(crate) fn new(gsi: Gsi) -> Self {
        let line = gsi.line();
        Self {
            status: Arc::new(AtomicU8::new(0)),
            line,
            gsi: Some(Arc::new(gsi)),
        }
    }

    /// An interrupt with no `irqfd` behind it. Unit tests use this because
    /// creating a real line needs a KVM VM.
    #[cfg(test)]
    pub(crate) fn for_test() -> Self {
        Self {
            status: Arc::new(AtomicU8::new(0)),
            line: 0,
            gsi: None,
        }
    }

    /// The GSI (IOAPIC pin) this device is wired to.
    pub(crate) fn line(&self) -> u32 {
        self.line
    }

    /// A device worker finished with some buffers: set the used-ring bit and
    /// raise the line.
    pub(crate) fn signal_used(&self) {
        self.raise(INT_VRING);
    }

    /// The device config space changed (virtio-fs, virtio-net MAC): set the
    /// config bit and raise the line.
    pub(crate) fn signal_config(&self) {
        self.raise(INT_CONFIG);
    }

    fn raise(&self, bits: u8) {
        self.status.fetch_or(bits, Ordering::AcqRel);
        if let Some(gsi) = &self.gsi {
            gsi.trigger();
        }
    }

    /// Read the pending-bits register.
    pub(crate) fn status(&self) -> u8 {
        self.status.load(Ordering::Acquire)
    }

    /// Driver acked `bits`; clear them.
    pub(crate) fn ack(&self, bits: u8) {
        self.status.fetch_and(!bits, Ordering::AcqRel);
    }
}

/// Everything a device needs once the driver has activated it.
///
/// Handed over on the `DRIVER_OK` status write. The guest-memory handle and the
/// queue handles are all reference counted, so the transport (vCPU thread) and
/// the device worker share them.
pub(crate) struct Activate {
    pub(crate) mem: GuestMemoryMmap,
    pub(crate) queues: Vec<QueueSync>,
    pub(crate) interrupt: Interrupt,
}

/// The data-plane half of a virtio device.
///
/// Implementations must be movable to a worker thread (`Send`). The transport
/// calls `activate` once, when the driver writes `DRIVER_OK`, and `notify` on
/// every doorbell.
pub(crate) trait VirtioDevice: Send {
    /// Virtio device id, as reported by `VIRTIO_MMIO_DEVICE_ID`
    /// (e.g. `VIRTIO_ID_VSOCK`).
    fn device_id(&self) -> u32;

    /// Bits the device offers. The transport handles `VIRTIO_F_VERSION_1`
    /// automatically; this should list the device-specific feature bits.
    fn features(&self) -> u64;

    /// Read the device-specific config space at `offset`.
    fn read_config(&self, offset: u64, data: &mut [u8]);

    /// Write the device-specific config space at `offset`. Most devices are
    /// read-only and can keep the default.
    fn write_config(&mut self, offset: u64, data: &[u8]) {
        let _ = (offset, data);
    }

    /// Number of virtqueues.
    fn queue_count(&self) -> usize;

    /// Maximum queue size the transport advertises via `QUEUE_NUM_MAX`.
    fn queue_max_size(&self) -> u16 {
        256
    }

    /// Driver is ready: start whatever worker the device needs.
    fn activate(&mut self, ctx: Activate) -> crate::error::Result<()>;

    /// The driver rang the doorbell for `queue_index`.
    fn notify(&mut self, queue_index: u16);

    /// The driver reset the device (status written as 0).
    fn reset(&mut self);
}

/// The MMIO window each virtio-mmio device is mapped into (4 KiB), matching
/// both the bus registration and the `virtio_mmio.device=` command-line size.
pub(crate) const MMIO_SIZE: u64 = 0x1000;

/// One `virtio_mmio.device=` kernel-command-line property, e.g.
/// `virtio_mmio.device=4K@0xd0000000:5`.
///
/// The guest's `virtio_mmio` driver reads these at boot and probes each device
/// at `base` with interrupt line `irq`, which is why no ACPI or PCI enumeration
/// is needed.
pub(crate) fn mmio_device_arg(base: u64, irq: u32) -> String {
    format!("virtio_mmio.device=4K@{base:#x}:{irq}")
}

/// Append the transport properties for `(base, irq)` pairs to `cmdline`.
pub(crate) fn append_device_args(cmdline: &str, devices: &[(u64, u32)]) -> String {
    let mut out = cmdline.trim_end().to_string();
    for (base, irq) in devices {
        out.push(' ');
        out.push_str(&mmio_device_arg(*base, *irq));
    }
    out
}

/// Hands out the MMIO window and IRQ line for each virtio device.
///
/// Windows are placed at 4 KiB steps above the PCI hole (`0xd000_0000`, matching
/// the boot sequence) and lines start at GSI 5, so a device is fully described
/// by the `(base, irq)` pair returned here.
pub(crate) struct MmioLayout {
    next_base: u64,
    next_irq: u32,
}

impl MmioLayout {
    pub(crate) const FIRST_BASE: u64 = 0xd000_0000;
    pub(crate) const FIRST_IRQ: u32 = 5;

    pub(crate) fn new() -> Self {
        Self {
            next_base: Self::FIRST_BASE,
            next_irq: Self::FIRST_IRQ,
        }
    }

    /// Reserve the next window and line.
    pub(crate) fn place(&mut self) -> (u64, u32) {
        let placed = (self.next_base, self.next_irq);
        self.next_base += MMIO_SIZE;
        self.next_irq += 1;
        placed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_args_match_the_kernel_property_format() {
        assert_eq!(mmio_device_arg(0xd000_0000, 5), "virtio_mmio.device=4K@0xd0000000:5");
    }

    #[test]
    fn appending_transports_keeps_the_base_cmdline() {
        let cmdline = append_device_args(
            "console=ttyS0 pci=off",
            &[(0xd000_0000, 5), (0xd000_1000, 6)],
        );
        assert_eq!(
            cmdline,
            "console=ttyS0 pci=off virtio_mmio.device=4K@0xd0000000:5 virtio_mmio.device=4K@0xd0001000:6"
        );
    }

    #[test]
    fn layout_places_devices_above_the_pci_hole() {
        let mut layout = MmioLayout::new();
        let vsock = layout.place();
        let fs = layout.place();
        assert_eq!(vsock, (0xd000_0000, 5));
        assert_eq!(fs, (0xd000_1000, 6));

        // The two placements produce exactly the boot-sequence cmdline.
        let cmdline = append_device_args("console=ttyS0", &[vsock, fs]);
        assert_eq!(
            cmdline,
            "console=ttyS0 virtio_mmio.device=4K@0xd0000000:5 virtio_mmio.device=4K@0xd0001000:6"
        );
    }

    #[test]
    fn interrupt_state_is_shared_across_clones() {
        let interrupt = Interrupt::for_test();
        let worker = interrupt.clone();
        worker.signal_used();
        assert_eq!(interrupt.status(), INT_VRING);
        interrupt.ack(INT_VRING);
        assert_eq!(worker.status(), 0);
    }
}
