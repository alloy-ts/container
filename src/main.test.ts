import { expect, test } from "vite-plus/test";
import { main, JsContainer } from "./main.ts";

test("main returns Container initialized", () => {
  expect(main()).toBe("Container initialized");
});

test("JsContainer is defined", () => {
  expect(JsContainer).toBeDefined();
});
