import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerMachine, containerMachineCli } from "./container-machine.ts";

test("ContainerMachine programmable NAPI bindings", async () => {
  const name = await ContainerMachine.create("alpine:latest", "m1");
  assert.equal(name, "machine_m1");

  const exitCode = await ContainerMachine.run("echo", ["hello"]);
  assert.equal(exitCode, 0);

  const list = await ContainerMachine.list();
  assert.ok(Array.isArray(list));

  const info = await ContainerMachine.inspect("m1");
  assert.equal(info.state, "running");

  const logs = await ContainerMachine.logs("m1");
  assert.ok(Array.isArray(logs));

  const setRes = await ContainerMachine.set("m1", { cpus: "2" });
  assert.equal(setRes, "m1");

  const defRes = await ContainerMachine.setDefault("m1");
  assert.equal(defRes, "m1");

  const caps = await ContainerMachine.capabilities();
  assert.equal(caps.nestedVirtualization, true);

  await ContainerMachine.stop("m1");
  await ContainerMachine.delete("m1");

  await containerMachineCli.create("alpine:latest");
});
