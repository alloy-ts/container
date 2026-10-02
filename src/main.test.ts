import { expect, test } from "vite-plus/test";
import {
  JsBuildTransfer,
  JsContainer,
  JsEfiVarStore,
  JsImageTransfer,
  JsServerStream,
} from "./lib.ts";

test("JsContainer initializes default config", () => {
  const runtime = JsContainer.withDefaultConfig();
  expect(runtime).toBeDefined();
});

test("JsContainer create, start, inspect and stop container", async () => {
  const runtime = JsContainer.withDefaultConfig();
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

test("JsContainer sub-handles accessible", () => {
  const runtime = JsContainer.withDefaultConfig();
  expect(runtime.machines).toBeDefined();
  expect(runtime.k8s).toBeDefined();
  expect(runtime.network).toBeDefined();
  expect(runtime.registry).toBeDefined();
  expect(runtime.system).toBeDefined();
  expect(runtime.builder).toBeDefined();
  expect(runtime.compose).toBeDefined();
  expect(runtime.compose.system).toBeDefined();
});

test("JsContainer new CLI methods work", async () => {
  const runtime = JsContainer.withDefaultConfig();

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

test("ContainerBuildHandler and CLI handlers work", async () => {
  const { ContainerBuildHandler, ContainerCliHandler, ContainerComposeCliHandler } =
    await import("./lib.ts");

  const buildHandler = new ContainerBuildHandler();
  const builtTag = await buildHandler.run({ contextDir: ".", tag: ["my-image:v1"] });
  expect(builtTag).toBe("my-image:v1");

  const cliHandler = new ContainerCliHandler();
  const container = await cliHandler.runContainer({ image: "alpine" }, "cli-c1");
  expect(container).toBeDefined();

  const composeHandler = new ContainerComposeCliHandler();
  expect(await composeHandler.version()).toContain("container-compose");
});

test("JsBuildFile, JsBufferedCopyReader, JsBuildFsSync and JsTerminalCommand work", async () => {
  const { JsBuildFile, JsBufferedCopyReader, JsBuildFsSync, JsTerminalCommand } =
    await import("./lib.ts");

  expect(JsBuildFile.resolvePath("./")).toBeNull();

  const winch = JsTerminalCommand.createWinch(24, 80);
  expect(winch.rows).toBe(24);
  expect(winch.cols).toBe(80);

  const fsSync = new JsBuildFsSync("./");
  expect(fsSync.acceptStage("fssync")).toBe(true);

  const reader = new JsBufferedCopyReader("package.json");
  expect(reader.hasFinished).toBe(false);
});

test("JsEfiVarStore initializes", () => {
  const store = new JsEfiVarStore();
  expect(store).toBeDefined();
  expect(typeof store.getSetupMode()).toBe("boolean");
});

test("JsBuildTransfer and JsImageTransfer ported methods work", () => {
  const bt = new JsBuildTransfer({
    stage: "builder",
    method: "dockerfile",
    "include-patterns": "a,b",
    size: "100",
  });
  expect(bt.stage()).toBe("builder");
  expect(bt.method()).toBe("dockerfile");
  expect(bt.includePatterns()).toEqual(["a", "b"]);
  expect(bt.size()).toBe(100);

  const it = new JsImageTransfer({
    ref: "alpine:latest",
  });
  expect(it.refName()).toBe("alpine:latest");

  const stream = new JsServerStream(it, bt);
  expect(stream.getImageTransfer()).toBeDefined();
});
