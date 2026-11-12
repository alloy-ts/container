# @alloy-ts/container

In-process OCI microVM runtime. One `Container` is one microVM — the same model Apple container uses — instead of a CLI wrapper around an external `container` / `docker` / `podman` binary.

The public surface is `ContainerOpts`, `Container`, `ExecOpts`, `Process`, and `ContainerState`. Hypervisor handles, TAP devices, virtio shares, and the vsock guest-agent protocol stay inside the crate.

## Rust

```rust
use container_native::{Container, ContainerOpts, RootfsSource};

let opts = ContainerOpts::builder("app")
    .rootfs(RootfsSource::Directory("/var/lib/app/rootfs".into()))
    .kernel_path("/var/lib/container-native/vmlinux")
    .vcpus(2)
    .memory_mb(512)
    .args(["/bin/sh", "-c", "echo hello"])
    .env("HOME", "/root")
    .build()?;

// Or ingest a bundle: ContainerOpts::from_oci_bundle(Path::new("/bundle"))
let mut container = Container::create(opts).await?;
let mut process = container.start().await?;
let status = process.wait().await?;
container.delete().await?;
```

`create` boots the microVM and connects to the guest init without running the entrypoint. `start` runs it and returns a `Process` (pid, stdin/stdout/stderr, `wait`, `kill`, `resize_tty`). `exec` starts another process in the same VM. `state`, `kill`, `wait`, `logs`, `stats`, `pause`, `resume`, `id`, and `ip_addr` cover the OCI lifecycle and the Apple-container extras. `Drop` shuts the VM down if `delete` was not called.

## JavaScript

`Container.create` and `containerOptsFromOciBundle` are the NAPI bindings. Lifecycle methods return promises.

```ts
import { Container, containerOptsFromOciBundle } from "@alloy-ts/container";

const opts = containerOptsFromOciBundle("/path/to/bundle");
const container = await Container.create({
  id: "app",
  rootfs: { kind: "directory", path: "/var/lib/app/rootfs" },
  kernelPath: "/var/lib/container-native/vmlinux",
  vcpus: 2,
  memoryMb: 512,
  args: ["/bin/sh", "-c", "echo hello"],
});
const process = await container.start();
console.log(process.pid, await process.wait());
await container.delete();
```

## Layout

```
src/
  lib.rs            public exports
  container.rs      Container, Process, and the NAPI class
  proto/            length-prefixed vsock RPC and stdio frames
  hypervisor/       VMM trait
    linux/          KVM, bzImage boot, TAP, AF_VSOCK (x86_64)
    macos/          stub: Virtualization.framework + vmnet + VZVirtioSocket
    windows/        stub: WHPX + HNS/slirp + AF_HYPERV
  resource/         per-VM IP plan and virtio-fs share list
  guest/            host agent client and the reference guest init
```

## What runs today

The guest-agent protocol (create, start, exec, stdio, signals, tty resize, stats, logs, delete) is implemented and tested. The reference guest init is the PID 1 this protocol expects; tests run it in-process so the public lifecycle does not need `/dev/kvm`.

On Linux x86_64 the hypervisor really opens `/dev/kvm`, maps guest RAM, creates vCPUs, loads a bzImage, builds a long-mode identity map, and can open a TAP device and an AF_VSOCK port. virtio-fs, virtio-net, and virtio-vsock device emulation are not done, so `Container::create` on the real backend rolls the machine back with an explicit error instead of returning a VM the agent can never reach. macOS and Windows backends compile and return the same kind of error. The Linux path is the one implemented here because this crate is built on Linux x86_64, where KVM, TAP, and AF_VSOCK exist; Virtualization.framework and WHPX do not.

## Development

```bash
cargo test
cargo build
npm run build:napi
npm test
```

Rust 1.88 or newer is required (`napi` 3.13).
