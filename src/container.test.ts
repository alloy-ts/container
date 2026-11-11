import { expect, test } from "vite-plus/test";
import { Container } from "./main.ts";

// ===========================================================================
// Construction & identity
// ===========================================================================

test("Container() auto-detects an available OCI runtime on the host", () => {
  const c = new Container();
  expect(typeof c.runtime).toBe("string");
  expect(c.runtime.length).toBeGreaterThan(0);
});

test("Container(name) selects the explicitly named runtime", () => {
  const c = new Container("container");
  expect(c.runtime).toBe("container");
});

test("Container(badName) throws when the requested runtime is missing", () => {
  let threw = false;
  try {
    // eslint-disable-next-line no-new
    new Container("definitely-not-a-real-runtime-xyz");
  } catch {
    threw = true;
  }
  expect(threw).toBe(true);
});

test("runtime getter exposes the resolved runtime name", () => {
  const c = new Container("container");
  expect(c.runtime).toBe("container");
});

test("version getter exposes a non-empty detected version string", () => {
  const c = new Container();
  const v = c.version;
  expect(typeof v).toBe("string");
  expect((v as string).length).toBeGreaterThan(0);
});

// ===========================================================================
// Generic forwarding
// ===========================================================================

test("command() forwards arbitrary args and returns stdout", () => {
  const c = new Container();
  const out = c.command(["--help"]);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("command() throws when the runtime exits non-zero", () => {
  const c = new Container();
  let threw = false;
  try {
    c.command(["definitely-not-a-real-subcommand-xyz"]);
  } catch {
    threw = true;
  }
  expect(threw).toBe(true);
});

// ===========================================================================
// Container lifecycle — top-level CONTAINER SUBCOMMANDS
//   create, rm(delete), exec, inspect, kill, ls(list), logs, start, stats, stop
// ===========================================================================

test("lifecycle methods forward to the runtime (daemon-safe --help)", () => {
  const c = new Container();
  const cases: Array<[string, string[]]> = [
    ["create", ["--help"]],
    ["rm", ["--help"]],
    ["exec", ["--help"]],
    ["inspect", ["--help"]],
    ["kill", ["--help"]],
    ["ls", ["--help"]],
    ["logs", ["--help"]],
    ["start", ["--help"]],
    ["stop", ["--help"]],
    ["stats", ["--help"]],
  ];
  const cast = c as unknown as Record<string, (a: string[]) => string>;
  for (const [method, args] of cases) {
    // Call inline (not detached) so the NAPI instance method keeps `this`.
    const out = cast[method](args);
    expect(typeof out).toBe("string");
    expect(out.length).toBeGreaterThan(0);
  }
});

// ===========================================================================
// run / build convenience wrappers
// ===========================================================================

test("run() builds `container run <image> [args]` and returns stdout", () => {
  const c = new Container();
  const out = c.run("--help", []);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("build() builds `container build <context> [args]` and returns stdout", () => {
  const c = new Container();
  const out = c.build("--help", []);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

// ===========================================================================
// Management groups — image / volume / network / builder / registry / system
//   Each is forwarded as `<group> <subcommand> [args...]`.
// ===========================================================================

test("image(subcommand, args) forwards to `container image ...`", () => {
  const c = new Container();
  const out = c.image("ls", ["--help"]);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("volume(subcommand, args) forwards to `container volume ...`", () => {
  const c = new Container();
  const out = c.volume("--help", []);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("network(subcommand, args) forwards to `container network ...`", () => {
  const c = new Container();
  const out = c.network("--help", []);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("builder(subcommand, args) forwards to `container builder ...`", () => {
  const c = new Container();
  const out = c.builder("--help", []);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("registry(subcommand, args) forwards to `container registry ...`", () => {
  const c = new Container();
  const out = c.registry("login", ["--help"]);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("system(subcommand, args) forwards to `container system ...`", () => {
  const c = new Container();
  const out = c.system("version", []);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

// ===========================================================================
// Documented aliases
// ===========================================================================

test("list() is an alias of ls and forwards to `container list`", () => {
  const c = new Container();
  const out = c.list(["--help"]);
  expect(typeof out).toBe("string");
  expect(out.length).toBeGreaterThan(0);
});

test("single-letter group shortcuts forward (i / r / v / n / s)", () => {
  const c = new Container();
  const cast = c as unknown as Record<string, (s: string, a: string[]) => string>;
  const cases: Array<[string, string, string[]]> = [
    ["i", "ls", ["--help"]], // image
    ["r", "login", ["--help"]], // registry
    ["v", "--help", []], // volume
    ["n", "--help", []], // network
    ["s", "version", []], // system
  ];
  for (const [method, sub, args] of cases) {
    const out = cast[method](sub, args);
    expect(typeof out).toBe("string");
    expect(out.length).toBeGreaterThan(0);
  }
});

// ===========================================================================
// Typed subcommand methods — every real subcommand per `container <group> --help`
// ===========================================================================

test("typed subcommand methods forward to the runtime (daemon-safe --help)", () => {
  const c = new Container();
  const cast = c as unknown as Record<string, (a: string[]) => string>;
  const typed: Array<[string, string[]]> = [
    // image
    ["imageDelete", ["--help"]],
    ["imageInspect", ["--help"]],
    ["imageList", ["--help"]],
    ["imageLoad", ["--help"]],
    ["imagePrune", ["--help"]],
    ["imagePull", ["--help"]],
    ["imagePush", ["--help"]],
    ["imageSave", ["--help"]],
    ["imageTag", ["--help"]],
    // volume
    ["volumeCreate", ["--help"]],
    ["volumeDelete", ["--help"]],
    ["volumeList", ["--help"]],
    ["volumeInspect", ["--help"]],
    ["volumePrune", ["--help"]],
    // network
    ["networkCreate", ["--help"]],
    ["networkDelete", ["--help"]],
    ["networkList", ["--help"]],
    ["networkInspect", ["--help"]],
    ["networkPrune", ["--help"]],
    // builder
    ["builderStart", ["--help"]],
    ["builderStatus", ["--help"]],
    ["builderStop", ["--help"]],
    ["builderDelete", ["--help"]],
    // registry
    ["registryLogin", ["--help"]],
    ["registryLogout", ["--help"]],
    // system
    ["systemDf", ["--help"]],
    ["systemDns", ["--help"]],
    ["systemKernel", ["--help"]],
    ["systemLogs", ["--help"]],
    ["systemProperty", ["--help"]],
    ["systemStart", ["--help"]],
    ["systemStatus", ["--help"]],
    ["systemStop", ["--help"]],
    ["systemVersion", ["--help"]],
  ];
  for (const [method, args] of typed) {
    const out = cast[method](args);
    expect(typeof out).toBe("string");
    expect(out.length).toBeGreaterThan(0);
  }
});
