//! Host-side client for the guest init.
//!
//! [`AgentClient`] speaks the [`crate::proto`] framing over any
//! `AsyncRead + AsyncWrite` stream. Production Linux code connects that stream
//! with AF_VSOCK; tests (and any backend that can hand back a pipe) pass the
//! stream in directly. [`reference`] is the guest-side peer of this client.

mod reference;

use std::collections::HashMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};
use tokio::sync::{mpsc, oneshot, watch};

use crate::error::{Error, Result};
use crate::proto::{Frame, FrameKind, Request, Response};
use crate::types::{ContainerOpts, ContainerStats, ContainerStatus, ExecOpts, ExitStatus, Signal};

pub(crate) use reference::serve as serve_reference;

/// Monomorphize the reference guest init so a non-test build keeps it.
///
/// Production microVMs will run this same function as PID 1. Unit tests drive
/// it through the loopback hypervisor; without this reference the cdylib would
/// treat the whole guest as dead code.
pub(crate) fn link_reference_guest() {
    let _entry = serve_reference::<tokio::io::DuplexStream>;
    std::hint::black_box(_entry);
}

/// Shared exit notification. `watch` does not drop updates between the
/// readiness check and the wait, so a fast process cannot lose its status.
#[derive(Clone)]
pub(crate) struct ExitWait {
    rx: watch::Receiver<Option<ExitStatus>>,
}

impl ExitWait {
    pub(crate) fn channel() -> (watch::Sender<Option<ExitStatus>>, Self) {
        let (tx, rx) = watch::channel(None);
        (tx, Self { rx })
    }

    pub(crate) async fn wait(&mut self) -> Result<ExitStatus> {
        loop {
            if let Some(status) = *self.rx.borrow() {
                return Ok(status);
            }
            if self.rx.changed().await.is_err() {
                return Err(Error::Agent(
                    "guest closed the connection before the process exited".into(),
                ));
            }
        }
    }
}

struct ProcSink {
    stdout: mpsc::UnboundedSender<Vec<u8>>,
    stderr: mpsc::UnboundedSender<Vec<u8>>,
    exit_tx: watch::Sender<Option<ExitStatus>>,
}

#[derive(Default)]
struct EarlyIo {
    stdout: Vec<Vec<u8>>,
    stderr: Vec<Vec<u8>>,
    exit: Option<ExitStatus>,
}

enum ResponseMsg {
    Ack,
    Started(ProcessIo),
    State { status: String, pid: Option<u32> },
    Stats(ContainerStats),
    Logs { stdout: Vec<u8>, stderr: Vec<u8> },
}

pub(crate) struct ProcessIo {
    pub pid: u32,
    pub stdin: Option<Box<dyn AsyncWrite + Unpin + Send>>,
    pub stdout: Option<Box<dyn AsyncRead + Unpin + Send>>,
    pub stderr: Option<Box<dyn AsyncRead + Unpin + Send>>,
    pub exit: ExitWait,
}

struct AgentInner {
    tx: mpsc::Sender<Frame>,
    next_id: std::sync::atomic::AtomicU32,
    pending: Mutex<HashMap<u32, oneshot::Sender<std::result::Result<ResponseMsg, String>>>>,
    procs: Mutex<HashMap<u32, ProcSink>>,
    early: Mutex<HashMap<u32, EarlyIo>>,
}

impl AgentInner {
    fn fail_all(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            for (_, tx) in pending.drain() {
                let _ = tx.send(Err("guest connection closed".into()));
            }
        }
        if let Ok(mut procs) = self.procs.lock() {
            for (_, proc) in procs.drain() {
                let _ = proc.exit_tx.send(Some(ExitStatus {
                    code: -1,
                    signal: None,
                }));
            }
        }
    }

    fn complete(&self, id: u32, msg: std::result::Result<ResponseMsg, String>) {
        let sender = self
            .pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(&id));
        if let Some(sender) = sender {
            let _ = sender.send(msg);
        }
    }

    fn dispatch(&self, frame: Frame) {
        match frame.kind {
            FrameKind::Response => self.on_response(frame),
            FrameKind::Stdout => self.push_stdio(frame.id, true, frame.payload),
            FrameKind::Stderr => self.push_stdio(frame.id, false, frame.payload),
            FrameKind::Exit => self.on_exit(frame),
            FrameKind::Request | FrameKind::Stdin | FrameKind::Resize => {}
        }
    }

    fn on_response(&self, frame: Frame) {
        let parsed = match serde_json::from_slice::<Response>(&frame.payload) {
            Ok(response) => response,
            Err(err) => {
                self.complete(frame.id, Err(err.to_string()));
                return;
            }
        };
        match parsed {
            Response::Error { message } => self.complete(frame.id, Err(message)),
            Response::Ok => self.complete(frame.id, Ok(ResponseMsg::Ack)),
            Response::Started { pid } => {
                let io = self.register_process(pid);
                self.complete(frame.id, Ok(ResponseMsg::Started(io)));
            }
            Response::State { status, pid } => {
                self.complete(frame.id, Ok(ResponseMsg::State { status, pid }));
            }
            Response::Stats {
                cpu_usage_usec,
                memory_usage_bytes,
                memory_limit_bytes,
                blkio_read_bytes,
                blkio_write_bytes,
                pids,
            } => self.complete(
                frame.id,
                Ok(ResponseMsg::Stats(ContainerStats {
                    cpu_usage_usec,
                    memory_usage_bytes,
                    memory_limit_bytes,
                    blkio_read_bytes,
                    blkio_write_bytes,
                    pids,
                })),
            ),
            Response::Logs {
                stdout_b64,
                stderr_b64,
            } => {
                let stdout = STANDARD.decode(stdout_b64).unwrap_or_default();
                let stderr = STANDARD.decode(stderr_b64).unwrap_or_default();
                self.complete(frame.id, Ok(ResponseMsg::Logs { stdout, stderr }));
            }
        }
    }

    fn register_process(&self, pid: u32) -> ProcessIo {
        let (out_tx, out_rx) = mpsc::unbounded_channel();
        let (err_tx, err_rx) = mpsc::unbounded_channel();
        let (exit_tx, exit_rx) = watch::channel(None);
        if let Ok(mut early) = self.early.lock() {
            if let Some(buffered) = early.remove(&pid) {
                for chunk in buffered.stdout {
                    let _ = out_tx.send(chunk);
                }
                for chunk in buffered.stderr {
                    let _ = err_tx.send(chunk);
                }
                if let Some(status) = buffered.exit {
                    let _ = exit_tx.send(Some(status));
                }
            }
        }
        if let Ok(mut procs) = self.procs.lock() {
            procs.insert(
                pid,
                ProcSink {
                    stdout: out_tx,
                    stderr: err_tx,
                    exit_tx,
                },
            );
        }
        ProcessIo {
            pid,
            // Stdio halves are attached by the caller, which knows which
            // directions the request asked for. The raw receivers live in
            // `ProcessIo` until then; see `AgentClient::finish_stdio`.
            stdin: None,
            stdout: Some(Box::new(ByteReader::new(out_rx))),
            stderr: Some(Box::new(ByteReader::new(err_rx))),
            exit: ExitWait { rx: exit_rx },
        }
    }

    fn push_stdio(&self, pid: u32, is_stdout: bool, data: Vec<u8>) {
        if let Ok(procs) = self.procs.lock() {
            if let Some(proc) = procs.get(&pid) {
                let tx = if is_stdout {
                    &proc.stdout
                } else {
                    &proc.stderr
                };
                let _ = tx.send(data);
                return;
            }
        }
        if let Ok(mut early) = self.early.lock() {
            let slot = early.entry(pid).or_default();
            if is_stdout {
                slot.stdout.push(data);
            } else {
                slot.stderr.push(data);
            }
        }
    }

    fn on_exit(&self, frame: Frame) {
        let code = frame
            .payload
            .get(..4)
            .and_then(|b| b.try_into().ok())
            .map(i32::from_le_bytes)
            .unwrap_or(-1);
        let signal = if frame.flags == 0 {
            None
        } else {
            Some(Signal::from_raw(i32::from(frame.flags)))
        };
        let status = ExitStatus { code, signal };
        if let Ok(mut procs) = self.procs.lock() {
            if let Some(proc) = procs.remove(&frame.id) {
                let _ = proc.exit_tx.send(Some(status));
                return;
            }
        }
        if let Ok(mut early) = self.early.lock() {
            early.entry(frame.id).or_default().exit = Some(status);
        }
    }
}

/// Cloneable handle to one guest-agent connection.
#[derive(Clone)]
pub(crate) struct AgentClient {
    inner: Arc<AgentInner>,
}

impl AgentClient {
    pub(crate) async fn connect_stream<S>(stream: S) -> Result<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (reader, mut writer) = tokio::io::split(stream);
        let (tx, mut rx) = mpsc::channel::<Frame>(256);
        tokio::spawn(async move {
            while let Some(frame) = rx.recv().await {
                if frame.write_to(&mut writer).await.is_err() {
                    break;
                }
            }
        });
        let inner = Arc::new(AgentInner {
            tx,
            next_id: std::sync::atomic::AtomicU32::new(1),
            pending: Mutex::new(HashMap::new()),
            procs: Mutex::new(HashMap::new()),
            early: Mutex::new(HashMap::new()),
        });
        let reader_inner = Arc::clone(&inner);
        tokio::spawn(async move {
            let mut reader = reader;
            loop {
                match Frame::read_from(&mut reader).await {
                    Ok(frame) => reader_inner.dispatch(frame),
                    Err(_) => {
                        reader_inner.fail_all();
                        break;
                    }
                }
            }
        });
        Ok(Self { inner })
    }

    pub(crate) async fn prepare(&self, opts: &ContainerOpts) -> Result<()> {
        let namespaces = opts
            .oci_spec
            .as_ref()
            .map(crate::types::namespaces_from_spec)
            .unwrap_or_default()
            .into_iter()
            .map(|(typ, path)| crate::proto::NamespaceSpec { typ, path })
            .collect();
        let oci_config_json = opts
            .oci_spec
            .as_ref()
            .and_then(|spec| serde_json::to_string(spec).ok());
        match self
            .call(Request::Create {
                args: opts.args.clone(),
                env: opts.env.clone(),
                cwd: opts.working_dir.display().to_string(),
                mounts: opts
                    .mounts
                    .iter()
                    .map(|m| crate::proto::MountSpec {
                        source: m.source.display().to_string(),
                        destination: m.destination.display().to_string(),
                        fs_type: m.fs_type.clone(),
                        options: m.options.clone(),
                    })
                    .collect(),
                namespaces,
                tty: opts.tty,
                stdin: opts.attach_stdin,
                stdout: opts.attach_stdout,
                stderr: opts.attach_stderr,
                oci_config_json,
                memory_limit_bytes: opts.memory_mb.saturating_mul(1024 * 1024),
            })
            .await?
        {
            ResponseMsg::Ack => Ok(()),
            _ => Err(Error::Agent("unexpected reply to create".into())),
        }
    }

    pub(crate) async fn start(&self, opts: &ContainerOpts) -> Result<ProcessIo> {
        self.finish_start(
            self.call(Request::Start).await?,
            opts.attach_stdin,
            opts.attach_stdout,
            opts.attach_stderr,
        )
    }

    pub(crate) async fn exec(&self, opts: &ExecOpts) -> Result<ProcessIo> {
        self.finish_start(
            self.call(Request::Exec {
                args: opts.args.clone(),
                env: opts.env.clone(),
                cwd: opts.working_dir.as_ref().map(|p| p.display().to_string()),
                tty: opts.tty,
                stdin: opts.stdin,
                stdout: opts.stdout,
                stderr: opts.stderr,
                user: opts.user.clone(),
            })
            .await?,
            opts.stdin,
            opts.stdout,
            opts.stderr,
        )
    }

    fn finish_start(
        &self,
        msg: ResponseMsg,
        stdin: bool,
        stdout: bool,
        stderr: bool,
    ) -> Result<ProcessIo> {
        let ResponseMsg::Started(mut io) = msg else {
            return Err(Error::Agent("unexpected reply to start/exec".into()));
        };
        if !stdout {
            io.stdout = None;
        }
        if !stderr {
            io.stderr = None;
        }
        if stdin {
            io.stdin = Some(self.spawn_stdin_pump(io.pid));
        }
        Ok(io)
    }

    fn spawn_stdin_pump(&self, pid: u32) -> Box<dyn AsyncWrite + Unpin + Send> {
        let (mut reader, writer) = tokio::io::duplex(64 * 1024);
        let tx = self.inner.tx.clone();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 8192];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        let _ = tx.send(Frame::stdin_eof(pid)).await;
                        break;
                    }
                    Ok(n) => {
                        let frame = Frame::stdio(FrameKind::Stdin, pid, buf[..n].to_vec());
                        if tx.send(frame).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Box::new(writer)
    }

    pub(crate) async fn kill(&self, pid: u32, signal: Signal) -> Result<()> {
        self.expect_ack(
            self.call(Request::Kill {
                pid,
                signal: signal.as_raw(),
            })
            .await?,
        )
    }

    pub(crate) async fn resize(&self, pid: u32, rows: u16, cols: u16) -> Result<()> {
        self.expect_ack(self.call(Request::Resize { pid, rows, cols }).await?)
    }

    pub(crate) async fn state(&self) -> Result<(ContainerStatus, Option<u32>)> {
        match self.call(Request::State).await? {
            ResponseMsg::State { status, pid } => {
                let status = ContainerStatus::parse(&status).ok_or_else(|| {
                    Error::Agent(format!("guest reported unknown status {status}"))
                })?;
                Ok((status, pid))
            }
            _ => Err(Error::Agent("unexpected reply to state".into())),
        }
    }

    pub(crate) async fn stats(&self) -> Result<ContainerStats> {
        match self.call(Request::Stats).await? {
            ResponseMsg::Stats(stats) => Ok(stats),
            _ => Err(Error::Agent("unexpected reply to stats".into())),
        }
    }

    pub(crate) async fn logs(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        match self.call(Request::Logs).await? {
            ResponseMsg::Logs { stdout, stderr } => Ok((stdout, stderr)),
            _ => Err(Error::Agent("unexpected reply to logs".into())),
        }
    }

    pub(crate) async fn delete(&self) -> Result<()> {
        self.expect_ack(self.call(Request::Delete).await?)
    }

    fn expect_ack(&self, msg: ResponseMsg) -> Result<()> {
        match msg {
            ResponseMsg::Ack => Ok(()),
            _ => Err(Error::Agent("unexpected guest reply".into())),
        }
    }

    async fn call(&self, request: Request) -> Result<ResponseMsg> {
        let id = self
            .inner
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.inner
            .pending
            .lock()
            .map_err(|_| Error::Agent("pending map poisoned".into()))?
            .insert(id, tx);
        let payload = serde_json::to_vec(&request)?;
        self.inner
            .tx
            .send(Frame::request(id, payload))
            .await
            .map_err(|_| Error::Agent("guest connection closed".into()))?;
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(Ok(msg))) => Ok(msg),
            Ok(Ok(Err(message))) => Err(Error::Agent(message)),
            Ok(Err(_)) => Err(Error::Agent("guest connection closed".into())),
            Err(_) => Err(Error::Agent(format!("request {id} timed out"))),
        }
    }
}

/// Channel-backed [`AsyncRead`] used for process stdout/stderr.
struct ByteReader {
    rx: mpsc::UnboundedReceiver<Vec<u8>>,
    buf: Vec<u8>,
    pos: usize,
    eof: bool,
}

impl ByteReader {
    fn new(rx: mpsc::UnboundedReceiver<Vec<u8>>) -> Self {
        Self {
            rx,
            buf: Vec::new(),
            pos: 0,
            eof: false,
        }
    }
}

impl AsyncRead for ByteReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buf.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        loop {
            if self.pos < self.buf.len() {
                let n = (self.buf.len() - self.pos).min(buf.remaining());
                buf.put_slice(&self.buf[self.pos..self.pos + n]);
                self.pos += n;
                return Poll::Ready(Ok(()));
            }
            if self.eof {
                return Poll::Ready(Ok(()));
            }
            match self.rx.poll_recv(cx) {
                Poll::Ready(Some(chunk)) => {
                    if chunk.is_empty() {
                        continue;
                    }
                    self.buf = chunk;
                    self.pos = 0;
                }
                Poll::Ready(None) => {
                    self.eof = true;
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
