const assert = require("node:assert/strict");
const test = require("node:test");

const { JsContainer, JsEfiVarStore } = require("./build/index.js");

test("JsContainer withDefaultConfig and create container", async () => {
  const runtime = JsContainer.withDefaultConfig();
  assert.ok(runtime);

  const container = await runtime.create({ image: "alpine:latest" }, "test-box");
  assert.ok(container);

  const info = await runtime.getInfo("test-box");
  assert.equal(info.name, "test-box");
  assert.equal(info.state.status, "running");

  const metrics = await runtime.metrics();
  assert.ok(metrics.containeresCreatedTotal >= 1);
});

test("JsEfiVarStore initialization and setup mode", () => {
  const store = new JsEfiVarStore();
  assert.ok(store);
  assert.equal(typeof store.getSetupMode(), "boolean");
});
