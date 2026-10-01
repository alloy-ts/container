import native from "../build/index.js";

export const JsContainer = native.JsContainer;
export const JsGetOrCreateResult = native.JsGetOrCreateResult;
export const JsImageHandle = native.JsImageHandle;
export const JsVolumeHandle = native.JsVolumeHandle;

export const Container = native.JsContainer;

export type JsOptions = {
  homeDir?: string;
};

export type JsContainerOptions = {
  image?: string;
  memoryMib?: number;
  cpus?: number;
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

export default JsContainer;
