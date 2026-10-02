import { expect, test } from "vite-plus/test";
import { Container, buildCommand } from "./container-build.ts";

test("Container.build runs with default options", async () => {
  const result = await Container.build();
  expect(result).toBe("latest");
});

test("Container.build runs with custom tags and contextDir", async () => {
  const result = await Container.build({
    contextDir: "./app",
    tag: ["my-app:v1.0.0"],
  });
  expect(result).toBe("my-app:v1.0.0");
});

test("buildCommand helper functions identically to Container.build", async () => {
  const result = await buildCommand({
    tag: ["helper-image:latest"],
  });
  expect(result).toBe("helper-image:latest");
});

test("Container.BuildHandler validates dockerfile options", async () => {
  const handler = new Container.BuildHandler();
  await expect(handler.run({ file: "" })).rejects.toThrow("Dockerfile path cannot be empty");
});
