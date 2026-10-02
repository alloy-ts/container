import { Container as NativeContainer, MachineHandle } from "../lib.ts";

export namespace Container {
  export function machines(): MachineHandle {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.machines;
  }
}

export type { MachineHandle };
export default Container;
