//! Virtio-fs device.
//!
//! Shares a host directory tree with the guest by carrying FUSE requests and
//! replies over a virtqueue. The guest's `virtiofs` driver mounts a filesystem
//! tagged with the name in the config space; every VFS operation then becomes a
//! FUSE message on the request queue.
//!
//! The FUSE protocol handling lives in `fuse-backend-rs`: [`Server`] parses each
//! request and dispatches it to a filesystem driver, here [`PassthroughFs`],
//! which serves the exported directory with ordinary host syscalls. This device
//! is only the transport glue: it pops descriptor chains off the queues, builds a
//! [`Reader`]/[`VirtioFsWriter`] pair over guest memory, and hands them to the
//! server, then reports the reply length back in the used ring.
//!
//! Configuration follows `include/uapi/linux/virtio_fs.h`: a 36-byte tag which
//! the guest mounts by name, plus the number of request queues.

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use fuse_backend_rs::api::server::Server;
use fuse_backend_rs::passthrough::{Config as PassthroughConfig, PassthroughFs};
use fuse_backend_rs::transport::{Reader, VirtioFsWriter};
use virtio_bindings::virtio_ids::VIRTIO_ID_FS;
use virtio_queue::{QueueSync, QueueT};
use vm_memory::GuestMemoryMmap;

use super::{Activate, Interrupt, VirtioDevice};
use crate::error::{Error, Result};

/// Queue indices. Queue 0 is the high-priority queue (forget/interrupt), the
/// remaining queues carry ordinary requests. Both are served identically here.
const QUEUE_HIPRIO: usize = 0;
const QUEUE_REQUEST: usize = 1;
/// One request queue plus the high-priority queue.
const QUEUE_COUNT: usize = 2;
/// Reported in the config space; the guest allocates `NUM_REQUEST_QUEUES + 1`
/// virtqueues from it.
const NUM_REQUEST_QUEUES: u32 = 1;
/// Descriptor-chain size the driver is told it may use.
const QUEUE_SIZE: u16 = 256;

/// Maximum length of the mount tag, per `struct virtio_fs_config`.
const TAG_LEN: usize = 36;

/// How long the worker sleeps between polls when the queues are idle.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// The filesystem driver this device serves. Passthrough over the host VFS.
type Fs = PassthroughFs<()>;

/// Configuration for one virtio-fs device.
pub(crate) struct FsConfig {
    /// Mount tag, matching the `tag` the guest passes to `mount -t virtiofs`.
    pub tag: String,
    /// Host directory tree to export.
    pub root_dir: PathBuf,
}

/// The queue-facing device. Activation hands the data plane to a worker thread.
pub(crate) struct VirtioFs {
    config: FsConfig,
    /// The FUSE server, shared with the worker. Kept here so a re-activation
    /// after a driver reset reuses the same filesystem state.
    server: Arc<Server<Fs>>,
    plane: Option<Arc<(Mutex<Plane>, Condvar)>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl VirtioFs {
    /// Open the exported directory and build the FUSE server. Fails if the
    /// share cannot be served, so `boot` reports a bad rootfs instead of
    /// exposing an empty mount.
    pub(crate) fn new(config: FsConfig) -> Result<Self> {
        let passthrough = PassthroughFs::new(PassthroughConfig {
            root_dir: config.root_dir.to_string_lossy().into_owned(),
            do_import: true,
            ..Default::default()
        })
        .map_err(|err| {
            Error::Resource(format!(
                "serve virtio-fs share {}: {err}",
                config.root_dir.display()
            ))
        })?;
        Ok(Self {
            config,
            server: Arc::new(Server::new(passthrough)),
            plane: None,
            worker: None,
        })
    }
}

impl VirtioDevice for VirtioFs {
    fn device_id(&self) -> u32 {
        VIRTIO_ID_FS
    }

    fn features(&self) -> u64 {
        // The transport handles VIRTIO_F_VERSION_1; no device-specific feature
        // (no DAX window, no notification queue) is offered.
        0
    }

    fn read_config(&self, offset: u64, data: &mut [u8]) {
        let mut space = [0u8; TAG_LEN + 4];
        let tag = self.config.tag.as_bytes();
        let len = tag.len().min(TAG_LEN);
        space[..len].copy_from_slice(&tag[..len]);
        space[TAG_LEN..].copy_from_slice(&NUM_REQUEST_QUEUES.to_le_bytes());
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
                "virtio-fs needs {QUEUE_COUNT} queues, got {}",
                ctx.queues.len()
            )));
        }
        let plane = Arc::new((
            Mutex::new(Plane {
                mem: ctx.mem,
                queues: ctx.queues,
                interrupt: ctx.interrupt,
                server: self.server.clone(),
                stop: false,
            }),
            Condvar::new(),
        ));
        let worker_plane = plane.clone();
        let worker = std::thread::Builder::new()
            .name("virtio-fs".into())
            .spawn(move || run_worker(worker_plane))
            .map_err(|err| Error::Hypervisor(format!("spawn virtio-fs worker: {err}")))?;
        self.plane = Some(plane);
        self.worker = Some(worker);
        Ok(())
    }

    fn notify(&mut self, queue_index: u16) {
        if let Some(plane) = &self.plane {
            if queue_index as usize == QUEUE_REQUEST || queue_index as usize == QUEUE_HIPRIO {
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

impl Drop for VirtioFs {
    fn drop(&mut self) {
        self.reset();
    }
}

/// All mutable state the worker touches, shared with the transport's `notify`.
struct Plane {
    mem: GuestMemoryMmap,
    queues: Vec<QueueSync>,
    interrupt: Interrupt,
    server: Arc<Server<Fs>>,
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
    /// Serve every pending request on every queue.
    fn tick(&mut self) {
        for index in 0..self.queues.len() {
            self.process(index);
        }
    }

    /// Drain one queue: run each FUSE request and report its reply length.
    fn process(&mut self, queue_index: usize) {
        let mem = self.mem.clone();
        let mut queue = self.queues[queue_index].clone();
        while let Some(chain) = queue.pop_descriptor_chain(&mem) {
            let head = chain.head_index();
            let used = self.serve(&mem, chain);
            let _ = queue.add_used(&mem, head, used);
            self.interrupt.signal_used();
        }
    }

    /// Dispatch one descriptor chain to the FUSE server, returning the number
    /// of reply bytes to publish in the used ring.
    fn serve(
        &self,
        mem: &GuestMemoryMmap,
        chain: virtio_queue::DescriptorChain<&GuestMemoryMmap>,
    ) -> u32 {
        let reader = match Reader::from_descriptor_chain(mem, chain.clone()) {
            Ok(reader) => reader,
            Err(_) => return 0,
        };
        let writer = match VirtioFsWriter::new(mem, chain) {
            Ok(writer) => writer,
            Err(_) => return 0,
        };
        match self
            .server
            .handle_message(reader, writer.into(), None, None)
        {
            Ok(len) => len as u32,
            Err(_) => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use virtio_queue::desc::split::Descriptor;
    use virtio_queue::desc::RawDescriptor;
    use virtio_queue::mock::MockSplitQueue;
    use vm_memory::{Bytes, GuestAddress};

    /// FUSE `InHeader` (40 bytes) followed by the `Init` body (16 bytes).
    const IN_HEADER_LEN: usize = 40;
    const INIT_IN_LEN: usize = 16;
    const OPCODE_INIT: u32 = 26;

    fn guest_memory() -> GuestMemoryMmap {
        GuestMemoryMmap::from_ranges(&[(GuestAddress(0), 0x10_0000)]).unwrap()
    }

    const IN_ADDR: u64 = 0x8_0000;
    const OUT_ADDR: u64 = 0x9_0000;

    /// A scratch directory to export as a share.
    fn share_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "container-native-virtiofs-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("hello"), b"world").unwrap();
        dir
    }

    #[test]
    fn config_space_reports_tag_and_queue_count() {
        let dir = share_dir();
        let device = VirtioFs::new(FsConfig {
            tag: "rootfs".into(),
            root_dir: dir.clone(),
        })
        .unwrap();
        let mut space = [0u8; TAG_LEN + 4];
        device.read_config(0, &mut space);
        assert_eq!(&space[..6], b"rootfs");
        assert_eq!(space[6], 0, "tag must be NUL terminated");
        assert_eq!(
            u32::from_le_bytes(space[TAG_LEN..].try_into().unwrap()),
            NUM_REQUEST_QUEUES
        );
        assert_eq!(device.device_id(), VIRTIO_ID_FS);
        assert_eq!(device.queue_count(), QUEUE_COUNT);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn init_request() -> Vec<u8> {
        let mut req = vec![0u8; IN_HEADER_LEN + INIT_IN_LEN];
        // InHeader: len counts the whole message; opcode INIT; unique 1. The
        // nodeid, uid, gid, pid and padding fields stay zero.
        let total = req.len() as u32;
        req[0..4].copy_from_slice(&total.to_le_bytes());
        req[4..8].copy_from_slice(&OPCODE_INIT.to_le_bytes());
        req[8..16].copy_from_slice(&1u64.to_le_bytes());
        // InitIn { major, minor, max_readahead, flags }.
        req[IN_HEADER_LEN..IN_HEADER_LEN + 4].copy_from_slice(&7u32.to_le_bytes());
        req[IN_HEADER_LEN + 4..IN_HEADER_LEN + 8].copy_from_slice(&38u32.to_le_bytes());
        req
    }

    fn queue_for_chain(mem: &GuestMemoryMmap) -> QueueSync {
        let mock = MockSplitQueue::new(mem, 8);
        // Readable request descriptor, then a writable reply descriptor.
        let readable = RawDescriptor::from(Descriptor::new(
            IN_ADDR,
            (IN_HEADER_LEN + INIT_IN_LEN) as u32,
            0,
            1,
        ));
        let writable = RawDescriptor::from(Descriptor::new(OUT_ADDR, 4096, 2, 1));
        mock.build_desc_chain(&[readable, writable]).unwrap();
        mock.create_queue::<QueueSync>().unwrap()
    }

    fn plane(mem: &GuestMemoryMmap, queues: Vec<QueueSync>, root: PathBuf) -> Plane {
        let passthrough = PassthroughFs::new(PassthroughConfig {
            root_dir: root.to_string_lossy().into_owned(),
            do_import: true,
            ..Default::default()
        })
        .unwrap();
        Plane {
            mem: mem.clone(),
            queues,
            interrupt: Interrupt::for_test(),
            server: Arc::new(Server::new(passthrough)),
            stop: false,
        }
    }

    #[test]
    fn an_init_request_gets_an_init_reply() {
        let dir = share_dir();
        let mem = guest_memory();
        mem.write_slice(&init_request(), GuestAddress(IN_ADDR))
            .unwrap();
        let queue = queue_for_chain(&mem);
        let mut plane = plane(
            &mem,
            vec![QueueSync::new(QUEUE_SIZE).unwrap(), queue],
            dir.clone(),
        );
        plane.tick();

        let mut out = vec![0u8; 16];
        mem.read_slice(&mut out, GuestAddress(OUT_ADDR)).unwrap();
        // OutHeader { len, error, unique } — a successful INIT reply.
        let len = u32::from_le_bytes(out[0..4].try_into().unwrap());
        let error = i32::from_le_bytes(out[4..8].try_into().unwrap());
        let unique = u64::from_le_bytes(out[8..16].try_into().unwrap());
        assert_eq!(error, 0, "INIT must succeed");
        assert_eq!(unique, 1, "reply must echo the request unique");
        assert!(len as usize > out.len(), "reply carries an InitOut body");
        let _ = std::fs::remove_dir_all(dir);
    }
}
