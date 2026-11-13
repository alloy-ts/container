use std::net::IpAddr;
use std::path::{Path, PathBuf};

use oci_spec::runtime::Spec;

use crate::error::{Error, Result};

/// Where the container root filesystem comes from.
///
/// One microVM consumes exactly one of these. The Linux backend shares a
/// directory through virtio-fs, or attaches an erofs / block image as a
/// virtio-blk disk (device emulation is still a TODO on that backend).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootfsSource {
    /// Unpacked OCI bundle directory (`config.json` plus a rootfs).
    OciBundle(PathBuf),
    /// EROFS image file.
    Erofs(PathBuf),
    /// Host block device.
    BlockDevice(PathBuf),
    /// Plain directory shared into the guest.
    Directory(PathBuf),
}

impl RootfsSource {
    pub fn path(&self) -> &Path {
        match self {
            Self::OciBundle(p) | Self::Erofs(p) | Self::BlockDevice(p) | Self::Directory(p) => p,
        }
    }
}

/// A mount forwarded to the guest init (OCI mount entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mount {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub fs_type: String,
    pub options: Vec<String>,
}

impl Mount {
    pub fn bind(source: impl Into<PathBuf>, destination: impl Into<PathBuf>) -> Self {
        Self {
            source: source.into(),
            destination: destination.into(),
            fs_type: "bind".into(),
            options: vec!["rbind".into(), "rw".into()],
        }
    }
}

/// Per-microVM network attachment. Apple container gives every container its own IP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// Dedicated TAP device and guest IP.
    Dedicated,
    /// No network device.
    None,
    /// User-mode networking. The Linux backend records this and does not
    /// implement slirp yet (see the hypervisor TODO).
    Slirp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    /// Optional CIDR describing the host subnet, e.g. `192.168.127.0/24`.
    pub cidr: Option<String>,
    pub guest_ip: Option<IpAddr>,
    pub gateway: Option<IpAddr>,
    /// Optional MAC address (`aa:bb:cc:dd:ee:ff`). Assigned when omitted.
    pub mac: Option<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Dedicated,
            cidr: None,
            guest_ip: None,
            gateway: None,
            mac: None,
        }
    }
}

impl NetworkConfig {
    pub fn none() -> Self {
        Self {
            mode: NetworkMode::None,
            ..Self::default()
        }
    }

    pub fn dedicated() -> Self {
        Self::default()
    }
}

/// Builder-style configuration for one microVM container.
///
/// Fields are public so callers can tweak a value after
/// [`ContainerOpts::from_oci_bundle`]. Prefer the builder for new code.
#[derive(Debug, Clone)]
pub struct ContainerOpts {
    pub id: String,
    pub rootfs: RootfsSource,
    pub kernel_path: PathBuf,
    /// Optional initramfs/initrd image. It carries the guest `/init` for a full
    /// boot; when `None` the kernel runs with only its built-in initramfs.
    pub initramfs_path: Option<PathBuf>,
    pub vcpus: u32,
    pub memory_mb: u64,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub working_dir: PathBuf,
    pub mounts: Vec<Mount>,
    pub network: NetworkConfig,
    /// Raw OCI config when the options were loaded from a bundle, or supplied
    /// by the caller. Forwarded to the guest init over vsock.
    pub oci_spec: Option<Spec>,
    /// Bundle directory, when known. Reported by [`crate::ContainerState`].
    pub bundle_path: Option<PathBuf>,
    pub tty: bool,
    pub attach_stdin: bool,
    pub attach_stdout: bool,
    pub attach_stderr: bool,
}

impl ContainerOpts {
    pub fn builder(id: impl Into<String>) -> ContainerOptsBuilder {
        ContainerOptsBuilder::new(id)
    }

    /// Parse `bundle/config.json` with the `oci-spec` crate and map the
    /// process, root, mounts, resources, and annotations into options.
    ///
    /// The container id defaults to the bundle directory name. Kernel path and
    /// vCPU count are read from the `org.container.kernel` and
    /// `org.container.vcpus` annotations when present; otherwise the kernel
    /// path defaults to `/var/lib/container-native/vmlinux` and vCPUs are
    /// derived from the OCI CPU quota (or 1).
    pub fn from_oci_bundle(bundle_path: &Path) -> Result<Self> {
        let config_path = bundle_path.join("config.json");
        let spec = Spec::load(&config_path)?;
        let mut opts = Self::from_spec(spec, Some(bundle_path.to_path_buf()))?;
        opts.bundle_path = Some(bundle_path.to_path_buf());
        opts.validate()?;
        Ok(opts)
    }

    pub fn validate(&self) -> Result<()> {
        if !valid_id(&self.id) {
            return Err(Error::InvalidOpts(format!(
                "container id {:?} must match [A-Za-z0-9][A-Za-z0-9_.-]{{0,127}}",
                self.id
            )));
        }
        if self.vcpus == 0 || self.vcpus > 256 {
            return Err(Error::InvalidOpts(format!(
                "vcpus must be in 1..=256 (got {})",
                self.vcpus
            )));
        }
        if self.memory_mb == 0 {
            return Err(Error::InvalidOpts("memory_mb must be at least 1".into()));
        }
        if self.kernel_path.as_os_str().is_empty() {
            return Err(Error::InvalidOpts("kernel_path is required".into()));
        }
        if self.rootfs.path().as_os_str().is_empty() {
            return Err(Error::InvalidOpts("rootfs path is required".into()));
        }
        if self.working_dir.as_os_str().is_empty() {
            return Err(Error::InvalidOpts("working_dir is required".into()));
        }
        if self.args.is_empty() {
            return Err(Error::InvalidOpts(
                "args must contain the entrypoint".into(),
            ));
        }
        if self.args.iter().any(|a| a.is_empty()) {
            return Err(Error::InvalidOpts(
                "args must not contain empty strings".into(),
            ));
        }
        Ok(())
    }

    fn from_spec(spec: Spec, bundle: Option<PathBuf>) -> Result<Self> {
        let process = spec
            .process()
            .clone()
            .ok_or_else(|| Error::Oci("OCI config.json has no process section".into()))?;
        let args = process
            .args()
            .clone()
            .filter(|args| !args.is_empty())
            .ok_or_else(|| Error::Oci("OCI process.args is empty".into()))?;
        let env = process
            .env()
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| split_env(&entry))
            .collect();
        let working_dir = process.cwd().clone();
        let tty = process.terminal().unwrap_or(false);

        let rootfs = if let Some(root) = spec.root() {
            let path = resolve_bundle_path(bundle.as_deref(), root.path());
            RootfsSource::Directory(path)
        } else if let Some(bundle) = bundle.as_ref() {
            RootfsSource::OciBundle(bundle.clone())
        } else {
            return Err(Error::Oci("OCI config.json has no root path".into()));
        };

        let mounts = spec
            .mounts()
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|m| Mount {
                source: m.source().clone().unwrap_or_default(),
                destination: m.destination().clone(),
                fs_type: m.typ().clone().unwrap_or_else(|| "bind".into()),
                options: m.options().clone().unwrap_or_default(),
            })
            .collect();

        let annotations = spec.annotations().clone().unwrap_or_default();
        let kernel_path = annotations
            .get("org.container.kernel")
            .or_else(|| annotations.get("io.container.kernel"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/container-native/vmlinux"));
        let initramfs_path = annotations
            .get("org.container.initramfs")
            .or_else(|| annotations.get("io.container.initramfs"))
            .map(PathBuf::from);

        let mut vcpus = annotations
            .get("org.container.vcpus")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(1);
        let mut memory_mb = 512u64;
        if let Some(linux) = spec.linux() {
            if let Some(resources) = linux.resources() {
                if let Some(memory) = resources.memory() {
                    if let Some(limit) = memory.limit() {
                        if limit > 0 {
                            memory_mb = ((limit as u64) + (1024 * 1024) - 1) / (1024 * 1024);
                            memory_mb = memory_mb.max(1);
                        }
                    }
                }
                if annotations.get("org.container.vcpus").is_none() {
                    if let Some(cpu) = resources.cpu() {
                        if let (Some(quota), Some(period)) = (cpu.quota(), cpu.period()) {
                            if quota > 0 && period > 0 {
                                let q = quota as u64;
                                vcpus = ((q + period - 1) / period).clamp(1, 256) as u32;
                            }
                        }
                    }
                }
            }
        }

        let id = bundle
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .filter(|n| valid_id(n))
            .unwrap_or_else(|| "container".into());

        Ok(Self {
            id,
            rootfs,
            kernel_path,
            initramfs_path,
            vcpus,
            memory_mb,
            args,
            env,
            working_dir,
            mounts,
            network: NetworkConfig::dedicated(),
            oci_spec: Some(spec),
            bundle_path: bundle,
            tty,
            attach_stdin: tty,
            attach_stdout: true,
            attach_stderr: !tty,
        })
    }
}

/// Fluent builder for [`ContainerOpts`].
#[derive(Debug, Clone)]
pub struct ContainerOptsBuilder {
    inner: ContainerOpts,
}

impl ContainerOptsBuilder {
    fn new(id: impl Into<String>) -> Self {
        Self {
            inner: ContainerOpts {
                id: id.into(),
                rootfs: RootfsSource::Directory(PathBuf::new()),
                kernel_path: PathBuf::from("/var/lib/container-native/vmlinux"),
                initramfs_path: None,
                vcpus: 1,
                memory_mb: 512,
                args: Vec::new(),
                env: Vec::new(),
                working_dir: PathBuf::from("/"),
                mounts: Vec::new(),
                network: NetworkConfig::dedicated(),
                oci_spec: None,
                bundle_path: None,
                tty: false,
                attach_stdin: false,
                attach_stdout: true,
                attach_stderr: true,
            },
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.inner.id = id.into();
        self
    }

    pub fn rootfs(mut self, rootfs: RootfsSource) -> Self {
        self.inner.rootfs = rootfs;
        self
    }

    pub fn kernel_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.inner.kernel_path = path.into();
        self
    }

    pub fn initramfs_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.inner.initramfs_path = Some(path.into());
        self
    }

    pub fn vcpus(mut self, vcpus: u32) -> Self {
        self.inner.vcpus = vcpus;
        self
    }

    pub fn memory_mb(mut self, memory_mb: u64) -> Self {
        self.inner.memory_mb = memory_mb;
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.inner.args = args.into_iter().map(Into::into).collect();
        self
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.inner.args.push(arg.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner.env.push((key.into(), value.into()));
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.inner.working_dir = dir.into();
        self
    }

    pub fn mount(mut self, mount: Mount) -> Self {
        self.inner.mounts.push(mount);
        self
    }

    pub fn network(mut self, network: NetworkConfig) -> Self {
        self.inner.network = network;
        self
    }

    pub fn oci_spec(mut self, spec: Spec) -> Self {
        self.inner.oci_spec = Some(spec);
        self
    }

    pub fn bundle_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.inner.bundle_path = Some(path.into());
        self
    }

    pub fn tty(mut self, tty: bool) -> Self {
        self.inner.tty = tty;
        if tty {
            self.inner.attach_stdin = true;
            self.inner.attach_stderr = false;
        }
        self
    }

    pub fn attach_stdin(mut self, attach: bool) -> Self {
        self.inner.attach_stdin = attach;
        self
    }

    pub fn build(self) -> Result<ContainerOpts> {
        self.inner.validate()?;
        Ok(self.inner)
    }
}

/// Options for [`crate::Container::exec`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOpts {
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub working_dir: Option<PathBuf>,
    pub tty: bool,
    pub stdin: bool,
    pub stdout: bool,
    pub stderr: bool,
    pub user: Option<String>,
}

impl ExecOpts {
    pub fn new<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            args: args.into_iter().map(Into::into).collect(),
            env: Vec::new(),
            working_dir: None,
            tty: false,
            stdin: false,
            stdout: true,
            stderr: true,
            user: None,
        }
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn tty(mut self, tty: bool) -> Self {
        self.tty = tty;
        if tty {
            self.stdin = true;
            self.stderr = false;
        }
        self
    }

    pub fn stdin(mut self, stdin: bool) -> Self {
        self.stdin = stdin;
        self
    }

    pub fn user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }
}

/// OCI runtime lifecycle status, plus the optional `paused` extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerStatus {
    Creating,
    Created,
    Running,
    Paused,
    Stopped,
}

impl ContainerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Creating => "creating",
            Self::Created => "created",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
        }
    }
}

impl std::fmt::Display for ContainerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ContainerStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "creating" => Some(Self::Creating),
            "created" => Some(Self::Created),
            "running" => Some(Self::Running),
            "paused" => Some(Self::Paused),
            "stopped" => Some(Self::Stopped),
            _ => None,
        }
    }
}

/// OCI-style state query result, plus the microVM's dedicated IP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerState {
    pub oci_version: String,
    pub id: String,
    pub status: ContainerStatus,
    pub pid: Option<u32>,
    pub bundle: Option<PathBuf>,
    pub ip_addr: Option<IpAddr>,
}

/// Guest cgroup / process metrics collected over vsock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContainerStats {
    pub cpu_usage_usec: u64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
    pub blkio_read_bytes: u64,
    pub blkio_write_bytes: u64,
    pub pids: u32,
}

/// Buffered stdout/stderr for a container that was started without attached pipes,
/// or a snapshot of what the guest init retained.
pub struct LogStream {
    pub stdout: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
    pub stderr: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
}

/// Exit of a primary or exec'd process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitStatus {
    pub code: i32,
    pub signal: Option<Signal>,
}

impl ExitStatus {
    pub fn success(self) -> bool {
        self.code == 0 && self.signal.is_none()
    }

    pub fn code(self) -> i32 {
        self.code
    }
}

/// POSIX signal forwarded to a process inside the guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Hup,
    Int,
    Quit,
    Kill,
    Usr1,
    Usr2,
    Term,
    Other(i32),
}

impl Signal {
    pub fn as_raw(self) -> i32 {
        match self {
            Self::Hup => 1,
            Self::Int => 2,
            Self::Quit => 3,
            Self::Kill => 9,
            Self::Usr1 => 10,
            Self::Usr2 => 12,
            Self::Term => 15,
            Self::Other(n) => n,
        }
    }

    pub fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Hup,
            2 => Self::Int,
            3 => Self::Quit,
            9 => Self::Kill,
            10 => Self::Usr1,
            12 => Self::Usr2,
            15 => Self::Term,
            other => Self::Other(other),
        }
    }
}

pub(crate) fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    if id.len() > 128 {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub(crate) fn split_env(entry: &str) -> Option<(String, String)> {
    let (k, v) = entry.split_once('=')?;
    if k.is_empty() {
        return None;
    }
    Some((k.to_string(), v.to_string()))
}

fn resolve_bundle_path(bundle: Option<&Path>, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else if let Some(bundle) = bundle {
        bundle.join(path)
    } else {
        path.to_path_buf()
    }
}

/// `(type, optional path)` pairs from `spec.linux.namespaces`.
pub(crate) fn namespaces_from_spec(spec: &Spec) -> Vec<(String, Option<String>)> {
    let Some(linux) = spec.linux() else {
        return Vec::new();
    };
    let Some(namespaces) = linux.namespaces() else {
        return Vec::new();
    };
    namespaces
        .iter()
        .map(|ns| {
            (
                ns.typ().to_string(),
                ns.path().as_ref().map(|p| p.display().to_string()),
            )
        })
        .collect()
}
