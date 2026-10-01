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
