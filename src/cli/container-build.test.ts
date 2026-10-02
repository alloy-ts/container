import { expect, test } from "vite-plus/test";
import { Container } from "./container-build.ts";

test("Container.build function works with default options", () => {
  const result = Container.build("./");
  expect(result).toBe("image-built:latest");
});

test("Container.build function works with custom build options", () => {
  const result = Container.build("./", {
    dockerfile: "Dockerfile.test",
    target: "builder",
    tags: ["my-app:v1.0.0"],
    buildArgs: { ENV: "production" },
  });
  expect(result).toBe("image-built:latest");
});
