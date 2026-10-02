import { JsContainer, JsContainerOptions } from "../lib.ts";
import { ContainerBuildHandler } from "./container-build.ts";

export class ContainerCliHandler {
  public runtime: JsContainer;
  public buildHandler: ContainerBuildHandler;

  constructor(runtime?: JsContainer) {
    this.runtime = runtime ?? JsContainer.withDefaultConfig();
    this.buildHandler = new ContainerBuildHandler(this.runtime);
  }

  public async runContainer(options: JsContainerOptions, name?: string): Promise<JsContainer> {
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
