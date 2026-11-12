//! Virtio-vsock device.
//!
//! Emulates the guest side of a `virtio-vsock` device and bridges it to a host
//! **Unix domain socket**. The guest's `AF_VSOCK` stack sends a `REQUEST` to
//! connect to `(cid, port)`; this device turns that into a `connect()` on the
//! host socket path and, on success, completes the handshake with `RESPONSE`.
//! `RW` frames then relay payload bytes in both directions.
//!
//! This is the transport the GuestAgent speaks over: the guest connects to the
//! proxy port, the device dials the agent's Unix socket, and the two are spliced
//! together frame by frame.
//!
//! The packet header layout and op/type constants follow
//! `include/uapi/linux/virtio_vsock.h`.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use virtio_bindings::virtio_ids::VIRTIO_ID_VSOCK;
use virtio_queue::{QueueSync, QueueT, Reader, Writer};
use vm_memory::GuestMemoryMmap;

use super::{Activate, Interrupt, VirtioDevice};
use crate::error::{Error, Result};

/// Queue indices, in the order the driver expects (rx, tx, event).
const QUEUE_RX: usize = 0;
const QUEUE_TX: usize = 1;
const QUEUE_EVENT: usize = 2;
const QUEUE_COUNT: usize = 3;
/// Descriptor-chain size the driver is told it may use.
const QUEUE_SIZE: u16 = 256;

/// The largest frame we will relay in one descriptor chain.
const MAX_FRAME: usize = 64 * 1024;

/// Credit (bytes) we advertise to the guest. Large enough that the agent RPC
/// never needs a flow-control update in practice.
const BUF_ALLOC: u32 = 256 * 1024;

/// How long the worker sleeps between polls when the queues are idle.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

mod proto {
    //! The `virtio_vsock_hdr` packet header and its op/type constants.

    /// `struct virtio_vsock_hdr`: 44 bytes, little-endian.
    pub(super) const HDR_LEN: usize = 44;

    pub(super) const OP_INVALID: u16 = 0;
    pub(super) const OP_REQUEST: u16 = 1;
    pub(super) const OP_RESPONSE: u16 = 2;
    pub(super) const OP_RST: u16 = 3;
    pub(super) const OP_SHUTDOWN: u16 = 4;
    pub(super) const OP_RW: u16 = 5;
    pub(super) const OP_CREDIT_UPDATE: u16 = 6;
    pub(super) const OP_CREDIT_REQUEST: u16 = 7;

    pub(super) const TYPE_STREAM: u16 = 1;

    /// `VMADDR_CID_HOST`: the cid a peer uses to reach the host.
    pub(super) const CID_HOST: u64 = 2;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) struct Hdr {
        pub src_cid: u64,
        pub dst_cid: u64,
        pub src_port: u32,
        pub dst_port: u32,
        pub len: u32,
        pub type_: u16,
        pub op: u16,
        pub flags: u32,
        pub buf_alloc: u32,
        pub fwd_cnt: u32,
    }

    impl Hdr {
        pub(super) fn encode(&self) -> [u8; HDR_LEN] {
            let mut buf = [0u8; HDR_LEN];
            buf[0..8].copy_from_slice(&self.src_cid.to_le_bytes());
            buf[8..16].copy_from_slice(&self.dst_cid.to_le_bytes());
            buf[16..20].copy_from_slice(&self.src_port.to_le_bytes());
            buf[20..24].copy_from_slice(&self.dst_port.to_le_bytes());
            buf[24..28].copy_from_slice(&self.len.to_le_bytes());
            buf[28..30].copy_from_slice(&self.type_.to_le_bytes());
            buf[30..32].copy_from_slice(&self.op.to_le_bytes());
            buf[32..36].copy_from_slice(&self.flags.to_le_bytes());
            buf[36..40].copy_from_slice(&self.buf_alloc.to_le_bytes());
            buf[40..44].copy_from_slice(&self.fwd_cnt.to_le_bytes());
            buf
        }

        /// Decode a header if `bytes` holds at least one full header.
        pub(super) fn decode(bytes: &[u8]) -> Option<Hdr> {
            if bytes.len() < HDR_LEN {
                return None;
            }
            Some(Hdr {
                src_cid: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
                dst_cid: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
                src_port: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
                dst_port: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
                len: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
                type_: u16::from_le_bytes(bytes[28..30].try_into().ok()?),
                op: u16::from_le_bytes(bytes[30..32].try_into().ok()?),
                flags: u32::from_le_bytes(bytes[32..36].try_into().ok()?),
                buf_alloc: u32::from_le_bytes(bytes[36..40].try_into().ok()?),
                fwd_cnt: u32::from_le_bytes(bytes[40..44].try_into().ok()?),
            })
        }
    }
}

use proto::*;

/// Configuration for one virtio-vsock device.
pub(crate) struct VsockConfig {
    /// The cid the guest is assigned (`guest_cid` in the device config space).
    pub guest_cid: u64,
    /// The vsock port the guest connects to.
    pub port: u32,
    /// The host Unix socket this port is proxied to.
    pub proxy_path: PathBuf,
}

/// The queue-facing device. Activation hands the data plane to a worker thread.
pub(crate) struct VirtioVsock {
    config: VsockConfig,
    plane: Option<Arc<(Mutex<Plane>, Condvar)>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl VirtioVsock {
    pub(crate) fn new(config: VsockConfig) -> Self {
        Self {
            config,
            plane: None,
            worker: None,
        }
    }
}

impl VirtioDevice for VirtioVsock {
    fn device_id(&self) -> u32 {
        VIRTIO_ID_VSOCK
    }

    fn features(&self) -> u64 {
        0
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        let cid = self.config.guest_cid.to_le_bytes();
        for (i, byte) in data.iter_mut().enumerate() {
            *byte = cid.get(offset as usize + i).copied().unwrap_or(0);
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
                "virtio-vsock needs {QUEUE_COUNT} queues, got {}",
                ctx.queues.len()
            )));
        }
        let plane = Arc::new((
            Mutex::new(Plane {
                mem: ctx.mem,
                queues: ctx.queues,
                interrupt: ctx.interrupt,
                proxy_path: self.config.proxy_path.clone(),
                port: self.config.port,
                guest_cid: self.config.guest_cid,
                conn: None,
                pending_rx: Vec::new(),
                stop: false,
            }),
            Condvar::new(),
        ));
        let worker_plane = plane.clone();
        let worker = std::thread::Builder::new()
            .name("virtio-vsock".into())
            .spawn(move || run_worker(worker_plane))
            .map_err(|err| Error::Hypervisor(format!("spawn virtio-vsock worker: {err}")))?;
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
        self.plane = None;
    }
}

impl Drop for VirtioVsock {
    fn drop(&mut self) {
        self.reset();
    }
}

/// One accepted guest connection, spliced to a host Unix socket.
struct Conn {
    stream: UnixStream,
    /// The guest's port (the source port of its `REQUEST`).
    guest_port: u32,
    /// Bytes we have consumed from the guest, echoed back as `fwd_cnt`.
    fwd_cnt: u32,
    /// Host socket has hit EOF; drain the guest, then close.
    closed: bool,
}

/// All mutable state the worker touches, shared with the transport's `notify`.
struct Plane {
    mem: GuestMemoryMmap,
    queues: Vec<QueueSync>,
    interrupt: Interrupt,
    proxy_path: PathBuf,
    port: u32,
    guest_cid: u64,
    conn: Option<Conn>,
    /// Host bytes with no RX descriptor to write them into yet.
    pending_rx: Vec<u8>,
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
    /// One pass: drain the guest's TX ring, push host data into the RX ring.
    fn tick(&mut self) {
        self.drain_tx();
        self.pump_host();
        self.flush_pending();
        self.ack_events();
    }

    fn drain_tx(&mut self) {
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
                    if let Some(hdr) = Hdr::decode(&buf[..n]) {
                        let payload_end = (HDR_LEN + hdr.len as usize).min(n);
                        let payload = &buf[HDR_LEN.min(n)..payload_end];
                        self.handle_guest_packet(hdr, payload);
                    }
                }
            }
            let _ = tx.add_used(&mem, head, used);
        }
    }

    fn handle_guest_packet(&mut self, hdr: Hdr, payload: &[u8]) {
        match hdr.op {
            OP_REQUEST => self.open_connection(&hdr),
            OP_RW => {
                if let Some(conn) = &mut self.conn {
                    conn.fwd_cnt = conn.fwd_cnt.wrapping_add(hdr.len);
                    if conn.stream.write_all(payload).is_err() {
                        self.conn = None;
                        self.send_rst(hdr.src_port);
                    }
                } else {
                    self.send_rst(hdr.src_port);
                }
            }
            OP_SHUTDOWN => {
                if let Some(conn) = &mut self.conn {
                    let _ = conn.stream.shutdown(std::net::Shutdown::Write);
                    conn.closed = true;
                }
            }
            OP_RST => self.conn = None,
            OP_CREDIT_REQUEST => self.send_credit(hdr.src_port),
            _ => {}
        }
    }

    fn open_connection(&mut self, hdr: &Hdr) {
        if hdr.dst_port != self.port {
            self.send_rst(hdr.src_port);
            return;
        }
        match UnixStream::connect(&self.proxy_path) {
            Ok(stream) => {
                let _ = stream.set_nonblocking(true);
                self.conn = Some(Conn {
                    stream,
                    guest_port: hdr.src_port,
                    fwd_cnt: 0,
                    closed: false,
                });
                self.send_control(hdr.src_port, OP_RESPONSE);
            }
            Err(_) => self.send_rst(hdr.src_port),
        }
    }

    fn send_rst(&mut self, guest_port: u32) {
        self.conn = None;
        self.send_control(guest_port, OP_RST);
    }

    fn send_credit(&mut self, guest_port: u32) {
        self.send_control(guest_port, OP_CREDIT_UPDATE);
    }

    fn send_control(&mut self, guest_port: u32, op: u16) {
        let hdr = Hdr {
            src_cid: CID_HOST,
            dst_cid: self.guest_cid,
            src_port: self.port,
            dst_port: guest_port,
            len: 0,
            type_: TYPE_STREAM,
            op,
            flags: 0,
            buf_alloc: BUF_ALLOC,
            fwd_cnt: self.conn.as_ref().map(|c| c.fwd_cnt).unwrap_or(0),
        };
        self.send_to_guest(&hdr, &[]);
    }

    /// A `CREDIT_REQUEST` from the guest is answered by the kernel driver
    /// automatically; we only need to ack the event queue so it does not fill.
    fn ack_events(&mut self) {
        let mem = self.mem.clone();
        let mut ev = self.queues[QUEUE_EVENT].clone();
        while let Some(chain) = ev.pop_descriptor_chain(&mem) {
            let head = chain.head_index();
            let _ = ev.add_used(&mem, head, 0);
        }
    }

    fn pump_host(&mut self) {
        let Some(conn) = &mut self.conn else {
            return;
        };
        let mut buf = [0u8; 8 * 1024];
        match conn.stream.read(&mut buf) {
            Ok(0) => {
                // Host hung up: tell the guest the connection is gone.
                let guest_port = conn.guest_port;
                self.conn = None;
                self.send_rst(guest_port);
            }
            Ok(n) => {
                let guest_port = conn.guest_port;
                let payload = buf[..n].to_vec();
                self.relay_to_guest(guest_port, &payload);
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => {
                let guest_port = conn.guest_port;
                self.conn = None;
                self.send_rst(guest_port);
            }
        }
    }

    fn relay_to_guest(&mut self, guest_port: u32, payload: &[u8]) {
        let hdr = Hdr {
            src_cid: CID_HOST,
            dst_cid: self.guest_cid,
            src_port: self.port,
            dst_port: guest_port,
            len: payload.len() as u32,
            type_: TYPE_STREAM,
            op: OP_RW,
            flags: 0,
            buf_alloc: BUF_ALLOC,
            fwd_cnt: self.conn.as_ref().map(|c| c.fwd_cnt).unwrap_or(0),
        };
        if !self.send_to_guest(&hdr, payload) {
            // No RX buffer: remember the frame so `flush_pending` can retry.
            let mut frame = hdr.encode().to_vec();
            frame.extend_from_slice(payload);
            if self.pending_rx.len() + frame.len() <= BUF_ALLOC as usize {
                self.pending_rx.extend_from_slice(&frame);
            }
        }
    }

    fn flush_pending(&mut self) {
        while !self.pending_rx.is_empty() {
            let frame = self.pending_rx.split_off(0);
            let (hdr, payload) = match Hdr::decode(&frame) {
                Some(hdr) => (hdr, frame[HDR_LEN..].to_vec()),
                None => {
                    self.pending_rx.clear();
                    return;
                }
            };
            if !self.send_to_guest(&hdr, &payload) {
                // Still no buffer; put the frame back and wait.
                self.pending_rx = frame;
                return;
            }
        }
    }

    /// Write `hdr` + `payload` into one RX descriptor chain. Returns false when
    /// the guest has not published a buffer yet.
    fn send_to_guest(&mut self, hdr: &Hdr, payload: &[u8]) -> bool {
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
        let header = hdr.encode();
        if writer.available_bytes() < HDR_LEN + payload.len() {
            // Descriptor is too small for this frame; drop it and reseat.
            let _ = rx.add_used(&mem, head, 0);
            return false;
        }
        let mut written = 0;
        if writer.write_all(&header).is_ok() {
            written += HDR_LEN;
        }
        if writer.write_all(payload).is_ok() {
            written += payload.len();
        }
        let _ = rx.add_used(&mem, head, written as u32);
        self.interrupt.signal_used();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::proto::*;
    use super::*;

    #[test]
    fn header_round_trips() {
        let hdr = Hdr {
            src_cid: 3,
            dst_cid: 2,
            src_port: 1024,
            dst_port: 5000,
            len: 12,
            type_: TYPE_STREAM,
            op: OP_RW,
            flags: 0,
            buf_alloc: BUF_ALLOC,
            fwd_cnt: 7,
        };
        let bytes = hdr.encode();
        assert_eq!(bytes.len(), HDR_LEN);
        assert_eq!(Hdr::decode(&bytes), Some(hdr));
    }

    #[test]
    fn short_buffer_is_not_a_header() {
        assert_eq!(Hdr::decode(&[0u8; HDR_LEN - 1]), None);
    }

    #[test]
    fn config_space_reports_the_guest_cid() {
        let device = VirtioVsock::new(VsockConfig {
            guest_cid: 3,
            port: 1024,
            proxy_path: PathBuf::from("/tmp/agent.sock"),
        });
        let mut config = [0u8; 8];
        device.read_config(0, &mut config);
        assert_eq!(u64::from_le_bytes(config), 3);
        assert_eq!(device.device_id(), VIRTIO_ID_VSOCK);
        assert_eq!(device.queue_count(), QUEUE_COUNT);
    }
}
