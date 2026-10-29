import { test } from "node:test";
import assert from "node:assert/strict";
import { containerNetworkCli } from "./index.ts";

test("ContainerNetwork CLI handler NAPI bindings", async () => {
  const name = await containerNetworkCli.create("custom-app-net");
  assert.equal(name, "custom-app-net");

  const list = await containerNetworkCli.list();
  assert.ok(list.includes("custom-app-net"));

  const inspectResult = await containerNetworkCli.inspect(["custom-app-net"]);
  assert.equal(inspectResult.length, 1);
  assert.equal(inspectResult[0].name, "custom-app-net");

  await containerNetworkCli.delete("custom-app-net");
  const listAfterDelete = await containerNetworkCli.list();
  assert.ok(!listAfterDelete.includes("custom-app-net"));
});
