const assert = require("node:assert/strict");
const test = require("node:test");

void test("creates and interacts with JsContainer", async () => {
  const { JsContainer } = await import("./index.js");
  assert.ok(JsContainer);
  const container = JsContainer.withDefaultConfig();
  assert.ok(container);
  const createdContainer = container.create({ image: "alpine" }, "my-test");
  assert.ok(createdContainer);
  const info = container.getInfo("my-test");
  assert.equal(info.id, "my-test");
});
