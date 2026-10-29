import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerBuild, containerRegistryCli, containerSystemCli } from "./index.ts";

test("ContainerBuild.build programmable NAPI binding", async () => {
  const result = await ContainerBuild.build({
    contextDir: ".",
    dockerfile: "Dockerfile",
    tags: ["test-app:latest"],
    cpus: 2,
    memory: "512M",
    noCache: true,
  });

  assert.equal(typeof result, "string");
  assert.equal(result, "image-built:latest");
});

test("ContainerRegistry CLI handler NAPI bindings with state persistence", async () => {
  await containerRegistryCli.login("ghcr.io", "testuser", "secretpass");

  const listAfterLogin = await containerRegistryCli.list();
  assert.ok(Array.isArray(listAfterLogin));
  assert.equal(listAfterLogin.length, 1);
  assert.equal(listAfterLogin[0].name, "ghcr.io");
  assert.equal(listAfterLogin[0].username, "testuser");

  await containerRegistryCli.logout("ghcr.io");
  const listAfterLogout = await containerRegistryCli.list();
  assert.equal(listAfterLogout.length, 0);
});

test("ContainerSystem CLI handler NAPI bindings with state persistence", async () => {
  const status = await containerSystemCli.status();
  assert.equal(status, "running");

  const ver = await containerSystemCli.version();
  assert.equal(ver.version, "1.0.0");

  const props = await containerSystemCli.listProperties();
  assert.ok(props);

  const dns = await containerSystemCli.dnsCreate("local.test");
  assert.equal(dns, "local.test");

  const kernel = await containerSystemCli.kernelSet("/tmp/vmlinux-custom");
  assert.equal(kernel, "/tmp/vmlinux-custom");

  const updatedProps = await containerSystemCli.listProperties();
  assert.equal(updatedProps["kernel.binary_path"], "/tmp/vmlinux-custom");
});
