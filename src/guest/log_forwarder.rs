//! Background task streaming guest kernel/agent logs over `config.log_port`.
//!
//! Logs are a diagnostic side channel, deliberately kept off the control
//! connection (see [`super::proto`]). The task dials the log port through the
//! same [`StreamConnector`] the control plane uses, reads raw bytes, and emits
//! newline-delimited records through `tracing`.
//!
//! Dial or read failures are non-fatal: a guest without a log listener must
//! never stop the control plane from connecting.

use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::task::JoinHandle;

use super::config::AgentConfig;
use super::transport::StreamConnector;

/// Owns the log-streaming task. Abort it with [`LogForwarder::abort`].
pub(crate) struct LogForwarder {
    task: JoinHandle<()>,
}

impl LogForwarder {
    /// Dial `config.log_port` and forward every chunk to `tracing`.
    pub(crate) fn spawn(connector: Arc<dyn StreamConnector>, config: AgentConfig) -> Self {
        let task = tokio::spawn(async move {
            let mut stream = match connector.connect(config.log_port).await {
                Ok(stream) => stream,
                Err(err) => {
                    tracing::debug!(
                        error = %err,
                        port = config.log_port,
                        "guest log stream unavailable"
                    );
                    return;
                }
            };

            let mut buf = vec![0u8; 8192];
            let mut pending = Vec::new();
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) => {
                        if !pending.is_empty() {
                            emit(&pending);
                        }
                        return;
                    }
                    Ok(n) => {
                        pending.extend_from_slice(&buf[..n]);
                        // Emit whole lines; keep the trailing fragment buffered.
                        while let Some(pos) = pending.iter().position(|byte| *byte == b'\n') {
                            let line: Vec<u8> = pending.drain(..=pos).collect();
                            emit(&line[..line.len() - 1]);
                        }
                    }
                    Err(err) => {
                        tracing::debug!(error = %err, "guest log stream ended");
                        return;
                    }
                }
            }
        });
        Self { task }
    }

    /// Stop streaming logs.
    pub(crate) fn abort(&self) {
        self.task.abort();
    }
}

fn emit(line: &[u8]) {
    if line.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(line);
    tracing::info!(target: "guest_agent::log", "{text}");
}
