//! Virtio over MMIO (virtio-mmio v2) transport.
//!
//! This is the register file the guest's `virtio_mmio` driver finds at a fixed
//! address from the `virtio_mmio.device=4K@0x...:irq` command-line property. It
//! implements the driver-side handshake — feature negotiation, per-queue ring
//! setup, and the `DRIVER_OK` activation — and forwards the data plane to the
//! [`VirtioDevice`] behind it.
//!
//! Register offsets and bit meanings come from `virtio-bindings`, so this module
//! only encodes the state machine, not the ABI.

use vm_device::bus::MmioAddress;
use vm_device::MutDeviceMmio;
use vm_memory::GuestMemoryMmap;
use virtio_bindings::bindings::virtio_config::{
    VIRTIO_CONFIG_S_ACKNOWLEDGE, VIRTIO_CONFIG_S_DRIVER, VIRTIO_CONFIG_S_DRIVER_OK,
    VIRTIO_CONFIG_S_FAILED, VIRTIO_CONFIG_S_FEATURES_OK,
};
use virtio_bindings::bindings::virtio_mmio::*;
use virtio_queue::{QueueSync, QueueT};

use super::{Activate, Interrupt, VirtioDevice};
use crate::error::{Error, Result};

/// `virtio-mmio` magic, the ASCII bytes `virt` read as a little-endian u32.
const MAGIC: u32 = 0x7472_6976;
/// Modern (v2) MMIO transport, which uses the split-ring queue registers rather
/// than the legacy PFN register.
const VERSION: u32 = 2;
/// `VIRTIO_F_VERSION_1`: the device speaks the 64-bit feature/flags layout.
const F_VERSION_1: u64 = 1 << 32;

/// The MMIO transport for one virtio device.
pub(crate) struct VirtioMmioTransport {
    device: Box<dyn VirtioDevice>,
    mem: GuestMemoryMmap,
    queues: Vec<QueueSync>,
    interrupt: Interrupt,
    device_features_sel: u32,
    driver_features_sel: u32,
    driver_features: u64,
    acked_features: u64,
    queue_sel: u32,
    status: u8,
    config_generation: u8,
    activated: bool,
}

impl VirtioMmioTransport {
    pub(crate) fn new(
        device: Box<dyn VirtioDevice>,
        mem: GuestMemoryMmap,
        interrupt: Interrupt,
    ) -> Result<Self> {
        let count = device.queue_count();
        if count == 0 {
            return Err(Error::Hypervisor(
                "virtio device exposes no queues".into(),
            ));
        }
        let max = device.queue_max_size();
        let mut queues = Vec::with_capacity(count);
        for _ in 0..count {
            queues.push(
                QueueSync::new(max)
                    .map_err(|err| Error::Hypervisor(format!("create virtqueue ({max}): {err}")))?,
            );
        }
        Ok(Self {
            device,
            mem,
            queues,
            interrupt,
            device_features_sel: 0,
            driver_features_sel: 0,
            driver_features: 0,
            acked_features: 0,
            queue_sel: 0,
            status: 0,
            config_generation: 0,
            activated: false,
        })
    }

    /// Feature bits offered to the driver: the device's own bits plus the
    /// transport-implied `VIRTIO_F_VERSION_1`.
    fn features(&self) -> u64 {
        self.device.features() | F_VERSION_1
    }

    fn queue_at_sel(&mut self) -> Option<&mut QueueSync> {
        self.queues.get_mut(self.queue_sel as usize)
    }

    fn read_reg(&self, offset: u32) -> u32 {
        match offset {
            VIRTIO_MMIO_MAGIC_VALUE => MAGIC,
            VIRTIO_MMIO_VERSION => VERSION,
            VIRTIO_MMIO_DEVICE_ID => self.device.device_id(),
            VIRTIO_MMIO_VENDOR_ID => 0,
            VIRTIO_MMIO_DEVICE_FEATURES => {
                (self.features() >> (self.device_features_sel.min(1) * 32)) as u32
            }
            VIRTIO_MMIO_QUEUE_SEL => self.queue_sel,
            VIRTIO_MMIO_QUEUE_NUM_MAX => {
                if (self.queue_sel as usize) < self.queues.len() {
                    u32::from(self.device.queue_max_size())
                } else {
                    0
                }
            }
            VIRTIO_MMIO_QUEUE_READY => {
                let ready = self
                    .queues
                    .get(self.queue_sel as usize)
                    .map(|q| q.ready())
                    .unwrap_or(false);
                u32::from(ready)
            }
            VIRTIO_MMIO_INTERRUPT_STATUS => u32::from(self.interrupt.status()),
            VIRTIO_MMIO_STATUS => u32::from(self.status),
            VIRTIO_MMIO_CONFIG_GENERATION => u32::from(self.config_generation),
            _ => 0,
        }
    }

    fn write_reg(&mut self, offset: u32, value: u32) {
        match offset {
            VIRTIO_MMIO_DEVICE_FEATURES_SEL => self.device_features_sel = value,
            VIRTIO_MMIO_DRIVER_FEATURES_SEL => self.driver_features_sel = value,
            VIRTIO_MMIO_DRIVER_FEATURES => {
                if self.driver_features_sel <= 1 {
                    let shift = self.driver_features_sel * 32;
                    let mask = 0xFFFF_FFFFu64 << shift;
                    self.driver_features =
                        (self.driver_features & !mask) | (u64::from(value) << shift);
                }
            }
            VIRTIO_MMIO_QUEUE_SEL => {
                if (value as usize) < self.queues.len() {
                    self.queue_sel = value;
                }
            }
            VIRTIO_MMIO_QUEUE_NUM => {
                if let Some(queue) = self.queue_at_sel() {
                    queue.set_size(value as u16);
                }
            }
            VIRTIO_MMIO_QUEUE_READY => {
                if let Some(queue) = self.queue_at_sel() {
                    queue.set_ready(value & 1 != 0);
                }
            }
            VIRTIO_MMIO_QUEUE_DESC_LOW => self.set_desc_low(Some(value), None),
            VIRTIO_MMIO_QUEUE_DESC_HIGH => self.set_desc_low(None, Some(value)),
            VIRTIO_MMIO_QUEUE_AVAIL_LOW => self.set_avail(Some(value), None),
            VIRTIO_MMIO_QUEUE_AVAIL_HIGH => self.set_avail(None, Some(value)),
            VIRTIO_MMIO_QUEUE_USED_LOW => self.set_used(Some(value), None),
            VIRTIO_MMIO_QUEUE_USED_HIGH => self.set_used(None, Some(value)),
            VIRTIO_MMIO_QUEUE_NOTIFY => self.device.notify(value as u16),
            VIRTIO_MMIO_INTERRUPT_ACK => self.interrupt.ack(value as u8),
            VIRTIO_MMIO_STATUS => self.write_status(value as u8),
            _ => {}
        }
    }

    fn set_desc_low(&mut self, low: Option<u32>, high: Option<u32>) {
        if let Some(queue) = self.queue_at_sel() {
            queue.set_desc_table_address(low, high);
        }
    }

    fn set_avail(&mut self, low: Option<u32>, high: Option<u32>) {
        if let Some(queue) = self.queue_at_sel() {
            queue.set_avail_ring_address(low, high);
        }
    }

    fn set_used(&mut self, low: Option<u32>, high: Option<u32>) {
        if let Some(queue) = self.queue_at_sel() {
            queue.set_used_ring_address(low, high);
        }
    }

    fn write_status(&mut self, value: u8) {
        if value == 0 {
            self.reset();
            return;
        }
        // Once FAILED is set the device stays failed until the driver resets
        // it; later status writes must not revive it.
        if self.status & VIRTIO_CONFIG_S_FAILED as u8 != 0 {
            return;
        }
        // The driver promises FEATURES_OK only after it has inspected the
        // device features. Reject the handshake if it claimed a bit we do not
        // offer, per the virtio spec.
        if value & (VIRTIO_CONFIG_S_FEATURES_OK as u8) != 0
            && self.status & (VIRTIO_CONFIG_S_FEATURES_OK as u8) == 0
        {
            if self.driver_features & !self.features() != 0 {
                self.status = value | VIRTIO_CONFIG_S_FAILED as u8;
                return;
            }
            self.acked_features = self.driver_features & self.features();
        }
        self.status = value;
        if value & (VIRTIO_CONFIG_S_DRIVER_OK as u8) != 0 && !self.activated {
            self.activate();
        }
    }

    fn activate(&mut self) {
        let ctx = Activate {
            mem: self.mem.clone(),
            queues: self.queues.clone(),
            interrupt: self.interrupt.clone(),
        };
        match self.device.activate(ctx) {
            Ok(()) => self.activated = true,
            Err(_) => self.status |= VIRTIO_CONFIG_S_FAILED as u8,
        }
    }

    fn reset(&mut self) {
        self.device.reset();
        for queue in &mut self.queues {
            queue.reset();
        }
        self.status = 0;
        self.driver_features = 0;
        self.acked_features = 0;
        self.driver_features_sel = 0;
        self.device_features_sel = 0;
        self.queue_sel = 0;
        self.activated = false;
        self.interrupt.ack(0xFF);
    }

    /// Driver-accepted features, exposed so a device worker can react to them.
    #[allow(dead_code)]
    pub(crate) fn acked_features(&self) -> u64 {
        self.acked_features
    }
}

impl MutDeviceMmio for VirtioMmioTransport {
    fn mmio_read(&mut self, _base: MmioAddress, offset: u64, data: &mut [u8]) {
        if offset >= u64::from(VIRTIO_MMIO_CONFIG) {
            self.device
                .read_config(offset - u64::from(VIRTIO_MMIO_CONFIG), data);
            return;
        }
        let value = u64::from(self.read_reg(offset as u32));
        let bytes = value.to_le_bytes();
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = bytes.get(i).copied().unwrap_or(0);
        }
    }

    fn mmio_write(&mut self, _base: MmioAddress, offset: u64, data: &[u8]) {
        if offset >= u64::from(VIRTIO_MMIO_CONFIG) {
            self.device
                .write_config(offset - u64::from(VIRTIO_MMIO_CONFIG), data);
            return;
        }
        // Every transport register is 32 bits; a guest write is 4 bytes (or a
        // narrower access, zero-extended here).
        let mut buf = [0u8; 4];
        let n = data.len().min(4);
        buf[..n].copy_from_slice(&data[..n]);
        self.write_reg(offset as u32, u32::from_le_bytes(buf));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use vm_memory::{GuestAddress, GuestMemory};

    /// Records what the transport drives and offers a fixed register identity.
    #[derive(Default)]
    struct FakeDevice {
        config: Vec<u8>,
        activated: Arc<AtomicBool>,
        notified: Arc<std::sync::Mutex<Vec<u16>>>,
        resets: Arc<AtomicU32>,
    }

    impl VirtioDevice for FakeDevice {
        fn device_id(&self) -> u32 {
            19 // VIRTIO_ID_VSOCK
        }
        fn features(&self) -> u64 {
            0x3
        }
        fn read_config(&self, offset: u64, data: &mut [u8]) {
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = self
                    .config
                    .get(offset as usize + i)
                    .copied()
                    .unwrap_or(0);
            }
        }
        fn queue_count(&self) -> usize {
            2
        }
        fn queue_max_size(&self) -> u16 {
            8
        }
        fn activate(&mut self, ctx: Activate) -> Result<()> {
            assert_eq!(ctx.queues.len(), 2);
            assert!(ctx.mem.get_host_address(GuestAddress(0)).is_ok());
            self.activated.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn notify(&mut self, queue_index: u16) {
            self.notified.lock().unwrap().push(queue_index);
        }
        fn reset(&mut self) {
            self.resets.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn transport() -> (VirtioMmioTransport, Arc<AtomicBool>, Arc<std::sync::Mutex<Vec<u16>>>) {
        let mem: GuestMemoryMmap =
            GuestMemoryMmap::from_ranges(&[(GuestAddress(0), 0x10_000)]).unwrap();
        let activated = Arc::new(AtomicBool::new(false));
        let notified = Arc::new(std::sync::Mutex::new(Vec::new()));
        let device = FakeDevice {
            config: vec![0xAA, 0xBB],
            activated: activated.clone(),
            notified: notified.clone(),
            resets: Arc::new(AtomicU32::new(0)),
        };
        // A Gsi needs a real irqfd, so the transport tests build the interrupt
        // around a plain eventfd-backed line via `Interrupt::new` only when KVM
        // is present; here we use a standalone eventfd by constructing a
        // dummy Gsi through the crate-internal test hook below.
        let interrupt = test_interrupt();
        let transport = VirtioMmioTransport::new(Box::new(device), mem, interrupt).unwrap();
        (transport, activated, notified)
    }

    /// An [`Interrupt`] with no KVM behind it, for tests.
    fn test_interrupt() -> Interrupt {
        // `Gsi` is only constructible with a VmFd; expose a test constructor.
        Interrupt::for_test()
    }

    fn write(transport: &mut VirtioMmioTransport, offset: u32, value: u32) {
        transport.mmio_write(MmioAddress(0), offset.into(), &value.to_le_bytes());
    }

    fn read(transport: &mut VirtioMmioTransport, offset: u32) -> u32 {
        let mut buf = [0u8; 4];
        transport.mmio_read(MmioAddress(0), offset.into(), &mut buf);
        u32::from_le_bytes(buf)
    }

    #[test]
    fn reports_the_mmio_identity() {
        let (mut t, _, _) = transport();
        assert_eq!(read(&mut t, VIRTIO_MMIO_MAGIC_VALUE), MAGIC);
        assert_eq!(read(&mut t, VIRTIO_MMIO_VERSION), 2);
        assert_eq!(read(&mut t, VIRTIO_MMIO_DEVICE_ID), 19);
        assert_eq!(read(&mut t, VIRTIO_MMIO_QUEUE_NUM_MAX), 8);
    }

    #[test]
    fn negotiates_features_then_activates() {
        let (mut t, activated, notified) = transport();
        write(&mut t, VIRTIO_MMIO_STATUS, VIRTIO_CONFIG_S_ACKNOWLEDGE);
        write(&mut t, VIRTIO_MMIO_STATUS, VIRTIO_CONFIG_S_DRIVER);

        // Device features low half; VIRTIO_F_VERSION_1 is in the high half.
        write(&mut t, VIRTIO_MMIO_DEVICE_FEATURES_SEL, 0);
        assert_eq!(read(&mut t, VIRTIO_MMIO_DEVICE_FEATURES), 0x3);
        write(&mut t, VIRTIO_MMIO_DEVICE_FEATURES_SEL, 1);
        assert_eq!(read(&mut t, VIRTIO_MMIO_DEVICE_FEATURES), 1);

        // Accept the low bits and VERSION_1.
        write(&mut t, VIRTIO_MMIO_DRIVER_FEATURES_SEL, 0);
        write(&mut t, VIRTIO_MMIO_DRIVER_FEATURES, 0x3);
        write(&mut t, VIRTIO_MMIO_DRIVER_FEATURES_SEL, 1);
        write(&mut t, VIRTIO_MMIO_DRIVER_FEATURES, 1);
        write(
            &mut t,
            VIRTIO_MMIO_STATUS,
            VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER | VIRTIO_CONFIG_S_FEATURES_OK,
        );
        assert_eq!(t.acked_features(), 0x3 | F_VERSION_1);

        // Set up queue 0 and mark it ready, then activate.
        write(&mut t, VIRTIO_MMIO_QUEUE_SEL, 0);
        write(&mut t, VIRTIO_MMIO_QUEUE_NUM, 8);
        write(&mut t, VIRTIO_MMIO_QUEUE_DESC_LOW, 0x1000);
        write(&mut t, VIRTIO_MMIO_QUEUE_AVAIL_LOW, 0x2000);
        write(&mut t, VIRTIO_MMIO_QUEUE_USED_LOW, 0x3000);
        write(&mut t, VIRTIO_MMIO_QUEUE_READY, 1);
        assert_eq!(read(&mut t, VIRTIO_MMIO_QUEUE_READY), 1);

        write(
            &mut t,
            VIRTIO_MMIO_STATUS,
            VIRTIO_CONFIG_S_ACKNOWLEDGE
                | VIRTIO_CONFIG_S_DRIVER
                | VIRTIO_CONFIG_S_FEATURES_OK
                | VIRTIO_CONFIG_S_DRIVER_OK,
        );
        assert!(activated.load(Ordering::SeqCst));

        write(&mut t, VIRTIO_MMIO_QUEUE_NOTIFY, 0);
        assert_eq!(*notified.lock().unwrap(), vec![0]);
    }

    #[test]
    fn rejects_unsupported_features_at_features_ok() {
        let (mut t, activated, _) = transport();
        write(
            &mut t,
            VIRTIO_MMIO_DRIVER_FEATURES,
            0xFFFF_FFFF,
        );
        write(
            &mut t,
            VIRTIO_MMIO_STATUS,
            VIRTIO_CONFIG_S_ACKNOWLEDGE | VIRTIO_CONFIG_S_DRIVER | VIRTIO_CONFIG_S_FEATURES_OK,
        );
        assert_eq!(read(&mut t, VIRTIO_MMIO_STATUS) & VIRTIO_CONFIG_S_FAILED, VIRTIO_CONFIG_S_FAILED);
        write(&mut t, VIRTIO_MMIO_STATUS, 0xFF);
        assert!(!activated.load(Ordering::SeqCst));
    }

    #[test]
    fn routes_config_space_and_reset() {
        let (mut t, _, _) = transport();
        let mut config = [0u8; 2];
        t.mmio_read(
            MmioAddress(0),
            u64::from(VIRTIO_MMIO_CONFIG),
            &mut config,
        );
        assert_eq!(config, [0xAA, 0xBB]);

        // A status write of 0 resets the device and its queues.
        write(&mut t, VIRTIO_MMIO_QUEUE_SEL, 0);
        write(&mut t, VIRTIO_MMIO_QUEUE_READY, 1);
        write(&mut t, VIRTIO_MMIO_STATUS, 0);
        assert_eq!(read(&mut t, VIRTIO_MMIO_QUEUE_READY), 0);
    }
}
