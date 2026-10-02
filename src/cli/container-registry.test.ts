import { expect, test } from "vite-plus/test";
import { Registry } from "./container-registry.ts";

test("Registry login and logout work", () => {
  expect(() => Registry.login("docker.io", "user", "secret")).not.toThrow();
  expect(() => Registry.logout("docker.io")).not.toThrow();
});

test("Registry list returns array of logins", () => {
  const logins = Registry.list();
  expect(Array.isArray(logins)).toBe(true);
});
