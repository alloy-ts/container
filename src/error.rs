use thiserror::Error;

use crate::types::ContainerStatus;

pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the public container API and the private backends.
#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid container options: {0}")]
    InvalidOpts(String),

    #[error("invalid state for {op}: container is {from}")]
    InvalidState {
        from: ContainerStatus,
        op: &'static str,
    },

    #[error("guest agent: {0}")]
    Agent(String),

    #[error("hypervisor: {0}")]
    Hypervisor(String),

    #[error("resource: {0}")]
    Resource(String),

    #[error("oci spec: {0}")]
    Oci(String),

    #[error("protocol: {0}")]
    Protocol(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl From<oci_spec::OciSpecError> for Error {
    fn from(value: oci_spec::OciSpecError) -> Self {
        Error::Oci(value.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::Protocol(value.to_string())
    }
}
