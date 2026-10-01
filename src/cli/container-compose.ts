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
