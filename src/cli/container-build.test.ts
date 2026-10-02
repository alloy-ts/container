import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerBuildHandler } from "./container-build.ts";

test("ContainerBuildHandler.handleBuild programmable NAPI binding", async () => {
  const result = await ContainerBuildHandler.handleBuild({
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
