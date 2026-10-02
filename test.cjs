const assert = require("node:assert/strict");
const test = require("node:test");

const {
  Container,
  JsContainer,
  EfiVarStore,
  JsEfiVarStore,
  BuildTransfer,
  JsBuildTransfer,
  ImageTransfer,
  JsImageTransfer,
  ServerStream,
  JsServerStream,
  RegistryCliHandler,
  SystemCliHandler,
} = require("./build/index.js");

test("Container / JsContainer withDefaultConfig and create container", async () => {
  const runtime = Container.withDefaultConfig();
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

test("Container sub-handles (machines, k8s, network, registry, system, compose)", () => {
  const runtime = Container.withDefaultConfig();

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

test("RegistryCliHandler login, list, and logout", () => {
  const handler = new RegistryCliHandler();
  handler.login("docker.io", "testuser", "secretpass");

  const list = handler.list();
  assert.equal(list.length, 1);
  assert.equal(list[0].name, "docker.io");
  assert.equal(list[0].username, "testuser");

  handler.logout("docker.io");
  assert.equal(handler.list().length, 0);
});

test("SystemCliHandler default configuration snapshot", () => {
  const handler = new SystemCliHandler();
  const jsonStr = handler.getConfigJson();
  assert.ok(jsonStr.length > 0);

  const config = JSON.parse(jsonStr);
  assert.equal(config.registry.domain, "docker.io");
  assert.equal(config.build.cpus, 2);
  assert.equal(config.container.cpus, 4);
});

test("Container ported CLI methods and sub-handles", async () => {
  const runtime = Container.withDefaultConfig();

  // Container prune
  const box = await runtime.create({ image: "alpine:latest" }, "prune-box");
  await box.start();
  await box.stop();
  const pruned = await runtime.prune();
  assert.ok(Array.isArray(pruned));
  assert.ok(pruned.includes("cnt_prune-box"));

  // Builder handle
  const builder = runtime.builder;
  assert.equal(builder.start(2, "1024M", ["8.8.8.8"]), "buildkit");
  assert.equal(builder.status("table", false).status, "running");
  assert.doesNotThrow(() => builder.stop());
  assert.doesNotThrow(() => builder.delete(true));

  // Image handle ported methods with BuildOptions
  const images = runtime.images;
  assert.deepEqual(images.inspect(["alpine:latest"]), [
    { reference: "alpine:latest", status: "available" },
  ]);
  assert.deepEqual(images.load("archive.tar"), ["loaded-image:latest"]);
  assert.ok(Array.isArray(images.save(["alpine:latest"])));
  assert.equal(images.tag("alpine:latest", "alpine:v1"), "alpine:v1");
  assert.equal(
    images.build("./", {
      dockerfile: "Dockerfile",
      tags: ["app:v1"],
      cpus: 4,
      memory: "2GB",
      noCache: true,
      platform: ["linux/arm64"],
      ssh: "default",
    }),
    "image-built:latest"
  );

  // Machine handle ported methods
  const machines = runtime.machines;
  assert.equal(machines.inspect("m1").state, "running");
  assert.deepEqual(machines.logs("m1"), []);
  assert.equal(machines.set("m1", { cpus: "4" }), "m1");
  assert.equal(machines.setDefault("m1"), "m1");
  assert.equal(machines.capabilities().nestedVirtualization, true);

  // Volume handle inspect
  const volumes = runtime.volumes;
  assert.deepEqual(volumes.inspect(["vol1"]), [{ name: "vol1", driver: "local" }]);

  // Network handle inspect
  const network = runtime.network;
  assert.deepEqual(network.inspect(["net1"]), [{ name: "net1", driver: "bridge" }]);

  // System handle ported methods
  const system = runtime.system;
  assert.deepEqual(system.logs(), []);
  assert.equal(system.listProperties()["log.level"], "info");
  assert.equal(system.dnsCreate("test.local"), "test.local");
  assert.deepEqual(system.dnsList(), []);
  assert.doesNotThrow(() => system.dnsDelete("test.local"));
  assert.equal(system.kernelSet("/path/to/kernel"), "/path/to/kernel");

  // K8s writeConfig
  const k8s = runtime.k8s;
  assert.equal(k8s.writeConfig("k8s-dev"), "~/.kube/config");
});

test("EfiVarStore initialization and setup mode", () => {
  const store = new EfiVarStore();
  assert.ok(store);
  assert.equal(typeof store.getSetupMode(), "boolean");

  const legacyStore = new JsEfiVarStore();
  assert.ok(legacyStore);
});

test("BuildTransfer and ImageTransfer ported helper methods", () => {
  const bt = new BuildTransfer({
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

  const it = new ImageTransfer({
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

  const stream = new ServerStream(it, bt, { data: Array.from(Buffer.from("hello")) });

  assert.ok(stream.getImageTransfer());
  assert.ok(stream.getBuildTransfer());
  assert.ok(stream.getIo());
});
