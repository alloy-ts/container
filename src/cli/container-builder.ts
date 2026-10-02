import native from "../../build/index.js";

export class ContainerBuilderCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get builder() {
    return this.runtime.builder;
  }

  async start(cpus?: number, memory?: string, dns?: string[]): Promise<string> {
    return this.builder.start(cpus, memory, dns);
  }

  async stop(): Promise<void> {
    return this.builder.stop();
  }

  async status(format?: string, quiet?: boolean): Promise<Record<string, string>> {
    return this.builder.status(format, quiet);
  }

  async delete(force?: boolean): Promise<void> {
    return this.builder.delete(force);
  }
}

export const containerBuilderCli = new ContainerBuilderCliHandler();

export class ContainerBuilder {
  static async start(cpus?: number, memory?: string, dns?: string[]): Promise<string> {
    return containerBuilderCli.start(cpus, memory, dns);
  }

  static async stop(): Promise<void> {
    return containerBuilderCli.stop();
  }

  static async status(format?: string, quiet?: boolean): Promise<Record<string, string>> {
    return containerBuilderCli.status(format, quiet);
  }

  static async delete(force?: boolean): Promise<void> {
    return containerBuilderCli.delete(force);
  }
}

export const handleContainerBuilder = ContainerBuilder;
