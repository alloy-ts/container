import { expect, test } from "vite-plus/test";
import { JsContainer, JsEfiVarStore } from "./lib.ts";

test("JsContainer initializes default config", () => {
  const runtime = JsContainer.withDefaultConfig();
  expect(runtime).toBeDefined();
});

test("JsContainer create and get container", async () => {
  const runtime = JsContainer.withDefaultConfig();
  const created = await runtime.create({ image: "alpine" }, "my-test-container");
  expect(created).toBeDefined();

  const retrieved = await runtime.get("my-test-container");
  expect(retrieved).not.toBeNull();
});

test("JsEfiVarStore initializes", () => {
  const store = new JsEfiVarStore();
  expect(store).toBeDefined();
  expect(typeof store.getSetupMode()).toBe("boolean");
});
