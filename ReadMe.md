# @lib/container

High-performance Node.js / TypeScript native bindings for container management, virtualization, and runtime components built with Rust and `napi-rs`.

## Features

- **Container Management**: Create, start, stop, inspect, kill, remove, and monitor containers (`Container`).
- **Sub-Handles**: Specialized management handles for machines, Kubernetes clusters, networks, registries, compose projects, and system status.
- **EFI Variable Store**: Managed EFI variable storage (`EfiVarStore`) supporting setup mode, SecureBoot configuration, key enrollments, and quirks.
- **Image & Build Transfers**: High-level abstractions for streaming image transfers (`ImageTransfer`), build contexts (`BuildTransfer`), and server streams (`ServerStream`).

## Installation & Setup

1. Install dependencies:

```bash
vp install
# or
npm install
```

2. Build the native extension:

```bash
npm run build
```

For a release build:

```bash
npm run build:release
```

## Quick Start

### Basic Container Usage

```typescript
import { Container } from "@lib/container";

async function main() {
  // Initialize runtime state
  const runtime = Container.withDefaultConfig();

  // Create a new container
  const container = await runtime.create(
    { image: "alpine:latest", cpus: 2, memoryMib: 512 },
    "my-container",
  );

  // Start the container
  await container.start();

  // Inspect status
  const info = await container.inspect();
  console.log("Container status:", info.state.status);

  // Stop the container
  await container.stop();
}

main();
```

### Accessing Sub-Handles

```typescript
import { Container } from "@lib/container";

const runtime = Container.withDefaultConfig();

// Machine Handle
const machineId = runtime.machines.create("alpine:latest");

// Kubernetes Handle
const clusterName = runtime.k8s.create("my-k8s-cluster");

// Network Handle
const net = runtime.network.create("custom-bridge");

// Compose Handle
const composeVersion = runtime.compose.version();

// System Status
const status = runtime.system.status();
```

### EFI Variable Store

```typescript
import { EfiVarStore } from "@lib/container";

const store = new EfiVarStore();
console.log("Setup mode:", store.getSetupMode());

store.setSecureBootEnable(true);
store.enrollPkMicrosoft();
```

## Development Commands

- **Build Native Addon**: `npm run build`
- **Run Unit Tests**: `npm run test` or `vp test`
- **Watch Mode**: `npm run dev`
- **Check Code**: `npm run check`
- **Format Code**: `npm run fmt`
- **Lint Code**: `npm run lint`
