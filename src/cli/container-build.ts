import native from "../../build/index.js";
import type { ContainerBuildOptions } from "../../build/index.js";

export interface BuildCommandOptions {
  contextDir?: string;
  file?: string;
  target?: string;
  buildArg?: Record<string, string>;
  tag?: string[];
  arch?: string[];
  os?: string[];
  platform?: string[];
  noCache?: boolean;
  quiet?: boolean;
  cpus?: number;
  memory?: string;
}

export const build = async (options: BuildCommandOptions = {}): Promise<string> => {
  const napiOptions: ContainerBuildOptions = {
    contextDir: options.contextDir ?? ".",
    file: options.file,
    target: options.target,
    buildArgs: options.buildArg,
    tags: options.tag ?? ["latest"],
    arch: options.arch,
    os: options.os,
    platform: options.platform,
    noCache: options.noCache,
    quiet: options.quiet,
    cpus: options.cpus,
    memory: options.memory,
  };

  return native.containerBuild(napiOptions);
};

export class ContainerBuildHandler {
  private runtime: InstanceType<typeof native.Container>;

  constructor(runtime?: InstanceType<typeof native.Container>) {
    this.runtime = runtime ?? native.Container.withDefaultConfig();
  }

  public validateOptions(options: BuildCommandOptions): void {
    if (options.file !== undefined && options.file !== "-" && options.file.length === 0) {
      throw new Error("Dockerfile path cannot be empty");
    }
  }

  public async run(options: BuildCommandOptions = {}): Promise<string> {
    this.validateOptions(options);
    return build(options);
  }
}

export const Container = {
  build,
  BuildHandler: ContainerBuildHandler,
};

export const buildCommand = build;

export default Container;
