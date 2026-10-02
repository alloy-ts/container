import native from "../../build/index.js";
import type { BuildOptions } from "../lib.ts";

export interface ContainerBuildParams extends BuildOptions {
  contextDir?: string;
}

export class ContainerBuildHandler {
  static async handleBuild(params: ContainerBuildParams = {}): Promise<string> {
    const contextDir = params.contextDir || ".";
    const container = native.Container.withDefaultConfig();
    return container.images.build(contextDir, params);
  }

  static async build(params: ContainerBuildParams = {}): Promise<string> {
    return this.handleBuild(params);
  }
}

export const handleContainerBuild = ContainerBuildHandler.handleBuild;
export const ContainerBuild = ContainerBuildHandler;

export namespace ContainerBuildNamespace {
  export async function build(params: ContainerBuildParams = {}): Promise<string> {
    return ContainerBuildHandler.handleBuild(params);
  }
}

export namespace Container {
  export const build = ContainerBuildNamespace.build;
}

declare global {
  namespace Container {
    export const build: typeof ContainerBuildNamespace.build;
  }
}
