import native from "../build/index.js";

export const Container = native.Container;
export const GetOrCreateResult = native.GetOrCreateResult;
export const ImageHandle = native.ImageHandle;
export const VolumeHandle = native.VolumeHandle;
export const MachineHandle = native.MachineHandle;
export const K8sHandle = native.K8sHandle;
export const NetworkHandle = native.NetworkHandle;
export const RegistryHandle = native.RegistryHandle;
export const SystemHandle = native.SystemHandle;
export const BuilderHandle = native.BuilderHandle;
export const EfiVarStore = native.EfiVarStore;

export const BuildTransfer = native.BuildTransfer;
export const ImageTransfer = native.ImageTransfer;
export const ServerStream = native.ServerStream;

export const BuildFile = native.BuildFile;
export const BufferedCopyReader = native.BufferedCopyReader;
export const BuildFsSync = native.BuildFsSync;
export const TerminalCommand = native.TerminalCommand;

export const ComposeHandle = native.ComposeHandle;
export const ComposeSystemHandle = native.ComposeSystemHandle;

export const HealthState = {
  None: "None",
  Starting: "Starting",
  Healthy: "Healthy",
  Unhealthy: "Unhealthy",
} as const;

export type HealthState = (typeof HealthState)[keyof typeof HealthState];

export type BuildOptions = {
  dockerfile?: string;
  target?: string;
  buildArgs?: Record<string, string>;
  tags?: string[];
};

export type Options = {
  homeDir?: string;
};

export type ContainerOptions = {
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

export type ContainerRestOptions = {
  endpoint?: string;
};

export type PublishedPort = {
  guestPort: number;
  hostIp: string;
  hostPort: number;
  protocol: string;
};

export type OutboundNetworkInfo = {
  mode: string;
  allowNet: string[];
};

export type InboundNetworkInfo = {
  mode: string;
  allowNet: string[];
};

export type NetworkInfo = {
  outbound: OutboundNetworkInfo;
  inbound: InboundNetworkInfo;
  mode: string;
  allowNet: string[];
  publishedPorts?: PublishedPort[] | null;
};

export type HealthStatus = {
  state: HealthState;
  failures: number;
  lastCheck?: string | null;
};

export type ContainerStateInfo = {
  status: string;
  running: boolean;
  pid?: number | null;
  exitCode?: number | null;
};

export type ContainerInfo = {
  id: string;
  name?: string | null;
  state: ContainerStateInfo;
  createdAt: string;
  startedAt?: string | null;
  lastActivityAt?: string | null;
  image: string;
  cpus: number;
  memoryMib: number;
  network?: NetworkInfo | null;
  autoStop: number;
  autoDelete: number;
  autoResume: boolean;
  healthStatus: HealthStatus;
};

export type RuntimeMetrics = {
  containeresCreatedTotal: number;
  numRunningContaineres: number;
};

export type Io = {
  data: Uint8Array;
};

export type InfoRequest = {
  id: string;
};

export type InfoResponse = {
  id: string;
  status: string;
};

export type ClientStream = {
  data: Uint8Array;
};

export {
  ContainerBuildHandler,
  Container as ContainerBuildNamespace,
  buildCommand,
} from "./cli/container-build.ts";
export { ContainerCliHandler } from "./cli/container.ts";
export { ContainerComposeCliHandler } from "./cli/container-compose.ts";

export default Container;
