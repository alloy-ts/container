import native from "../../build/index.js";

export interface RegistryLoginParams {
  server: string;
  username?: string;
  password?: string;
  passwordStdin?: boolean;
}

export interface RegistryLogoutParams {
  server: string;
}

export interface RegistryItem {
  name: string;
  username: string;
  modificationDate: string;
  creationDate: string;
}

const globalRegistryCli = new native.RegistryCliHandler();

export class RegistryHandler {
  private handler = globalRegistryCli;

  async login(params: RegistryLoginParams): Promise<void> {
    return this.handler.login(params.server, params.username, params.password);
  }

  async logout(params: RegistryLogoutParams): Promise<void> {
    return this.handler.logout(params.server);
  }

  async list(): Promise<RegistryItem[]> {
    return this.handler.list();
  }
}

export namespace Registry {
  export async function login(server: string, username?: string, password?: string): Promise<void> {
    const handler = new RegistryHandler();
    return handler.login({ server, username, password });
  }

  export async function logout(server: string): Promise<void> {
    const handler = new RegistryHandler();
    return handler.logout({ server });
  }

  export async function list(): Promise<RegistryItem[]> {
    const handler = new RegistryHandler();
    return handler.list();
  }
}

declare global {
  namespace Container {
    export const registry: typeof Registry;
  }
}

if (typeof globalThis !== "undefined") {
  (globalThis as any).Container = (globalThis as any).Container || {};
  (globalThis as any).Container.registry = Registry;
}
