import { Container as NativeContainer, RegistryHandle } from "../lib.ts";

export namespace Container {
  export function registry(): RegistryHandle {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.registry;
  }
}

export type { RegistryHandle };
export default Container;
