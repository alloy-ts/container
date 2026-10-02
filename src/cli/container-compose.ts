import { JsComposeHandle, JsContainer } from "../lib.ts";

export class ContainerComposeCliHandler {
  public runtime: JsContainer;
  public compose: JsComposeHandle;

  constructor(runtime?: JsContainer) {
    this.runtime = runtime ?? JsContainer.withDefaultConfig();
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
