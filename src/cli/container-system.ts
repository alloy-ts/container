import native from "../../build/index.js";

export class SystemHandler {
  private handler = new native.SystemCliHandler();

  getConfig(): Record<string, any> {
    const json = this.handler.getConfigJson();
    try {
      return JSON.parse(json);
    } catch {
      return {};
    }
  }
}

export const containerSystem = new SystemHandler();

export namespace ContainerSystemNamespace {
  export function getConfig(): Record<string, any> {
    return containerSystem.getConfig();
  }
}

export namespace Container {
  export const system = ContainerSystemNamespace;
}

declare global {
  namespace Container {
    export const system: typeof ContainerSystemNamespace;
  }
}
