import native from "../../build/index.js";

export class ContainerSystemHandler {
  private runtime = native.Container.withDefaultConfig();
  private systemHandle = this.runtime.system;

  get system() {
    return this.systemHandle;
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

  async dnsCreate(domain: string, ip?: string): Promise<string> {
    return this.system.dnsCreate(domain, ip);
  }

  async dnsList(): Promise<Array<Record<string, string>>> {
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
export const ContainerSystem = ContainerSystemHandler;
