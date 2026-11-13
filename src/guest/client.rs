//! The `tower`-backed [`GuestAgent`] and the [`Agent`] control-plane trait.
//!
//! A [`GuestAgent`] owns exactly one control connection. [`GuestAgent::connect`]
//! dials through an injected [`StreamConnector`], wraps the raw
//! [`MultiplexedTransport`] in a `tower` stack
//! (`Buffer` → `ConcurrencyLimit` → `Timeout`) and stores the resulting
//! [`BoxCloneService`]s. Because the stack is buffered, every [`Agent`] method
//! takes `&self` and is safe to call concurrently from many host tasks — the
//! underlying multiplexer pairs each reply with its caller via `request_id`.
//!
//! Nothing here is Unix-specific: there is no `RawFd` and no `unsafe impl Send`.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tower::buffer::BufferLayer;
use tower::limit::ConcurrencyLimitLayer;
use tower::timeout::error::Elapsed;
use tower::timeout::TimeoutLayer;
use tower::util::{BoxCloneService, MapErr};
use tower::{BoxError, ServiceBuilder, ServiceExt};

use super::config::AgentConfig;
use super::error::AgentError;
use super::log_forwarder::LogForwarder;
use super::proto::{
    ContainerStats, CopyFileRequest, CreateContainerRequest, ExecProcessRequest, HealthStatus,
    RpcRequest, RpcResponse,
};
use super::transport::{BoxedStream, MultiplexedTransport, StreamConnector};

/// The buffered, cloneable RPC handle stored in [`GuestAgentInner`].
type AgentService = BoxCloneService<RpcRequest, RpcResponse, AgentError>;

/// The control-plane surface of a single-container guest.
///
/// One method per [`RpcRequest`] variant. All calls are multiplexed over a
/// single connection, so a `GuestAgent` is cheap to share across tasks.
#[async_trait]
pub trait Agent: Send + Sync {
    // Lifecycle & health.
    async fn health_check(&self) -> Result<HealthStatus, AgentError>;
    async fn create_container(&self, req: CreateContainerRequest) -> Result<(), AgentError>;
    async fn start_container(&self, container_id: &str) -> Result<(), AgentError>;
    async fn remove_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), AgentError>;
    async fn pause_container(&self, container_id: &str) -> Result<(), AgentError>;
    async fn resume_container(&self, container_id: &str) -> Result<(), AgentError>;

    // Process control.
    async fn exec_process(&self, req: ExecProcessRequest) -> Result<u32, AgentError>;
    async fn signal_process(
        &self,
        container_id: &str,
        exec_id: &str,
        signal: i32,
    ) -> Result<(), AgentError>;
    async fn wait_process(&self, container_id: &str, exec_id: &str) -> Result<i32, AgentError>;
    async fn close_stdin(&self, container_id: &str, exec_id: &str) -> Result<(), AgentError>;
    async fn tty_win_resize(
        &self,
        container_id: &str,
        exec_id: &str,
        rows: u16,
        cols: u16,
    ) -> Result<(), AgentError>;

    // Telemetry & guest utilities.
    async fn stats_container(&self, container_id: &str) -> Result<ContainerStats, AgentError>;
    async fn copy_file(&self, req: CopyFileRequest) -> Result<(), AgentError>;
    async fn reseed_random(&self, entropy: Vec<u8>) -> Result<(), AgentError>;
    async fn set_guest_time(&self, unix_secs: i64, unix_nanos: u32) -> Result<(), AgentError>;
    async fn shutdown_guest(&self) -> Result<(), AgentError>;
}

/// One live connection plus the services and tasks it owns.
struct Connected {
    transport: MultiplexedTransport,
    /// Timed handle: every call except `wait_process` goes through this.
    request: AgentService,
    /// Untimed handle: long-lived blocking RPCs (`wait_process`).
    wait: AgentService,
    log_task: LogForwarder,
}

struct GuestAgentInner {
    config: AgentConfig,
    connector: Arc<dyn StreamConnector>,
    connected: Mutex<Option<Connected>>,
}

/// Cross-platform host client for the guest init.
///
/// Construct with [`GuestAgent::new`], dial with [`GuestAgent::connect`], then
/// drive the container through the [`Agent`] trait. Cloning shares one
/// connection.
#[derive(Clone)]
pub struct GuestAgent {
    inner: Arc<GuestAgentInner>,
}

impl GuestAgent {
    /// Build an agent. No I/O happens until [`connect`](GuestAgent::connect).
    pub fn new(config: AgentConfig, connector: Arc<dyn StreamConnector>) -> Self {
        Self {
            inner: Arc::new(GuestAgentInner {
                config,
                connector,
                connected: Mutex::new(None),
            }),
        }
    }

    /// Dial the guest, retrying with exponential backoff until
    /// [`AgentConfig::dial_timeout`] elapses.
    ///
    /// The guest init is not up the instant the VM boots, so this budget is a
    /// retry window rather than a single dial timeout.
    pub async fn connect(&self) -> Result<(), AgentError> {
        // Drop any previous connection before dialing a fresh one.
        let _ = self.disconnect();

        let deadline = Instant::now() + self.inner.config.dial_timeout;
        let mut backoff = Duration::from_millis(20);
        loop {
            match self
                .inner
                .connector
                .connect(self.inner.config.server_port)
                .await
            {
                Ok(stream) => {
                    let connected =
                        build_connected(stream, &self.inner.config, &self.inner.connector);
                    *lock(&self.inner.connected) = Some(connected);
                    return Ok(());
                }
                Err(err) => {
                    if Instant::now() >= deadline {
                        return Err(AgentError::Io(err));
                    }
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_millis(500));
                }
            }
        }
    }

    /// Abort the background multiplexer and log tasks, dropping the connection.
    ///
    /// Idempotent: calling it while already disconnected is a no-op.
    pub fn disconnect(&self) -> Result<(), AgentError> {
        if let Some(connected) = lock(&self.inner.connected).take() {
            connected.log_task.abort();
            connected.transport.shutdown();
        }
        Ok(())
    }

    /// Open a raw data stream (stdio, log follow, ...) on `port`.
    ///
    /// High-throughput stdio never travels over the control RPC channel; the
    /// caller attaches these raw pipes to the process directly.
    pub async fn open_data_stream(&self, port: u32) -> Result<BoxedStream, AgentError> {
        if lock(&self.inner.connected).is_none() {
            return Err(AgentError::NotConnected);
        }
        self.inner
            .connector
            .connect(port)
            .await
            .map_err(AgentError::Io)
    }

    fn timed_service(&self) -> Result<AgentService, AgentError> {
        lock(&self.inner.connected)
            .as_ref()
            .map(|connected| connected.request.clone())
            .ok_or(AgentError::NotConnected)
    }

    fn wait_service(&self) -> Result<AgentService, AgentError> {
        lock(&self.inner.connected)
            .as_ref()
            .map(|connected| connected.wait.clone())
            .ok_or(AgentError::NotConnected)
    }

    /// One timed round trip through the `tower` stack.
    async fn rpc(&self, request: RpcRequest) -> Result<RpcResponse, AgentError> {
        self.timed_service()?.oneshot(request).await
    }

    /// One round trip that bypasses the request timeout (`wait_process`).
    async fn rpc_no_timeout(&self, request: RpcRequest) -> Result<RpcResponse, AgentError> {
        self.wait_service()?.oneshot(request).await
    }
}

/// Build the connection state: multiplexer, `tower` stacks, log task.
fn build_connected(
    stream: BoxedStream,
    config: &AgentConfig,
    connector: &Arc<dyn StreamConnector>,
) -> Connected {
    let transport = MultiplexedTransport::new(stream);
    let bound = config.max_in_flight.max(1);

    // Buffer makes the handle cheaply clonable for concurrent callers and owns
    // the worker task. Both the timed and untimed stacks share this one buffer.
    //
    // Note tower 0.5's `Buffer` reports `BoxError` (the worker may fail for
    // reasons the inner service cannot express), so both stacks map that back
    // to a typed `AgentError`.
    let buffered = ServiceBuilder::new()
        .layer(BufferLayer::new(bound))
        .service(transport.clone());

    let timed = ServiceBuilder::new()
        .layer(TimeoutLayer::new(config.request_timeout))
        .layer(ConcurrencyLimitLayer::new(bound))
        .service(buffered.clone());

    let request: AgentService = MapErr::new(timed, map_box_error).boxed_clone();
    let wait: AgentService = MapErr::new(buffered, map_box_error).boxed_clone();

    let log_task = LogForwarder::spawn(Arc::clone(connector), config.clone());

    Connected {
        transport,
        request,
        wait,
        log_task,
    }
}

/// Turn the `tower` timeout layer's `BoxError` back into a typed [`AgentError`].
fn map_box_error(err: BoxError) -> AgentError {
    if err.is::<Elapsed>() {
        return AgentError::Timeout;
    }
    match err.downcast::<AgentError>() {
        Ok(agent_error) => *agent_error,
        Err(other) => AgentError::Codec(other.to_string()),
    }
}

/// Lock helper that survives a poisoned mutex (a panic must not wedge the
/// control plane).
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|err| err.into_inner())
}

/// Surface an application-level guest error as [`AgentError::RemoteError`],
/// passing every other reply through unchanged.
fn into_ok(response: RpcResponse) -> Result<RpcResponse, AgentError> {
    match response {
        RpcResponse::Error { message } => Err(AgentError::RemoteError(message)),
        other => Ok(other),
    }
}

fn reply_name(response: &RpcResponse) -> &'static str {
    match response {
        RpcResponse::HealthStatus(_) => "HealthStatus",
        RpcResponse::Started { .. } => "Started",
        RpcResponse::WaitResult { .. } => "WaitResult",
        RpcResponse::Stats(_) => "Stats",
        RpcResponse::Ack => "Ack",
        RpcResponse::Error { .. } => "Error",
    }
}

fn expect_ack(response: RpcResponse, method: &str) -> Result<(), AgentError> {
    match response {
        RpcResponse::Ack => Ok(()),
        other => Err(AgentError::unexpected(reply_name(&other), method)),
    }
}

#[async_trait]
impl Agent for GuestAgent {
    async fn health_check(&self) -> Result<HealthStatus, AgentError> {
        match into_ok(self.rpc(RpcRequest::HealthCheck).await?)? {
            RpcResponse::HealthStatus(status) => Ok(status),
            other => Err(AgentError::unexpected(reply_name(&other), "health_check")),
        }
    }

    async fn create_container(&self, req: CreateContainerRequest) -> Result<(), AgentError> {
        expect_ack(
            into_ok(self.rpc(RpcRequest::CreateContainer(req)).await?)?,
            "create_container",
        )
    }

    async fn start_container(&self, container_id: &str) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::StartContainer {
                    container_id: container_id.to_owned(),
                })
                .await?,
            )?,
            "start_container",
        )
    }

    async fn remove_container(
        &self,
        container_id: &str,
        timeout_secs: u32,
    ) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::RemoveContainer {
                    container_id: container_id.to_owned(),
                    timeout_secs,
                })
                .await?,
            )?,
            "remove_container",
        )
    }

    async fn pause_container(&self, container_id: &str) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::PauseContainer {
                    container_id: container_id.to_owned(),
                })
                .await?,
            )?,
            "pause_container",
        )
    }

    async fn resume_container(&self, container_id: &str) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::ResumeContainer {
                    container_id: container_id.to_owned(),
                })
                .await?,
            )?,
            "resume_container",
        )
    }

    async fn exec_process(&self, req: ExecProcessRequest) -> Result<u32, AgentError> {
        match into_ok(self.rpc(RpcRequest::ExecProcess(req)).await?)? {
            RpcResponse::Started { pid } => Ok(pid),
            other => Err(AgentError::unexpected(reply_name(&other), "exec_process")),
        }
    }

    async fn signal_process(
        &self,
        container_id: &str,
        exec_id: &str,
        signal: i32,
    ) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::SignalProcess {
                    container_id: container_id.to_owned(),
                    exec_id: exec_id.to_owned(),
                    signal,
                })
                .await?,
            )?,
            "signal_process",
        )
    }

    async fn wait_process(&self, container_id: &str, exec_id: &str) -> Result<i32, AgentError> {
        let request = RpcRequest::WaitProcess {
            container_id: container_id.to_owned(),
            exec_id: exec_id.to_owned(),
        };
        match into_ok(self.rpc_no_timeout(request).await?)? {
            RpcResponse::WaitResult { exit_code } => Ok(exit_code),
            other => Err(AgentError::unexpected(reply_name(&other), "wait_process")),
        }
    }

    async fn close_stdin(&self, container_id: &str, exec_id: &str) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::CloseStdin {
                    container_id: container_id.to_owned(),
                    exec_id: exec_id.to_owned(),
                })
                .await?,
            )?,
            "close_stdin",
        )
    }

    async fn tty_win_resize(
        &self,
        container_id: &str,
        exec_id: &str,
        rows: u16,
        cols: u16,
    ) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::TtyWinResize {
                    container_id: container_id.to_owned(),
                    exec_id: exec_id.to_owned(),
                    rows,
                    cols,
                })
                .await?,
            )?,
            "tty_win_resize",
        )
    }

    async fn stats_container(&self, container_id: &str) -> Result<ContainerStats, AgentError> {
        match into_ok(
            self.rpc(RpcRequest::StatsContainer {
                container_id: container_id.to_owned(),
            })
            .await?,
        )? {
            RpcResponse::Stats(stats) => Ok(stats),
            other => Err(AgentError::unexpected(
                reply_name(&other),
                "stats_container",
            )),
        }
    }

    async fn copy_file(&self, req: CopyFileRequest) -> Result<(), AgentError> {
        expect_ack(
            into_ok(self.rpc(RpcRequest::CopyFile(req)).await?)?,
            "copy_file",
        )
    }

    async fn reseed_random(&self, entropy: Vec<u8>) -> Result<(), AgentError> {
        expect_ack(
            into_ok(self.rpc(RpcRequest::ReseedRandom { entropy }).await?)?,
            "reseed_random",
        )
    }

    async fn set_guest_time(&self, unix_secs: i64, unix_nanos: u32) -> Result<(), AgentError> {
        expect_ack(
            into_ok(
                self.rpc(RpcRequest::SetGuestTime {
                    unix_secs,
                    unix_nanos,
                })
                .await?,
            )?,
            "set_guest_time",
        )
    }

    async fn shutdown_guest(&self) -> Result<(), AgentError> {
        expect_ack(
            into_ok(self.rpc(RpcRequest::ShutdownGuest).await?)?,
            "shutdown_guest",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::transport::test_support::{jitter, spawn_mock_server, tag, OneShotConnector};
    use super::*;
    use tokio::task::JoinHandle;

    /// Dial a mock guest bound to `server_port` and return the live agent.
    async fn connected_agent<F, T>(handler: F, tune: T) -> (GuestAgent, JoinHandle<()>)
    where
        F: Fn(&RpcRequest) -> (Duration, RpcResponse) + Send + Sync + 'static,
        T: FnOnce(AgentConfig) -> AgentConfig,
    {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server_task = spawn_mock_server(server, handler);
        let connector = Arc::new(OneShotConnector::new(1024, client));
        let agent = GuestAgent::new(tune(AgentConfig::new(1024)), connector);
        agent
            .connect()
            .await
            .expect("mock guest should accept the dial");
        (agent, server_task)
    }

    fn exec_request(exec_id: &str) -> ExecProcessRequest {
        ExecProcessRequest {
            container_id: "ctr".into(),
            exec_id: exec_id.into(),
            args: vec!["/bin/true".into()],
            env: Vec::new(),
            cwd: None,
            tty: false,
            user: None,
            stdin_port: None,
            stdout_port: None,
            stderr_port: None,
        }
    }

    /// 20 concurrent calls over one connection; replies arrive out of order but
    /// each caller must receive its own answer.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_calls_are_multiplexed_and_matched() {
        let (agent, server_task) = connected_agent(
            |request| match request {
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
                _ => (Duration::ZERO, RpcResponse::Ack),
            },
            |config| config,
        )
        .await;

        let mut handles = Vec::new();
        for i in 0..20u64 {
            let agent = agent.clone();
            handles.push(tokio::spawn(async move {
                if i % 2 == 0 {
                    let exec_id = format!("exec-{i}");
                    let pid = agent
                        .exec_process(exec_request(&exec_id))
                        .await
                        .unwrap_or_else(|err| panic!("caller {i} failed: {err:?}"));
                    assert_eq!(pid, (tag(&exec_id) % 100_000) as u32, "caller {i} mismatch");
                } else {
                    let container_id = format!("ctr-{i}");
                    let stats = agent
                        .stats_container(&container_id)
                        .await
                        .unwrap_or_else(|err| panic!("caller {i} failed: {err:?}"));
                    assert_eq!(
                        stats.memory_usage_bytes,
                        tag(&container_id),
                        "caller {i} mismatch"
                    );
                }
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }

        agent.disconnect().unwrap();
        let _ = server_task.await;
    }

    /// A reply slower than `request_timeout` is cut off by the `tower` timeout
    /// layer and surfaces as `AgentError::Timeout`.
    #[tokio::test]
    async fn slow_reply_surfaces_timeout() {
        let (agent, server_task) = connected_agent(
            |_| {
                (
                    Duration::from_millis(200),
                    RpcResponse::Stats(ContainerStats::default()),
                )
            },
            |config| config.with_request_timeout(Duration::from_millis(50)),
        )
        .await;

        let err = agent
            .stats_container("ctr")
            .await
            .expect_err("stalled reply must time out");
        assert!(
            matches!(err, AgentError::Timeout),
            "expected Timeout, got {err:?}"
        );

        agent.disconnect().unwrap();
        let _ = server_task.await;
    }

    /// `wait_process` blocks past `request_timeout` and still succeeds, because
    /// it goes through the untimed service.
    #[tokio::test]
    async fn wait_process_outlives_request_timeout() {
        let (agent, server_task) = connected_agent(
            |request| match request {
                RpcRequest::WaitProcess { .. } => (
                    Duration::from_millis(200),
                    RpcResponse::WaitResult { exit_code: 7 },
                ),
                _ => (Duration::ZERO, RpcResponse::Ack),
            },
            |config| config.with_request_timeout(Duration::from_millis(50)),
        )
        .await;

        let exit_code = agent
            .wait_process("ctr", "exec-1")
            .await
            .expect("long wait must not be cut off");
        assert_eq!(exit_code, 7);

        agent.disconnect().unwrap();
        let _ = server_task.await;
    }

    /// Dropping the guest half mid-flight must fail in-flight calls promptly
    /// with `AgentError::Disconnected` rather than hanging.
    #[tokio::test]
    async fn dropping_the_guest_fails_in_flight_calls() {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let connector = Arc::new(OneShotConnector::new(1024, client));
        let agent = GuestAgent::new(AgentConfig::new(1024), connector);
        agent.connect().await.unwrap();

        let caller = agent.clone();
        let call = tokio::spawn(async move { caller.stats_container("ctr").await });

        // Let the request reach the (never-read) server half, then kill it.
        tokio::time::sleep(Duration::from_millis(40)).await;
        drop(server);

        let result = tokio::time::timeout(Duration::from_secs(2), call)
            .await
            .expect("call should not hang after the guest is dropped")
            .unwrap();
        assert!(
            matches!(result, Err(AgentError::Disconnected)),
            "expected Disconnected, got {result:?}"
        );

        agent.disconnect().unwrap();
    }

    /// A call made before `connect` is rejected rather than blocking.
    #[tokio::test]
    async fn calls_before_connect_are_rejected() {
        let connector = Arc::new(OneShotConnector::new(1024, tokio::io::duplex(64).0));
        let agent = GuestAgent::new(AgentConfig::new(1024), connector);
        let err = agent.stats_container("ctr").await.unwrap_err();
        assert!(matches!(err, AgentError::NotConnected), "got {err:?}");
    }
}
