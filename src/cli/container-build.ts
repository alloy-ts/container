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
}

export const handleContainerBuild = ContainerBuildHandler.handleBuild;
