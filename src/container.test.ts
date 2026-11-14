import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "vite-plus/test";
import { Container, containerOptsFromOciBundle } from "./main.ts";

const bundleConfig = `{
  "ociVersion": "1.2.0",
  "process": {
    "terminal": false,
    "user": { "uid": 0, "gid": 0 },
    "args": ["/bin/sh", "-c", "echo hi"],
    "env": ["PATH=/usr/bin", "HOME=/root"],
    "cwd": "/work"
  },
  "root": { "path": "rootfs", "readonly": false },
  "mounts": [
    { "destination": "/data", "type": "bind", "source": "/tmp", "options": ["rbind", "rw"] }
  ],
  "linux": {
    "resources": {
      "memory": { "limit": 536870912 },
      "cpu": { "quota": 200000, "period": 100000 }
    }
  },
  "annotations": {
    "org.container.kernel": "/tmp/vmlinuz",
    "org.container.vcpus": "2"
  }
}`;

function writeBundle(): string {
  const dir = mkdtempSync(join(tmpdir(), "cn-js-bundle-"));
  mkdirSync(join(dir, "rootfs"));
  writeFileSync(join(dir, "config.json"), bundleConfig);
  return dir;
}

test("containerOptsFromOciBundle maps an OCI bundle without booting", () => {
  const opts = containerOptsFromOciBundle(writeBundle());
  expect(opts.id.length).toBeGreaterThan(0);
  expect(opts.args).toEqual(["/bin/sh", "-c", "echo hi"]);
  expect(opts.env).toContain("PATH=/usr/bin");
  expect(opts.env).toContain("HOME=/root");
  expect(opts.workingDir).toBe("/work");
  expect(opts.kernelPath).toBe("/tmp/vmlinuz");
  expect(opts.vcpus).toBe(2);
  expect(opts.memoryMb).toBe(512);
  expect(opts.rootfs.kind).toBe("directory");
  expect(opts.mounts?.[0]?.destination).toBe("/data");
});

test("Container.create rejects an empty id before touching a hypervisor", async () => {
  await expect(
    Container.create({
      id: "",
      rootfs: { kind: "directory", path: "/" },
      kernelPath: "/var/lib/container-native/vmlinux",
      args: ["/bin/true"],
    }),
  ).rejects.toThrow(/id/i);
});

test("Container.create rejects a missing rootfs", async () => {
  await expect(
    Container.create({
      id: "missing-root",
      rootfs: { kind: "directory", path: "/no/such/container-native-rootfs" },
      kernelPath: "/var/lib/container-native/vmlinux",
      args: ["/bin/true"],
    }),
  ).rejects.toThrow(/rootfs/i);
});

test("Container.create boots through libkrun instead of an external runtime", async () => {
  const root = mkdtempSync(join(tmpdir(), "cn-js-root-"));
  await expect(
    Container.create({
      id: "no-kernel",
      rootfs: { kind: "directory", path: root },
      kernelPath: "/no/such/container-native-kernel",
      args: ["/bin/true"],
      network: { mode: "none" },
    }),
  ).rejects.toThrow(/libkrun|hypervisor/i);
});
