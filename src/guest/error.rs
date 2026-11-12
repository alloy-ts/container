//! Typed errors for the host → guest control plane.

use thiserror::Error;

/// Everything a control-plane call can fail with.
///
/// The variants are deliberately transport-shaped: the guest never reports a
/// kernel or hypervisor failure directly, so a caller only needs to reason
/// about the RPC round trip.
#[derive(Debug, Error)]
pub enum AgentError {
    /// The request did not complete inside [`super::AgentConfig::request_timeout`].
    /// Produced by the `tower::timeout::Timeout` layer.
    #[error("request to the guest agent timed out")]
    Timeout,

    /// The control stream was lost (guest reset, VM shutdown, or a dropped
    /// test duplex). Every in-flight request fails with this immediately.
    #[error("guest agent connection closed")]
    Disconnected,

    /// The peer sent bytes that are not a valid `postcard` frame, or a request
    /// could not be encoded.
    #[error("frame codec error: {0}")]
    Codec(String),

    /// The guest answered a request with an application-level error.
    #[error("guest agent returned an error: {0}")]
    RemoteError(String),

    /// A call was attempted before [`super::GuestAgent::connect`] succeeded.
    #[error("guest agent is not connected")]
    NotConnected,

    /// Socket / pipe level failure (dialing the guest, or a broken stream).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl AgentError {
    /// Build the protocol error used when a reply does not match the request
    /// that produced it.
    pub(crate) fn unexpected(reply: &str, method: &str) -> Self {
        AgentError::RemoteError(format!("unexpected `{reply}` reply to `{method}`"))
    }
}
