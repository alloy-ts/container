import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  Container,
  handleContainerBuild,
  containerCli,
  containerComposeCli,
} from "./lib.ts";

describe("CLI subhandlers tests", () => {
  it("container-build handler", async () => {
    const res = await handleContainerBuild({
      contextDir: ".",
      dockerfile: "Dockerfile",
      cpus: 2,
      memory: "1GB",
      noCache: true,
      tags: ["my-app:v1"],
    });
    assert.equal(res, "image-built:latest");
  });

  it("container CLI handler", async () => {
    const created = await containerCli.create({ image: "alpine:latest" }, "cli-box");
    assert.ok(created);

    const info = await containerCli.get("cli-box");
    assert.ok(info);

    const list = await containerCli.list();
    assert.ok(Array.isArray(list));

    await containerCli.remove("cli-box");
  });

  it("container compose CLI handler", async () => {
    await containerComposeCli.up(true, true);
    const ver = await containerComposeCli.version();
    assert.ok(ver.includes("container-compose"));
    await containerComposeCli.down();
  });
});
