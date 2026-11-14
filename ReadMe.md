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
  hypervisor/       VMM trait; libkrun on Linux and macOS, stub on Windows
  resource/         per-VM IP plan and virtio-fs share list
  guest/            host agent client and the reference guest init
```

## Hypervisor

Linux (KVM) and macOS (Hypervisor.framework, including Apple Silicon) boot through the [`libkrun`](https://crates.io/crates/libkrun) crate. That crate is the VMM: Cargo compiles it into this package. Nothing links a system `libkrun.so` / `libkrun.dylib`.

The guest kernel is [libkrunfw](https://github.com/containers/libkrunfw), loaded at runtime:

| Host | Library libkrun dlopens |
| --- | --- |
| Linux | `libkrunfw.so.5` |
| macOS | `libkrunfw.5.dylib` |

`kernelPath` stays on the options object for callers that already set it. This backend does not boot that file; libkrunfw is the kernel.

Windows has no libkrun port. `Container.create` there returns a hypervisor error.

### Published ports (Mac mini / Node)

libkrun's TSI port map is how a process inside the guest is reached from the host. A Node 24 server listening on 3000, published on 8080:

```ts
const container = await Container.create({
  id: "node",
  rootfs: { kind: "directory", path: "/opt/node-root" },
  kernelPath: "/var/lib/container-native/vmlinux",
  vcpus: 2,
  memoryMb: 2048,
  args: ["/usr/local/bin/node", "server.js"],
  workingDir: "/app",
  network: {
    mode: "dedicated",
    ports: [{ host: 8080, guest: 3000 }],
  },
});
```

On an 8 GB Apple Silicon machine, keep `memoryMb` at or below 2048 so the host and virtio-fs window still fit. Published ports use the host network stack. libkrun cannot combine them with a dedicated TAP device; when `ports` is non-empty the guest does not get its own NIC.

`create` asks libkrun to boot libkrunfw and run `args` via libkrun's init. `start` returns a `Process` whose stdout/stderr are the virtio console. A second `exec` needs an in-guest agent, which this kernel does not start, so `exec` fails for that container. `delete` drops the microVM.

### macOS build

```bash
brew install libkrunfw
cargo build
# The .node (or the binary that loads it) must be allowed to call Hypervisor.framework.
codesign --sign - --entitlements entitlements.plist --force \
  dist/container.darwin-arm64.node
```

`entitlements.plist` grants `com.apple.security.hypervisor`. Without that signature a Mac mini refuses the VM.

### Linux build

```bash
sudo apt-get install build-essential pkg-config
# Prebuilt guest kernel (soname libkrunfw.so.5). Point the dynamic loader at it.
curl -fsSL -L -o /tmp/libkrunfw-x86_64.tgz \
  https://github.com/containers/libkrunfw/releases/download/v5.6.4/libkrunfw-x86_64.tgz
sudo mkdir -p /opt/libkrunfw && sudo tar -C /opt/libkrunfw -xzf /tmp/libkrunfw-x86_64.tgz
cargo build --example krun_boot
# /dev/kvm is usually root:kvm. This VM needed sudo.
sudo LD_LIBRARY_PATH=/opt/libkrunfw/lib/x86_64-linux-gnu \
  ./target/debug/examples/krun_boot /path/to/alpine-minirootfs
```

A dedicated NIC (no `ports`) also needs `CAP_NET_ADMIN` for the TAP device libkrun opens. CI compiles and tests without libkrunfw; `Container::create` then fails with a libkrun error instead of starting a VM.

Verified on this Linux host with `/dev/kvm` (via sudo), libkrunfw 5.6.4, and Alpine minirootfs 3.20.3:

```text
sudo LD_LIBRARY_PATH=/tmp/krunfw/lib/x86_64-linux-gnu \
  ./target/debug/examples/krun_boot /tmp/alpine
```

stdout:

```text
hello-from-libkrun
```

The guest command was `/bin/busybox sh -c "echo hello-from-libkrun; sleep 3"`.

### Apple Silicon Mac mini

The same `krun_*` calls run on Hypervisor.framework when this crate is built on macOS arm64. libkrunfw is a separate install. The tap that currently publishes bottles is `libkrun/krun` (the `slp/krun` GitHub repo `slp/homebrew-krun` documents that tap name):

```bash
brew tap libkrun/krun
brew install libkrunfw
# Optional: the C library. This package does not link it; Cargo compiles the libkrun crate.
brew install libkrun

cargo build --example krun_boot
codesign --sign - --entitlements entitlements.plist --force \
  target/debug/examples/krun_boot
# If you ship the NAPI addon, sign that too:
# codesign --sign - --entitlements entitlements.plist --force dist/container.darwin-arm64.node

# Node 24 unpacked rootfs, server listening on 3000, published on 8080.
# network.ports is [{ host: 8080, guest: 3000 }] (see the TypeScript snippet above).
LD_LIBRARY_PATH="$(brew --prefix libkrunfw)/lib" \
  ./target/debug/examples/krun_boot /path/to/node-rootfs
```

`entitlements.plist` is `com.apple.security.hypervisor`. An ad-hoc signature (`--sign -`) is enough for a local binary; a Developer ID signature is required if the binary is quarantined or shipped. Without the entitlement, Hypervisor.framework denies the VM. Keep `memoryMb` at or below 2048 on an 8 GB machine.

## Development

```bash
cargo test
cargo build
npm run build:napi
npm test
```

Rust 1.88 or newer is required (`napi` 3.13, `rust-toolchain.toml`).
