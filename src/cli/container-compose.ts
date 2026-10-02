import native from "../../build/index.js";

export class ContainerComposeCliHandler {
  private runtime = native.Container.withDefaultConfig();

  get compose() {
    return this.runtime.compose;
  }

  async up(detach?: boolean, build?: boolean) {
    return this.compose.up(detach, build);
  }

  async down(volumes?: boolean) {
    return this.compose.down(volumes);
  }

  async ps() {
    return this.compose.ps();
  }

  async logs(follow?: boolean) {
    return this.compose.logs(follow);
  }

  async version() {
    return this.compose.version();
  }
}

export const containerComposeCli = new ContainerComposeCliHandler();

export namespace ContainerComposeNamespace {
  export async function up(detach?: boolean, build?: boolean) {
    return containerComposeCli.up(detach, build);
  }

  export async function down(volumes?: boolean) {
    return containerComposeCli.down(volumes);
  }

  export async function ps() {
    return containerComposeCli.ps();
  }

  export async function logs(follow?: boolean) {
    return containerComposeCli.logs(follow);
  }

  export async function version() {
    return containerComposeCli.version();
  }
}

export namespace Container {
  export const compose = ContainerComposeNamespace;
}

declare global {
  namespace Container {
    export const compose: typeof ContainerComposeNamespace;
  }
}
