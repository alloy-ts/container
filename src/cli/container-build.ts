import native from "../../build/index.js";

export const JsBuilderCommand = native.JsBuilderCommand;
export const JsBuildFile = native.JsBuildFile;
export const JsBufferedCopyReader = native.JsBufferedCopyReader;
export const JsBuildFSSync = native.JsBuildFSSync;
export const JsCliContainerHandler = native.JsCliContainerHandler;

export interface BuildOptions {
  arch?: string[];
  buildArg?: string[];
  cpus?: number;
  file?: string;
  label?: string[];
  memory?: string;
  noCache?: boolean;
  output?: string[];
  os?: string[];
  platform?: string[];
  progress?: "auto" | "plain" | "tty";
  quiet?: boolean;
  secret?: string[];
  ssh?: string;
  tag?: string[];
  target?: string;
  vsockPort?: number;
  contextDir?: string;
  pull?: boolean;
}

export const Container = {
  async build(options: BuildOptions = {}) {
    const handler = new native.JsCliContainerHandler();
    const config = handler.buildConfig({
      arch: options.arch,
      buildArg: options.buildArg,
      cpus: options.cpus ? BigInt(options.cpus) : undefined,
      file: options.file,
      label: options.label,
      memory: options.memory,
      noCache: options.noCache,
      output: options.output,
      os: options.os,
      platform: options.platform,
      progress: options.progress,
      quiet: options.quiet,
      secret: options.secret,
      ssh: options.ssh,
      targetImageNames: options.tag,
      target: options.target,
      vsockPort: options.vsockPort,
      contextDir: options.contextDir,
      pull: options.pull,
    });

    const builderCmd = new native.JsBuilderCommand();
    const status = builderCmd.status({ quiet: options.quiet });

    const resolvedFile = native.JsBuildFile.resolvePath(options.contextDir || ".");

    return {
      success: true,
      config,
      status,
      resolvedDockerfile: resolvedFile,
      contextDir: options.contextDir || ".",
      tags: options.tag || [],
    };
  },
};

export default Container;
