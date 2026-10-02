import native from "../../build/index.js";

export class ContainerVolumeCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get volumes() {
    return this.runtime.volumes;
  }

  async create(name: string, size?: string): Promise<string> {
    return this.volumes.create(name, size);
  }

  async list(): Promise<string[]> {
    return this.volumes.list();
  }

  async delete(name: string): Promise<void> {
    return this.volumes.delete(name);
  }

  async prune(): Promise<void> {
    return this.volumes.prune();
  }

  async inspect(names: string[]): Promise<Record<string, string>[]> {
    return this.volumes.inspect(names);
  }
}

export const containerVolumeCli = new ContainerVolumeCliHandler();

export class ContainerVolume {
  static async create(name: string, size?: string): Promise<string> {
    return containerVolumeCli.create(name, size);
  }

  static async list(): Promise<string[]> {
    return containerVolumeCli.list();
  }

  static async delete(name: string): Promise<void> {
    return containerVolumeCli.delete(name);
  }

  static async prune(): Promise<void> {
    return containerVolumeCli.prune();
  }

  static async inspect(names: string[]): Promise<Record<string, string>[]> {
    return containerVolumeCli.inspect(names);
  }
}

export const handleContainerVolume = ContainerVolume;
