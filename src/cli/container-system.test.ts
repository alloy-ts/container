import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerSystem, containerSystemCli } from "./container-system.ts";

test("ContainerSystem programmable NAPI bindings", async () => {
  await ContainerSystem.start();
  const status = await ContainerSystem.status();
  assert.equal(status, "running");

  const ver = await ContainerSystem.version();
  assert.equal(ver.version, "1.0.0");

  const df = await ContainerSystem.df();
  assert.ok(df.reclaimable);

  const logs = await ContainerSystem.logs();
  assert.ok(Array.isArray(logs));

  const props = await ContainerSystem.listProperties();
  assert.equal(props["build.cpus"], "2");
  assert.equal(props["container.cpus"], "4");
  assert.equal(props["registry.domain"], "docker.io");

  const domain = await ContainerSystem.dnsCreate("test.local");
  assert.equal(domain, "test.local");

  const dnsList = await ContainerSystem.dnsList();
  assert.ok(Array.isArray(dnsList));

  await ContainerSystem.dnsDelete("test.local");

  const kernel = await ContainerSystem.kernelSet("/boot/vmlinuz");
  assert.equal(kernel, "/boot/vmlinuz");

  await ContainerSystem.stop();

  await containerSystemCli.start();
});
