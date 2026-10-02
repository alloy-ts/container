import native from "../../build/index.js";

export class ContainerRegistryCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get registry() {
    return this.runtime.registry;
  }

  async login(server: string, username?: string, password?: string, passwordStdin?: boolean) {
    return this.registry.login(server, username, password, passwordStdin);
  }

  async logout(server: string) {
    return this.registry.logout(server);
  }

  async list() {
    return this.registry.list();
  }
}

export const containerRegistryCli = new ContainerRegistryCliHandler();

export namespace Container {
  export function registry(): ContainerRegistryCliHandler {
    return containerRegistryCli;
  }
}

declare global {
  interface Container {
    registry(): ContainerRegistryCliHandler;
  }
}
