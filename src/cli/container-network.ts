import { Container as NativeContainer, NetworkHandle } from "../lib.ts";

export namespace Container {
  export function network(): NetworkHandle {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.network;
  }
}

export type { NetworkHandle };
export default Container;
