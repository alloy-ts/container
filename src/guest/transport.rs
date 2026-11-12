//! Cross-platform transport and the multiplexing `tower::Service` client.
//!
//! The control connection is a single byte stream. To let many host callers use
//! it at once without head-of-line blocking, [`MultiplexedTransport`] splits the
//! stream, spawns a reader and a writer task, and pairs each request with a
//! `oneshot` reply channel keyed by `request_id`.
//!
//! Nothing here assumes Unix: the underlying stream is any
//! [`TransportStream`] — `AF_VSOCK` on Linux, a Unix domain socket mux on macOS,
//! a Named Pipe / `AF_HYPERV` on Windows, or a `tokio::io::duplex` in tests.
//! There is no `RawFd` and no `unsafe impl Send`.

use std::collections::HashMap;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use async_trait::async_trait;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tower::Service;

use super::error::AgentError;
use super::proto::{self, RpcFrame, RpcRequest, RpcResponse};

/// A cross-platform, bidirectional byte stream.
///
/// This is a single supertrait over `AsyncRead + AsyncWrite + Unpin + Send` so
/// that the erased handle [`BoxedStream`] is a legal trait object: a trait
/// object admits only one non-auto trait, so `Box<dyn AsyncRead + AsyncWrite>`
/// does not compile. Blanket-implemented for every qualifying stream, which
/// keeps `GuestAgent` free of `RawFd` and `unsafe impl Send`.
pub trait TransportStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> TransportStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

/// Erased, owned handle to a guest stream — the control connection or a data
/// pipe returned by `open_data_stream`.
pub type BoxedStream = Box<dyn TransportStream>;

/// Opens streams to the guest. One implementation per host backend; the
/// control plane never names a socket type directly.
///
/// An `Arc<dyn StreamConnector>` is injected into [`super::GuestAgent`], so the
/// same agent works over `AF_VSOCK`, a Unix domain socket, a Named Pipe, or an
/// in-memory `tokio::io::duplex` in unit tests.
#[async_trait]
pub trait StreamConnector: Send + Sync {
    /// Open a stream to the guest listener bound to `port`.
    async fn connect(&self, port: u32) -> io::Result<BoxedStream>;
}

/// Shared state between the service handle and its background tasks.
struct Shared {
    next_id: AtomicU64,
    pending: Mutex<HashMap<u64, oneshot::Sender<RpcResponse>>>,
    outbound: mpsc::UnboundedSender<Bytes>,
}

impl Shared {
    fn complete(&self, request_id: u64, response: RpcResponse) {
        let sender = self
            .pending
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .remove(&request_id);
        if let Some(sender) = sender {
            let _ = sender.send(response);
        }
    }

    /// Drop every waiter. Callers see `RecvError`, which the service maps to
    /// [`AgentError::Disconnected`].
    fn fail_all(&self) {
        let mut pending = self.pending.lock().unwrap_or_else(|err| err.into_inner());
        for (_, sender) in pending.drain() {
            drop(sender);
        }
    }
}

/// Multiplexing RPC client. Cloning shares the same connection; the background
/// reader/writer tasks live until the last clone is dropped or [`shutdown`] is
/// called.
///
/// [`shutdown`]: MultiplexedTransport::shutdown
#[derive(Clone)]
pub struct MultiplexedTransport {
    shared: Arc<Shared>,
    tasks: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl MultiplexedTransport {
    /// Split `stream` and start the background I/O loop.
    pub fn new<S>(stream: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (reader_half, writer_half) = tokio::io::split(stream);
        let (outbound, inbound) = mpsc::unbounded_channel::<Bytes>();
        let shared = Arc::new(Shared {
            next_id: AtomicU64::new(1),
            pending: Mutex::new(HashMap::new()),
            outbound,
        });

        let reader_shared = Arc::clone(&shared);
        let reader = tokio::spawn(async move {
            let mut framed = FramedRead::new(reader_half, LengthDelimitedCodec::new());
            while let Some(frame) = framed.next().await {
                match frame {
                    Ok(bytes) => match proto::decode_response(&bytes[..]) {
                        Ok(frame) => reader_shared.complete(frame.request_id, frame.payload),
                        Err(err) => {
                            tracing::warn!(error = %err, "dropping undecodable guest reply frame");
                        }
                    },
                    Err(err) => {
                        tracing::debug!(error = %err, "guest control stream read error");
                        break;
                    }
                }
            }
            // EOF or read error: nothing more will ever complete.
            reader_shared.fail_all();
        });

        let writer_shared = Arc::clone(&shared);
        let writer = tokio::spawn(async move {
            let mut inbound = inbound;
            let mut framed = FramedWrite::new(writer_half, LengthDelimitedCodec::new());
            while let Some(bytes) = inbound.recv().await {
                if let Err(err) = framed.send(bytes).await {
                    tracing::debug!(error = %err, "guest control stream write error");
                    break;
                }
            }
            writer_shared.fail_all();
        });

        Self {
            shared,
            tasks: Arc::new(Mutex::new(vec![reader, writer])),
        }
    }

    /// Fail all pending calls and abort the reader/writer tasks.
    pub fn shutdown(&self) {
        self.shared.fail_all();
        if let Ok(mut tasks) = self.tasks.lock() {
            for task in tasks.drain(..) {
                task.abort();
            }
        }
    }
}

impl Service<RpcRequest> for MultiplexedTransport {
    type Response = RpcResponse;
    type Error = AgentError;
    type Future = Pin<Box<dyn Future<Output = Result<RpcResponse, AgentError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: RpcRequest) -> Self::Future {
        let shared = Arc::clone(&self.shared);
        Box::pin(async move {
            let request_id = shared.next_id.fetch_add(1, Ordering::Relaxed);
            let (tx, rx) = oneshot::channel();
            shared
                .pending
                .lock()
                .unwrap_or_else(|err| err.into_inner())
                .insert(request_id, tx);

            let frame = RpcFrame {
                request_id,
                payload: request,
            };
            let bytes = match proto::encode_request(&frame) {
                Ok(bytes) => Bytes::from(bytes),
                Err(err) => {
                    shared
                        .pending
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&request_id);
                    return Err(err);
                }
            };

            if shared.outbound.send(bytes).is_err() {
                shared
                    .pending
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&request_id);
                return Err(AgentError::Disconnected);
            }

            match rx.await {
                Ok(response) => Ok(response),
                Err(_) => Err(AgentError::Disconnected),
            }
        })
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;

    /// Spawn a mock guest over one end of a duplex stream.
    ///
    /// `handler` maps each request to a `(delay, reply)`. Each reply is written
    /// from its own task, so replies can land out of order — exactly what the
    /// multiplexer must survive.
    pub(crate) fn spawn_mock_server<S, F>(stream: S, handler: F) -> JoinHandle<()>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        F: Fn(&RpcRequest) -> (Duration, RpcResponse) + Send + Sync + 'static,
    {
        tokio::spawn(async move {
            let (reader_half, writer_half) = tokio::io::split(stream);
            let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Bytes>();
            let writer = tokio::spawn(async move {
                let mut framed = FramedWrite::new(writer_half, LengthDelimitedCodec::new());
                while let Some(bytes) = out_rx.recv().await {
                    if framed.send(bytes).await.is_err() {
                        break;
                    }
                }
            });

            let mut framed = FramedRead::new(reader_half, LengthDelimitedCodec::new());
            while let Some(item) = framed.next().await {
                let Ok(bytes) = item else { break };
                let Ok(frame) = proto::decode_request(&bytes[..]) else {
                    break;
                };
                let (delay, response) = handler(&frame.payload);
                let out = out_tx.clone();
                let request_id = frame.request_id;
                tokio::spawn(async move {
                    if !delay.is_zero() {
                        tokio::time::sleep(delay).await;
                    }
                    let reply = RpcFrame {
                        request_id,
                        payload: response,
                    };
                    if let Ok(bytes) = proto::encode_response(&reply) {
                        let _ = out.send(Bytes::from(bytes));
                    }
                });
            }
            drop(out_tx);
            let _ = writer.await;
        })
    }

    /// A connector that hands out a single pre-built stream for one port.
    /// Any other port (e.g. the log port) is refused, so optional subsystems
    /// fail fast instead of stealing the control stream.
    pub(crate) struct OneShotConnector {
        port: u32,
        stream: StdMutex<Option<BoxedStream>>,
    }

    impl OneShotConnector {
        pub(crate) fn new<S>(port: u32, stream: S) -> Self
        where
            S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        {
            let boxed: BoxedStream = Box::new(stream);
            Self {
                port,
                stream: StdMutex::new(Some(boxed)),
            }
        }
    }

    #[async_trait]
    impl StreamConnector for OneShotConnector {
        async fn connect(&self, port: u32) -> io::Result<BoxedStream> {
            if port != self.port {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    format!("no test listener on port {port}"),
                ));
            }
            self.stream
                .lock()
                .unwrap()
                .take()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "stream already taken"))
        }
    }

    /// Small deterministic FNV-1a hash, used to tag replies with request data.
    pub(crate) fn tag(input: &str) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in input.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// Deterministic pseudo-random delay in `1..=25ms`.
    pub(crate) fn jitter(seed: u64) -> Duration {
        Duration::from_millis((seed.wrapping_mul(2_654_435_761) % 25) + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{jitter, spawn_mock_server, tag, OneShotConnector};
    use super::*;
    use crate::guest::proto::{ContainerStats, ExecProcessRequest};
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_multiplexing_routes_each_reply_to_its_caller() {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server_task = spawn_mock_server(server, |request| match request {
            RpcRequest::ExecProcess(req) => (
                jitter(tag(&req.exec_id)),
                RpcResponse::Started {
                    pid: (tag(&req.exec_id) % 100_000) as u32,
                },
            ),
            RpcRequest::StatsContainer { container_id } => (
                jitter(tag(container_id)),
                RpcResponse::Stats(ContainerStats {
                    memory_usage_bytes: tag(container_id),
                    ..ContainerStats::default()
                }),
            ),
            _ => (Duration::from_millis(1), RpcResponse::Ack),
        });
        let transport = MultiplexedTransport::new(client);

        // 20 interleaved calls share one connection. Replies come back in a
        // jumbled order; each caller must still get its own answer.
        let mut handles = Vec::new();
        for i in 0..20u64 {
            let mut transport = transport.clone();
            handles.push(tokio::spawn(async move {
                if i % 2 == 0 {
                    let exec_id = format!("exec-{i}");
                    let request = RpcRequest::ExecProcess(ExecProcessRequest {
                        container_id: "ctr".into(),
                        exec_id: exec_id.clone(),
                        args: vec!["/bin/true".into()],
                        env: Vec::new(),
                        cwd: None,
                        tty: false,
                        user: None,
                        stdin_port: None,
                        stdout_port: None,
                        stderr_port: None,
                    });
                    match transport.call(request).await.unwrap() {
                        RpcResponse::Started { pid } => {
                            assert_eq!(pid, (tag(&exec_id) % 100_000) as u32)
                        }
                        other => panic!("caller {i} got a mismatched reply: {other:?}"),
                    }
                } else {
                    let container_id = format!("ctr-{i}");
                    let request = RpcRequest::StatsContainer {
                        container_id: container_id.clone(),
                    };
                    match transport.call(request).await.unwrap() {
                        RpcResponse::Stats(stats) => {
                            assert_eq!(stats.memory_usage_bytes, tag(&container_id))
                        }
                        other => panic!("caller {i} got a mismatched reply: {other:?}"),
                    }
                }
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        transport.shutdown();
        let _ = server_task.await;
    }

    #[tokio::test]
    async fn dropping_the_stream_fails_in_flight_calls_with_disconnected() {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let transport = MultiplexedTransport::new(client);
        let mut caller = transport.clone();
        let call = tokio::spawn(async move {
            caller
                .call(RpcRequest::StatsContainer {
                    container_id: "ctr".into(),
                })
                .await
        });

        // Let the request reach the (never-read) server half, then kill it.
        tokio::time::sleep(Duration::from_millis(40)).await;
        drop(server);

        let result = tokio::time::timeout(Duration::from_secs(2), call)
            .await
            .expect("call should not hang after the stream is dropped")
            .unwrap();
        assert!(
            matches!(result, Err(AgentError::Disconnected)),
            "expected Disconnected, got {result:?}"
        );
        transport.shutdown();
    }

    #[tokio::test]
    async fn connector_refuses_unexpected_ports() {
        let (_client, _server) = tokio::io::duplex(64);
        let connector = OneShotConnector::new(1024, tokio::io::duplex(64).0);
        assert!(connector.connect(1025).await.is_err());
        assert!(connector.connect(1024).await.is_ok());
        // Only one stream is ever handed out.
        assert!(connector.connect(1024).await.is_err());
    }
}
