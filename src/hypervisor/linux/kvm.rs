//! KVM VM, guest RAM, and the vCPU run loop.
//!
//! `run_vcpu` is the steady-state loop `boot` will enter once virtio devices
//! exist. It is not started today: a guest with no virtio-vsock can never
//! connect back, and `KVM_RUN` on a real kernel would block the caller.

use std::io;

use kvm_bindings::{kvm_segment, kvm_userspace_memory_region, KVM_MAX_CPUID_ENTRIES};
use kvm_ioctls::{Kvm, VcpuExit, VcpuFd, VmFd};

use super::boot::{self, LoadedKernel};
use crate::error::{Error, Result};

pub(crate) struct KvmMachine {
    // Drop order is field order. vCPUs and the VM fd must close before the
    // mapping is unmapped, and the system fd last.
    vcpus: Vec<VcpuFd>,
    _vm: VmFd,
    _kvm: Kvm,
    memory: GuestMemory,
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
        let memory = GuestMemory::new(memory_bytes(memory_mb))?;
        // SAFETY: `memory` lives in this struct for as long as `vm` does, is
        // page-aligned (mmap), and is not aliased. Drop order closes the VM
        // fd before the mapping.
        unsafe {
            vm.set_user_memory_region(kvm_userspace_memory_region {
                slot: 0,
                flags: 0,
                guest_phys_addr: 0,
                memory_size: memory.size as u64,
                userspace_addr: memory.ptr as u64,
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
        })
    }

    pub(crate) fn load_kernel(&mut self, kernel: &[u8], cmdline: &str) -> Result<LoadedKernel> {
        let mem = self.memory.as_mut_slice();
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

/// Run one vCPU until it halts, shuts down, or `stop` is set.
///
/// TODO(linux): MMIO exits are where virtio-mmio devices will be dispatched.
/// They are acknowledged and ignored so an unexpected access cannot wedge the
/// thread, which also means the guest cannot yet drive a device.
#[allow(dead_code)]
pub(crate) fn run_vcpu(
    vcpu: &mut VcpuFd,
    serial: &mut Serial,
    stop: &std::sync::atomic::AtomicBool,
) {
    use std::sync::atomic::Ordering;
    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        match vcpu.run() {
            Ok(VcpuExit::Hlt | VcpuExit::Shutdown | VcpuExit::SystemEvent(_, _)) => break,
            Ok(VcpuExit::IoOut(port, data)) => serial.io_out(port, data),
            Ok(VcpuExit::IoIn(port, data)) => serial.io_in(port, data),
            Ok(VcpuExit::MmioRead(_addr, data)) => data.fill(0),
            Ok(VcpuExit::MmioWrite(_addr, _data)) => {}
            Ok(VcpuExit::X86Rdmsr(msr)) => {
                *msr.data = 0;
                *msr.error = 0;
            }
            Ok(VcpuExit::X86Wrmsr(msr)) => {
                *msr.error = 0;
            }
            Ok(VcpuExit::Intr) => continue,
            Ok(_) => break,
            Err(err) if err.errno() == libc::EINTR || err.errno() == libc::EAGAIN => continue,
            Err(_) => break,
        }
    }
}

struct GuestMemory {
    ptr: *mut u8,
    size: usize,
}

// The mapping is uniquely owned by `KvmMachine` and only touched from the
// thread that loads the kernel, before any vCPU runs.
unsafe impl Send for GuestMemory {}

impl GuestMemory {
    fn new(size: usize) -> Result<Self> {
        // SAFETY: anonymous private mapping. Size is non-zero and page-aligned
        // by `memory_bytes`.
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if ptr == libc::MAP_FAILED {
            return Err(Error::Hypervisor(format!(
                "mmap guest RAM: {}",
                io::Error::last_os_error()
            )));
        }
        Ok(Self {
            ptr: ptr as *mut u8,
            size,
        })
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: the mapping is live, uniquely borrowed, and `size` bytes long.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl Drop for GuestMemory {
    fn drop(&mut self) {
        // SAFETY: `ptr` came from mmap of `size` bytes.
        unsafe {
            libc::munmap(self.ptr as *mut libc::c_void, self.size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
