import { Container as NativeContainer, ImageHandle } from "../lib.ts";

export namespace Container {
  export function images(): ImageHandle {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.images;
  }
}

export type { ImageHandle };
export default Container;
