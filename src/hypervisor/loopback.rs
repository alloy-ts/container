//! In-process hypervisor used by unit tests.
//!
//! It does not boot a kernel. It runs the reference guest init on a socket
//! pair that speaks the same vsock RPC the production path will, so
//! `Container::{create, start, exec, ...}` can be tested without `/dev/kvm`.

use std::sync::atomic::{AtomicU32, Ordering};

use async_trait::async_trait;

use super::{AgentLink, BootedVm, Hypervisor, VmConfig};
use crate::error::Result;
use crate::guest::serve_reference;

pub(crate) static SHUTDOWNS: AtomicU32 = AtomicU32::new(0);

pub(crate) struct LoopbackHypervisor {
    paused: bool,
    cleaned: bool,
}

impl LoopbackHypervisor {
    pub(crate) fn new() -> Self {
        Self {
            paused: false,
            cleaned: false,
        }
    }
}

#[async_trait]
impl Hypervisor for LoopbackHypervisor {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        let _ = cfg;
        let (host, guest) = tokio::io::duplex(256 * 1024);
        tokio::spawn(async move {
            let _ = serve_reference(guest).await;
        });
        Ok(BootedVm {
            link: AgentLink::Connected(Box::new(host)),
            host_pid: Some(std::process::id()),
        })
    }

    fn pause(&mut self) -> Result<()> {
        self.paused = true;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        self.paused = false;
        Ok(())
    }

    fn shutdown_blocking(&mut self) {
        let _ = self.paused;
        if !self.cleaned {
            self.cleaned = true;
            SHUTDOWNS.fetch_add(1, Ordering::Relaxed);
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.shutdown_blocking();
        Ok(())
    }
}

impl Drop for LoopbackHypervisor {
    fn drop(&mut self) {
        self.shutdown_blocking();
    }
}
