import { expect, test } from "vite-plus/test";
import { Container } from "./container-image.ts";

test("Container.images function works", () => {
  const img = Container.images();
  expect(img).toBeDefined();

  expect(() => img.pull("alpine:latest")).not.toThrow();
  expect(() => img.push("alpine:latest")).not.toThrow();
  expect(img.inspect(["alpine:latest"])[0].status).toBe("available");
  expect(img.load("test.tar")).toEqual(["loaded-image:latest"]);
  expect(img.tag("alpine:latest", "alpine:v1")).toBe("alpine:v1");
  expect(img.build("./")).toBe("image-built:latest");
  expect(() => img.delete("alpine:v1")).not.toThrow();
  expect(() => img.prune()).not.toThrow();
});
