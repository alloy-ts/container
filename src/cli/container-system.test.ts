import { expect, test } from "vite-plus/test";
import { System } from "./container-system.ts";

test("System status and version", async () => {
  const status = await System.status();
  expect(status).toBe("running");

  const ver = await System.version();
  expect(ver.version).toBe("1.0.0");
  expect(ver.component).toBe("container");
});

test("System propertyList reads config properties", async () => {
  const props = await System.propertyList();
  expect(props["build.cpus"]).toBeDefined();
  expect(props["container.cpus"]).toBeDefined();
  expect(props["registry.domain"]).toBe("docker.io");
});

test("System DNS and kernel operations", async () => {
  const dns = await System.dnsCreate("test.local");
  expect(dns).toBe("test.local");

  const dnsList = await System.dnsList();
  expect(Array.isArray(dnsList)).toBe(true);

  await System.dnsDelete("test.local");

  const kernel = await System.kernelSet("/path/vmlinux");
  expect(kernel).toBe("/path/vmlinux");
});
