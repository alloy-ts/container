import native from "../../build/index.js";

export class ContainerSystemHandler {
  private get system() {
    const runtime = native.Container.withDefaultConfig();
    return runtime.system;
  }

  async start(): Promise<void> {
    return this.system.start();
  }

  async stop(): Promise<void> {
    return this.system.stop();
  }

  async status(): Promise<string> {
    return this.system.status();
  }

  async version(): Promise<Record<string, string>> {
    return this.system.version();
  }

  async df(): Promise<Record<string, string>> {
    return this.system.df();
  }

  async logs(follow?: boolean, last?: string): Promise<string[]> {
    return this.system.logs(follow, last);
  }

  async listProperties(): Promise<Record<string, string>> {
    return this.system.listProperties();
  }

  async propertyList(format?: string): Promise<string> {
    return this.system.propertyList(format);
  }

  async dnsCreate(domain: string, ip?: string): Promise<string> {
    return this.system.dnsCreate(domain, ip);
  }

  async dnsList(): Promise<Record<string, string>[]> {
    return this.system.dnsList();
  }

  async dnsDelete(domain: string): Promise<void> {
    return this.system.dnsDelete(domain);
  }

  async kernelSet(path: string): Promise<string> {
    return this.system.kernelSet(path);
  }
}

export const containerSystemCli = new ContainerSystemHandler();

export class System {
  static async status(): Promise<string> {
    return containerSystemCli.status();
  }

  static async version(): Promise<Record<string, string>> {
    return containerSystemCli.version();
  }

  static async propertyList(format?: string): Promise<string> {
    return containerSystemCli.propertyList(format);
  }
}
