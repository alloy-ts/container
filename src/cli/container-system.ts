import native from "../../build/index.js";

export class ContainerSystemCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get system() {
    return this.runtime.system;
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

export const containerSystemCli = new ContainerSystemCliHandler();

export class ContainerSystem {
  static async start(): Promise<void> {
    return containerSystemCli.start();
  }

  static async stop(): Promise<void> {
    return containerSystemCli.stop();
  }

  static async status(): Promise<string> {
    return containerSystemCli.status();
  }

  static async version(): Promise<Record<string, string>> {
    return containerSystemCli.version();
  }

  static async df(): Promise<Record<string, string>> {
    return containerSystemCli.df();
  }

  static async logs(follow?: boolean, last?: string): Promise<string[]> {
    return containerSystemCli.logs(follow, last);
  }

  static async listProperties(): Promise<Record<string, string>> {
    return containerSystemCli.listProperties();
  }

  static async dnsCreate(domain: string, ip?: string): Promise<string> {
    return containerSystemCli.dnsCreate(domain, ip);
  }

  static async dnsList(): Promise<Record<string, string>[]> {
    return containerSystemCli.dnsList();
  }

  static async dnsDelete(domain: string): Promise<void> {
    return containerSystemCli.dnsDelete(domain);
  }

  static async kernelSet(path: string): Promise<string> {
    return containerSystemCli.kernelSet(path);
  }
}

export const handleContainerSystem = ContainerSystem;
