import { expect, test } from "vite-plus/test";
import { Container } from "./container-network.ts";

test("Container.network function works", () => {
  const net = Container.network();
  expect(net).toBeDefined();

  expect(net.create("test-net")).toBe("test-net");
  expect(net.list()).toContain("default");
  expect(net.inspect(["test-net"])[0].name).toBe("test-net");
  expect(() => net.delete("test-net")).not.toThrow();
  expect(() => net.prune()).not.toThrow();
});
