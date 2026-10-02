import { expect, test } from "vite-plus/test";
import {
  BuildTransfer,
  Container,
  EfiVarStore,
  ImageTransfer,
  ServerStream,
} from "./lib.ts";

test("Container initializes default config", () => {
  const runtime = Container.withDefaultConfig();
  expect(runtime).toBeDefined();
});

test("Container create, start, inspect and stop container", async () => {
  const runtime = Container.withDefaultConfig();
  const container = await runtime.create({ image: "alpine" }, "my-test-container");
  expect(container).toBeDefined();

  const code = await container.start(false, false);
  expect(code).toBe(0);

  const info = await container.inspect();
  expect(info.state.status).toBe("running");

  await container.stop();
  const stoppedInfo = await container.inspect();
  expect(stoppedInfo.state.status).toBe("stopped");
});

test("Container sub-handles accessible", () => {
  const runtime = Container.withDefaultConfig();
  expect(runtime.machines).toBeDefined();
  expect(runtime.k8s).toBeDefined();
  expect(runtime.network).toBeDefined();
  expect(runtime.registry).toBeDefined();
  expect(runtime.system).toBeDefined();
  expect(runtime.builder).toBeDefined();
  expect(runtime.compose).toBeDefined();
  expect(runtime.compose.system).toBeDefined();
});

test("Container new CLI methods work", async () => {
  const runtime = Container.withDefaultConfig();

  const container = await runtime.create({ image: "alpine" }, "c1");
  await container.start();
  await container.stop();

  const pruned = await runtime.prune();
  expect(pruned).toContain("cnt_c1");

  expect(runtime.images.inspect(["alpine"])).toBeDefined();
  expect(runtime.images.load("a.tar")).toEqual(["loaded-image:latest"]);
  expect(runtime.images.tag("a", "b")).toBe("b");

  expect(runtime.machines.capabilities().nestedVirtualization).toBe(true);
  expect(runtime.system.dnsCreate("test.local")).toBe("test.local");
  expect(runtime.builder.start()).toBe("buildkit");
});

test("EfiVarStore initializes", () => {
  const store = new EfiVarStore();
  expect(store).toBeDefined();
  expect(typeof store.getSetupMode()).toBe("boolean");
});

test("BuildTransfer and ImageTransfer ported methods work", () => {
  const bt = new BuildTransfer({
    stage: "builder",
    method: "dockerfile",
    "include-patterns": "a,b",
    size: "100",
  });
  expect(bt.stage()).toBe("builder");
  expect(bt.method()).toBe("dockerfile");
  expect(bt.includePatterns()).toEqual(["a", "b"]);
  expect(bt.size()).toBe(100);

  const it = new ImageTransfer({
    ref: "alpine:latest",
  });
  expect(it.refName()).toBe("alpine:latest");

  const stream = new ServerStream(it, bt);
  expect(stream.getImageTransfer()).toBeDefined();
});
