import { expect, test } from "vite-plus/test";
import { Container, containerOptsFromOciBundle } from "./main.ts";

test("the native module exports Container and the OCI bundle loader", () => {
  expect(typeof Container).toBe("function");
  expect(typeof containerOptsFromOciBundle).toBe("function");
  expect(typeof Container.create).toBe("function");
});
