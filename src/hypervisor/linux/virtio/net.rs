//! Virtio-net device.
//!
//! Emulates the guest side of a `virtio-net` NIC and bridges it to a host TAP
//! interface. The driver posts empty buffers on the receive queue (RX, index 0)
//! and hands us complete Ethernet frames on the transmit queue (TX, index 1);
//! the worker relays frames between those rings and the tap fd.
//!
//! Each frame in the ring is prefixed with `struct virtio_net_hdr`. We negotiate
//! neither `VIRTIO_NET_F_MRG_RXBUF` nor any checksum/segmentation offload, so the
//! prefix is the fixed 10-byte, all-zero header and the frame that follows is the
//! raw Ethernet frame the tap expects.
//!
//! The MAC address and link state live in the config space, which the driver
//! reads once during probe, so no config-change interrupt is needed.

use std::io::{self, Read, Write};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use virtio_bindings::bindings::virtio_net::{
    VIRTIO_NET_F_MAC, VIRTIO_NET_F_STATUS, VIRTIO_NET_S_LINK_UP,
};
use virtio_bindings::virtio_ids::VIRTIO_ID_NET;
use virtio_queue::{QueueSync, QueueT, Reader, Writer};
use vm_memory::GuestMemoryMmap;

use super::{Activate, Interrupt, VirtioDevice};
use crate::error::{Error, Result};
use crate::hypervisor::linux::tap::TapDevice;

/// Queue indices, in the order the driver expects (rx, tx).
const QUEUE_RX: usize = 0;
const QUEUE_TX: usize = 1;
const QUEUE_COUNT: usize = 2;
/// Descriptor-chain size the driver is told it may use.
const QUEUE_SIZE: u16 = 256;

/// Size of `struct virtio_net_hdr` without `MRG_RXBUF`.
const NET_HDR_LEN: usize = 10;

/// Largest frame we will relay, an upper bound that covers jumbo frames.
const MAX_FRAME: usize = 64 * 1024;

/// How long the worker sleeps between polls when both queues are idle.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The host side of the NIC, where the worker reads and writes Ethernet frames.
///
/// Production code uses [`TapDevice`]; unit tests substitute an in-memory pipe
/// so the data path can be exercised without `/dev/net/tun` or `CAP_NET_ADMIN`.
pub(crate) trait FrameIo: Send {
    /// Read one frame. Returns `WouldBlock` when none is available, the way a
    /// non-blocking tap does.
    fn read_frame(&self, buf: &mut [u8]) -> io::Result<usize>;
    /// Write one frame.
    fn write_frame(&self, buf: &[u8]) -> io::Result<usize>;
}

impl FrameIo for TapDevice {
    fn read_frame(&self, buf: &mut [u8]) -> io::Result<usize> {
        TapDevice::read_frame(self, buf)
    }

    fn write_frame(&self, buf: &[u8]) -> io::Result<usize> {
        TapDevice::write_frame(self, buf)
    }
}

/// Configuration for one virtio-net device.
pub(crate) struct NetConfig {
    /// Address reported in the config space (`VIRTIO_NET_F_MAC`).
    pub mac: [u8; 6],
    /// Link state reported in the config space (`VIRTIO_NET_F_STATUS`).
    pub link_up: bool,
}

/// The queue-facing device. Activation hands the data plane to a worker thread.
pub(crate) struct VirtioNet {
    config: NetConfig,
    plane: Option<Arc<(Mutex<Plane>, Condvar)>>,
    worker: Option<std::thread::JoinHandle<()>>,
    /// The tap handle, moved into the worker on activation and reclaimed on
    /// reset so the device can be activated again after a driver reset.
    io: Option<Box<dyn FrameIo>>,
}

impl VirtioNet {
    pub(crate) fn new(config: NetConfig, io: Box<dyn FrameIo>) -> Self {
        Self {
            config,
            plane: None,
            worker: None,
            io: Some(io),
        }
    }
}

impl VirtioDevice for VirtioNet {
    fn device_id(&self) -> u32 {
        VIRTIO_ID_NET
    }

    fn features(&self) -> u64 {
        (1u64 << VIRTIO_NET_F_MAC) | (1u64 << VIRTIO_NET_F_STATUS)
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        let mut space = [0u8; 8];
        space[..6].copy_from_slice(&self.config.mac);
        let status: u16 = if self.config.link_up {
            VIRTIO_NET_S_LINK_UP as u16
        } else {
            0
        };
        space[6..8].copy_from_slice(&status.to_le_bytes());
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = space.get(offset as usize + i).copied().unwrap_or(0);
        }
    }

    fn queue_count(&self) -> usize {
        QUEUE_COUNT
    }

    fn queue_max_size(&self) -> u16 {
        QUEUE_SIZE
    }

    fn activate(&mut self, ctx: Activate) -> Result<()> {
        if ctx.queues.len() != QUEUE_COUNT {
            return Err(Error::Hypervisor(format!(
                "virtio-net needs {QUEUE_COUNT} queues, got {}",
                ctx.queues.len()
            )));
        }
        let Some(io) = self.io.take() else {
            return Err(Error::Hypervisor(
                "virtio-net activated without a tap".into(),
            ));
        };
        let plane = Arc::new((
            Mutex::new(Plane {
                mem: ctx.mem,
                queues: ctx.queues,
                interrupt: ctx.interrupt,
                io: Some(io),
                pending_rx: None,
                stop: false,
            }),
            Condvar::new(),
        ));
        let worker_plane = plane.clone();
        let worker = std::thread::Builder::new()
            .name("virtio-net".into())
            .spawn(move || run_worker(worker_plane))
            .map_err(|err| Error::Hypervisor(format!("spawn virtio-net worker: {err}")))?;
        self.plane = Some(plane);
        self.worker = Some(worker);
        Ok(())
    }

    fn notify(&mut self, queue_index: u16) {
        if let Some(plane) = &self.plane {
            if queue_index as usize == QUEUE_TX {
                let (lock, cvar) = &**plane;
                let _guard = lock.lock().unwrap_or_else(|err| err.into_inner());
                cvar.notify_one();
            }
        }
    }

    fn reset(&mut self) {
        if let Some(plane) = &self.plane {
            let (lock, cvar) = &**plane;
            lock.lock().unwrap_or_else(|err| err.into_inner()).stop = true;
            cvar.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // Reclaim the tap so a later `DRIVER_OK` can start a fresh worker.
        if let Some(plane) = self.plane.take() {
            let (lock, _) = &*plane;
            let mut guard = lock.lock().unwrap_or_else(|err| err.into_inner());
            self.io = guard.io.take();
        }
    }
}

impl Drop for VirtioNet {
    fn drop(&mut self) {
        self.reset();
    }
}

/// All mutable state the worker touches, shared with the transport's `notify`.
struct Plane {
    mem: GuestMemoryMmap,
    queues: Vec<QueueSync>,
    interrupt: Interrupt,
    io: Option<Box<dyn FrameIo>>,
    /// A frame read from the tap with no RX buffer to hold it yet.
    pending_rx: Option<Vec<u8>>,
    stop: bool,
}

fn run_worker(plane: Arc<(Mutex<Plane>, Condvar)>) {
    let (lock, cvar) = &*plane;
    let mut guard = lock.lock().unwrap_or_else(|err| err.into_inner());
    while !guard.stop {
        guard.tick();
        if guard.stop {
            break;
        }
        let (next, _timeout) = cvar
            .wait_timeout(guard, POLL_INTERVAL)
            .unwrap_or_else(|err| err.into_inner());
        guard = next;
    }
}

impl Plane {
    /// One pass: drain the guest's TX ring to the tap, feed tap frames into RX.
    fn tick(&mut self) {
        self.drain_tx();
        self.pump_rx();
    }

    /// Guest -> tap: strip the virtio header and write the frame out.
    fn drain_tx(&mut self) {
        let Some(io) = self.io.as_deref() else {
            return;
        };
        let mem = self.mem.clone();
        let mut tx = self.queues[QUEUE_TX].clone();
        while let Some(chain) = tx.pop_descriptor_chain(&mem) {
            let head = chain.head_index();
            let mut used = 0u32;
            if let Ok(mut reader) = Reader::new(&mem, chain) {
                let avail = reader.available_bytes().min(MAX_FRAME);
                let mut buf = vec![0u8; avail];
                if let Ok(n) = reader.read(&mut buf) {
                    used = n as u32;
                    if n > NET_HDR_LEN {
                        let _ = io.write_frame(&buf[NET_HDR_LEN..n]);
                    }
                }
            }
            let _ = tx.add_used(&mem, head, used);
        }
    }

    /// Tap -> guest: read one frame and put it in an RX buffer.
    fn pump_rx(&mut self) {
        if self.pending_rx.is_some() {
            self.flush_pending();
            // Still no buffer: do not pull more frames off the tap.
            if self.pending_rx.is_some() {
                return;
            }
        }
        let Some(io) = self.io.as_deref() else {
            return;
        };
        let mut buf = vec![0u8; MAX_FRAME];
        match io.read_frame(&mut buf) {
            Ok(0) => {}
            Ok(n) => {
                let frame = buf[..n].to_vec();
                if !self.send_to_guest(&frame) {
                    self.pending_rx = Some(frame);
                }
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            Err(_) => {}
        }
    }

    fn flush_pending(&mut self) {
        if let Some(frame) = self.pending_rx.take() {
            if !self.send_to_guest(&frame) {
                self.pending_rx = Some(frame);
            }
        }
    }

    /// Write the header plus `frame` into one RX descriptor chain. Returns
    /// false when the guest has not published a buffer yet.
    fn send_to_guest(&mut self, frame: &[u8]) -> bool {
        let mem = self.mem.clone();
        let mut rx = self.queues[QUEUE_RX].clone();
        let Some(chain) = rx.pop_descriptor_chain(&mem) else {
            return false;
        };
        let head = chain.head_index();
        let Ok(mut writer) = Writer::new(&mem, chain) else {
            let _ = rx.add_used(&mem, head, 0);
            return false;
        };
        if writer.available_bytes() < NET_HDR_LEN + frame.len() {
            // Descriptor is too small for this frame; drop it and reseat.
            let _ = rx.add_used(&mem, head, 0);
            return false;
        }
        let header = [0u8; NET_HDR_LEN];
        let mut written = 0usize;
        if writer.write_all(&header).is_ok() {
            written += NET_HDR_LEN;
        }
        if writer.write_all(frame).is_ok() {
            written += frame.len();
        }
        let _ = rx.add_used(&mem, head, written as u32);
        self.interrupt.signal_used();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use virtio_queue::desc::split::Descriptor;
    use virtio_queue::desc::RawDescriptor;
    use virtio_queue::mock::MockSplitQueue;
    use vm_memory::{Bytes, GuestAddress, GuestMemory};

    /// An in-memory stand-in for the tap: frames written by the device are
    /// recorded, frames queued by the test are read back.
    #[derive(Clone, Default)]
    struct Recorder {
        writes: Arc<Mutex<Vec<Vec<u8>>>>,
        reads: Arc<Mutex<VecDeque<Vec<u8>>>>,
    }

    impl FrameIo for Recorder {
        fn read_frame(&self, buf: &mut [u8]) -> io::Result<usize> {
            match self.reads.lock().unwrap().pop_front() {
                Some(frame) => {
                    let n = frame.len().min(buf.len());
                    buf[..n].copy_from_slice(&frame[..n]);
                    Ok(n)
                }
                None => Err(io::Error::from(io::ErrorKind::WouldBlock)),
            }
        }

        fn write_frame(&self, buf: &[u8]) -> io::Result<usize> {
            self.writes.lock().unwrap().push(buf.to_vec());
            Ok(buf.len())
        }
    }

    fn guest_memory() -> GuestMemoryMmap {
        GuestMemoryMmap::from_ranges(&[(GuestAddress(0), 0x10_0000)]).unwrap()
    }

    const DATA_ADDR: u64 = 0x8_0000;

    #[test]
    fn config_space_reports_mac_and_link_status() {
        let device = VirtioNet::new(
            NetConfig {
                mac: [0x02, 1, 2, 3, 4, 5],
                link_up: true,
            },
            Box::new(Recorder::default()),
        );
        let mut space = [0u8; 8];
        device.read_config(0, &mut space);
        assert_eq!(&space[..6], &[0x02, 1, 2, 3, 4, 5]);
        assert_eq!(
            u16::from_le_bytes([space[6], space[7]]),
            VIRTIO_NET_S_LINK_UP as u16
        );
        assert_eq!(device.device_id(), VIRTIO_ID_NET);
        assert_eq!(device.queue_count(), QUEUE_COUNT);
        assert_ne!(device.features() & (1 << VIRTIO_NET_F_MAC), 0);
        assert_ne!(device.features() & (1 << VIRTIO_NET_F_STATUS), 0);
    }

    fn empty_queue() -> QueueSync {
        QueueSync::new(QUEUE_SIZE).unwrap()
    }

    fn plane(mem: &GuestMemoryMmap, rx: QueueSync, tx: QueueSync, io: Recorder) -> Plane {
        Plane {
            mem: mem.clone(),
            queues: vec![rx, tx],
            interrupt: Interrupt::for_test(),
            io: Some(Box::new(io)),
            pending_rx: None,
            stop: false,
        }
    }

    #[test]
    fn tx_frames_are_forwarded_without_the_virtio_header() {
        let mem = guest_memory();
        let mock = MockSplitQueue::new(&mem, 8);
        let mut buf = vec![0u8; NET_HDR_LEN];
        buf.extend_from_slice(b"frame-bytes");
        mem.write_slice(&buf, GuestAddress(DATA_ADDR)).unwrap();
        // Device-readable descriptor: no VRING_DESC_F_WRITE.
        let desc = RawDescriptor::from(Descriptor::new(DATA_ADDR, buf.len() as u32, 0, 0));
        mock.build_desc_chain(&[desc]).unwrap();
        let tx = mock.create_queue::<QueueSync>().unwrap();

        let recorder = Recorder::default();
        let mut plane = plane(&mem, empty_queue(), tx, recorder.clone());
        plane.drain_tx();

        assert_eq!(
            recorder.writes.lock().unwrap().as_slice(),
            &[b"frame-bytes".to_vec()]
        );
    }

    #[test]
    fn rx_frames_reach_the_guest_behind_a_header() {
        let mem = guest_memory();
        let mock = MockSplitQueue::new(&mem, 8);
        let bufsize = 2048u32;
        mem.write_slice(&vec![0u8; bufsize as usize], GuestAddress(DATA_ADDR))
            .unwrap();
        // Device-writable descriptor: VRING_DESC_F_WRITE.
        let desc = RawDescriptor::from(Descriptor::new(DATA_ADDR, bufsize, 2, 0));
        mock.build_desc_chain(&[desc]).unwrap();
        let rx = mock.create_queue::<QueueSync>().unwrap();

        let recorder = Recorder::default();
        recorder.reads.lock().unwrap().push_back(b"reply".to_vec());
        let mut plane = plane(&mem, rx, empty_queue(), recorder);
        plane.pump_rx();

        let mut out = vec![0u8; NET_HDR_LEN + 5];
        mem.read_slice(&mut out, GuestAddress(DATA_ADDR)).unwrap();
        assert_eq!(&out[..NET_HDR_LEN], &[0u8; NET_HDR_LEN]);
        assert_eq!(&out[NET_HDR_LEN..], b"reply");
    }

    #[test]
    fn a_frame_without_an_rx_buffer_waits_for_one() {
        let mem = guest_memory();
        let recorder = Recorder::default();
        recorder.reads.lock().unwrap().push_back(b"held".to_vec());
        // No RX buffers published yet.
        let mut plane = plane(&mem, empty_queue(), empty_queue(), recorder.clone());
        plane.pump_rx();
        assert_eq!(plane.pending_rx.as_deref(), Some(&b"held"[..]));
        // With no room we must not have consumed the next tap frame.
        assert!(recorder.reads.lock().unwrap().is_empty() || plane.pending_rx.is_some());
    }
}
