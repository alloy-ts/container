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
export const ContainerRegistry = native.ContainerRegistry;
export const ContainerSystem = native.ContainerSystem;
export const ContainerImage = native.ContainerImage;
export const ContainerNetwork = native.ContainerNetwork;
export const BuilderHandle = native.BuilderHandle;
export const EfiVarStore = native.EfiVarStore;

export const BuildTransfer = native.BuildTransfer;
export const ImageTransfer = native.ImageTransfer;
export const ServerStream = native.ServerStream;

export const ComposeHandle = native.ComposeHandle;
export const ComposeSystemHandle = native.ComposeSystemHandle;

// Backward compatibility aliases
export const JsContainer = Container;
export const JsGetOrCreateResult = GetOrCreateResult;
export const JsImageHandle = ImageHandle;
export const JsVolumeHandle = VolumeHandle;
export const JsMachineHandle = MachineHandle;
export const JsK8sHandle = K8sHandle;
export const JsNetworkHandle = NetworkHandle;
export const JsRegistryHandle = RegistryHandle;
export const JsSystemHandle = SystemHandle;
export const JsBuilderHandle = BuilderHandle;
export const JsEfiVarStore = EfiVarStore;
export const JsBuildTransfer = BuildTransfer;
export const JsImageTransfer = ImageTransfer;
export const JsServerStream = ServerStream;
export const JsComposeHandle = ComposeHandle;
export const JsComposeSystemHandle = ComposeSystemHandle;

export const HealthState = {
  None: "None",
  Starting: "Starting",
  Healthy: "Healthy",
  Unhealthy: "Unhealthy",
} as const;

export type HealthState = (typeof HealthState)[keyof typeof HealthState];
export const JsHealthState = HealthState;
export type JsHealthState = HealthState;

export type BuildOptions = {
  dockerfile?: string;
  target?: string;
  buildArgs?: Record<string, string>;
  tags?: string[];
  arch?: string[];
  cacheIn?: string[];
  cacheOut?: string[];
  cpus?: number;
  file?: string;
  label?: string[];
  memory?: string;
  noCache?: boolean;
  output?: string[];
  os?: string[];
  platform?: string[];
  progress?: string;
  quiet?: boolean;
  secret?: string[];
  ssh?: string;
  vsockPort?: number;
  contextDir?: string;
  pull?: boolean;
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

// Aliases
export type JsBuildOptions = BuildOptions;
export type JsOptions = Options;
export type JsContainerOptions = ContainerOptions;
export type JsContainerRestOptions = ContainerRestOptions;
export type JsPublishedPort = PublishedPort;
export type JsOutboundNetworkInfo = OutboundNetworkInfo;
export type JsInboundNetworkInfo = InboundNetworkInfo;
export type JsNetworkInfo = NetworkInfo;
export type JsHealthStatus = HealthStatus;
export type JsContainerStateInfo = ContainerStateInfo;
export type JsContainerInfo = ContainerInfo;
export type JsRuntimeMetrics = RuntimeMetrics;
export type JsIo = Io;
export type JsInfoRequest = InfoRequest;
export type JsInfoResponse = InfoResponse;
export type JsClientStream = ClientStream;

export * from "./cli/index.ts";

export default Container;
