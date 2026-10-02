import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerNetwork, containerNetworkCli } from "./container-network.ts";

test("ContainerNetwork programmable NAPI bindings", async () => {
  const name = await ContainerNetwork.create("test-net", "bridge");
  assert.equal(name, "test-net");

  const list = await ContainerNetwork.list();
  assert.ok(Array.isArray(list));

  const inspected = await ContainerNetwork.inspect(["test-net"]);
  assert.equal(inspected.length, 1);
  assert.equal(inspected[0].name, "test-net");

  await ContainerNetwork.prune();
  await ContainerNetwork.delete("test-net");

  await containerNetworkCli.create("cli-net");
  await containerNetworkCli.delete("cli-net");
});
