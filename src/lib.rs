//! In-process OCI microVM runtime.
//!
//! One [`Container`] is one microVM. Callers configure it with [`ContainerOpts`]
//! (or [`ContainerOpts::from_oci_bundle`]) and drive the OCI lifecycle
//! (`create`, `start`, `state`, `kill`, `delete`) plus the Apple-container-style
//! operations (`exec`, `wait`, `logs`, `ip_addr`, `stats`, `pause` / `resume`).
//! [`Process`] is the handle returned by `start` and `exec`.
//!
//! The hypervisor, virtio resources, and vsock framing stay private behind
//! [`Container`]. The cross-platform [`guest`] control plane is exposed for
//! hosts that drive a guest init directly.

mod container;
mod error;
pub mod guest;
mod hypervisor;
mod proto;
mod resource;
mod types;

pub use container::{Container, Process};
pub use error::{Error, Result};
pub use types::{
    ContainerOpts, ContainerOptsBuilder, ContainerState, ContainerStats, ContainerStatus, ExecOpts,
    ExitStatus, LogStream, Mount, NetworkConfig, NetworkMode, RootfsSource, Signal,
};
