import native from "../../build/index.js";
import type { BuildOptions } from "../lib.ts";

export class ContainerImageCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get images() {
    return this.runtime.images;
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

  async inspect(references: string[]): Promise<Record<string, string>[]> {
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

export const containerImageCli = new ContainerImageCliHandler();

export class ContainerImage {
  static async list(): Promise<string[]> {
    return containerImageCli.list();
  }

  static async pull(reference: string): Promise<void> {
    return containerImageCli.pull(reference);
  }

  static async push(reference: string): Promise<void> {
    return containerImageCli.push(reference);
  }

  static async delete(reference: string): Promise<void> {
    return containerImageCli.delete(reference);
  }

  static async prune(): Promise<void> {
    return containerImageCli.prune();
  }

  static async inspect(references: string[]): Promise<Record<string, string>[]> {
    return containerImageCli.inspect(references);
  }

  static async load(archivePath: string, force?: boolean): Promise<string[]> {
    return containerImageCli.load(archivePath, force);
  }

  static async save(references: string[], outputPath?: string, platform?: string): Promise<Uint8Array> {
    return containerImageCli.save(references, outputPath, platform);
  }

  static async tag(source: string, target: string): Promise<string> {
    return containerImageCli.tag(source, target);
  }

  static async build(contextDir: string, options?: BuildOptions): Promise<string> {
    return containerImageCli.build(contextDir, options);
  }
}

export const handleContainerImage = ContainerImage;
