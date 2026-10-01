import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const native = require("../index.js");

export const JsContainer = native.JsContainer;
export const JsGetOrCreateResult = native.JsGetOrCreateResult;
export const JsImageHandle = native.JsImageHandle;
export const JsVolumeHandle = native.JsVolumeHandle;

export type {
  JsOptions,
  JsContainerOptions,
  JsContainerRestOptions,
  JsContainerInfo,
  JsRuntimeMetrics,
} from "../index.d.ts";
