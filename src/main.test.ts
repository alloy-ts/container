import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  Container,
  handleContainerBuild,
  ContainerBuild,
  containerCli,
  containerComposeCli,
  containerRegistryCli,
  Registry,
  containerSystemCli,
  System,
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

    const res2 = await ContainerBuild.build({
      contextDir: ".",
      dockerfile: "Dockerfile",
    });
    assert.equal(res2, "image-built:latest");
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

  it("container registry CLI handler", async () => {
    await containerRegistryCli.login("ghcr.io", "testuser", "secretpass");
    const list = await containerRegistryCli.list();
    assert.ok(Array.isArray(list));
    assert.ok(list.includes("ghcr.io"));

    await Registry.logout("ghcr.io");
    const listAfter = await Registry.list();
    assert.ok(!listAfter.includes("ghcr.io"));
  });

  it("container system CLI handler", async () => {
    const status = await containerSystemCli.status();
    assert.equal(status, "running");

    const ver = await System.version();
    assert.ok(ver.version);

    const props = await containerSystemCli.listProperties();
    assert.equal(props["build.cpus"], "2");

    const propList = await System.propertyList("toml");
    assert.ok(propList.includes("[build]"));
  });
});
