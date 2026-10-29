import native from "../../build/index.js";
import type { ContainerOptions } from "../lib.ts";

export class ContainerCliHandler {
  private runtime = native.Container.withDefaultConfig();

  async create(options: ContainerOptions, name?: string) {
    return this.runtime.create(options, name);
  }

  async list() {
    return this.runtime.listInfo();
  }

  async get(idOrName: string) {
    return this.runtime.get(idOrName);
  }

  async remove(idOrName: string, force?: boolean) {
    return this.runtime.remove(idOrName, force);
  }

  async prune() {
    return this.runtime.prune();
  }
}

export const containerCli = new ContainerCliHandler();

export namespace Container {
  export async function create(options: ContainerOptions, name?: string) {
    return containerCli.create(options, name);
  }

  export async function list() {
    return containerCli.list();
  }

  export async function get(idOrName: string) {
    return containerCli.get(idOrName);
  }

  export async function remove(idOrName: string, force?: boolean) {
    return containerCli.remove(idOrName, force);
  }

  export async function prune() {
    return containerCli.prune();
  }
}

declare global {
  namespace Container {
    export function create(options: ContainerOptions, name?: string): Promise<any>;
    export function list(): Promise<any>;
    export function get(idOrName: string): Promise<any>;
    export function remove(idOrName: string, force?: boolean): Promise<any>;
    export function prune(): Promise<any>;
  }
}
