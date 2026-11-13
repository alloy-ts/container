//! In-kernel interrupt controller and guest interrupt lines.
//!
//! Bringing up the LAPIC, IOAPIC and PIT in the kernel is what removes
//! interrupt-controller emulation from this VMM: the guest's `apic`/`ioapic`
//! driver talks straight to KVM. This module only has to (a) create those
//! in-kernel devices and (b) hand each virtio-mmio device a line it can raise.
//!
//! A line is an `irqfd`: an [`EventFd`] whose signalling KVM turns into an
//! assertion of a GSI. Routing each GSI to an IOAPIC pin is the piece that
//! makes `virtio_mmio.device=...:5` on the kernel command line actually
//! deliver an interrupt.

use kvm_bindings::{
    kvm_irq_routing, kvm_irq_routing_entry, kvm_irq_routing_entry__bindgen_ty_1,
    kvm_irq_routing_irqchip, kvm_pit_config, KVM_IRQCHIP_IOAPIC, KVM_IRQ_ROUTING_IRQCHIP,
    KVM_PIT_SPEAKER_DUMMY,
};
use kvm_ioctls::VmFd;
use vmm_sys_util::eventfd::EventFd;

use crate::error::{Error, Result};

/// A guest interrupt line, raised by writing to an `irqfd`.
pub(crate) struct Gsi {
    fd: EventFd,
    line: u32,
}

impl Gsi {
    /// Create an eventfd and register it with KVM as the *irqfd* for `line`.
    /// Run [`install`] first so the line is routed before it is used.
    pub(crate) fn register(vm: &VmFd, line: u32) -> Result<Self> {
        let fd = EventFd::new(libc::EFD_NONBLOCK)
            .map_err(|err| Error::Hypervisor(format!("irqfd eventfd for irq {line}: {err}")))?;
        vm.register_irqfd(&fd, line)
            .map_err(|err| Error::Hypervisor(format!("register_irqfd irq {line}: {err}")))?;
        Ok(Self { fd, line })
    }

    /// The GSI this line is wired to.
    pub(crate) fn line(&self) -> u32 {
        self.line
    }

    /// Assert the line. A full eventfd counter means an interrupt is already
    /// pending and the guest has not acked it, so the extra write is dropped.
    pub(crate) fn trigger(&self) {
        let _ = self.fd.write(1);
    }
}

/// Create the in-kernel LAPIC/IOAPIC and PIT, and route each GSI in `lines`
/// to the matching IOAPIC pin.
pub(crate) fn install(vm: &VmFd, lines: &[u32]) -> Result<()> {
    vm.create_irq_chip()
        .map_err(|err| Error::Hypervisor(format!("create_irq_chip: {err}")))?;
    vm.create_pit2(kvm_pit_config {
        flags: KVM_PIT_SPEAKER_DUMMY,
        ..Default::default()
    })
    .map_err(|err| Error::Hypervisor(format!("create_pit2: {err}")))?;

    let routing = routing_buffer(lines);
    // SAFETY: `routing` is `u64`-aligned and holds a `kvm_irq_routing` header
    // followed by exactly `lines.len()` entries, which is the layout the kernel
    // reads back through the `nr` field.
    let routing = unsafe { &*(routing.as_ptr() as *const kvm_irq_routing) };
    vm.set_gsi_routing(routing)
        .map_err(|err| Error::Hypervisor(format!("set_gsi_routing: {err}")))?;
    Ok(())
}

/// Build the `KVM_SET_GSI_ROUTING` payload: a header plus one IRQCHIP entry per
/// line, each pinned to the IOAPIC. The buffer is `Vec<u64>` so the header is
/// correctly aligned for the cast in [`install`].
fn routing_buffer(lines: &[u32]) -> Vec<u64> {
    let header = std::mem::size_of::<kvm_irq_routing>();
    let entry = std::mem::size_of::<kvm_irq_routing_entry>();
    let bytes = header + lines.len() * entry;
    let mut buffer = vec![0u64; bytes.div_ceil(8)];
    let ptr = buffer.as_mut_ptr() as *mut kvm_irq_routing;
    // SAFETY: `buffer` is large enough for the header plus one entry per line,
    // and `u64` alignment satisfies the `kvm_irq_routing` alignment.
    unsafe {
        (*ptr).nr = lines.len() as u32;
        (*ptr).flags = 0;
        let entries = (*ptr).entries.as_mut_slice(lines.len());
        for (slot, line) in entries.iter_mut().zip(lines) {
            *slot = ioapic_route(*line);
        }
    }
    buffer
}

fn ioapic_route(line: u32) -> kvm_irq_routing_entry {
    kvm_irq_routing_entry {
        gsi: line,
        type_: KVM_IRQ_ROUTING_IRQCHIP,
        flags: 0,
        pad: 0,
        u: kvm_irq_routing_entry__bindgen_ty_1 {
            irqchip: kvm_irq_routing_irqchip {
                irqchip: KVM_IRQCHIP_IOAPIC,
                pin: line,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_buffer_carries_every_line_to_the_ioapic() {
        let buffer = routing_buffer(&[5, 6]);
        let ptr = buffer.as_ptr() as *const kvm_irq_routing;
        // SAFETY: `routing_buffer` wrote the header and two entries above.
        unsafe {
            assert_eq!((*ptr).nr, 2);
            let entries = (*ptr).entries.as_slice(2);
            assert_eq!(entries[0].gsi, 5);
            assert_eq!(entries[1].gsi, 6);
            assert_eq!(entries[0].type_, KVM_IRQ_ROUTING_IRQCHIP);
            assert_eq!(entries[0].u.irqchip.irqchip, KVM_IRQCHIP_IOAPIC);
            assert_eq!(entries[1].u.irqchip.pin, 6);
        }
    }

    #[test]
    fn routing_buffer_is_aligned_for_the_fam_header() {
        let buffer = routing_buffer(&[5]);
        assert_eq!(
            buffer.as_ptr() as usize % std::mem::align_of::<kvm_irq_routing>(),
            0
        );
    }
}
