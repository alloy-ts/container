import native from "../../build/index.js";

export interface RegistryItem {
  name: string;
  username: string;
  modificationDate: string;
  creationDate: string;
}

export class ContainerRegistryHandler {
  private runtime = native.Container.withDefaultConfig();
  private registryHandle = this.runtime.registry;

  get registry() {
    return this.registryHandle;
  }

  async login(server: string, username?: string, password?: string): Promise<void> {
    return this.registry.login(server, username, password);
  }

  async logout(server: string): Promise<void> {
    return this.registry.logout(server);
  }

  async list(): Promise<RegistryItem[]> {
    return this.registry.list();
  }
}

export const containerRegistryCli = new ContainerRegistryHandler();
export const ContainerRegistry = ContainerRegistryHandler;

declare global {
  interface Container {
    registry?: ContainerRegistryHandler;
  }
}
