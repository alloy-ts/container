const assert = require("node:assert/strict");
const test = require("node:test");

const {
  JsContainer,
  JsEfiVarStore,
  JsBuildTransfer,
  JsImageTransfer,
  JsServerStream,
} = require("./build/index.js");

test("JsContainer withDefaultConfig and create container", async () => {
  const runtime = JsContainer.withDefaultConfig();
  assert.ok(runtime);

  const container = await runtime.create({ image: "alpine:latest" }, "test-box");
  assert.ok(container);

  const startExit = await container.start(false, false);
  assert.equal(startExit, 0);

  const info = await container.inspect();
  assert.equal(info.name, "test-box");
  assert.equal(info.state.status, "running");
  assert.equal(info.state.running, true);

  await container.stop();
  const infoStopped = await container.inspect();
  assert.equal(infoStopped.state.status, "stopped");
  assert.equal(infoStopped.state.running, false);

  const metrics = await runtime.metrics();
  assert.ok(metrics.containeresCreatedTotal >= 1);
});

test("JsContainer sub-handles (machines, k8s, network, registry, system, compose)", () => {
  const runtime = JsContainer.withDefaultConfig();

  const machine = runtime.machines.create("alpine:latest");
  assert.ok(machine.startsWith("machine_"));

  const k8sCluster = runtime.k8s.create("my-cluster");
  assert.equal(k8sCluster, "my-cluster");

  const net = runtime.network.create("custom-net");
  assert.equal(net, "custom-net");

  const sysStatus = runtime.system.status();
  assert.equal(sysStatus, "running");

  const composeVer = runtime.compose.version();
  assert.ok(composeVer.includes("container-compose"));

  const composeSysStatus = runtime.compose.system.status();
  assert.ok(composeSysStatus.includes("daemon running"));
});

test("JsEfiVarStore initialization and setup mode", () => {
  const store = new JsEfiVarStore();
  assert.ok(store);
  assert.equal(typeof store.getSetupMode(), "boolean");
});

test("JsBuildTransfer and JsImageTransfer ported helper methods", () => {
  const bt = new JsBuildTransfer({
    stage: "builder",
    method: "dockerfile",
    "include-patterns": "src/*,package.json",
    followpaths: "/tmp,/var",
    mode: "0755",
    size: "1024",
    offset: "0",
    length: "512",
  });

  assert.equal(bt.stage(), "builder");
  assert.equal(bt.method(), "dockerfile");
  assert.deepEqual(bt.includePatterns(), ["src/*", "package.json"]);
  assert.deepEqual(bt.followPaths(), ["/tmp", "/var"]);
  assert.equal(bt.mode(), "0755");
  assert.equal(bt.size(), 1024);
  assert.equal(bt.offset(), 0);
  assert.equal(bt.len(), 512);

  const it = new JsImageTransfer({
    stage: "final",
    method: "pull",
    ref: "ubuntu:latest",
    platform: "linux/arm64",
    size: "2048",
  });

  assert.equal(it.stage(), "final");
  assert.equal(it.method(), "pull");
  assert.equal(it.refName(), "ubuntu:latest");
  assert.equal(it.platform(), "linux/arm64");
  assert.equal(it.size(), 2048);

  const stream = new JsServerStream(it, bt, { data: Array.from(Buffer.from("hello")) });

  assert.ok(stream.getImageTransfer());
  assert.ok(stream.getBuildTransfer());
  assert.ok(stream.getIo());
});
