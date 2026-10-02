import { Container as NativeContainer, RegistryHandle } from "../lib.ts";

export namespace Registry {
  export function login(server: string, username?: string, password?: string): void {
    const runtime = NativeContainer.withDefaultConfig();
    runtime.registry.login(server, username, password);
  }

  export function logout(server: string): void {
    const runtime = NativeContainer.withDefaultConfig();
    runtime.registry.logout(server);
  }

  export function list(): Record<string, string>[] {
    const runtime = NativeContainer.withDefaultConfig();
    return runtime.registry.list();
  }
}

export type { RegistryHandle };
export default Registry;
