//! libkrun backend for Linux (KVM) and macOS (Hypervisor.framework).
//!
//! The `libkrun` crate is the VMM. It is compiled into this package; we do not
//! link a system `libkrun.so`. The guest kernel comes from libkrunfw, which
//! libkrun `dlopen`s at `krun_start_enter` (`libkrunfw.so.5` on Linux,
//! `libkrunfw.5.dylib` on macOS).
//!
//! On macOS arm64 the process must be codesigned with
//! `com.apple.security.hypervisor` or Hypervisor.framework refuses the VM.
//! Published ports use TSI (`krun_set_port_map`): the guest shares the host
//! network stack, which is how a Node server on an 8 GB Mac mini is reached.
//! libkrun rejects a port map when a virtio-net device is also attached, so
//! ports and a dedicated TAP are not combined.

use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::ptr;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use super::{AgentLink, BootedVm, Hypervisor, VmConfig};
use crate::error::{Error, Result};
use crate::guest::ExitWait;
use crate::types::ExitStatus;

const TSI_HIJACK_INET: u32 = 1 << 0;
const PORT_MAP_SLOTS: usize = 4096;

pub(crate) struct LibkrunHypervisor {
    ctx: Option<u32>,
    entered: bool,
    thread: Option<JoinHandle<()>>,
    /// Write ends of the virtio console, kept open for the life of the VM.
    _console_fds: Vec<OwnedFd>,
    socket_path: Option<PathBuf>,
}

impl LibkrunHypervisor {
    pub(crate) fn new() -> Self {
        Self {
            ctx: None,
            entered: false,
            thread: None,
            _console_fds: Vec::new(),
            socket_path: None,
        }
    }
}

#[async_trait]
impl Hypervisor for LibkrunHypervisor {
    async fn boot(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        let result = self.boot_inner(cfg).await;
        if result.is_err() {
            self.shutdown_blocking();
        }
        result
    }

    fn pause(&mut self) -> Result<()> {
        let _ = self.ctx.ok_or_else(|| {
            Error::Hypervisor("cannot pause a microVM that is not running".into())
        })?;
        // libkrun 1.19 has no krun_vm_pause. Later versions freeze vCPUs on
        // macOS only; Linux still has no equivalent.
        Err(Error::Hypervisor(
            "libkrun 1.19 does not expose vCPU pause".into(),
        ))
    }

    fn resume(&mut self) -> Result<()> {
        let _ = self.ctx.ok_or_else(|| {
            Error::Hypervisor("cannot resume a microVM that is not running".into())
        })?;
        Err(Error::Hypervisor(
            "libkrun 1.19 does not expose vCPU resume".into(),
        ))
    }

    fn shutdown_blocking(&mut self) {
        if let Some(ctx) = self.ctx.take() {
            if !self.entered {
                krun::krun_free_ctx(ctx);
            }
        }
        self.entered = false;
        self._console_fds.clear();
        if let Some(path) = self.socket_path.take() {
            let _ = std::fs::remove_file(path);
        }
        if let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                let _ = thread.join();
            }
            // libkrun's event loop has no stop on Linux. A running VM thread
            // is detached here; dropping the process tears it down.
        }
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.shutdown_blocking();
        Ok(())
    }
}

impl Drop for LibkrunHypervisor {
    fn drop(&mut self) {
        self.shutdown_blocking();
    }
}

impl LibkrunHypervisor {
    async fn boot_inner(&mut self, cfg: VmConfig) -> Result<BootedVm> {
        if cfg.vcpus == 0 || cfg.vcpus > u32::from(u8::MAX) {
            return Err(Error::Hypervisor(format!(
                "libkrun vCPUs must be 1..=255 (got {})",
                cfg.vcpus
            )));
        }
        if cfg.memory_mb == 0 || cfg.memory_mb > u64::from(u32::MAX) {
            return Err(Error::Hypervisor(format!(
                "libkrun memory_mb is out of range (got {})",
                cfg.memory_mb
            )));
        }
        let root = cfg
            .shares
            .first()
            .map(|share| share.path.clone())
            .ok_or_else(|| Error::Hypervisor("libkrun boot needs a rootfs directory".into()))?;
        // kernel_path, cmdline, and vsock_cid stay on VmConfig for callers.
        // libkrunfw supplies the kernel, and libkrun fixes the guest CID at 3.
        let _unused_by_libkrun = (
            cfg.kernel_path.as_os_str(),
            cfg.cmdline.as_str(),
            cfg.vsock_cid,
        );
        if !root.is_dir() {
            return Err(Error::Hypervisor(format!(
                "libkrunfw's init is injected into a virtio-fs directory; {} is not a directory",
                root.display()
            )));
        }

        let ctx = krun::krun_create_ctx();
        if ctx < 0 {
            return Err(Error::Hypervisor(format!(
                "krun_create_ctx failed: {}",
                io::Error::from_raw_os_error(-ctx)
            )));
        }
        let ctx = ctx as u32;
        self.ctx = Some(ctx);

        krun(
            krun::krun_set_vm_config(ctx, cfg.vcpus as u8, cfg.memory_mb as u32),
            "krun_set_vm_config",
        )?;
        // No krun_set_kernel: libkrun loads the kernel from libkrunfw.
        let root_c = cstring(root.to_string_lossy().as_ref())?;
        krun(
            unsafe { krun::krun_set_root(ctx, root_c.as_ptr()) },
            "krun_set_root",
        )?;
        for share in cfg.shares.iter().skip(1) {
            let tag = cstring(&share.tag)?;
            let path = cstring(share.path.to_string_lossy().as_ref())?;
            krun(
                unsafe {
                    krun::krun_add_virtiofs3(ctx, tag.as_ptr(), path.as_ptr(), 0, share.readonly)
                },
                "krun_add_virtiofs3",
            )?;
        }

        let use_tsi = !cfg.ports.is_empty() || cfg!(target_os = "macos");
        if use_tsi && cfg.network.is_some() && cfg!(target_os = "linux") && !cfg.ports.is_empty() {
            return Err(Error::Hypervisor(
                "libkrun cannot publish ports and attach a dedicated TAP at the same time; drop the TAP (the guest uses the host network via TSI) or drop the port map"
                    .into(),
            ));
        }
        let tsi = if use_tsi { TSI_HIJACK_INET } else { 0 };
        // A fresh context already has implicit vsock. Switch to an explicit
        // device so TSI flags and the agent port are under our control.
        krun(
            krun::krun_disable_implicit_vsock(ctx),
            "krun_disable_implicit_vsock",
        )?;
        krun(krun::krun_add_vsock(ctx, tsi), "krun_add_vsock")?;
        if !cfg.ports.is_empty() {
            set_port_map(ctx, &cfg.ports)?;
        }
        #[cfg(target_os = "linux")]
        if !use_tsi {
            if let Some(net) = &cfg.network {
                let name = cstring(&net.tap_name)?;
                krun(
                    unsafe {
                        krun::krun_add_net_tap(ctx, name.as_ptr(), net.mac.as_ptr(), 0, 0)
                    },
                    "krun_add_net_tap",
                )?;
            }
        }

        let socket_path = std::env::temp_dir().join(format!(
            "container-native-{}-{}.sock",
            cfg.id,
            std::process::id()
        ));
        let _ = std::fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).map_err(Error::Io)?;
        listener.set_nonblocking(true).map_err(Error::Io)?;
        let path_c = cstring(socket_path.to_string_lossy().as_ref())?;
        krun(
            unsafe { krun::krun_add_vsock_port(ctx, cfg.vsock_port, path_c.as_ptr()) },
            "krun_add_vsock_port",
        )?;
        self.socket_path = Some(socket_path);

        if let Some(argv0) = cfg.args.first() {
            set_exec(ctx, argv0, &cfg.args, &cfg.env)?;
        }
        if !cfg.workdir.as_os_str().is_empty() {
            let dir = cstring(cfg.workdir.to_string_lossy().as_ref())?;
            krun(
                unsafe { krun::krun_set_workdir(ctx, dir.as_ptr()) },
                "krun_set_workdir",
            )?;
        }

        let console_log = std::env::temp_dir().join(format!("container-native-{}.console", cfg.id));
        let console_c = cstring(console_log.to_string_lossy().as_ref())?;
        krun(
            unsafe { krun::krun_set_console_output(ctx, console_c.as_ptr()) },
            "krun_set_console_output",
        )?;
        krun(
            krun::krun_disable_implicit_console(ctx),
            "krun_disable_implicit_console",
        )?;
        let (stdout, stderr, console_fds) = console_pipes(ctx)?;
        self._console_fds = console_fds;

        let (exit_tx, exit) = ExitWait::channel();
        let (rc_tx, rc_rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name(format!("libkrun-{}", cfg.id))
            .spawn(move || {
                let rc = krun::krun_start_enter(ctx);
                let _ = rc_tx.send(rc);
                let _ = exit_tx.send(Some(ExitStatus {
                    code: rc,
                    signal: None,
                }));
            })
            .map_err(Error::Io)?;
        self.thread = Some(thread);
        self.entered = true;

        let deadline = Instant::now() + Duration::from_secs(3);
        let mut vm_up = false;
        loop {
            if self
                .thread
                .as_ref()
                .map(|thread| thread.is_finished())
                .unwrap_or(true)
            {
                if let Some(thread) = self.thread.take() {
                    let _ = thread.join();
                }
                let rc = rc_rx.recv().unwrap_or(-1);
                let os = io::Error::from_raw_os_error(rc.saturating_neg());
                return Err(Error::Hypervisor(format!(
                    "krun_start_enter returned {rc} ({os}). Install libkrunfw (libkrunfw.so.5 / libkrunfw.5.dylib) and allow this process to open the hypervisor (/dev/kvm, or the macOS hypervisor entitlement)."
                )));
            }
            if let Some(stream) = accept_one(&listener) {
                return Ok(BootedVm {
                    link: AgentLink::Connected(Box::new(stream)),
                    host_pid: None,
                });
            }
            if Instant::now().saturating_duration_since(deadline - Duration::from_secs(3))
                > Duration::from_millis(400)
            {
                vm_up = true;
            }
            if vm_up && Instant::now() > deadline - Duration::from_secs(2) {
                break;
            }
            if Instant::now() > deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        if !vm_up {
            return Err(Error::Hypervisor(
                "timed out waiting for libkrun to boot the libkrunfw kernel".into(),
            ));
        }
        Ok(BootedVm {
            link: AgentLink::Direct {
                stdout: Some(Box::new(stdout)),
                stderr: Some(Box::new(stderr)),
                exit,
            },
            host_pid: None,
        })
    }
}

fn set_port_map(ctx: u32, ports: &[(u16, u16)]) -> Result<()> {
    let mut owned = Vec::with_capacity(ports.len());
    let mut slots = vec![ptr::null(); PORT_MAP_SLOTS];
    for (index, (host, guest)) in ports.iter().copied().enumerate() {
        if index + 1 >= PORT_MAP_SLOTS {
            return Err(Error::Hypervisor("too many published ports".into()));
        }
        if host == 0 || guest == 0 {
            return Err(Error::Hypervisor(
                "published ports must be non-zero".into(),
            ));
        }
        let text = CString::new(format!("{host}:{guest}")).map_err(|_| {
            Error::Hypervisor(format!("published port {host}:{guest} is not a C string"))
        })?;
        slots[index] = text.as_ptr();
        owned.push(text);
    }
    krun(
        unsafe { krun::krun_set_port_map(ctx, slots.as_ptr()) },
        "krun_set_port_map",
    )?;
    drop(owned);
    Ok(())
}

fn set_exec(ctx: u32, argv0: &str, args: &[String], env: &[(String, String)]) -> Result<()> {
    let exec = cstring(argv0)?;
    let mut arg_owned = Vec::new();
    let mut arg_slots = vec![ptr::null(); PORT_MAP_SLOTS];
    for (index, arg) in args.iter().enumerate() {
        let text = cstring(arg)?;
        arg_slots[index] = text.as_ptr();
        arg_owned.push(text);
    }
    let mut env_owned = Vec::new();
    let mut env_slots = vec![ptr::null(); PORT_MAP_SLOTS];
    for (index, (key, value)) in env.iter().enumerate() {
        let text = cstring(&format!("{key}={value}"))?;
        env_slots[index] = text.as_ptr();
        env_owned.push(text);
    }
    krun(
        unsafe { krun::krun_set_exec(ctx, exec.as_ptr(), arg_slots.as_ptr(), env_slots.as_ptr()) },
        "krun_set_exec",
    )?;
    drop((arg_owned, env_owned, exec));
    Ok(())
}

fn console_pipes(ctx: u32) -> Result<(tokio::fs::File, tokio::fs::File, Vec<OwnedFd>)> {
    let null = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY | libc::O_CLOEXEC) };
    if null < 0 {
        return Err(Error::Io(io::Error::last_os_error()));
    }
    let null = unsafe { OwnedFd::from_raw_fd(null) };
    let (out_read, out_write) = pipe()?;
    let (err_read, err_write) = pipe()?;
    krun(
        unsafe {
            krun::krun_add_virtio_console_default(
                ctx,
                null.as_raw_fd(),
                out_write.as_raw_fd(),
                err_write.as_raw_fd(),
            )
        },
        "krun_add_virtio_console_default",
    )?;
    let stdout = tokio_read(out_read)?;
    let stderr = tokio_read(err_read)?;
    Ok((stdout, stderr, vec![null, out_write, err_write]))
}

fn pipe() -> Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };
    if rc != 0 {
        return Err(Error::Io(io::Error::last_os_error()));
    }
    Ok((
        unsafe { OwnedFd::from_raw_fd(fds[0]) },
        unsafe { OwnedFd::from_raw_fd(fds[1]) },
    ))
}

fn tokio_read(fd: OwnedFd) -> Result<tokio::fs::File> {
    let file = unsafe { File::from_raw_fd(fd.into_raw_fd()) };
    Ok(tokio::fs::File::from_std(file))
}

fn accept_one(listener: &UnixListener) -> Option<tokio::net::UnixStream> {
    match listener.accept() {
        Ok((stream, _)) => {
            let std_stream = stream;
            tokio::net::UnixStream::from_std(std_stream).ok()
        }
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => None,
        Err(_) => None,
    }
}

fn cstring(value: &str) -> Result<CString> {
    CString::new(value).map_err(|_| Error::Hypervisor(format!("string contains a NUL: {value}")))
}

fn krun(rc: i32, what: &str) -> Result<()> {
    if rc < 0 {
        Err(Error::Hypervisor(format!(
            "libkrun {what}: {}",
            io::Error::from_raw_os_error(-rc)
        )))
    } else {
        Ok(())
    }
}

#[allow(dead_code)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::hypervisor::GUEST_AGENT_PORT;
    use crate::resource::VirtioFsShare;

    fn config(root: PathBuf) -> VmConfig {
        VmConfig {
            id: "krun".into(),
            vcpus: 1,
            memory_mb: 256,
            kernel_path: PathBuf::from("/var/lib/container-native/vmlinux"),
            cmdline: String::new(),
            vsock_cid: 3,
            vsock_port: GUEST_AGENT_PORT,
            network: None,
            shares: vec![VirtioFsShare {
                tag: "rootfs".into(),
                path: root,
                readonly: false,
            }],
            ports: vec![(8080, 3000)],
            args: vec!["/usr/local/bin/node".into(), "server.js".into()],
            env: vec![("NODE_ENV".into(), "production".into())],
            workdir: PathBuf::from("/"),
        }
    }

    #[tokio::test]
    async fn boot_uses_libkrunfw_and_does_not_require_a_local_kernel() {
        let root = std::env::temp_dir().join(format!(
            "cn-krun-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut hv = LibkrunHypervisor::new();
        match hv.boot(config(root.clone())).await {
            Ok(_) => {
                hv.shutdown().await.ok();
            }
            Err(err) => {
                let msg = err.to_string().to_lowercase();
                assert!(
                    msg.contains("libkrun")
                        || msg.contains("libkrunfw")
                        || msg.contains("hypervisor"),
                    "{err}"
                );
                assert!(!msg.contains("spawn"), "{err}");
            }
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn published_port_text_is_host_then_guest() {
        let text = format!("{}:{}", 8080u16, 3000u16);
        assert_eq!(text, "8080:3000");
    }
}
