import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerRegistry, containerRegistryCli } from "./container-registry.ts";

test("ContainerRegistry programmable NAPI bindings", async () => {
  await ContainerRegistry.login("registry.example.com", "user", "pass");
  const list = await ContainerRegistry.list();
  assert.ok(Array.isArray(list));
  await ContainerRegistry.logout("registry.example.com");

  await containerRegistryCli.login("ghcr.io", "admin", "secret");
  const cliList = await containerRegistryCli.list();
  assert.ok(Array.isArray(cliList));
  await containerRegistryCli.logout("ghcr.io");
});
