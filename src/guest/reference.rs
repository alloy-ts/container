//! Reference guest init.
//!
//! This is the in-guest peer of [`super::AgentClient`]. A production microVM
//! runs it as PID 1; the test hypervisor runs the same function on a socket
//! pair so the public lifecycle can be exercised without KVM. Process
//! execution here is the guest init's `exec`, not the old host CLI delegation.
//!
//! TODO(guest-vm): ship this as a static binary inside the guest rootfs and
//! teach it to apply OCI namespaces, mounts, and user switching. Those steps
//! need a real guest mount namespace and are recorded but not performed.

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;

use crate::error::{Error, Result};
use crate::proto::{Frame, FrameKind, MountSpec, NamespaceSpec, Request, Response, FLAG_EOF};
use crate::types::ContainerStats;

const LOG_CAP: usize = 1024 * 1024;

struct GuestConfig {
    args: Vec<String>,
    env: Vec<(String, String)>,
    cwd: String,
    tty: bool,
    #[allow(dead_code)]
    stdin: bool,
    #[allow(dead_code)]
    stdout: bool,
    #[allow(dead_code)]
    stderr: bool,
    memory_limit_bytes: u64,
    #[allow(dead_code)]
    mounts: Vec<MountSpec>,
    #[allow(dead_code)]
    namespaces: Vec<NamespaceSpec>,
    #[allow(dead_code)]
    oci_config_json: Option<String>,
}

struct LiveProc {
    stdin: Arc<tokio::sync::Mutex<Option<StdinSink>>>,
    winsize_fd: Option<std::os::fd::RawFd>,
    /// Keeps the pty master open for `TIOCSWINSZ`.
    _master: Option<std::os::fd::OwnedFd>,
}

enum StdinSink {
    Pipe(ChildStdin),
    #[cfg(unix)]
    Pty(tokio::fs::File),
}

struct Guest {
    tx: mpsc::Sender<Frame>,
    config: Option<GuestConfig>,
    status: &'static str,
    primary_pid: Option<u32>,
    procs: Arc<Mutex<HashMap<u32, LiveProc>>>,
    /// Child tasks insert a pid here once `wait` returns so `state` does not
    /// depend on the read loop observing the exit.
    finished: Arc<Mutex<HashSet<u32>>>,
    pending: HashMap<u32, PendingIo>,
    log_out: Arc<Mutex<Vec<u8>>>,
    log_err: Arc<Mutex<Vec<u8>>>,
}

pub(crate) async fn serve<S>(stream: S) -> Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut reader, mut writer) = tokio::io::split(stream);
    let (tx, mut rx) = mpsc::channel::<Frame>(256);
    tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if frame.write_to(&mut writer).await.is_err() {
                break;
            }
        }
    });
    let mut guest = Guest {
        tx,
        config: None,
        status: "created",
        primary_pid: None,
        procs: Arc::new(Mutex::new(HashMap::new())),
        finished: Arc::new(Mutex::new(HashSet::new())),
        pending: HashMap::new(),
        log_out: Arc::new(Mutex::new(Vec::new())),
        log_err: Arc::new(Mutex::new(Vec::new())),
    };
    loop {
        let frame = match Frame::read_from(&mut reader).await {
            Ok(frame) => frame,
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(_) => break,
        };
        if let Err(err) = guest.handle(frame).await {
            // `handle` reports request failures itself. A transport error ends the session.
            if matches!(err, Error::Io(_)) {
                break;
            }
        }
    }
    guest.kill_all();
    Ok(())
}

impl Guest {
    async fn handle(&mut self, frame: Frame) -> Result<()> {
        match frame.kind {
            FrameKind::Request => {
                let id = frame.id;
                let request = match serde_json::from_slice::<Request>(&frame.payload) {
                    Ok(request) => request,
                    Err(err) => {
                        send_reply(
                            &self.tx,
                            id,
                            Response::Error {
                                message: err.to_string(),
                            },
                        )
                        .await?;
                        return Ok(());
                    }
                };
                if let Err(err) = self.handle_request(id, request).await {
                    let _ = send_reply(
                        &self.tx,
                        id,
                        Response::Error {
                            message: err.to_string(),
                        },
                    )
                    .await;
                }
                Ok(())
            }
            FrameKind::Stdin => write_stdin(&self.procs, frame).await,
            FrameKind::Resize => {
                if frame.payload.len() >= 4 {
                    let rows = u16::from_le_bytes([frame.payload[0], frame.payload[1]]);
                    let cols = u16::from_le_bytes([frame.payload[2], frame.payload[3]]);
                    let _ = self.resize(frame.id, rows, cols);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn handle_request(&mut self, id: u32, request: Request) -> Result<()> {
        match request {
            Request::Create {
                args,
                env,
                cwd,
                mounts,
                namespaces,
                tty,
                stdin,
                stdout,
                stderr,
                oci_config_json,
                memory_limit_bytes,
            } => {
                // TODO(guest-vm): create the mount/pid/uts/ipc/user/net namespaces
                // listed in `namespaces` and mount `mounts` before exec.
                self.config = Some(GuestConfig {
                    args,
                    env,
                    cwd,
                    tty,
                    stdin,
                    stdout,
                    stderr,
                    memory_limit_bytes,
                    mounts,
                    namespaces,
                    oci_config_json,
                });
                self.status = "created";
                send_reply(&self.tx, id, Response::Ok).await
            }
            Request::Start => {
                let config = self
                    .config
                    .as_ref()
                    .ok_or_else(|| Error::Agent("create was not called".into()))?;
                let spec = SpawnSpec {
                    args: config.args.clone(),
                    env: config.env.clone(),
                    cwd: Some(config.cwd.clone()),
                    tty: config.tty,
                    capture_logs: true,
                };
                let pid = self.spawn(spec).await?;
                self.primary_pid = Some(pid);
                self.status = "running";
                send_reply(&self.tx, id, Response::Started { pid }).await?;
                self.arm_io(pid).await;
                Ok(())
            }
            Request::Exec {
                args,
                env,
                cwd,
                tty,
                stdin: _,
                stdout: _,
                stderr: _,
                user,
            } => {
                if self.status != "running" && self.status != "paused" {
                    return Err(Error::Agent("exec requires a running container".into()));
                }
                if user.is_some() {
                    // TODO(guest-vm): setuid/setgid inside the guest before exec.
                    // The reference init records the request and refuses it so
                    // callers do not think the switch happened.
                    return Err(Error::Agent(
                        "TODO: guest user switching is not implemented".into(),
                    ));
                }
                let spec = SpawnSpec {
                    args,
                    env,
                    cwd,
                    tty,
                    capture_logs: false,
                };
                let pid = self.spawn(spec).await?;
                send_reply(&self.tx, id, Response::Started { pid }).await?;
                self.arm_io(pid).await;
                Ok(())
            }
            Request::Kill { pid, signal } => {
                let target = if pid == 0 {
                    self.primary_pid.unwrap_or(0)
                } else {
                    pid
                };
                if target == 0 || !self.alive(target) {
                    return Err(Error::Agent(format!("no such process {target}")));
                }
                if !(1..=64).contains(&signal) {
                    return Err(Error::Agent(format!("invalid signal {signal}")));
                }
                signal_pid(target, signal)?;
                send_reply(&self.tx, id, Response::Ok).await
            }
            Request::State => {
                if self.status == "running" {
                    if let Some(pid) = self.primary_pid {
                        if self
                            .finished
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .contains(&pid)
                        {
                            self.status = "stopped";
                        }
                    }
                }
                send_reply(
                    &self.tx,
                    id,
                    Response::State {
                        status: self.status.to_string(),
                        pid: self.primary_pid,
                    },
                )
                .await
            }
            Request::Stats => {
                let limit = self
                    .config
                    .as_ref()
                    .map(|c| c.memory_limit_bytes)
                    .unwrap_or(0);
                let pids: Vec<u32> = self
                    .procs
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .keys()
                    .copied()
                    .collect();
                let stats = collect_stats(&pids, limit);
                send_reply(&self.tx, id, Response::stats(&stats)).await
            }
            Request::Resize { pid, rows, cols } => {
                self.resize(pid, rows, cols)?;
                send_reply(&self.tx, id, Response::Ok).await
            }
            Request::Logs => {
                let stdout = self
                    .log_out
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                let stderr = self
                    .log_err
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                send_reply(
                    &self.tx,
                    id,
                    Response::Logs {
                        stdout_b64: STANDARD.encode(stdout),
                        stderr_b64: STANDARD.encode(stderr),
                    },
                )
                .await
            }
            Request::Delete => {
                self.kill_all();
                self.status = "stopped";
                send_reply(&self.tx, id, Response::Ok).await
            }
        }
    }

    /// Queue the `started` reply before any stdio frame.
    ///
    /// `spawn` only creates the child. [`Guest::arm_io`] starts the copy tasks
    /// after the reply is in the write queue.
    async fn spawn(&mut self, spec: SpawnSpec) -> Result<u32> {
        if spec.args.is_empty() {
            return Err(Error::Agent("missing argv".into()));
        }
        let mut launched = launch(&spec)?;
        if spec.capture_logs {
            launched.io.log_out = Some(Arc::clone(&self.log_out));
            launched.io.log_err = Some(Arc::clone(&self.log_err));
        }
        let pid = launched.pid;
        self.procs.lock().unwrap_or_else(|e| e.into_inner()).insert(
            pid,
            LiveProc {
                stdin: launched.stdin,
                winsize_fd: launched.winsize_fd,
                _master: launched.master,
            },
        );
        self.pending.insert(pid, launched.io);
        Ok(pid)
    }

    async fn arm_io(&mut self, pid: u32) {
        let Some(io) = self.pending.remove(&pid) else {
            return;
        };
        let tx = self.tx.clone();
        let mut copies = Vec::new();
        if let Some(mut stdout) = io.stdout {
            let tx = tx.clone();
            let log = io.log_out.clone();
            copies.push(tokio::spawn(async move {
                copy_stream(&mut stdout, tx, pid, FrameKind::Stdout, log).await;
            }));
        }
        if let Some(mut stderr) = io.stderr {
            let tx = tx.clone();
            let log = io.log_err.clone();
            copies.push(tokio::spawn(async move {
                copy_stream(&mut stderr, tx, pid, FrameKind::Stderr, log).await;
            }));
        }
        let mut child = io.child;
        let procs = Arc::clone(&self.procs);
        let finished = Arc::clone(&self.finished);
        tokio::spawn(async move {
            let status = child.wait().await;
            // Drain stdout/stderr before the exit frame so the host cannot
            // observe "exited" while bytes are still queued behind it.
            for copy in copies {
                let _ = copy.await;
            }
            let (code, signal) = decode_wait(status);
            finished
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(pid);
            procs.lock().unwrap_or_else(|e| e.into_inner()).remove(&pid);
            let _ = tx.send(Frame::exit(pid, code, signal)).await;
        });
    }

    fn resize(&self, pid: u32, rows: u16, cols: u16) -> Result<()> {
        let procs = self.procs.lock().unwrap_or_else(|e| e.into_inner());
        let proc = procs
            .get(&pid)
            .ok_or_else(|| Error::Agent(format!("no such process {pid}")))?;
        let Some(fd) = proc.winsize_fd else {
            return Err(Error::Agent(
                "process has no tty; resize_tty requires ExecOpts::tty".into(),
            ));
        };
        #[cfg(unix)]
        {
            set_winsize(fd, rows, cols).map_err(Error::Io)
        }
        #[cfg(not(unix))]
        {
            let _ = (fd, rows, cols);
            Err(Error::Agent(
                "TODO: tty resize is only implemented on unix".into(),
            ))
        }
    }

    fn alive(&self, pid: u32) -> bool {
        if self
            .finished
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(&pid)
        {
            return false;
        }
        self.procs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&pid)
    }

    fn kill_all(&mut self) {
        let pids: Vec<u32> = self
            .procs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .copied()
            .collect();
        for pid in pids {
            let _ = signal_pid(pid, 9);
        }
        self.procs.lock().unwrap_or_else(|e| e.into_inner()).clear();
        self.pending.clear();
    }
}

async fn send_reply(tx: &mpsc::Sender<Frame>, id: u32, response: Response) -> Result<()> {
    let payload = serde_json::to_vec(&response)?;
    tx.send(Frame::response(id, payload))
        .await
        .map_err(|_| Error::Agent("guest writer closed".into()))
}

async fn write_stdin(procs: &Arc<Mutex<HashMap<u32, LiveProc>>>, frame: Frame) -> Result<()> {
    let sink = {
        let procs = procs.lock().unwrap_or_else(|err| err.into_inner());
        let Some(proc) = procs.get(&frame.id) else {
            return Ok(());
        };
        Arc::clone(&proc.stdin)
    };
    let mut guard = sink.lock().await;
    if frame.flags & FLAG_EOF != 0 {
        guard.take();
        return Ok(());
    }
    if frame.payload.is_empty() {
        return Ok(());
    }
    match guard.as_mut() {
        Some(StdinSink::Pipe(pipe)) => {
            let _ = pipe.write_all(&frame.payload).await;
        }
        #[cfg(unix)]
        Some(StdinSink::Pty(file)) => {
            let _ = file.write_all(&frame.payload).await;
        }
        None => {}
    }
    Ok(())
}

struct SpawnSpec {
    args: Vec<String>,
    env: Vec<(String, String)>,
    cwd: Option<String>,
    tty: bool,
    capture_logs: bool,
}

struct PendingIo {
    child: Child,
    stdout: Option<Box<dyn AsyncRead + Unpin + Send>>,
    stderr: Option<Box<dyn AsyncRead + Unpin + Send>>,
    log_out: Option<Arc<Mutex<Vec<u8>>>>,
    log_err: Option<Arc<Mutex<Vec<u8>>>>,
}

struct Launched {
    pid: u32,
    stdin: Arc<tokio::sync::Mutex<Option<StdinSink>>>,
    winsize_fd: Option<std::os::fd::RawFd>,
    master: Option<std::os::fd::OwnedFd>,
    io: PendingIo,
}

fn launch(spec: &SpawnSpec) -> Result<Launched> {
    if spec.tty {
        #[cfg(unix)]
        {
            return launch_pty(spec);
        }
        #[cfg(not(unix))]
        {
            return Err(Error::Agent(
                "TODO: pty allocation is only implemented on unix".into(),
            ));
        }
    }
    launch_pipes(spec)
}

fn launch_pipes(spec: &SpawnSpec) -> Result<Launched> {
    let mut cmd = Command::new(&spec.args[0]);
    cmd.args(&spec.args[1..])
        .envs(spec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
    let mut child = cmd
        .spawn()
        .map_err(|err| Error::Agent(format!("failed to exec {}: {err}", spec.args[0])))?;
    let pid = child
        .id()
        .ok_or_else(|| Error::Agent("spawned process has no pid".into()))?;
    let stdin = child.stdin.take();
    let stdout = child
        .stdout
        .take()
        .map(|s| Box::new(s) as Box<dyn AsyncRead + Unpin + Send>);
    let stderr = child
        .stderr
        .take()
        .map(|s| Box::new(s) as Box<dyn AsyncRead + Unpin + Send>);
    Ok(Launched {
        pid,
        stdin: Arc::new(tokio::sync::Mutex::new(stdin.map(StdinSink::Pipe))),
        winsize_fd: None,
        master: None,
        io: PendingIo {
            child,
            stdout,
            stderr,
            log_out: None,
            log_err: None,
        },
    })
}

#[cfg(unix)]
fn launch_pty(spec: &SpawnSpec) -> Result<Launched> {
    use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
    use std::process::Stdio;

    let (master, slave) = open_pty(24, 80)?;
    let slave_in = slave
        .try_clone()
        .map_err(|err| Error::Agent(format!("dup pty slave: {err}")))?;
    let slave_out = slave
        .try_clone()
        .map_err(|err| Error::Agent(format!("dup pty slave: {err}")))?;
    let mut cmd = Command::new(&spec.args[0]);
    cmd.args(&spec.args[1..])
        .envs(spec.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(Stdio::from(slave_in))
        .stdout(Stdio::from(slave_out))
        .stderr(Stdio::from(slave))
        .kill_on_drop(true);
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }
    let mut child = cmd
        .spawn()
        .map_err(|err| Error::Agent(format!("failed to exec {}: {err}", spec.args[0])))?;
    let pid = child
        .id()
        .ok_or_else(|| Error::Agent("spawned process has no pid".into()))?;
    let reader_fd = master
        .try_clone()
        .map_err(|err| Error::Agent(format!("dup pty master: {err}")))?;
    let writer_fd = master
        .try_clone()
        .map_err(|err| Error::Agent(format!("dup pty master: {err}")))?;
    let winsize_fd = Some(master.as_raw_fd());
    let reader =
        tokio::fs::File::from_std(unsafe { std::fs::File::from_raw_fd(reader_fd.into_raw_fd()) });
    let writer =
        tokio::fs::File::from_std(unsafe { std::fs::File::from_raw_fd(writer_fd.into_raw_fd()) });
    // stdout and stderr share the pty; stderr is not a separate stream.
    let _ = child.stdout.take();
    let _ = child.stderr.take();
    let _ = child.stdin.take();
    Ok(Launched {
        pid,
        stdin: Arc::new(tokio::sync::Mutex::new(Some(StdinSink::Pty(writer)))),
        winsize_fd,
        master: Some(master),
        io: PendingIo {
            child,
            stdout: Some(Box::new(reader)),
            stderr: None,
            log_out: None,
            log_err: None,
        },
    })
}

#[cfg(unix)]
fn open_pty(rows: u16, cols: u16) -> Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    use std::os::fd::{FromRawFd, OwnedFd};

    let mut master = -1;
    let mut slave = -1;
    let mut winsize = libc::winsize {
        ws_row: rows.max(1),
        ws_col: cols.max(1),
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: openpty initializes both fds on success. We take ownership of them.
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut winsize,
        )
    };
    if rc != 0 {
        return Err(Error::Io(io::Error::last_os_error()));
    }
    Ok((unsafe { OwnedFd::from_raw_fd(master) }, unsafe {
        OwnedFd::from_raw_fd(slave)
    }))
}

fn signal_pid(pid: u32, signal: i32) -> Result<()> {
    #[cfg(unix)]
    {
        // SAFETY: `pid` is a process this agent spawned. `signal` is range-checked.
        let rc = unsafe { libc::kill(pid as i32, signal) };
        if rc != 0 {
            Err(Error::Io(io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, signal);
        Err(Error::Agent(
            "TODO: process signals are only implemented on unix".into(),
        ))
    }
}

#[cfg(unix)]
fn set_winsize(fd: std::os::fd::RawFd, rows: u16, cols: u16) -> io::Result<()> {
    let winsize = libc::winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: `fd` is a live pty master owned by the process table entry.
    let rc = unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &winsize) };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

async fn copy_stream(
    reader: &mut (dyn AsyncRead + Unpin + Send),
    tx: mpsc::Sender<Frame>,
    pid: u32,
    kind: FrameKind,
    log: Option<Arc<Mutex<Vec<u8>>>>,
) {
    let mut buf = vec![0u8; 8192];
    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        if let Some(log) = &log {
            if let Ok(mut guard) = log.lock() {
                push_log(&mut guard, &buf[..n]);
            }
        }
        let frame = Frame::stdio(kind, pid, buf[..n].to_vec());
        if tx.send(frame).await.is_err() {
            break;
        }
    }
}

fn push_log(buf: &mut Vec<u8>, data: &[u8]) {
    if buf.len() >= LOG_CAP {
        return;
    }
    let room = LOG_CAP - buf.len();
    buf.extend_from_slice(&data[..data.len().min(room)]);
}

fn decode_wait(status: std::result::Result<std::process::ExitStatus, io::Error>) -> (i32, u16) {
    let Ok(status) = status else {
        return (1, 0);
    };
    if let Some(code) = status.code() {
        return (code, 0);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return (128 + signal, signal as u16);
        }
    }
    (128, 0)
}

fn collect_stats(pids: &[u32], memory_limit_bytes: u64) -> ContainerStats {
    #[cfg(target_os = "linux")]
    {
        let clk = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(1) as u64;
        let mut cpu_ticks = 0u64;
        let mut rss_pages = 0u64;
        let mut read_bytes = 0u64;
        let mut write_bytes = 0u64;
        let mut alive = 0u32;
        for pid in pids {
            let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok();
            let Some(stat) = stat else { continue };
            alive += 1;
            if let Some((utime, stime, rss)) = parse_stat(&stat) {
                cpu_ticks = cpu_ticks.saturating_add(utime).saturating_add(stime);
                rss_pages = rss_pages.saturating_add(rss);
            }
            if let Ok(io) = std::fs::read_to_string(format!("/proc/{pid}/io")) {
                read_bytes = read_bytes.saturating_add(proc_io_field(&io, "read_bytes"));
                write_bytes = write_bytes.saturating_add(proc_io_field(&io, "write_bytes"));
            }
        }
        return ContainerStats {
            cpu_usage_usec: cpu_ticks.saturating_mul(1_000_000) / clk,
            memory_usage_bytes: rss_pages.saturating_mul(page),
            memory_limit_bytes,
            blkio_read_bytes: read_bytes,
            blkio_write_bytes: write_bytes,
            pids: alive,
        };
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pids;
        ContainerStats {
            memory_limit_bytes,
            pids: pids.len() as u32,
            ..ContainerStats::default()
        }
    }
}

#[cfg(target_os = "linux")]
fn parse_stat(text: &str) -> Option<(u64, u64, u64)> {
    let rest = text.rsplit_once(')')?.1.trim_start();
    let parts: Vec<&str> = rest.split_whitespace().collect();
    // After `comm`, field 3 is `parts[0]`. utime is field 14, stime 15, rss 24.
    let utime = parts.get(11)?.parse().ok()?;
    let stime = parts.get(12)?.parse().ok()?;
    let rss = parts.get(21)?.parse().ok()?;
    Some((utime, stime, rss))
}

#[cfg(target_os = "linux")]
fn proc_io_field(text: &str, key: &str) -> u64 {
    text.lines()
        .find_map(|line| {
            let (k, v) = line.split_once(':')?;
            if k.trim() == key {
                v.trim().parse().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}
