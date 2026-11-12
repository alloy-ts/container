# @alloy-ts/container

In-process OCI microVM runtime. One `Container` is one microVM.

The JavaScript surface is `ContainerOpts`, `Container`, `ExecOpts`, `Process`, and `ContainerState`. Hypervisor handles, TAP devices, virtio shares, and the vsock guest-agent protocol stay internal.

## Usage

`Container.create` and `containerOptsFromOciBundle` are the NAPI bindings. Lifecycle methods return promises.

```ts
import { Container } from "@alloy-ts/container";

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

`create` boots one microVM and connects to its guest agent without running the entrypoint. `start` runs the configured entrypoint and returns an independent `Process` handle. Use `exec` for additional processes, `state` and `stats` for inspection, `pause` and `resume` for vCPU control, and `delete` for deterministic cleanup.

For an OCI bundle, parse `config.json` into the same options object before creating the container:

```ts
import { Container, containerOptsFromOciBundle } from "@alloy-ts/container";

const opts = containerOptsFromOciBundle("/path/to/bundle");
const container = await Container.create(opts);

try {
  const process = await container.start();
  console.log(await process.wait());
} finally {
  await container.delete();
}
```

See [`examples/container.ts`](examples/container.ts) for lifecycle, `exec`, output streaming, signals, and cleanup in one example.

## Layout

```
src/
  lib.rs            public exports
  container.rs      Container, Process, and the NAPI class
  proto/            length-prefixed vsock RPC and stdio frames
  hypervisor/       VMM trait
    linux/          KVM, bzImage boot, virtio-mmio devices, TAP (x86_64)
    macos/          stub: Virtualization.framework + vmnet + VZVirtioSocket
    windows/        stub: WHPX + HNS/slirp + AF_HYPERV
  resource/         per-VM IP plan and virtio-fs share list
  guest/            host agent client and the reference guest init
```

## What runs today

The guest-agent protocol (create, start, exec, stdio, signals, tty resize, stats, logs, delete) is implemented and tested. The reference guest init is the PID 1 this protocol expects; tests run it in-process so the public lifecycle does not need `/dev/kvm`.

On Linux x86_64 the hypervisor really opens `/dev/kvm`, maps guest RAM, creates vCPUs, loads a bzImage, builds a long-mode identity map, opens a TAP device, and serves virtio-vsock, virtio-net, and virtio-fs devices over virtio-mmio. Each device model is exercised against in-memory virtqueues in the unit tests; the virtio-fs device carries FUSE requests through an in-process `fuse-backend-rs` passthrough filesystem rooted at the share. Booting still needs a guest image that mounts its rootfs over virtio-fs and connects to the vsock proxy, so without one `Container::create` on the real backend rolls the machine back with an explicit handshake-timeout error instead of returning a VM the agent can never reach. macOS and Windows backends compile and return the same kind of error. The Linux path is the one implemented here because this crate is built on Linux x86_64, where KVM and TAP exist; Virtualization.framework and WHPX do not.

## Development

```bash
cargo test
cargo build
npm run build:napi
npm test
```

Rust 1.88 or newer is required (`napi` 3.13).
