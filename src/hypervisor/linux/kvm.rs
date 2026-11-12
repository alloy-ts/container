//! KVM VM, guest RAM, and vCPU construction.
//!
//! This module owns the machine — the `/dev/kvm` handle, the VM fd, guest RAM,
//! and the configured vCPUs — and hands the pieces out to the runtime in
//! [`super::vmm`]: `vm()` for interrupt/irqfd setup, `memory()` for the device
//! model, and `take_vcpus()` for the run-loop threads.

use kvm_bindings::{kvm_segment, kvm_userspace_memory_region, KVM_MAX_CPUID_ENTRIES};
use kvm_ioctls::{Kvm, VcpuFd, VmFd};
use vm_memory::{GuestAddress, GuestMemory, GuestMemoryMmap};

use super::boot::{self, LoadedKernel};
use crate::error::{Error, Result};

pub(crate) struct KvmMachine {
    // Drop order is field order. vCPUs and the VM fd must close before the
    // mapping is unmapped, and the system fd last.
    vcpus: Vec<VcpuFd>,
    _vm: VmFd,
    _kvm: Kvm,
    memory: GuestMemoryMmap,
    size: usize,
}

impl KvmMachine {
    pub(crate) fn create(vcpus: u32, memory_mb: u64) -> Result<Self> {
        if vcpus == 0 {
            return Err(Error::Hypervisor("vcpus must be at least 1".into()));
        }
        let kvm = Kvm::new().map_err(|err| {
            Error::Hypervisor(format!(
                "kvm: {err} (is /dev/kvm accessible to this process?)"
            ))
        })?;
        let vm = kvm
            .create_vm()
            .map_err(|err| Error::Hypervisor(format!("kvm create_vm: {err}")))?;
        let size = memory_bytes(memory_mb);
        let memory = GuestMemoryMmap::from_ranges(&[(GuestAddress(0), size)])
            .map_err(|err| Error::Hypervisor(format!("allocate guest RAM ({size} bytes): {err}")))?;
        let host = memory
            .get_host_address(GuestAddress(0))
            .map_err(|err| Error::Hypervisor(format!("guest RAM host address: {err}")))?;
        // SAFETY: `memory` lives in this struct for as long as `vm` does, is
        // page-aligned (mmap), and is not aliased. Drop order closes the VM
        // fd before the mapping.
        unsafe {
            vm.set_user_memory_region(kvm_userspace_memory_region {
                slot: 0,
                flags: 0,
                guest_phys_addr: 0,
                memory_size: size as u64,
                userspace_addr: host as u64,
            })
            .map_err(|err| Error::Hypervisor(format!("kvm set_user_memory_region: {err}")))?;
        }
        let mut cpuids = kvm
            .get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)
            .map_err(|err| Error::Hypervisor(format!("kvm get_supported_cpuid: {err}")))?;
        let mut vcpu_fds = Vec::with_capacity(vcpus as usize);
        for id in 0..vcpus {
            let vcpu = vm
                .create_vcpu(u64::from(id))
                .map_err(|err| Error::Hypervisor(format!("kvm create_vcpu {id}: {err}")))?;
            vcpu.set_cpuid2(&cpuids)
                .map_err(|err| Error::Hypervisor(format!("kvm set_cpuid2: {err}")))?;
            vcpu_fds.push(vcpu);
        }
        let _ = &mut cpuids;
        Ok(Self {
            vcpus: vcpu_fds,
            _vm: vm,
            _kvm: kvm,
            memory,
            size,
        })
    }

    /// Guest RAM, as the `vm-memory` view shared with the vCPU and device
    /// threads. The mapping is reference counted, so cloning this handle is
    /// what a worker thread uses to reach guest memory.
    pub(crate) fn memory(&self) -> &GuestMemoryMmap {
        &self.memory
    }

    /// The VM fd, for interrupt-controller setup, irqfd, and ioeventfd.
    pub(crate) fn vm(&self) -> &VmFd {
        &self._vm
    }

    /// Hand the vCPU fds to the run-loop threads. Called once, after the boot
    /// state is configured, so the machine keeps only the VM fd and the RAM.
    pub(crate) fn take_vcpus(&mut self) -> Vec<VcpuFd> {
        std::mem::take(&mut self.vcpus)
    }

    pub(crate) fn load_kernel(&mut self, kernel: &[u8], cmdline: &str) -> Result<LoadedKernel> {
        let ptr = self
            .memory
            .get_host_address(GuestAddress(0))
            .map_err(|err| Error::Hypervisor(format!("guest RAM host address: {err}")))?;
        // SAFETY: the single guest RAM region is uniquely owned by this machine
        // and no vCPU has run yet, so nothing else can observe the slice. It is
        // `size` bytes, the region allocated above.
        let mem = unsafe { std::slice::from_raw_parts_mut(ptr, self.size) };
        let loaded = boot::load_bzimage(kernel, mem, cmdline)?;
        let vcpu = self
            .vcpus
            .first()
            .ok_or_else(|| Error::Hypervisor("VM has no vCPU".into()))?;
        configure_long_mode(vcpu, loaded.entry, loaded.boot_params, loaded.pml4)?;
        Ok(loaded)
    }
}

fn memory_bytes(memory_mb: u64) -> usize {
    let bytes = memory_mb.saturating_mul(1024 * 1024);
    let page = 2 * 1024 * 1024;
    let rounded = (bytes + page - 1) / page * page;
    rounded.max(page) as usize
}

fn configure_long_mode(vcpu: &VcpuFd, entry: u64, boot_params: u64, pml4: u64) -> Result<()> {
    let mut sregs = vcpu
        .get_sregs()
        .map_err(|err| Error::Hypervisor(format!("kvm get_sregs: {err}")))?;
    let code = segment(0x8, 0xB, 1, 0);
    let data = segment(0x10, 0x3, 0, 1);
    sregs.cs = code;
    sregs.ds = data;
    sregs.es = data;
    sregs.fs = data;
    sregs.gs = data;
    sregs.ss = data;
    sregs.gdt.base = 0x8000;
    sregs.gdt.limit = 3 * 8 - 1;
    sregs.cr3 = pml4;
    sregs.cr4 |= 1 << 5; // PAE
    sregs.cr0 |= 1 | (1 << 31); // PE | PG
    sregs.efer |= (1 << 8) | (1 << 10); // LME | LMA
    vcpu.set_sregs(&sregs)
        .map_err(|err| Error::Hypervisor(format!("kvm set_sregs: {err}")))?;

    let mut regs = vcpu
        .get_regs()
        .map_err(|err| Error::Hypervisor(format!("kvm get_regs: {err}")))?;
    regs.rip = entry;
    regs.rsi = boot_params;
    regs.rflags = 2;
    regs.rsp = 0;
    vcpu.set_regs(&regs)
        .map_err(|err| Error::Hypervisor(format!("kvm set_regs: {err}")))?;
    Ok(())
}

fn segment(selector: u16, ty: u8, long: u8, db: u8) -> kvm_segment {
    kvm_segment {
        base: 0,
        limit: 0xF_FFFF,
        selector,
        type_: ty,
        present: 1,
        dpl: 0,
        db,
        s: 1,
        l: long,
        g: 1,
        avl: 0,
        unusable: 0,
        ..kvm_segment::default()
    }
}

/// COM1 (0x3F8) with a permanently-empty transmitter, so a guest that writes
/// the serial port makes forward progress instead of spinning on THRE.
#[derive(Debug, Default)]
pub(crate) struct Serial {
    pub out: Vec<u8>,
}

impl Serial {
    pub(crate) fn io_out(&mut self, port: u16, data: &[u8]) {
        if port == 0x3F8 {
            self.out.extend_from_slice(data);
        }
    }

    pub(crate) fn io_in(&self, port: u16, data: &mut [u8]) {
        match port {
            0x3F8 => data.fill(0),
            // Transmitter holding register empty | transmitter empty.
            0x3FD => data.fill(0x60),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_ram_is_a_single_contiguous_region() {
        let machine_memory = memory_bytes(64);
        assert!(machine_memory % (2 * 1024 * 1024) == 0);
        let memory: GuestMemoryMmap =
            GuestMemoryMmap::from_ranges(&[(GuestAddress(0), machine_memory)]).unwrap();
        // The whole allocation is one region reachable from guest address 0,
        // which is what the KVM memory slot and the kernel loader both assume.
        assert_eq!(memory.iter().count(), 1);
        assert!(memory.get_host_address(GuestAddress(0)).is_ok());
        assert!(memory
            .get_host_address(GuestAddress(machine_memory as u64))
            .is_err());
    }

    #[test]
    fn serial_reports_transmitter_empty() {
        let mut serial = Serial::default();
        serial.io_out(0x3F8, b"hi");
        assert_eq!(serial.out, b"hi");
        let mut status = [0u8; 1];
        serial.io_in(0x3FD, &mut status);
        assert_eq!(status[0], 0x60);
    }
}
