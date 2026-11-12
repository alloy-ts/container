//! MMIO and port-I/O device bus.
//!
//! The vCPU run loop turns `VcpuExit::MmioRead`/`MmioWrite` and `IoIn`/`IoOut`
//! into calls on [`DeviceBus`], which routes each access to the device
//! registered for that address range. The range bookkeeping (overlap detection,
//! lookup) comes from the `vm-device` bus, so it is not reimplemented here.
//!
//! Devices are shared handles (`Arc<Mutex<..>>`) because a virtio device is
//! touched by two threads at once: the vCPU thread drives its MMIO transport,
//! while a device worker thread drains its virtqueues.

use std::sync::{Arc, Mutex};

use vm_device::bus::{Bus, MmioAddress, MmioRange, PioAddress, PioRange};
use vm_device::{MutDeviceMmio, MutDevicePio};

use crate::error::{Error, Result};

/// An MMIO device handle. A virtio-mmio transport, a serial UART, and the
/// devices behind them all live behind one of these.
pub(crate) type MmioDevice = Arc<Mutex<dyn MutDeviceMmio + Send>>;
/// A port-I/O device handle (16550 serial, i8042).
pub(crate) type PioDevice = Arc<Mutex<dyn MutDevicePio + Send>>;

/// Routes guest MMIO and port-I/O accesses to registered devices.
#[derive(Default)]
pub(crate) struct DeviceBus {
    mmio: Bus<MmioAddress, MmioDevice>,
    pio: Bus<PioAddress, PioDevice>,
}

impl DeviceBus {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn register_mmio(&mut self, base: u64, size: u64, dev: MmioDevice) -> Result<()> {
        let range = MmioRange::new(MmioAddress(base), size)
            .map_err(|err| Error::Hypervisor(format!("mmio range {base:#x}+{size:#x}: {err}")))?;
        self.mmio
            .register(range, dev)
            .map_err(|err| Error::Hypervisor(format!("register mmio {base:#x}: {err}")))
    }

    pub(crate) fn register_pio(&mut self, base: u16, size: u16, dev: PioDevice) -> Result<()> {
        let range = PioRange::new(PioAddress(base), size)
            .map_err(|err| Error::Hypervisor(format!("pio range {base:#x}+{size:#x}: {err}")))?;
        self.pio
            .register(range, dev)
            .map_err(|err| Error::Hypervisor(format!("register pio {base:#x}: {err}")))
    }

    /// Serve a guest read. Unmapped addresses read as zero, matching what a
    /// real bus does for a hole, so an unexpected access cannot wedge a vCPU.
    pub(crate) fn mmio_read(&self, addr: u64, data: &mut [u8]) -> Result<()> {
        match self.mmio.device(MmioAddress(addr)) {
            Some((range, dev)) => {
                let offset = addr - range.base().0;
                let mut dev = dev.lock().unwrap_or_else(|err| err.into_inner());
                dev.mmio_read(range.base(), offset, data);
                Ok(())
            }
            None => {
                data.fill(0);
                Ok(())
            }
        }
    }

    /// Serve a guest write. Writes to unmapped addresses are dropped.
    pub(crate) fn mmio_write(&self, addr: u64, data: &[u8]) -> Result<()> {
        if let Some((range, dev)) = self.mmio.device(MmioAddress(addr)) {
            let offset = addr - range.base().0;
            let mut dev = dev.lock().unwrap_or_else(|err| err.into_inner());
            dev.mmio_write(range.base(), offset, data);
        }
        Ok(())
    }

    /// Serve a guest port read. Unmapped ports read as zero.
    pub(crate) fn pio_read(&self, port: u16, data: &mut [u8]) -> Result<()> {
        match self.pio.device(PioAddress(port)) {
            Some((range, dev)) => {
                let offset = port - range.base().0;
                let mut dev = dev.lock().unwrap_or_else(|err| err.into_inner());
                dev.pio_read(range.base(), offset, data);
                Ok(())
            }
            None => {
                data.fill(0);
                Ok(())
            }
        }
    }

    /// Serve a guest port write. Writes to unmapped ports are dropped.
    pub(crate) fn pio_write(&self, port: u16, data: &[u8]) -> Result<()> {
        if let Some((range, dev)) = self.pio.device(PioAddress(port)) {
            let offset = port - range.base().0;
            let mut dev = dev.lock().unwrap_or_else(|err| err.into_inner());
            dev.pio_write(range.base(), offset, data);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Echo {
        log: Arc<Mutex<Vec<u8>>>,
    }

    impl MutDeviceMmio for Echo {
        fn mmio_read(&mut self, _base: MmioAddress, _offset: u64, data: &mut [u8]) {
            data.fill(0xAB);
        }

        fn mmio_write(&mut self, _base: MmioAddress, _offset: u64, data: &[u8]) {
            self.log.lock().unwrap().extend_from_slice(data);
        }
    }

    impl MutDevicePio for Echo {
        fn pio_read(&mut self, _base: PioAddress, _offset: u16, data: &mut [u8]) {
            data.fill(0xCD);
        }

        fn pio_write(&mut self, _base: PioAddress, _offset: u16, data: &[u8]) {
            self.log.lock().unwrap().extend_from_slice(data);
        }
    }

    fn echo(log: &Arc<Mutex<Vec<u8>>>) -> Echo {
        Echo { log: log.clone() }
    }

    #[test]
    fn routes_mmio_and_pio_to_the_registered_device() {
        let mut bus = DeviceBus::new();
        let mmio_log = Arc::new(Mutex::new(Vec::new()));
        let pio_log = Arc::new(Mutex::new(Vec::new()));
        let dev: MmioDevice = Arc::new(Mutex::new(echo(&mmio_log)));
        let dev_pio: PioDevice = Arc::new(Mutex::new(echo(&pio_log)));
        bus.register_mmio(0xd000_0000, 0x1000, dev).unwrap();
        bus.register_pio(0x3f8, 8, dev_pio).unwrap();

        bus.mmio_write(0xd000_0004, &[1, 2, 3]).unwrap();
        let mut read = [0u8; 4];
        bus.mmio_read(0xd000_0004, &mut read).unwrap();
        assert_eq!(read, [0xAB; 4]);

        bus.pio_write(0x3f8, &[b'h', b'i']).unwrap();
        let mut status = [0u8; 1];
        bus.pio_read(0x3fd, &mut status).unwrap();
        assert_eq!(status[0], 0xCD);

        assert_eq!(*mmio_log.lock().unwrap(), vec![1, 2, 3]);
        assert_eq!(*pio_log.lock().unwrap(), b"hi".to_vec());
    }

    #[test]
    fn unmapped_accesses_do_not_wedge_the_vcpu() {
        let bus = DeviceBus::new();
        let mut read = [0xFFu8; 2];
        bus.mmio_read(0xdead_0000, &mut read).unwrap();
        assert_eq!(read, [0, 0]);
        bus.mmio_write(0xdead_0000, &[9]).unwrap();
        bus.pio_write(0xffff, &[9]).unwrap();
    }

    #[test]
    fn overlapping_ranges_are_rejected() {
        let mut bus = DeviceBus::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        bus.register_mmio(0xd000_0000, 0x1000, Arc::new(Mutex::new(echo(&log))))
            .unwrap();
        let err = bus
            .register_mmio(0xd000_0f00, 0x1000, Arc::new(Mutex::new(echo(&log))))
            .unwrap_err();
        assert!(err.to_string().contains("overlap") || err.to_string().contains("mmio"));
    }
}
