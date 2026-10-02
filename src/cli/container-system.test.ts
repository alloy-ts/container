import { expect, test } from "vite-plus/test";
import { Container } from "../lib.ts";

test("Container system handle methods work", () => {
  const runtime = Container.withDefaultConfig();
  const system = runtime.system;

  expect(() => system.start()).not.toThrow();
  expect(system.status()).toBe("running");
  expect(system.version().version).toBe("1.0.0");
  expect(system.df().reclaimable).toBe("0B");
  expect(system.listProperties()["log.level"]).toBe("info");
  expect(system.dnsCreate("test.local")).toBe("test.local");
  expect(() => system.dnsDelete("test.local")).not.toThrow();
  expect(system.kernelSet("/opt/kernel")).toBe("/opt/kernel");
  expect(() => system.stop()).not.toThrow();
});
