import console from "node:console";
import { JsContainer } from "./lib.ts";

export const main = () => {
  const runtime = JsContainer.withDefaultConfig();
  return runtime;
};

console.log(main());
