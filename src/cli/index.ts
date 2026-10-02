export * from "./container.ts";
export * from "./container-compose.ts";
export * from "./container-build.ts";
export * from "./container-registry.ts";
export * from "./container-system.ts";

import { Container as ContainerCore } from "./container.ts";
import { ContainerBuildNamespace } from "./container-build.ts";
import { Registry } from "./container-registry.ts";
import { ContainerSystemNamespace } from "./container-system.ts";
import { ContainerComposeNamespace } from "./container-compose.ts";

export namespace Container {
  export const create = ContainerCore.create;
  export const list = ContainerCore.list;
  export const get = ContainerCore.get;
  export const remove = ContainerCore.remove;
  export const prune = ContainerCore.prune;

  export const build = ContainerBuildNamespace.build;
  export const registry = Registry;
  export const system = ContainerSystemNamespace;
  export const compose = ContainerComposeNamespace;
}
