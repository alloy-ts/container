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

export const JsComposeHandle = native.JsComposeHandle;
export const JsComposeSystemHandle = native.JsComposeSystemHandle;

export const JsHealthState = {
  None: "None",
  Starting: "Starting",
  Healthy: "Healthy",
  Unhealthy: "Unhealthy",
} as const;

export type JsHealthState = (typeof JsHealthState)[keyof typeof JsHealthState];

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

export type JsPublishedPort = {
  guestPort: number;
  hostIp: string;
  hostPort: number;
  protocol: string;
};

export type JsOutboundNetworkInfo = {
  mode: string;
  allowNet: string[];
};

export type JsInboundNetworkInfo = {
  mode: string;
  allowNet: string[];
};

export type JsNetworkInfo = {
  outbound: JsOutboundNetworkInfo;
  inbound: JsInboundNetworkInfo;
  mode: string;
  allowNet: string[];
  publishedPorts?: JsPublishedPort[] | null;
};

export type JsHealthStatus = {
  state: JsHealthState;
  failures: number;
  lastCheck?: string | null;
};

export type JsContainerStateInfo = {
  status: string;
  running: boolean;
  pid?: number | null;
  exitCode?: number | null;
};

export type JsContainerInfo = {
  id: string;
  name?: string | null;
  state: JsContainerStateInfo;
  createdAt: string;
  startedAt?: string | null;
  lastActivityAt?: string | null;
  image: string;
  cpus: number;
  memoryMib: number;
  network?: JsNetworkInfo | null;
  autoStop: number;
  autoDelete: number;
  autoResume: boolean;
  healthStatus: JsHealthStatus;
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
