import native from "../../build/index.js";

export class ContainerComposeCliHandler {
  public runtime: InstanceType<typeof native.Container>;
  public compose: InstanceType<typeof native.ComposeHandle>;

  constructor(runtime?: InstanceType<typeof native.Container>) {
    this.runtime = runtime ?? native.Container.withDefaultConfig();
    this.compose = this.runtime.compose;
  }

  public async up(detach = true, build = false): Promise<void> {
    return this.compose.up(detach, build);
  }

  public async down(volumes = false): Promise<void> {
    return this.compose.down(volumes);
  }

  public async ps(): Promise<string[]> {
    return this.compose.ps();
  }

  public async version(): Promise<string> {
    return this.compose.version();
  }
}

export default ContainerComposeCliHandler;
