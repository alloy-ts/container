import native from "../../build/index.js";

export class ContainerNetworkCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get network() {
    return this.runtime.network;
  }

  async create(name: string, plugin?: string, subnet?: string): Promise<string> {
    return this.network.create(name, plugin, subnet);
  }

  async list(): Promise<string[]> {
    return this.network.list();
  }

  async delete(name: string): Promise<void> {
    return this.network.delete(name);
  }

  async prune(): Promise<void> {
    return this.network.prune();
  }

  async inspect(names: string[]): Promise<Record<string, string>[]> {
    return this.network.inspect(names);
  }
}

export const containerNetworkCli = new ContainerNetworkCliHandler();

export class ContainerNetwork {
  static async create(name: string, plugin?: string, subnet?: string): Promise<string> {
    return containerNetworkCli.create(name, plugin, subnet);
  }

  static async list(): Promise<string[]> {
    return containerNetworkCli.list();
  }

  static async delete(name: string): Promise<void> {
    return containerNetworkCli.delete(name);
  }

  static async prune(): Promise<void> {
    return containerNetworkCli.prune();
  }

  static async inspect(names: string[]): Promise<Record<string, string>[]> {
    return containerNetworkCli.inspect(names);
  }
}

export const handleContainerNetwork = ContainerNetwork;
