import native from "../../build/index.js";
import type { BuildOptions } from "../lib.ts";

export class ContainerImageHandler {
  private runtime = native.Container.withDefaultConfig();
  private imageHandle = this.runtime.images;

  get images() {
    return this.imageHandle;
  }

  async list(): Promise<string[]> {
    return this.images.list();
  }

  async pull(reference: string): Promise<void> {
    return this.images.pull(reference);
  }

  async push(reference: string): Promise<void> {
    return this.images.push(reference);
  }

  async delete(reference: string): Promise<void> {
    return this.images.delete(reference);
  }

  async prune(): Promise<void> {
    return this.images.prune();
  }

  async inspect(references: string[]): Promise<Array<Record<string, string>>> {
    return this.images.inspect(references);
  }

  async load(archivePath: string, force?: boolean): Promise<string[]> {
    return this.images.load(archivePath, force);
  }

  async save(references: string[], outputPath?: string, platform?: string): Promise<Uint8Array> {
    return this.images.save(references, outputPath, platform);
  }

  async tag(source: string, target: string): Promise<string> {
    return this.images.tag(source, target);
  }

  async build(contextDir: string, options?: BuildOptions): Promise<string> {
    return this.images.build(contextDir, options);
  }
}

export const containerImageCli = new ContainerImageHandler();
export const ContainerImage = ContainerImageHandler;
