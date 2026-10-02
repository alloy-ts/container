import native from "../../build/index.js";
import type { RegistryLoginOptions, RegistryResource } from "../../build/index.js";

export const login = async (options: RegistryLoginOptions): Promise<string> => {
  return native.registryLogin(options);
};

export const logout = async (server: string): Promise<void> => {
  return native.registryLogout(server);
};

export const list = async (): Promise<RegistryResource[]> => {
  return native.registryList();
};

export class ContainerRegistryHandler {
  public async login(options: RegistryLoginOptions): Promise<string> {
    return login(options);
  }

  public async logout(server: string): Promise<void> {
    return logout(server);
  }

  public async list(): Promise<RegistryResource[]> {
    return list();
  }
}

export const Registry = {
  login,
  logout,
  list,
  RegistryHandler: ContainerRegistryHandler,
};

export const Container = {
  registry: Registry,
  RegistryHandler: ContainerRegistryHandler,
};

export default Registry;
