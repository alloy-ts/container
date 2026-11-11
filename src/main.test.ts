import { expect, test } from "vite-plus/test";
import { add } from "./main.ts";

test("add() returns the sum of two integers (NAPI round-trip)", () => {
  expect(add(1, 2)).toBe(3);
});
