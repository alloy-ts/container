import console from "node:console";
import { Container } from "./lib.ts";

export const main = () => {
  const runtime = Container.withDefaultConfig();
  return runtime;
};

console.log(main());
