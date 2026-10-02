import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerVolume, containerVolumeCli } from "./container-volume.ts";

test("ContainerVolume programmable NAPI bindings", async () => {
  const name = await ContainerVolume.create("vol1", "10G");
  assert.equal(name, "vol1");

  const list = await ContainerVolume.list();
  assert.ok(Array.isArray(list));

  const inspected = await ContainerVolume.inspect(["vol1"]);
  assert.equal(inspected.length, 1);
  assert.equal(inspected[0].name, "vol1");

  await ContainerVolume.prune();
  await ContainerVolume.delete("vol1");

  await containerVolumeCli.create("cli-vol");
  await containerVolumeCli.delete("cli-vol");
});
