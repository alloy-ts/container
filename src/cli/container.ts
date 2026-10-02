import native from "../build/index.js";
import type { ContainerOptions } from "../build/index.js";
import { ContainerBuildHandler } from "./container-build.ts";

export class ContainerCliHandler {
  public runtime: InstanceType<typeof native.Container>;
  public buildHandler: ContainerBuildHandler;

  constructor(runtime?: InstanceType<typeof native.Container>) {
    this.runtime = runtime ?? native.Container.withDefaultConfig();
    this.buildHandler = new ContainerBuildHandler(this.runtime);
  }

  public async runContainer(
    options: ContainerOptions,
    name?: string,
  ): Promise<InstanceType<typeof native.Container>> {
    const container = await this.runtime.create(options, name);
    await container.start();
    return container;
  }

  public async buildImage(contextDir = ".", file?: string, tags: string[] = []): Promise<string> {
    return this.buildHandler.run({
      contextDir,
      file,
      tag: tags,
    });
  }

  public async pruneContainers(): Promise<string[]> {
    return this.runtime.prune();
  }
}

export default ContainerCliHandler;
