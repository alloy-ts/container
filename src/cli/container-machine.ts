import native from "../../build/index.js";

export class ContainerMachineCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get machines() {
    return this.runtime.machines;
  }

  async create(image: string, name?: string): Promise<string> {
    return this.machines.create(image, name);
  }

  async run(executable?: string, args?: string[]): Promise<number> {
    return this.machines.run(executable, args);
  }

  async list(): Promise<string[]> {
    return this.machines.list();
  }

  async stop(id: string): Promise<void> {
    return this.machines.stop(id);
  }

  async delete(id: string): Promise<void> {
    return this.machines.delete(id);
  }

  async inspect(id: string): Promise<Record<string, string>> {
    return this.machines.inspect(id);
  }

  async logs(id: string, follow?: boolean, tail?: number, boot?: boolean): Promise<string[]> {
    return this.machines.logs(id, follow, tail, boot);
  }

  async set(id?: string, keyValues: Record<string, string> = {}): Promise<string> {
    return this.machines.set(id, keyValues);
  }

  async setDefault(id: string): Promise<string> {
    return this.machines.setDefault(id);
  }

  async capabilities(): Promise<Record<string, boolean>> {
    return this.machines.capabilities();
  }
}

export const containerMachineCli = new ContainerMachineCliHandler();

export class ContainerMachine {
  static async create(image: string, name?: string): Promise<string> {
    return containerMachineCli.create(image, name);
  }

  static async run(executable?: string, args?: string[]): Promise<number> {
    return containerMachineCli.run(executable, args);
  }

  static async list(): Promise<string[]> {
    return containerMachineCli.list();
  }

  static async stop(id: string): Promise<void> {
    return containerMachineCli.stop(id);
  }

  static async delete(id: string): Promise<void> {
    return containerMachineCli.delete(id);
  }

  static async inspect(id: string): Promise<Record<string, string>> {
    return containerMachineCli.inspect(id);
  }

  static async logs(id: string, follow?: boolean, tail?: number, boot?: boolean): Promise<string[]> {
    return containerMachineCli.logs(id, follow, tail, boot);
  }

  static async set(id?: string, keyValues: Record<string, string> = {}): Promise<string> {
    return containerMachineCli.set(id, keyValues);
  }

  static async setDefault(id: string): Promise<string> {
    return containerMachineCli.setDefault(id);
  }

  static async capabilities(): Promise<Record<string, boolean>> {
    return containerMachineCli.capabilities();
  }
}

export const handleContainerMachine = ContainerMachine;
