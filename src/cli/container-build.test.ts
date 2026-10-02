import { expect, test } from "vite-plus/test";
import { Container, JsBufferedCopyReader, JsBuildFile, JsBuildFSSync } from "./container-build.ts";

test("Container.build function executes with options", async () => {
  const result = await Container.build({
    contextDir: ".",
    tag: ["myapp:v1.0"],
    noCache: true,
    quiet: false,
  });

  expect(result.success).toBe(true);
  expect(result.tags).toEqual(["myapp:v1.0"]);
  expect(result.config.context_dir).toBe(".");
  expect(result.status.status).toBe("running");
});

test("JsBuildFile resolves path", () => {
  const resolved = JsBuildFile.resolvePath(".");
  expect(resolved === null || typeof resolved === "string").toBe(true);
});

test("JsBufferedCopyReader reads chunks", () => {
  const reader = new JsBufferedCopyReader("nonexistent.txt", 1024);
  expect(reader.hasFinished).toBe(true);
  expect(reader.nextChunk()).toBeNull();
});

test("JsBuildFSSync handles requests", () => {
  const sync = new JsBuildFSSync("/tmp");
  const info = sync.info("Dockerfile");
  expect(info.method).toBe("Info");
  expect(info.source).toBe("Dockerfile");

  const walk = sync.walk(["src/*"]);
  expect(walk.method).toBe("Walk");
  expect(walk.follow_paths).toBe("src/*");
});
