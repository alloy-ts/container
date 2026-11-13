//! Public container lifecycle and the NAPI bindings that expose it to JavaScript.
//!
//! `Container::create` is async. The design table writes it as
//! `Container::create(opts) -> Result<Self>` because the value it returns is the
//! container, but the work (allocate resources, boot the microVM, connect the
//! guest agent) cannot block a runtime, so the method is `async` like `start`.

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;

use oci_spec::runtime::Spec;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{Error, Result};
use crate::guest::{AgentClient, ExitWait, ProcessIo};
use crate::hypervisor::{self, AgentLink, Hypervisor, VmConfig};
use crate::resource::ResourceManager;
use crate::types::{
    ContainerOpts, ContainerState, ContainerStats, ContainerStatus, ExecOpts, ExitStatus,
    LogStream, Signal,
};

/// One microVM and the guest agent connected to it.
pub struct Container {
    id: String,
    ip: Option<IpAddr>,
    bundle: Option<PathBuf>,
    inner: Mutex<Inner>,
}

struct Inner {
    opts: ContainerOpts,
    status: ContainerStatus,
    hypervisor: Option<Box<dyn Hypervisor>>,
    resources: ResourceManager,
    agent: AgentClient,
    primary_pid: Option<u32>,
    primary_exit: Option<ExitWait>,
    host_pid: Option<u32>,
    oci_version: String,
    cleaned: bool,
}

impl Container {
    /// Allocate host resources, boot the microVM, and connect to the guest
    /// init. The entrypoint is not executed until [`Container::start`].
    pub async fn create(opts: ContainerOpts) -> Result<Self> {
        Self::create_with(opts, hypervisor::platform()).await
    }

    pub(crate) async fn create_with(
        opts: ContainerOpts,
        mut hypervisor: Box<dyn Hypervisor>,
    ) -> Result<Self> {
        opts.validate()?;
        crate::guest::link_reference_guest();
        let resources = ResourceManager::plan(&opts)?;
        let vm = vm_config(&opts, &resources);
        let ip = resources.guest_ip();
        let id = opts.id.clone();
        let bundle = opts.bundle_path.clone();
        let oci_version = opts
            .oci_spec
            .as_ref()
            .map(|spec| spec.version().clone())
            .unwrap_or_else(|| "1.2.0".into());

        let booted = match hypervisor.boot(vm).await {
            Ok(booted) => booted,
            Err(err) => {
                hypervisor.shutdown().await.ok();
                return Err(err);
            }
        };
        let agent = match connect_link(booted.link).await {
            Ok(agent) => agent,
            Err(err) => {
                hypervisor.shutdown().await.ok();
                return Err(err);
            }
        };
        if let Err(err) = agent.prepare(&opts).await {
            let _ = agent.delete().await;
            hypervisor.shutdown().await.ok();
            return Err(err);
        }

        Ok(Self {
            id,
            ip,
            bundle,
            inner: Mutex::new(Inner {
                opts,
                status: ContainerStatus::Created,
                hypervisor: Some(hypervisor),
                resources,
                agent,
                primary_pid: None,
                primary_exit: None,
                host_pid: booted.host_pid,
                oci_version,
                cleaned: false,
            }),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn ip_addr(&self) -> Option<IpAddr> {
        self.ip
    }

    /// Run the container entrypoint. The container moves from `created` to `running`.
    pub async fn start(&mut self) -> Result<Process> {
        let (agent, opts) = {
            let inner = lock(&self.inner)?;
            if inner.status != ContainerStatus::Created {
                return Err(Error::InvalidState {
                    from: inner.status,
                    op: "start",
                });
            }
            (inner.agent.clone(), inner.opts.clone())
        };
        let io = agent.start(&opts).await?;
        let mut inner = lock(&self.inner)?;
        inner.status = ContainerStatus::Running;
        inner.primary_pid = Some(io.pid);
        inner.primary_exit = Some(io.exit.clone());
        Ok(Process::from_io(io, agent, opts.tty))
    }

    pub async fn state(&self) -> Result<ContainerState> {
        let agent = {
            let inner = lock(&self.inner)?;
            inner.agent.clone()
        };
        if let Ok((status, pid)) = agent.state().await {
            let mut inner = lock(&self.inner)?;
            if status == ContainerStatus::Stopped
                && matches!(
                    inner.status,
                    ContainerStatus::Running | ContainerStatus::Paused | ContainerStatus::Created
                )
            {
                inner.status = ContainerStatus::Stopped;
            }
            let mut snapshot = self.snapshot(&inner);
            if snapshot.pid.is_none() {
                snapshot.pid = pid;
            }
            return Ok(snapshot);
        }
        let inner = lock(&self.inner)?;
        Ok(self.snapshot(&inner))
    }

    /// Signal the primary process. `kill` before `start` is rejected.
    pub async fn kill(&self, signal: Signal) -> Result<()> {
        let (agent, pid) = {
            let inner = lock(&self.inner)?;
            if inner.status == ContainerStatus::Stopped {
                return Err(Error::InvalidState {
                    from: inner.status,
                    op: "kill",
                });
            }
            let Some(pid) = inner.primary_pid else {
                return Err(Error::InvalidState {
                    from: inner.status,
                    op: "kill",
                });
            };
            (inner.agent.clone(), pid)
        };
        agent.kill(pid, signal).await
    }

    /// Shut the guest down, stop the microVM, and release host resources.
    /// [`Drop`] does the same if the caller skips this.
    pub async fn delete(self) -> Result<()> {
        let (agent, hypervisor, mut resources) = {
            let mut inner = lock(&self.inner)?;
            if inner.cleaned {
                return Ok(());
            }
            inner.cleaned = true;
            (
                inner.agent.clone(),
                inner.hypervisor.take(),
                std::mem::replace(
                    &mut inner.resources,
                    ResourceManager::plan(&fallback_opts())?,
                ),
            )
        };
        // `resources` above is the real plan; the placeholder exists only so
        // the field stays inhabited for Drop. Release the real plan below.
        let _ = agent.delete().await;
        let hv_result = match hypervisor {
            Some(mut hv) => hv.shutdown().await,
            None => Ok(()),
        };
        resources.release();
        hv_result
    }

    pub async fn exec(&self, opts: ExecOpts) -> Result<Process> {
        if opts.args.is_empty() {
            return Err(Error::InvalidOpts(
                "exec args must contain a command".into(),
            ));
        }
        let agent = {
            let inner = lock(&self.inner)?;
            if inner.status != ContainerStatus::Running {
                return Err(Error::InvalidState {
                    from: inner.status,
                    op: "exec",
                });
            }
            inner.agent.clone()
        };
        let tty = opts.tty;
        let io = agent.exec(&opts).await?;
        Ok(Process::from_io(io, agent, tty))
    }

    /// Snapshot handle for the primary process exit.
    ///
    /// Split out of [`Container::wait`] so the NAPI layer can await the exit
    /// without holding the container mutex. Holding it across the whole wait is
    /// what let `container.wait()` block a concurrent `kill` / `delete`.
    pub(crate) fn exit_handle(&self) -> Result<ExitWait> {
        let inner = lock(&self.inner)?;
        inner.primary_exit.clone().ok_or(Error::InvalidState {
            from: inner.status,
            op: "wait",
        })
    }

    /// Record that the primary process has exited.
    pub(crate) fn mark_stopped(&self) -> Result<()> {
        let mut inner = lock(&self.inner)?;
        if matches!(
            inner.status,
            ContainerStatus::Running | ContainerStatus::Paused
        ) {
            inner.status = ContainerStatus::Stopped;
        }
        Ok(())
    }

    pub async fn wait(&self) -> Result<ExitStatus> {
        let mut exit = self.exit_handle()?;
        let status = exit.wait().await?;
        self.mark_stopped()?;
        Ok(status)
    }

    /// Snapshot of the primary process output retained by the guest init.
    pub async fn logs(&self) -> Result<LogStream> {
        let agent = {
            let inner = lock(&self.inner)?;
            inner.agent.clone()
        };
        let (stdout, stderr) = agent.logs().await?;
        Ok(LogStream {
            stdout: Box::new(std::io::Cursor::new(stdout)),
            stderr: Box::new(std::io::Cursor::new(stderr)),
        })
    }

    pub async fn stats(&self) -> Result<ContainerStats> {
        let agent = {
            let inner = lock(&self.inner)?;
            inner.agent.clone()
        };
        agent.stats().await
    }

    pub async fn pause(&mut self) -> Result<()> {
        let mut inner = lock(&self.inner)?;
        if inner.status != ContainerStatus::Running {
            return Err(Error::InvalidState {
                from: inner.status,
                op: "pause",
            });
        }
        let hv = inner
            .hypervisor
            .as_mut()
            .ok_or_else(|| Error::Hypervisor("hypervisor is gone".into()))?;
        hv.pause()?;
        inner.status = ContainerStatus::Paused;
        Ok(())
    }

    pub async fn resume(&mut self) -> Result<()> {
        let mut inner = lock(&self.inner)?;
        if inner.status != ContainerStatus::Paused {
            return Err(Error::InvalidState {
                from: inner.status,
                op: "resume",
            });
        }
        let hv = inner
            .hypervisor
            .as_mut()
            .ok_or_else(|| Error::Hypervisor("hypervisor is gone".into()))?;
        hv.resume()?;
        inner.status = ContainerStatus::Running;
        Ok(())
    }

    fn snapshot(&self, inner: &Inner) -> ContainerState {
        ContainerState {
            oci_version: inner.oci_version.clone(),
            id: self.id.clone(),
            status: inner.status,
            pid: inner.primary_pid.or(inner.host_pid),
            bundle: self.bundle.clone(),
            ip_addr: self.ip,
        }
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        if inner.cleaned {
            return;
        }
        inner.cleaned = true;
        if let Some(mut hv) = inner.hypervisor.take() {
            hv.shutdown_blocking();
        }
        inner.resources.release();
    }
}

/// A process started by [`Container::start`] or [`Container::exec`].
///
/// `stdin` / `stdout` / `stderr` are independent of the container borrow, so
/// several exec sessions can stream at once.
pub struct Process {
    pub pid: u32,
    pub stdin: Option<Box<dyn AsyncWrite + Unpin + Send>>,
    pub stdout: Option<Box<dyn AsyncRead + Unpin + Send>>,
    pub stderr: Option<Box<dyn AsyncRead + Unpin + Send>>,
    exit: ExitWait,
    agent: AgentClient,
    tty: bool,
}

impl Process {
    fn from_io(io: ProcessIo, agent: AgentClient, tty: bool) -> Self {
        Self {
            pid: io.pid,
            stdin: io.stdin,
            stdout: io.stdout,
            stderr: io.stderr,
            exit: io.exit,
            agent,
            tty,
        }
    }

    pub async fn wait(&mut self) -> Result<ExitStatus> {
        self.exit.wait().await
    }

    pub async fn kill(&self, signal: Signal) -> Result<()> {
        self.agent.kill(self.pid, signal).await
    }

    pub async fn resize_tty(&self, rows: u16, cols: u16) -> Result<()> {
        if !self.tty {
            return Err(Error::Agent(
                "process has no tty; resize_tty requires a tty process".into(),
            ));
        }
        self.agent.resize(self.pid, rows, cols).await
    }
}

fn lock(mutex: &Mutex<Inner>) -> Result<std::sync::MutexGuard<'_, Inner>> {
    mutex
        .lock()
        .map_err(|_| Error::Agent("container lock poisoned".into()))
}

fn vm_config(opts: &ContainerOpts, resources: &ResourceManager) -> VmConfig {
    VmConfig {
        id: opts.id.clone(),
        vcpus: opts.vcpus,
        memory_mb: opts.memory_mb,
        kernel_path: opts.kernel_path.clone(),
        initramfs_path: opts.initramfs_path.clone(),
        cmdline:
            "console=ttyS0 reboot=k panic=1 pci=off rw rootfstype=virtiofs root=rootfs init=/init"
                .to_string(),
        vsock_cid: hypervisor::alloc_guest_cid(),
        vsock_port: hypervisor::GUEST_AGENT_PORT,
        network: resources.network.clone(),
        shares: resources.shares.clone(),
    }
}

async fn connect_link(link: AgentLink) -> Result<AgentClient> {
    match link {
        AgentLink::Connected(stream) => AgentClient::connect_stream(stream).await,
        AgentLink::Vsock { cid, port } => {
            #[cfg(target_os = "linux")]
            {
                let stream = tokio::task::spawn_blocking(move || {
                    crate::hypervisor::linux::vsock::connect_blocking(
                        cid,
                        port,
                        std::time::Duration::from_secs(2),
                    )
                })
                .await
                .map_err(|err| Error::Hypervisor(format!("vsock connect task: {err}")))??;
                AgentClient::connect_stream(stream).await
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = (cid, port);
                Err(Error::Hypervisor(
                    "TODO: vsock connect is only implemented on Linux".into(),
                ))
            }
        }
    }
}

/// Used only to keep `Inner.resources` inhabited after `delete` moves the real
/// plan out. `delete` sets `cleaned` first, so this plan is never applied.
fn fallback_opts() -> ContainerOpts {
    ContainerOpts::builder("deleted")
        .rootfs(crate::types::RootfsSource::Directory(PathBuf::from("/")))
        .kernel_path("/var/lib/container-native/vmlinux")
        .args(["/bin/true"])
        .network(crate::types::NetworkConfig::none())
        .build()
        .expect("fallback options are valid")
}

// --- NAPI bindings -----------------------------------------------------------

use napi::bindgen_prelude::Buffer;
use napi_derive::napi;

fn to_napi(err: Error) -> napi::Error {
    napi::Error::from_reason(err.to_string())
}

#[napi(object, js_name = "Rootfs")]
pub struct JsRootfs {
    /// `directory`, `oci-bundle`, `erofs`, or `block`.
    pub kind: String,
    pub path: String,
}

#[napi(object, js_name = "Mount")]
pub struct JsMount {
    pub source: String,
    pub destination: String,
    pub fs_type: Option<String>,
    pub options: Option<Vec<String>>,
}

#[napi(object, js_name = "NetworkConfig")]
pub struct JsNetwork {
    /// `dedicated` (default), `none`, or `slirp`.
    pub mode: Option<String>,
    pub guest_ip: Option<String>,
    pub gateway: Option<String>,
    pub mac: Option<String>,
}

#[napi(object, js_name = "ContainerOpts")]
pub struct JsContainerOpts {
    pub id: String,
    pub rootfs: JsRootfs,
    pub kernel_path: String,
    /// Optional initramfs/initrd image path.
    pub initramfs_path: Option<String>,
    pub vcpus: Option<u32>,
    pub memory_mb: Option<u32>,
    pub args: Option<Vec<String>>,
    /// `KEY=VALUE` entries.
    pub env: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub mounts: Option<Vec<JsMount>>,
    pub network: Option<JsNetwork>,
    pub tty: Option<bool>,
    /// Raw OCI `config.json` as JSON. Preserved by
    /// [`container_opts_from_oci_bundle`] so that namespaces and the rest of the
    /// raw OCI configuration survive the trip through JavaScript and are
    /// forwarded to the guest over vsock.
    pub oci_spec: Option<String>,
    /// Bundle directory, when the options came from an OCI bundle.
    pub bundle_path: Option<String>,
}

#[napi(object, js_name = "ExecOpts")]
pub struct JsExecOpts {
    pub args: Vec<String>,
    pub env: Option<Vec<String>>,
    pub working_dir: Option<String>,
    pub tty: Option<bool>,
    pub stdin: Option<bool>,
    pub user: Option<String>,
}

#[napi(object, js_name = "ContainerState")]
pub struct JsContainerState {
    pub oci_version: String,
    pub id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub bundle: Option<String>,
    pub ip_addr: Option<String>,
}

#[napi(object, js_name = "ExitStatus")]
pub struct JsExitStatus {
    pub code: i32,
    pub signal: Option<i32>,
}

#[napi(object, js_name = "ContainerStats")]
pub struct JsContainerStats {
    pub cpu_usage_usec: i64,
    pub memory_usage_bytes: i64,
    pub memory_limit_bytes: i64,
    pub blkio_read_bytes: i64,
    pub blkio_write_bytes: i64,
    pub pids: u32,
}

#[napi(object, js_name = "Logs")]
pub struct JsLogs {
    pub stdout: String,
    pub stderr: String,
}

fn js_opts(opts: JsContainerOpts) -> Result<ContainerOpts> {
    use crate::types::{Mount, NetworkConfig, NetworkMode, RootfsSource};
    let rootfs = match opts.rootfs.kind.as_str() {
        "directory" | "dir" => RootfsSource::Directory(PathBuf::from(opts.rootfs.path)),
        "oci-bundle" | "bundle" => RootfsSource::OciBundle(PathBuf::from(opts.rootfs.path)),
        "erofs" => RootfsSource::Erofs(PathBuf::from(opts.rootfs.path)),
        "block" => RootfsSource::BlockDevice(PathBuf::from(opts.rootfs.path)),
        other => {
            return Err(Error::InvalidOpts(format!(
                "unknown rootfs kind {other} (expected directory, oci-bundle, erofs, or block)"
            )))
        }
    };
    let mut network = NetworkConfig::dedicated();
    if let Some(js) = opts.network {
        network.mode = match js.mode.as_deref().unwrap_or("dedicated") {
            "none" => NetworkMode::None,
            "slirp" => NetworkMode::Slirp,
            "dedicated" => NetworkMode::Dedicated,
            other => return Err(Error::InvalidOpts(format!("unknown network mode {other}"))),
        };
        if let Some(ip) = js.guest_ip {
            network.guest_ip = Some(
                ip.parse()
                    .map_err(|_| Error::InvalidOpts(format!("invalid guest_ip {ip}")))?,
            );
        }
        if let Some(ip) = js.gateway {
            network.gateway = Some(
                ip.parse()
                    .map_err(|_| Error::InvalidOpts(format!("invalid gateway {ip}")))?,
            );
        }
        network.mac = js.mac;
    }
    let mut builder = ContainerOpts::builder(opts.id)
        .rootfs(rootfs)
        .kernel_path(opts.kernel_path)
        .vcpus(opts.vcpus.unwrap_or(1))
        .memory_mb(u64::from(opts.memory_mb.unwrap_or(512)))
        .args(opts.args.unwrap_or_default())
        .working_dir(opts.working_dir.unwrap_or_else(|| "/".into()))
        .network(network);
    if let Some(initramfs) = opts.initramfs_path {
        builder = builder.initramfs_path(initramfs);
    }
    if let Some(tty) = opts.tty {
        builder = builder.tty(tty);
    }
    if let Some(env) = opts.env {
        for entry in env {
            if let Some((k, v)) = crate::types::split_env(&entry) {
                builder = builder.env(k, v);
            }
        }
    }
    if let Some(mounts) = opts.mounts {
        for mount in mounts {
            builder = builder.mount(Mount {
                source: PathBuf::from(mount.source),
                destination: PathBuf::from(mount.destination),
                fs_type: mount.fs_type.unwrap_or_else(|| "bind".into()),
                options: mount.options.unwrap_or_default(),
            });
        }
    }
    // The builder only carries the fields it knows about. Restore the raw OCI
    // configuration and bundle path so `Container::create` can forward
    // namespaces and `oci_config_json` to the guest instead of dropping them.
    if let Some(raw) = opts.oci_spec {
        let spec: Spec = serde_json::from_str(&raw)
            .map_err(|err| Error::InvalidOpts(format!("invalid ociSpec JSON: {err}")))?;
        builder = builder.oci_spec(spec);
    }
    if let Some(bundle) = opts.bundle_path {
        builder = builder.bundle_path(bundle);
    }
    builder.build()
}

fn js_state(state: ContainerState) -> JsContainerState {
    JsContainerState {
        oci_version: state.oci_version,
        id: state.id,
        status: state.status.as_str().into(),
        pid: state.pid,
        bundle: state.bundle.map(|p| p.display().to_string()),
        ip_addr: state.ip_addr.map(|ip| ip.to_string()),
    }
}

fn js_exit(status: ExitStatus) -> JsExitStatus {
    JsExitStatus {
        code: status.code,
        signal: status.signal.map(Signal::as_raw),
    }
}

/// Parse an OCI bundle (`config.json`) into the object `Container.create` accepts.
/// This does not boot a microVM.
#[napi]
#[allow(dead_code)]
pub fn container_opts_from_oci_bundle(bundle_path: String) -> napi::Result<JsContainerOpts> {
    let opts =
        ContainerOpts::from_oci_bundle(std::path::Path::new(&bundle_path)).map_err(to_napi)?;
    // Carry the raw OCI configuration and bundle directory out to JavaScript so
    // `Container.create` can hand them back to the builder (see `js_opts`).
    let oci_spec = opts
        .oci_spec
        .as_ref()
        .and_then(|spec| serde_json::to_string(spec).ok());
    let bundle = opts.bundle_path.as_ref().map(|p| p.display().to_string());
    Ok(JsContainerOpts {
        id: opts.id,
        rootfs: JsRootfs {
            kind: match &opts.rootfs {
                crate::types::RootfsSource::Directory(_) => "directory".into(),
                crate::types::RootfsSource::OciBundle(_) => "oci-bundle".into(),
                crate::types::RootfsSource::Erofs(_) => "erofs".into(),
                crate::types::RootfsSource::BlockDevice(_) => "block".into(),
            },
            path: opts.rootfs.path().display().to_string(),
        },
        kernel_path: opts.kernel_path.display().to_string(),
        initramfs_path: opts
            .initramfs_path
            .as_ref()
            .map(|p| p.display().to_string()),
        vcpus: Some(opts.vcpus),
        memory_mb: Some(u32::try_from(opts.memory_mb).unwrap_or(u32::MAX)),
        args: Some(opts.args),
        env: Some(
            opts.env
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect(),
        ),
        working_dir: Some(opts.working_dir.display().to_string()),
        mounts: Some(
            opts.mounts
                .into_iter()
                .map(|m| JsMount {
                    source: m.source.display().to_string(),
                    destination: m.destination.display().to_string(),
                    fs_type: Some(m.fs_type),
                    options: Some(m.options),
                })
                .collect(),
        ),
        network: Some(JsNetwork {
            mode: Some(
                match opts.network.mode {
                    crate::types::NetworkMode::Dedicated => "dedicated",
                    crate::types::NetworkMode::None => "none",
                    crate::types::NetworkMode::Slirp => "slirp",
                }
                .into(),
            ),
            guest_ip: opts.network.guest_ip.map(|ip| ip.to_string()),
            gateway: opts.network.gateway.map(|ip| ip.to_string()),
            mac: opts.network.mac,
        }),
        tty: Some(opts.tty),
        oci_spec,
        bundle_path: bundle,
    })
}

#[napi(js_name = "Container")]
pub struct JsContainer {
    inner: tokio::sync::Mutex<Option<Container>>,
}

#[napi]
impl JsContainer {
    /// Boot a microVM from options. Resolves to a `Container` handle.
    #[napi(factory)]
    pub async fn create(opts: JsContainerOpts) -> napi::Result<Self> {
        let opts = js_opts(opts).map_err(to_napi)?;
        let container = Container::create(opts).await.map_err(to_napi)?;
        Ok(Self {
            inner: tokio::sync::Mutex::new(Some(container)),
        })
    }

    #[napi]
    pub async fn id(&self) -> napi::Result<String> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        Ok(container.id().to_string())
    }

    #[napi]
    pub async fn ip_addr(&self) -> napi::Result<Option<String>> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        Ok(container.ip_addr().map(|ip| ip.to_string()))
    }

    #[napi]
    pub async fn start(&self) -> napi::Result<JsProcess> {
        let mut guard = self.inner.lock().await;
        let container = guard.as_mut().ok_or_else(deleted)?;
        let process = container.start().await.map_err(to_napi)?;
        Ok(JsProcess::from_process(process))
    }

    #[napi]
    pub async fn state(&self) -> napi::Result<JsContainerState> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        Ok(js_state(container.state().await.map_err(to_napi)?))
    }

    /// `signal` is a POSIX number (`15` TERM, `9` KILL, `2` INT, ...).
    #[napi]
    pub async fn kill(&self, signal: i32) -> napi::Result<()> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        container
            .kill(Signal::from_raw(signal))
            .await
            .map_err(to_napi)
    }

    #[napi]
    pub async fn delete(&self) -> napi::Result<()> {
        let container = self.inner.lock().await.take().ok_or_else(deleted)?;
        container.delete().await.map_err(to_napi)
    }

    #[napi]
    pub async fn exec(&self, opts: JsExecOpts) -> napi::Result<JsProcess> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        let mut exec = ExecOpts::new(opts.args);
        if let Some(env) = opts.env {
            for entry in env {
                if let Some((k, v)) = crate::types::split_env(&entry) {
                    exec = exec.env(k, v);
                }
            }
        }
        if let Some(dir) = opts.working_dir {
            exec = exec.working_dir(dir);
        }
        if opts.tty.unwrap_or(false) {
            exec = exec.tty(true);
        }
        if opts.stdin.unwrap_or(false) {
            exec = exec.stdin(true);
        }
        if let Some(user) = opts.user {
            exec = exec.user(user);
        }
        let process = container.exec(exec).await.map_err(to_napi)?;
        Ok(JsProcess::from_process(process))
    }

    #[napi]
    pub async fn wait(&self) -> napi::Result<JsExitStatus> {
        // Snapshot the exit handle under a brief lock and then await it *without*
        // the guard held. Holding the outer mutex across the whole wait is what
        // let a concurrent `kill` / `delete` starve: neither could reach the
        // container to make the process exit, so `wait` never returned.
        let mut exit = {
            let guard = self.inner.lock().await;
            let container = guard.as_ref().ok_or_else(deleted)?;
            container.exit_handle().map_err(to_napi)?
        };
        let status = exit.wait().await.map_err(to_napi)?;
        {
            // Re-lock only briefly to record the stop. The container may have
            // been deleted while we awaited, in which case there is nothing to
            // update.
            let guard = self.inner.lock().await;
            if let Some(container) = guard.as_ref() {
                container.mark_stopped().map_err(to_napi)?;
            }
        }
        Ok(js_exit(status))
    }

    #[napi]
    pub async fn logs(&self) -> napi::Result<JsLogs> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        let mut stream = container.logs().await.map_err(to_napi)?;
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        stream
            .stdout
            .read_to_end(&mut stdout)
            .await
            .map_err(|err| to_napi(Error::Io(err)))?;
        stream
            .stderr
            .read_to_end(&mut stderr)
            .await
            .map_err(|err| to_napi(Error::Io(err)))?;
        Ok(JsLogs {
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        })
    }

    #[napi]
    pub async fn stats(&self) -> napi::Result<JsContainerStats> {
        let guard = self.inner.lock().await;
        let container = guard.as_ref().ok_or_else(deleted)?;
        let stats = container.stats().await.map_err(to_napi)?;
        Ok(JsContainerStats {
            cpu_usage_usec: stats.cpu_usage_usec as i64,
            memory_usage_bytes: stats.memory_usage_bytes as i64,
            memory_limit_bytes: stats.memory_limit_bytes as i64,
            blkio_read_bytes: stats.blkio_read_bytes as i64,
            blkio_write_bytes: stats.blkio_write_bytes as i64,
            pids: stats.pids,
        })
    }

    #[napi]
    pub async fn pause(&self) -> napi::Result<()> {
        let mut guard = self.inner.lock().await;
        let container = guard.as_mut().ok_or_else(deleted)?;
        container.pause().await.map_err(to_napi)
    }

    #[napi]
    pub async fn resume(&self) -> napi::Result<()> {
        let mut guard = self.inner.lock().await;
        let container = guard.as_mut().ok_or_else(deleted)?;
        container.resume().await.map_err(to_napi)
    }
}

fn deleted() -> napi::Error {
    napi::Error::from_reason("container has been deleted")
}

#[napi(js_name = "Process")]
pub struct JsProcess {
    pid: u32,
    agent: AgentClient,
    exit: tokio::sync::Mutex<ExitWait>,
    tty: bool,
    stdin: tokio::sync::Mutex<Option<Box<dyn AsyncWrite + Unpin + Send>>>,
    stdout: tokio::sync::Mutex<Option<Box<dyn AsyncRead + Unpin + Send>>>,
    stderr: tokio::sync::Mutex<Option<Box<dyn AsyncRead + Unpin + Send>>>,
}

impl JsProcess {
    fn from_process(process: Process) -> Self {
        Self {
            pid: process.pid,
            agent: process.agent,
            exit: tokio::sync::Mutex::new(process.exit),
            tty: process.tty,
            stdin: tokio::sync::Mutex::new(process.stdin),
            stdout: tokio::sync::Mutex::new(process.stdout),
            stderr: tokio::sync::Mutex::new(process.stderr),
        }
    }
}

#[napi]
impl JsProcess {
    #[napi(getter)]
    pub fn pid(&self) -> u32 {
        self.pid
    }

    #[napi]
    pub async fn wait(&self) -> napi::Result<JsExitStatus> {
        let mut exit = self.exit.lock().await.clone();
        Ok(js_exit(exit.wait().await.map_err(to_napi)?))
    }

    #[napi]
    pub async fn kill(&self, signal: i32) -> napi::Result<()> {
        self.agent
            .kill(self.pid, Signal::from_raw(signal))
            .await
            .map_err(to_napi)
    }

    #[napi]
    pub async fn resize_tty(&self, rows: u32, cols: u32) -> napi::Result<()> {
        if !self.tty {
            return Err(to_napi(Error::Agent(
                "process has no tty; resize_tty requires a tty process".into(),
            )));
        }
        self.agent
            .resize(self.pid, rows as u16, cols as u16)
            .await
            .map_err(to_napi)
    }

    #[napi]
    pub async fn write_stdin(&self, data: Buffer) -> napi::Result<()> {
        let mut stdin = self.stdin.lock().await;
        let Some(pipe) = stdin.as_mut() else {
            return Err(napi::Error::from_reason("process has no stdin"));
        };
        pipe.write_all(&data)
            .await
            .map_err(|err| to_napi(Error::Io(err)))
    }

    #[napi]
    pub async fn close_stdin(&self) -> napi::Result<()> {
        let mut stdin = self.stdin.lock().await;
        if let Some(pipe) = stdin.as_mut() {
            pipe.shutdown()
                .await
                .map_err(|err| to_napi(Error::Io(err)))?;
        }
        *stdin = None;
        Ok(())
    }

    #[napi]
    pub async fn read_stdout(&self, max: u32) -> napi::Result<Option<Buffer>> {
        read_some(&self.stdout, max).await
    }

    #[napi]
    pub async fn read_stderr(&self, max: u32) -> napi::Result<Option<Buffer>> {
        read_some(&self.stderr, max).await
    }
}

async fn read_some(
    slot: &tokio::sync::Mutex<Option<Box<dyn AsyncRead + Unpin + Send>>>,
    max: u32,
) -> napi::Result<Option<Buffer>> {
    let mut guard = slot.lock().await;
    let Some(reader) = guard.as_mut() else {
        return Ok(None);
    };
    let max = max.clamp(1, 1024 * 1024) as usize;
    let mut buf = vec![0u8; max];
    let n = reader
        .read(&mut buf)
        .await
        .map_err(|err| to_napi(Error::Io(err)))?;
    if n == 0 {
        return Ok(None);
    }
    buf.truncate(n);
    Ok(Some(Buffer::from(buf)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypervisor::loopback::{LoopbackHypervisor, SHUTDOWNS};
    use crate::types::{NetworkConfig, RootfsSource};
    use std::sync::atomic::Ordering;

    fn scratch(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cn-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    async fn boot(args: &[&str]) -> Container {
        let dir = scratch("root");
        let opts = ContainerOpts::builder("demo")
            .rootfs(RootfsSource::Directory(dir))
            .kernel_path("/var/lib/container-native/vmlinux")
            .vcpus(2)
            .memory_mb(128)
            .args(args.iter().copied())
            .network(NetworkConfig::none())
            .build()
            .unwrap();
        Container::create_with(opts, Box::new(LoopbackHypervisor::new()))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn echo_lifecycle_stats_logs_and_drop() {
        let before = SHUTDOWNS.load(Ordering::Relaxed);
        let mut container = boot(&["/bin/echo", "hello"]).await;
        assert_eq!(container.id(), "demo");
        assert!(container.ip_addr().is_none());
        let created = container.state().await.unwrap();
        assert_eq!(created.status, ContainerStatus::Created);

        let mut process = container.start().await.unwrap();
        let status = process.wait().await.unwrap();
        assert!(status.success(), "{status:?}");
        let mut out = Vec::new();
        process
            .stdout
            .as_mut()
            .unwrap()
            .read_to_end(&mut out)
            .await
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out).trim(), "hello");

        let stopped = container.state().await.unwrap();
        assert_eq!(stopped.status, ContainerStatus::Stopped);
        let logs = container.logs().await.unwrap();
        let mut logged = Vec::new();
        let mut stdout = logs.stdout;
        stdout.read_to_end(&mut logged).await.unwrap();
        assert!(String::from_utf8_lossy(&logged).contains("hello"));

        container.delete().await.unwrap();
        assert!(SHUTDOWNS.load(Ordering::Relaxed) > before);
    }

    #[tokio::test]
    async fn exec_pause_and_signal() {
        let mut container = boot(&["/bin/sleep", "30"]).await;
        let primary = container.start().await.unwrap();
        container.pause().await.unwrap();
        assert_eq!(
            container.state().await.unwrap().status,
            ContainerStatus::Paused
        );
        container.resume().await.unwrap();

        let mut extra = container
            .exec(ExecOpts::new(["/bin/sh", "-c", "echo side; exit 3"]))
            .await
            .unwrap();
        let extra_status = extra.wait().await.unwrap();
        assert_eq!(extra_status.code, 3);
        let mut side = Vec::new();
        extra
            .stdout
            .as_mut()
            .unwrap()
            .read_to_end(&mut side)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&side).contains("side"));

        let stats = container.stats().await.unwrap();
        assert!(stats.pids >= 1, "{stats:?}");
        assert!(stats.memory_limit_bytes >= 128 * 1024 * 1024);

        primary.kill(Signal::Kill).await.unwrap();
        let status = container.wait().await.unwrap();
        assert!(status.code != 0);
        assert_eq!(
            container.state().await.unwrap().status,
            ContainerStatus::Stopped
        );
    }

    #[tokio::test]
    async fn tty_resize_and_stdin() {
        let mut container = boot(&["/bin/sleep", "30"]).await;
        let primary = container.start().await.unwrap();
        let err = primary.resize_tty(24, 80).await.unwrap_err();
        assert!(err.to_string().contains("tty"), "{err}");

        let mut cat = container
            .exec(ExecOpts::new(["/bin/cat"]).tty(false).stdin(true))
            .await
            .unwrap();
        cat.stdin
            .as_mut()
            .unwrap()
            .write_all(b"abc\n")
            .await
            .unwrap();
        cat.stdin.as_mut().unwrap().shutdown().await.unwrap();
        let mut out = Vec::new();
        cat.stdout
            .as_mut()
            .unwrap()
            .read_to_end(&mut out)
            .await
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "abc\n");
        assert!(cat.wait().await.unwrap().success());

        let mut tty = container
            .exec(ExecOpts::new(["/bin/sleep", "30"]).tty(true))
            .await
            .unwrap();
        tty.resize_tty(40, 120).await.unwrap();
        tty.kill(Signal::Kill).await.unwrap();
        let _ = tty.wait().await.unwrap();
        let _ = primary;
    }

    #[tokio::test]
    async fn rejects_illegal_transitions() {
        let mut container = boot(&["/bin/true"]).await;
        let err = match container.exec(ExecOpts::new(["/bin/true"])).await {
            Err(err) => err,
            Ok(_) => panic!("exec before start should fail"),
        };
        assert!(err.to_string().contains("exec"), "{err}");
        let err = match container.pause().await {
            Err(err) => err,
            Ok(()) => panic!("pause before start should fail"),
        };
        assert!(err.to_string().contains("pause"), "{err}");
    }

    #[tokio::test]
    async fn rejects_unsupported_isolation_instead_of_running_unconfined() {
        let dir = scratch("isolated-root");
        let opts = ContainerOpts::builder("isolated")
            .rootfs(RootfsSource::Directory(dir))
            .args(["/bin/true"])
            .mount(crate::types::Mount::bind("/tmp", "/data"))
            .network(NetworkConfig::none())
            .build()
            .unwrap();

        let err = match Container::create_with(opts, Box::new(LoopbackHypervisor::new())).await {
            Err(err) => err,
            Ok(_) => panic!("unsupported OCI isolation must fail closed"),
        };
        assert!(err.to_string().contains("refusing to run"), "{err}");
    }

    #[test]
    fn builder_rejects_a_bad_id() {
        let err = ContainerOpts::builder("has space")
            .rootfs(RootfsSource::Directory(PathBuf::from("/")))
            .args(["/bin/true"])
            .build()
            .unwrap_err();
        assert!(err.to_string().contains("id"), "{err}");
    }

    #[test]
    fn from_oci_bundle_maps_process_resources_and_annotations() {
        let dir = scratch("bundle");
        std::fs::create_dir_all(dir.join("rootfs")).unwrap();
        let config = r#"{
            "ociVersion": "1.2.0",
            "process": {
                "terminal": false,
                "user": {"uid": 0, "gid": 0},
                "args": ["/bin/sh", "-c", "echo hi"],
                "env": ["PATH=/usr/bin", "HOME=/root"],
                "cwd": "/work"
            },
            "root": {"path": "rootfs", "readonly": false},
            "mounts": [
                {"destination": "/data", "type": "bind", "source": "/tmp", "options": ["rbind", "rw"]}
            ],
            "linux": {
                "resources": {
                    "memory": {"limit": 536870912},
                    "cpu": {"quota": 200000, "period": 100000}
                },
                "namespaces": [
                    {"type": "pid"},
                    {"type": "mount"}
                ]
            },
            "annotations": {
                "org.container.kernel": "/tmp/vmlinuz",
                "org.container.vcpus": "2"
            }
        }"#;
        std::fs::write(dir.join("config.json"), config).unwrap();
        let opts = ContainerOpts::from_oci_bundle(&dir).unwrap();
        assert_eq!(opts.args, vec!["/bin/sh", "-c", "echo hi"]);
        assert_eq!(opts.working_dir, PathBuf::from("/work"));
        assert!(opts.env.iter().any(|(k, v)| k == "PATH" && v == "/usr/bin"));
        assert_eq!(opts.kernel_path, PathBuf::from("/tmp/vmlinuz"));
        assert_eq!(opts.vcpus, 2);
        assert_eq!(opts.memory_mb, 512);
        assert_eq!(opts.mounts.len(), 1);
        assert!(opts.oci_spec.is_some());
        assert_eq!(opts.rootfs.path(), dir.join("rootfs"));
    }

    #[test]
    fn oci_bundle_survives_the_js_round_trip() {
        let dir = scratch("js-bundle");
        std::fs::create_dir_all(dir.join("rootfs")).unwrap();
        let config = r#"{
            "ociVersion": "1.2.0",
            "process": {
                "terminal": false,
                "user": {"uid": 0, "gid": 0},
                "args": ["/bin/sh", "-c", "echo hi"],
                "cwd": "/work"
            },
            "root": {"path": "rootfs", "readonly": false},
            "linux": {
                "namespaces": [{"type": "pid"}, {"type": "mount"}]
            }
        }"#;
        std::fs::write(dir.join("config.json"), config).unwrap();

        // Mirrors `containerOptsFromOciBundle(dir)` followed by
        // `Container.create(opts)`.
        let js = container_opts_from_oci_bundle(dir.display().to_string()).unwrap();
        let opts = js_opts(js).unwrap();

        let spec = opts
            .oci_spec
            .as_ref()
            .expect("oci_spec must survive the JS round trip");
        assert_eq!(crate::types::namespaces_from_spec(spec).len(), 2);
        assert_eq!(opts.bundle_path.as_deref(), Some(dir.as_path()));
    }

    #[tokio::test]
    async fn js_wait_does_not_starve_a_concurrent_kill() {
        // Regression: the NAPI `wait` used to hold the container mutex across
        // the whole await, so a concurrent `kill` could never reach the process
        // to make it exit and `wait` deadlocked.
        let mut container = boot(&["/bin/sleep", "30"]).await;
        let _primary = container.start().await.unwrap();
        let js = JsContainer {
            inner: tokio::sync::Mutex::new(Some(container)),
        };

        let status = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let (status, killed) = tokio::join!(js.wait(), js.kill(9));
            killed.unwrap();
            status.unwrap()
        })
        .await
        .expect("wait() must not block a concurrent kill()");

        assert!(status.code != 0, "SIGKILL should not report success");
    }
}
