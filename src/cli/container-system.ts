import native from "../../build/index.js";

export class ContainerSystemCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get system() {
    return this.runtime.system;
  }

  async start() {
    return this.system.start();
  }

  async stop() {
    return this.system.stop();
  }

  async status() {
    return this.system.status();
  }

  async version() {
    return this.system.version();
  }

  async df() {
    return this.system.df();
  }

  async logs(follow?: boolean, last?: string) {
    return this.system.logs(follow, last);
  }

  async listProperties() {
    return this.system.listProperties();
  }
}

export const containerSystemCli = new ContainerSystemCliHandler();
