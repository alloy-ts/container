import { expect, test } from "vite-plus/test";
import { Container } from "./container-volume.ts";

test("Container.volumes function works", () => {
  const vol = Container.volumes();
  expect(vol).toBeDefined();

  expect(vol.create("vol1")).toBe("vol1");
  expect(vol.inspect(["vol1"])[0].driver).toBe("local");
  expect(() => vol.delete("vol1")).not.toThrow();
  expect(() => vol.prune()).not.toThrow();
});
