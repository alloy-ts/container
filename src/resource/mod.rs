//! Host resources owned by one microVM: an address plan and virtio-fs shares.
//!
//! Actually creating the TAP device is the Linux hypervisor's job (it owns the
//! file descriptor). This module decides the addresses and which directories
//! the guest will see.

mod network;
mod rootfs;

pub(crate) use network::NetworkPlan;
pub(crate) use rootfs::{PreparedRootfs, VirtioFsShare};

use crate::error::Result;
use crate::types::ContainerOpts;

pub(crate) struct ResourceManager {
    pub network: Option<NetworkPlan>,
    pub rootfs: PreparedRootfs,
    pub shares: Vec<VirtioFsShare>,
}

impl ResourceManager {
    pub(crate) fn plan(opts: &ContainerOpts) -> Result<Self> {
        let network = network::plan(&opts.id, &opts.network)?;
        let (rootfs, mut shares) = rootfs::prepare(opts)?;
        if let Some(net) = &network {
            // The guest learns its address from the agent create payload's
            // environment as well as from the (future) virtio-net device.
            let _ = net;
        }
        shares.insert(
            0,
            VirtioFsShare {
                tag: "rootfs".into(),
                path: rootfs.path.clone(),
                readonly: rootfs.readonly,
            },
        );
        Ok(Self {
            network,
            rootfs,
            shares,
        })
    }

    pub(crate) fn guest_ip(&self) -> Option<std::net::IpAddr> {
        self.network.as_ref().map(|net| net.guest_ip)
    }

    /// Release host state recorded here. The TAP file descriptor itself is
    /// closed by the hypervisor that opened it.
    pub(crate) fn release(&mut self) {}
}
