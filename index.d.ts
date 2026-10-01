export class JsContainer {
  constructor(options?: JsOptions);
  static withDefaultConfig(): JsContainer;
  static initDefault(options?: JsOptions): void;
  static rest(options: JsContainerRestOptions): JsContainer;
  importContainer(archivePath: string, name?: string): JsContainer;
  create(options?: JsContainerOptions, name?: string): JsContainer;
  getOrCreate(options?: JsContainerOptions, name?: string): JsGetOrCreateResult;
  listInfo(): Array<JsContainerInfo>;
  getInfo(idOrName: string): JsContainerInfo | null;
  get(idOrName: string): JsContainer | null;
  metrics(): JsRuntimeMetrics;
  get images(): JsImageHandle;
  get volumes(): JsVolumeHandle;
  remove(idOrName: string, force?: boolean): void;
  close(): void;
  shutdown(timeout?: number): void;
  exec(command: string, args?: Array<string>): string;
}

export class JsGetOrCreateResult {
  get created(): boolean;
  get container(): JsContainer;
}

export class JsImageHandle {
  list(): Array<string>;
}

export class JsVolumeHandle {
  list(): Array<string>;
}

export interface JsOptions {
  homeDir?: string;
}

export interface JsContainerOptions {
  image?: string;
  memoryMib?: number;
  cpus?: number;
}

export interface JsContainerRestOptions {
  endpoint?: string;
}

export interface JsContainerInfo {
  id: string;
  name?: string;
  status: string;
}

export interface JsRuntimeMetrics {
  containersCreatedTotal: number;
  numRunningContainers: number;
}
