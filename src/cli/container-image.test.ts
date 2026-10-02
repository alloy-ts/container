import { test } from "node:test";
import assert from "node:assert/strict";
import { containerImageCli } from "./index.ts";

test("ContainerImage CLI handler NAPI bindings", async () => {
  await containerImageCli.pull("redis:alpine");
  const list = await containerImageCli.list();
  assert.ok(list.includes("redis:alpine"));

  const tagged = await containerImageCli.tag("redis:alpine", "redis:v1");
  assert.equal(tagged, "redis:v1");

  const listAfterTag = await containerImageCli.list();
  assert.ok(listAfterTag.includes("redis:v1"));

  const inspectResult = await containerImageCli.inspect(["redis:v1"]);
  assert.equal(inspectResult.length, 1);
  assert.equal(inspectResult[0].reference, "redis:v1");

  await containerImageCli.delete("redis:v1");
  const listAfterDelete = await containerImageCli.list();
  assert.ok(!listAfterDelete.includes("redis:v1"));
});
