import { expect, test } from "vite-plus/test";
import { Container } from "./container-machine.ts";

test("Container.machines function works", () => {
  const mac = Container.machines();
  expect(mac).toBeDefined();

  const id = mac.create("alpine:latest");
  expect(id).toBe("machine_alpine:latest");
  expect(mac.inspect(id).state).toBe("running");
  expect(mac.capabilities().nestedVirtualization).toBe(true);
  expect(mac.set("m1", { cpus: "2" })).toBe("m1");
  expect(mac.setDefault("m1")).toBe("m1");
  expect(() => mac.stop(id)).not.toThrow();
  expect(() => mac.delete(id)).not.toThrow();
});
