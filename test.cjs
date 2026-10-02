const assert = require("node:assert/strict");
const test = require("node:test");

const {
  JsContainer,
  JsEfiVarStore,
  JsBuildTransfer,
  JsImageTransfer,
  JsServerStream,
  JsBuilderCommand,
  JsBuildFile,
  JsBufferedCopyReader,
  JsBuildFSSync,
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

test("JsContainer sub-handles (machines, k8s, network, registry, system, compose, builder)", async () => {
  const runtime = JsContainer.withDefaultConfig();

  const machine = runtime.machines.create("alpine:latest");
  assert.ok(machine.startsWith("machine_"));
  const machineInspect = runtime.machines.inspect("m1");
  assert.equal(machineInspect.name, "m1");
  const caps = runtime.machines.capabilities();
  assert.ok(Array.isArray(caps));
  runtime.machines.set("m1", 2, "1GB");
  runtime.machines.setDefault("m1");
  assert.deepEqual(runtime.machines.logs("m1"), []);

  const k8sCluster = runtime.k8s.create("my-cluster");
  assert.equal(k8sCluster, "my-cluster");

  const net = runtime.network.create("custom-net");
  assert.equal(net, "custom-net");
  const netInspect = runtime.network.inspect("custom-net");
  assert.equal(netInspect.name, "custom-net");

  const imageInspect = runtime.images.inspect("alpine:latest");
  assert.equal(imageInspect.reference, "alpine:latest");
  assert.equal(runtime.images.load("archive.tar"), "loaded:archive.tar");
  assert.doesNotThrow(() => runtime.images.save("alpine:latest", "out.tar"));
  assert.doesNotThrow(() => runtime.images.tag("alpine:latest", "alpine:v1"));

  const volInspect = runtime.volumes.inspect("v1");
  assert.equal(volInspect.name, "v1");

  const sysStatus = runtime.system.status();
  assert.equal(sysStatus, "running");
  assert.deepEqual(runtime.system.logs(), []);
  assert.deepEqual(runtime.system.dnsList(), []);
  assert.doesNotThrow(() => runtime.system.dnsSet(["8.8.8.8"]));
  assert.deepEqual(runtime.system.kernelList(), []);
  assert.doesNotThrow(() => runtime.system.kernelSet("6.1"));
  assert.equal(runtime.system.propertyGet("foo"), null);
  assert.doesNotThrow(() => runtime.system.propertySet("foo", "bar"));

  const builder = runtime.builder;
  assert.ok(builder);
  assert.equal(builder.status(), "running");
  assert.doesNotThrow(() => builder.start());
  assert.doesNotThrow(() => builder.stop());
  assert.doesNotThrow(() => builder.delete());

  const composeVer = runtime.compose.version();
  assert.ok(composeVer.includes("container-compose"));

  const composeSysStatus = runtime.compose.system.status();
  assert.ok(composeSysStatus.includes("daemon running"));

  const runContainer = await runtime.run({ image: "alpine:latest" }, "run-box");
  assert.ok(runContainer);
  const infoRun = await runContainer.inspect();
  assert.equal(infoRun.state.status, "running");

  await runContainer.stop();
  const pruned = await runtime.prune();
  assert.ok(Array.isArray(pruned));
});

test("JsEfiVarStore initialization and setup mode", () => {
  const store = new JsEfiVarStore();
  assert.ok(store);
  assert.equal(typeof store.getSetupMode(), "boolean");
});

test("JsBuilderCommand, JsBuildFile, JsBufferedCopyReader, and JsBuildFSSync functionality", () => {
  const cmd = new JsBuilderCommand();
  const startRes = cmd.start({ cpus: 2, memory: "2GB" });
  assert.equal(startRes.status, "started");
  assert.equal(startRes.cpus, "2");

  const statusRes = cmd.status({ quiet: true });
  assert.equal(statusRes.status, "running");

  const resolved = JsBuildFile.resolvePath(".");
  assert.ok(resolved === null || typeof resolved === "string");

  const reader = new JsBufferedCopyReader("non_existent_file.txt", 1024);
  assert.equal(reader.hasFinished, true);
  assert.equal(reader.nextChunk(), null);

  const fsSync = new JsBuildFSSync("/tmp");
  const readRes = fsSync.read("Dockerfile", 0, 100);
  assert.equal(readRes.method, "Read");
  assert.equal(readRes.source, "Dockerfile");

  const walkRes = fsSync.walk(["src/*"]);
  assert.equal(walkRes.method, "Walk");
  assert.equal(walkRes.follow_paths, "src/*");
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
