import { Container as NativeContainer, BuildOptions } from "../lib.ts";

export namespace Container {
  export function build(contextDir: string = ".", options?: BuildOptions): string {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.images.build(contextDir, options);
  }
}

export default Container;
