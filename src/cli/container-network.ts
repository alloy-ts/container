import native from "../../build/index.js";

export class ContainerNetworkHandler {
  private runtime = native.Container.withDefaultConfig();
  private networkHandle = this.runtime.network;

  get network() {
    return this.networkHandle;
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

  async inspect(names: string[]): Promise<Array<Record<string, string>>> {
    return this.network.inspect(names);
  }
}

export const containerNetworkCli = new ContainerNetworkHandler();
export const ContainerNetwork = ContainerNetworkHandler;
