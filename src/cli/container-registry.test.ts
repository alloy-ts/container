import { expect, test } from "vite-plus/test";
import { Container } from "./container-registry.ts";

test("Container.registry function works", () => {
  const reg = Container.registry();
  expect(reg).toBeDefined();

  expect(() => reg.login("docker.io", "user", "pass")).not.toThrow();
  expect(() => reg.logout("docker.io")).not.toThrow();
  expect(Array.isArray(reg.list())).toBe(true);
});
