use serde::{Deserialize, Serialize};

use crate::types::ContainerStats;

/// Mount forwarded inside a `create` / `exec` request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MountSpec {
    pub source: String,
    pub destination: String,
    pub fs_type: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NamespaceSpec {
    pub typ: String,
    pub path: Option<String>,
}

/// Host → guest RPC. Carried as JSON in a [`super::FrameKind::Request`] payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum Request {
    Create {
        args: Vec<String>,
        env: Vec<(String, String)>,
        cwd: String,
        mounts: Vec<MountSpec>,
        namespaces: Vec<NamespaceSpec>,
        tty: bool,
        stdin: bool,
        stdout: bool,
        stderr: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        oci_config_json: Option<String>,
        memory_limit_bytes: u64,
    },
    Start,
    Exec {
        args: Vec<String>,
        env: Vec<(String, String)>,
        cwd: Option<String>,
        tty: bool,
        stdin: bool,
        stdout: bool,
        stderr: bool,
        user: Option<String>,
    },
    Kill {
        pid: u32,
        signal: i32,
    },
    State,
    Stats,
    Resize {
        pid: u32,
        rows: u16,
        cols: u16,
    },
    Logs,
    Delete,
}

/// Guest → host RPC reply. Carried as JSON in a [`super::FrameKind::Response`] payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum Response {
    Ok,
    Started {
        pid: u32,
    },
    State {
        status: String,
        pid: Option<u32>,
    },
    Stats {
        cpu_usage_usec: u64,
        memory_usage_bytes: u64,
        memory_limit_bytes: u64,
        blkio_read_bytes: u64,
        blkio_write_bytes: u64,
        pids: u32,
    },
    Logs {
        stdout_b64: String,
        stderr_b64: String,
    },
    Error {
        message: String,
    },
}

impl Response {
    pub(crate) fn stats(stats: &ContainerStats) -> Self {
        Self::Stats {
            cpu_usage_usec: stats.cpu_usage_usec,
            memory_usage_bytes: stats.memory_usage_bytes,
            memory_limit_bytes: stats.memory_limit_bytes,
            blkio_read_bytes: stats.blkio_read_bytes,
            blkio_write_bytes: stats.blkio_write_bytes,
            pids: stats.pids,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_json_roundtrip() {
        let req = Request::Exec {
            args: vec!["/bin/echo".into(), "hi".into()],
            env: vec![("FOO".into(), "bar".into())],
            cwd: Some("/".into()),
            tty: false,
            stdin: false,
            stdout: true,
            stderr: true,
            user: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }
}
