import { JsBuildOptions, JsContainer } from "../lib.ts";

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

export class ContainerBuildHandler {
  private runtime: JsContainer;

  constructor(runtime?: JsContainer) {
    this.runtime = runtime ?? JsContainer.withDefaultConfig();
  }

  public validateOptions(options: BuildCommandOptions): void {
    if (options.file && options.file !== "-" && options.file.length === 0) {
      throw new Error("Dockerfile path cannot be empty");
    }
  }

  public async run(options: BuildCommandOptions = {}): Promise<string> {
    this.validateOptions(options);

    const contextDir = options.contextDir ?? ".";
    const buildOptions: JsBuildOptions = {
      dockerfile: options.file,
      target: options.target,
      buildArgs: options.buildArg,
      tags: options.tag ?? ["latest"],
    };

    return this.runtime.images.build(contextDir, buildOptions);
  }
}

export const buildCommand = async (options: BuildCommandOptions = {}): Promise<string> => {
  const handler = new ContainerBuildHandler();
  return handler.run(options);
};

export default ContainerBuildHandler;
