import { test } from "node:test";
import assert from "node:assert/strict";
import { ContainerBuild } from "./container-build.ts";

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
