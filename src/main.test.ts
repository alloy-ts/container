import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  Container,
  handleContainerBuild,
  containerCli,
  containerComposeCli,
  containerRegistryCli,
  containerNetworkCli,
  containerMachineCli,
  containerVolumeCli,
  containerSystemCli,
  containerBuilderCli,
  containerImageCli,
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

  it("container registry CLI handler", async () => {
    await containerRegistryCli.login("docker.io", "user", "pass");
    const list = await containerRegistryCli.list();
    assert.ok(Array.isArray(list));
    await containerRegistryCli.logout("docker.io");
  });

  it("container network CLI handler", async () => {
    const net = await containerNetworkCli.create("net1");
    assert.equal(net, "net1");
    const list = await containerNetworkCli.list();
    assert.ok(Array.isArray(list));
    await containerNetworkCli.delete("net1");
  });

  it("container machine CLI handler", async () => {
    const m = await containerMachineCli.create("alpine:latest", "m1");
    assert.equal(m, "machine_m1");
    await containerMachineCli.stop("m1");
    await containerMachineCli.delete("m1");
  });

  it("container volume CLI handler", async () => {
    const v = await containerVolumeCli.create("v1");
    assert.equal(v, "v1");
    await containerVolumeCli.delete("v1");
  });

  it("container system CLI handler", async () => {
    const status = await containerSystemCli.status();
    assert.equal(status, "running");
  });

  it("container builder CLI handler", async () => {
    const res = await containerBuilderCli.start();
    assert.equal(res, "buildkit");
  });

  it("container image CLI handler", async () => {
    const list = await containerImageCli.list();
    assert.ok(Array.isArray(list));
  });
});
