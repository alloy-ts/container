import native from "../../build/index.js";

export interface RegistryLoginParams {
  server: string;
  username?: string;
  password?: string;
}

export interface RegistryListParams {
  format?: string;
  quiet?: boolean;
}

export class ContainerRegistryHandler {
  private get registry() {
    const runtime = native.Container.withDefaultConfig();
    return runtime.registry;
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

export const containerRegistryCli = new ContainerRegistryHandler();

export class Registry {
  static async login(params: RegistryLoginParams): Promise<void> {
    return containerRegistryCli.login(params.server, params.username, params.password);
  }

  static async logout(server: string): Promise<void> {
    return containerRegistryCli.logout(server);
  }

  static async list(): Promise<string[]> {
    return containerRegistryCli.list();
  }
}
