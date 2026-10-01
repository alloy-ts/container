import native from "../build/index.js";

export const JsContainer = native.JsContainer;
export const JsGetOrCreateResult = native.JsGetOrCreateResult;
export const JsImageHandle = native.JsImageHandle;
export const JsVolumeHandle = native.JsVolumeHandle;
export const JsMachineHandle = native.JsMachineHandle;
export const JsK8sHandle = native.JsK8sHandle;
export const JsNetworkHandle = native.JsNetworkHandle;
export const JsRegistryHandle = native.JsRegistryHandle;
export const JsSystemHandle = native.JsSystemHandle;
export const JsEfiVarStore = native.JsEfiVarStore;

export const JsBuildTransfer = native.JsBuildTransfer;
export const JsImageTransfer = native.JsImageTransfer;
export const JsServerStream = native.JsServerStream;

export const Container = native.JsContainer;

export type JsOptions = {
  homeDir?: string;
};

export type JsContainerOptions = {
  image?: string;
  memoryMib?: number;
  cpus?: number;
  name?: string;
  env?: Record<string, string>;
  workdir?: string;
  user?: string;
  detach?: boolean;
  interactive?: boolean;
  tty?: boolean;
};

export type JsContainerRestOptions = {
  endpoint?: string;
};

export type JsContainerState = {
  status: string;
};

export type JsContainerInfo = {
  id: string;
  name?: string;
  state: JsContainerState;
};

export type JsRuntimeMetrics = {
  containeresCreatedTotal: number;
  numRunningContaineres: number;
};

export type JsIo = {
  data: Uint8Array;
};

export type JsInfoRequest = {
  id: string;
};

export type JsInfoResponse = {
  id: string;
  status: string;
};

export type JsClientStream = {
  data: Uint8Array;
};

export default JsContainer;
