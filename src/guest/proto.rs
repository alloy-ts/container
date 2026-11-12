//! Wire types and `postcard` framing for the control plane.
//!
//! Every message shares one multiplexing envelope, [`RpcFrame`], carrying the
//! `request_id` that pairs a reply with its request. Frames are serialized
//! with `postcard` and written to the stream behind a 4-byte big-endian length
//! prefix (`tokio_util::codec::LengthDelimitedCodec` — see
//! [`super::transport`]).
//!
//! High-throughput stdio deliberately has no place here: `create_container`
//! and `exec_process` only carry *port identifiers*, and the caller attaches
//! raw pipes through [`super::GuestAgent::open_data_stream`] instead.

use serde::{Deserialize, Serialize};

use super::error::AgentError;

/// Multiplexing envelope. `request_id` is echoed by the guest so the
/// background reader can route a reply to the right waiter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RpcFrame<T> {
    pub request_id: u64,
    pub payload: T,
}

/// Host → guest request. One variant per [`super::Agent`] method.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RpcRequest {
    HealthCheck,
    CreateContainer(CreateContainerRequest),
    StartContainer {
        container_id: String,
    },
    RemoveContainer {
        container_id: String,
        timeout_secs: u32,
    },
    PauseContainer {
        container_id: String,
    },
    ResumeContainer {
        container_id: String,
    },
    ExecProcess(ExecProcessRequest),
    SignalProcess {
        container_id: String,
        exec_id: String,
        signal: i32,
    },
    WaitProcess {
        container_id: String,
        exec_id: String,
    },
    CloseStdin {
        container_id: String,
        exec_id: String,
    },
    TtyWinResize {
        container_id: String,
        exec_id: String,
        rows: u16,
        cols: u16,
    },
    StatsContainer {
        container_id: String,
    },
    CopyFile(CopyFileRequest),
    ReseedRandom {
        entropy: Vec<u8>,
    },
    SetGuestTime {
        unix_secs: i64,
        unix_nanos: u32,
    },
    ShutdownGuest,
}

/// Guest → host reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RpcResponse {
    HealthStatus(HealthStatus),
    Started { pid: u32 },
    WaitResult { exit_code: i32 },
    Stats(ContainerStats),
    Ack,
    Error { message: String },
}

/// Liveness of the guest init.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub message: Option<String>,
}

impl HealthStatus {
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: None,
        }
    }
}

/// A bind mount forwarded to the guest init.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountSpec {
    pub source: String,
    pub destination: String,
    pub fs_type: String,
    pub options: Vec<String>,
}

/// Guest cgroup / process metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_usage_usec: u64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
    pub blkio_read_bytes: u64,
    pub blkio_write_bytes: u64,
    pub pids: u32,
}

/// Arguments for [`RpcRequest::CreateContainer`]. Stdio is referenced by port,
/// never carried inline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateContainerRequest {
    pub container_id: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: String,
    pub mounts: Vec<MountSpec>,
    pub tty: bool,
    pub stdin_port: Option<u32>,
    pub stdout_port: Option<u32>,
    pub stderr_port: Option<u32>,
    pub memory_limit_bytes: u64,
}

/// Arguments for [`RpcRequest::ExecProcess`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecProcessRequest {
    pub container_id: String,
    pub exec_id: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<String>,
    pub tty: bool,
    pub user: Option<String>,
    pub stdin_port: Option<u32>,
    pub stdout_port: Option<u32>,
    pub stderr_port: Option<u32>,
}

/// Arguments for [`RpcRequest::CopyFile`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyFileRequest {
    pub container_id: String,
    pub source: String,
    pub destination: String,
    pub mode: u32,
}

/// Serialize an outbound request envelope.
pub fn encode_request(frame: &RpcFrame<RpcRequest>) -> Result<Vec<u8>, AgentError> {
    postcard::to_allocvec(frame).map_err(|err| AgentError::Codec(err.to_string()))
}

/// Deserialize an inbound reply envelope.
pub fn decode_response(bytes: &[u8]) -> Result<RpcFrame<RpcResponse>, AgentError> {
    postcard::from_bytes(bytes).map_err(|err| AgentError::Codec(err.to_string()))
}

/// Deserialize an inbound request envelope (guest side / test servers).
#[allow(dead_code)] // consumed by the guest-side peer and the test mock server
pub fn decode_request(bytes: &[u8]) -> Result<RpcFrame<RpcRequest>, AgentError> {
    postcard::from_bytes(bytes).map_err(|err| AgentError::Codec(err.to_string()))
}

/// Serialize an outbound reply envelope (guest side / test servers).
#[allow(dead_code)] // consumed by the guest-side peer and the test mock server
pub fn encode_response(frame: &RpcFrame<RpcResponse>) -> Result<Vec<u8>, AgentError> {
    postcard::to_allocvec(frame).map_err(|err| AgentError::Codec(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrips_through_postcard() {
        let frame = RpcFrame {
            request_id: 42,
            payload: RpcRequest::ExecProcess(ExecProcessRequest {
                container_id: "ctr".into(),
                exec_id: "exec-1".into(),
                args: vec!["/bin/echo".into(), "hi".into()],
                env: vec![("FOO".into(), "bar".into())],
                cwd: Some("/".into()),
                tty: false,
                user: None,
                stdin_port: None,
                stdout_port: Some(9),
                stderr_port: Some(10),
            }),
        };
        let bytes = encode_request(&frame).unwrap();
        let back = decode_request(&bytes).unwrap();
        assert_eq!(back, frame);
    }

    #[test]
    fn response_roundtrips_through_postcard() {
        let frame = RpcFrame {
            request_id: 7,
            payload: RpcResponse::Stats(ContainerStats {
                memory_usage_bytes: 4096,
                pids: 3,
                ..ContainerStats::default()
            }),
        };
        let bytes = encode_response(&frame).unwrap();
        let back = decode_response(&bytes).unwrap();
        assert_eq!(back, frame);
    }
}
