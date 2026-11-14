use std::path::PathBuf;

use crate::error::{Error, Result};
use crate::types::{ContainerOpts, RootfsSource};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedRootfs {
    pub path: PathBuf,
    pub readonly: bool,
    pub source: RootfsSource,
}

/// A directory or image the hypervisor should expose to the guest.
///
/// TODO(linux): the virtio-fs (or virtio-blk, for erofs / block) device model
/// that consumes these shares is not implemented. The share list is still
/// validated and passed into the VM config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VirtioFsShare {
    pub tag: String,
    pub path: PathBuf,
    pub readonly: bool,
}

pub(crate) fn prepare(opts: &ContainerOpts) -> Result<(PreparedRootfs, Vec<VirtioFsShare>)> {
    let readonly = opts
        .oci_spec
        .as_ref()
        .and_then(|spec| spec.root().as_ref())
        .and_then(|root| root.readonly())
        .unwrap_or(false);
    let path = resolve(&opts.rootfs)?;
    if !path.exists() {
        return Err(Error::Resource(format!(
            "rootfs path does not exist: {}",
            path.display()
        )));
    }
    let rootfs = PreparedRootfs {
        path: path.clone(),
        readonly,
        source: opts.rootfs.clone(),
    };
    let mut shares = Vec::new();
    for (index, mount) in opts.mounts.iter().enumerate() {
        if mount.source.as_os_str().is_empty() || !mount.source.exists() {
            continue;
        }
        shares.push(VirtioFsShare {
            tag: format!("mnt{index}"),
            path: mount.source.clone(),
            readonly: mount.options.iter().any(|opt| opt == "ro"),
        });
    }
    let _ = path;
    Ok((rootfs, shares))
}

fn resolve(source: &RootfsSource) -> Result<PathBuf> {
    match source {
        RootfsSource::Directory(path)
        | RootfsSource::Erofs(path)
        | RootfsSource::BlockDevice(path) => Ok(path.clone()),
        RootfsSource::OciBundle(bundle) => {
            let nested = bundle.join("rootfs");
            if nested.is_dir() {
                Ok(nested)
            } else {
                Ok(bundle.clone())
            }
        }
    }
}
