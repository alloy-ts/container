//! x86_64 Linux bzImage loader and the page tables a 64-bit entry needs.
//!
//! Offsets follow Documentation/arch/x86/boot.rst. The protected-mode kernel
//! is copied to 1MiB. When `xloadflags` has `XLF_KERNEL_64`, the entry point
//! is 512 bytes into that image and the CPU is placed in long mode with
//! `rsi` pointing at `boot_params`.

use crate::error::{Error, Result};

const KERNEL_LOAD: u64 = 0x10_0000;
const BOOT_PARAMS: u64 = 0x7000;
const CMDLINE_ADDR: u64 = 0x2_0000;
const GDT_ADDR: u64 = 0x8000;
const PAGE: u64 = 4096;
const PAGE_2M: u64 = 2 * 1024 * 1024;

const PTE_PRESENT: u64 = 1;
const PTE_RW: u64 = 1 << 1;
const PTE_ACCESSED: u64 = 1 << 5;
const PTE_DIRTY: u64 = 1 << 6;
const PTE_PS: u64 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoadedKernel {
    pub entry: u64,
    pub boot_params: u64,
    pub pml4: u64,
    pub protocol: u16,
}

pub(crate) fn validate_header(kernel: &[u8]) -> Result<()> {
    parse_header(kernel).map(|_| ())
}

pub(crate) fn load_bzimage(kernel: &[u8], mem: &mut [u8], cmdline: &str) -> Result<LoadedKernel> {
    let header = parse_header(kernel)?;
    let payload = &kernel[header.setup_size..];
    let load_end = KERNEL_LOAD as usize + payload.len();
    if load_end > mem.len() {
        return Err(Error::Hypervisor(format!(
            "kernel payload ({} bytes at 0x{KERNEL_LOAD:x}) does not fit in {} bytes of guest RAM",
            payload.len(),
            mem.len()
        )));
    }
    mem[KERNEL_LOAD as usize..load_end].copy_from_slice(payload);

    write_boot_params(mem, kernel, cmdline)?;
    write_gdt(mem)?;
    let pml4 = install_page_tables(mem)?;
    Ok(LoadedKernel {
        entry: header.entry,
        boot_params: BOOT_PARAMS,
        pml4,
        protocol: header.protocol,
    })
}

struct Header {
    setup_size: usize,
    entry: u64,
    protocol: u16,
}

fn parse_header(kernel: &[u8]) -> Result<Header> {
    if kernel.len() < 0x250 {
        return Err(Error::Hypervisor(
            "kernel image is too small to be a Linux bzImage".into(),
        ));
    }
    let boot_flag = u16::from_le_bytes([kernel[0x1FE], kernel[0x1FF]]);
    if boot_flag != 0xAA55 {
        return Err(Error::Hypervisor(
            "kernel is not a bzImage (missing 0xAA55 boot flag)".into(),
        ));
    }
    if &kernel[0x202..0x206] != b"HdrS" {
        return Err(Error::Hypervisor(
            "kernel is not a bzImage (missing HdrS magic)".into(),
        ));
    }
    let mut setup_sects = kernel[0x1F1] as usize;
    if setup_sects == 0 {
        setup_sects = 4;
    }
    let setup_size = (setup_sects + 1) * 512;
    if kernel.len() < setup_size + 0x200 {
        return Err(Error::Hypervisor(
            "kernel image is truncated before the protected-mode payload".into(),
        ));
    }
    let protocol = u16::from_le_bytes([kernel[0x206], kernel[0x207]]);
    if protocol < 0x020C {
        return Err(Error::Hypervisor(format!(
            "TODO: kernel boot protocol {protocol:#x} is older than 2.12; only the 64-bit entry is implemented"
        )));
    }
    let xloadflags = u16::from_le_bytes([kernel[0x236], kernel[0x237]]);
    // XLF_KERNEL_64: 64-bit entry point at offset 0x200 in the protected-mode kernel.
    if xloadflags & 1 == 0 {
        return Err(Error::Hypervisor(
            "TODO: 32-bit kernel entry is not implemented (xloadflags lacks XLF_KERNEL_64)".into(),
        ));
    }
    Ok(Header {
        setup_size,
        entry: KERNEL_LOAD + 0x200,
        protocol,
    })
}

fn write_boot_params(mem: &mut [u8], kernel: &[u8], cmdline: &str) -> Result<()> {
    let params = BOOT_PARAMS as usize;
    if params + 4096 > mem.len() {
        return Err(Error::Hypervisor(
            "guest RAM is too small for boot_params".into(),
        ));
    }
    mem[params..params + 4096].fill(0);
    // Copy the setup header (from 0x1F1 through the end of the fixed header).
    let header_end = 0x290.min(kernel.len());
    mem[params + 0x1F1..params + header_end].copy_from_slice(&kernel[0x1F1..header_end]);
    mem[params + 0x210] = 0xFF; // type_of_loader
    mem[params + 0x211] |= 0x01; // LOADED_HIGH
    mem[params + 0x214..params + 0x218].copy_from_slice(&(KERNEL_LOAD as u32).to_le_bytes());
    let cmdline_bytes = cmdline.as_bytes();
    let cmdline_at = CMDLINE_ADDR as usize;
    if cmdline_at + cmdline_bytes.len() + 1 > mem.len() {
        return Err(Error::Hypervisor(
            "guest RAM is too small for cmdline".into(),
        ));
    }
    mem[cmdline_at..cmdline_at + cmdline_bytes.len()].copy_from_slice(cmdline_bytes);
    mem[cmdline_at + cmdline_bytes.len()] = 0;
    mem[params + 0x228..params + 0x22C].copy_from_slice(&(CMDLINE_ADDR as u32).to_le_bytes());
    mem[params + 0x238..params + 0x23C]
        .copy_from_slice(&(cmdline_bytes.len() as u32).to_le_bytes());

    // One e820 RAM entry covering the guest allocation.
    mem[params + 0x1E8] = 1;
    let e820 = params + 0x2D0;
    mem[e820..e820 + 8].copy_from_slice(&0u64.to_le_bytes());
    let mem_len = mem.len() as u64;
    mem[e820 + 8..e820 + 16].copy_from_slice(&mem_len.to_le_bytes());
    mem[e820 + 16..e820 + 20].copy_from_slice(&1u32.to_le_bytes());
    Ok(())
}

fn write_gdt(mem: &mut [u8]) -> Result<()> {
    let at = GDT_ADDR as usize;
    if at + 24 > mem.len() {
        return Err(Error::Hypervisor(
            "guest RAM is too small for the GDT".into(),
        ));
    }
    let null = 0u64;
    // access 0x9B: present, ring 0, code, readable, accessed. flags 0xA: L=1, G=1.
    let code = gdt_entry(0xF_FFFF, 0, 0x9B, 0xA);
    // access 0x93: present, data, writable, accessed. flags 0xC: G=1, DB=1.
    let data = gdt_entry(0xF_FFFF, 0, 0x93, 0xC);
    mem[at..at + 8].copy_from_slice(&null.to_le_bytes());
    mem[at + 8..at + 16].copy_from_slice(&code.to_le_bytes());
    mem[at + 16..at + 24].copy_from_slice(&data.to_le_bytes());
    Ok(())
}

fn gdt_entry(limit: u32, base: u32, access: u8, flags: u8) -> u64 {
    let mut desc = (limit as u64) & 0xFFFF;
    desc |= (base as u64 & 0xFF_FFFF) << 16;
    desc |= (access as u64) << 40;
    desc |= ((limit as u64) >> 16 & 0xF) << 48;
    desc |= (flags as u64 & 0xF) << 52;
    desc |= ((base as u64) >> 24 & 0xFF) << 56;
    desc
}

/// Identity-map `mem` with 2MiB pages. Returns the physical address of the PML4.
pub(crate) fn install_page_tables(mem: &mut [u8]) -> Result<u64> {
    let mem_size = mem.len() as u64;
    let pages = (mem_size + PAGE_2M - 1) / PAGE_2M;
    let pd_count = ((pages + 511) / 512).max(1);
    let table_bytes = (2 + pd_count) * PAGE;
    if mem_size < table_bytes + KERNEL_LOAD {
        return Err(Error::Hypervisor(
            "guest RAM is too small for page tables and the kernel".into(),
        ));
    }
    let tables = (mem_size - table_bytes) & !(PAGE - 1);
    let start = tables as usize;
    let end = start + table_bytes as usize;
    mem[start..end].fill(0);

    let pml4 = tables;
    let pdpt = tables + PAGE;
    let pd_base = tables + 2 * PAGE;
    write_u64(mem, pml4, pdpt | PTE_PRESENT | PTE_RW | PTE_ACCESSED);
    for i in 0..pd_count {
        let pd = pd_base + i * PAGE;
        write_u64(mem, pdpt + i * 8, pd | PTE_PRESENT | PTE_RW | PTE_ACCESSED);
        let count = pages.saturating_sub(i * 512).min(512);
        for j in 0..count {
            let phys = (i * 512 + j) * PAGE_2M;
            write_u64(
                mem,
                pd + j * 8,
                phys | PTE_PRESENT | PTE_RW | PTE_PS | PTE_ACCESSED | PTE_DIRTY,
            );
        }
    }
    Ok(pml4)
}

fn write_u64(mem: &mut [u8], addr: u64, value: u64) {
    let at = addr as usize;
    mem[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

/// Minimal bzImage (protocol 2.12, 64-bit entry) whose payload is `code`.
#[cfg(test)]
pub(crate) fn synthesize_bzimage(code: &[u8]) -> Vec<u8> {
    let setup_sects: u8 = 4; // setup_size = 5 * 512 = 2560
    let setup_size = (setup_sects as usize + 1) * 512;
    let mut image = vec![0u8; setup_size + code.len().max(0x200)];
    image[0x1F1] = setup_sects;
    image[0x1FE] = 0x55;
    image[0x1FF] = 0xAA;
    image[0x202..0x206].copy_from_slice(b"HdrS");
    image[0x206..0x208].copy_from_slice(&0x020Cu16.to_le_bytes());
    image[0x236..0x238].copy_from_slice(&1u16.to_le_bytes()); // XLF_KERNEL_64
    let payload_at = setup_size;
    image[payload_at..payload_at + code.len()].copy_from_slice(code);
    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_a_64bit_bzimage_and_maps_ram() {
        let code = b"\x90\xf4"; // nop; hlt
        let image = synthesize_bzimage(code);
        let mut mem = vec![0u8; 8 * 1024 * 1024];
        let loaded = load_bzimage(&image, &mut mem, "console=ttyS0").unwrap();
        assert_eq!(loaded.entry, KERNEL_LOAD + 0x200);
        assert_eq!(loaded.boot_params, BOOT_PARAMS);
        assert_eq!(loaded.protocol, 0x020C);
        let setup_size = 5 * 512;
        assert_eq!(
            &mem[KERNEL_LOAD as usize..KERNEL_LOAD as usize + code.len()],
            &image[setup_size..setup_size + code.len()]
        );
        // PML4[0] present and pointing at the PDPT.
        let pml4 = &mem[loaded.pml4 as usize..loaded.pml4 as usize + 8];
        let entry = u64::from_le_bytes(pml4.try_into().unwrap());
        assert_eq!(entry & PTE_PRESENT, PTE_PRESENT);
        assert!(mem[BOOT_PARAMS as usize + 0x202..BOOT_PARAMS as usize + 0x206] == *b"HdrS");
        assert!(mem[CMDLINE_ADDR as usize..].starts_with(b"console=ttyS0\0"));
    }

    #[test]
    fn rejects_a_non_kernel() {
        let err = validate_header(b"not a kernel").unwrap_err();
        assert!(err.to_string().contains("bzImage") || err.to_string().contains("too small"));
    }
}
