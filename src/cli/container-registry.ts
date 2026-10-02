import native from "../../build/index.js";

export interface RegistryLoginParams {
  server: string;
  username?: string;
  password?: string;
  passwordStdin?: boolean;
}

export class ContainerRegistryCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get registry() {
    return this.runtime.registry;
  }

  async login(server: string, username?: string, password?: string): Promise<void> {
    return this.registry.login(server, username, password);
  }

  async logout(server: string): Promise<void> {
    return this.registry.logout(server);
  }

  async list(): Promise<string[]> {
    return this.registry.list();
  }
}

export const containerRegistryCli = new ContainerRegistryCliHandler();

export class ContainerRegistry {
  static async login(server: string, username?: string, password?: string): Promise<void> {
    return containerRegistryCli.login(server, username, password);
  }

  static async logout(server: string): Promise<void> {
    return containerRegistryCli.logout(server);
  }

  static async list(): Promise<string[]> {
    return containerRegistryCli.list();
  }
}

export const handleContainerRegistry = ContainerRegistry;
