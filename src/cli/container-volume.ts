import { Container as NativeContainer, VolumeHandle } from "../lib.ts";

export namespace Container {
  export function volumes(): VolumeHandle {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.volumes;
  }
}

export type { VolumeHandle };
export default Container;
