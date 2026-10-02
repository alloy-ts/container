import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerBuild, handleContainerBuild } from "./container-build.ts";
import { containerRegistryCli, Container } from "./container-registry.ts";

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

test("handleContainerBuild helper function", async () => {
  const result = await handleContainerBuild({
    contextDir: ".",
    tags: ["helper-app:latest"],
  });

  assert.equal(typeof result, "string");
  assert.equal(result, "image-built:latest");
});

test("containerRegistryCli and Container.registry bindings", async () => {
  assert.ok(containerRegistryCli);
  await assert.doesNotReject(async () => {
    await containerRegistryCli.login("docker.io", "user", "pass");
    await containerRegistryCli.logout("docker.io");
    const list = await containerRegistryCli.list();
    assert.ok(Array.isArray(list));
  });

  const regHandler = Container.registry();
  assert.ok(regHandler);
});
