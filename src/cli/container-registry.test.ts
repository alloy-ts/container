import { expect, test } from "vite-plus/test";
import { Registry } from "./container-registry.ts";

test("Registry login, list, and logout", async () => {
  const server = "docker.io";
  const loginResult = await Registry.login({
    server,
    username: "testuser",
    password: "secretpassword",
  });
  expect(loginResult).toBe("docker.io");

  const listResults = await Registry.list();
  expect(listResults.length).toBeGreaterThanOrEqual(1);
  const found = listResults.find((r) => r.name === "docker.io");
  expect(found).toBeDefined();
  expect(found?.username).toBe("testuser");

  await Registry.logout("docker.io");
  const listAfterLogout = await Registry.list();
  expect(listAfterLogout.find((r) => r.name === "docker.io")).toBeUndefined();
});

test("Registry login throws on empty server name", async () => {
  await expect(Registry.login({ server: "" })).rejects.toThrow("Server name cannot be empty");
});
