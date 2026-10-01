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
  expect(runtime.compose).toBeDefined();
  expect(runtime.compose.system).toBeDefined();
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
