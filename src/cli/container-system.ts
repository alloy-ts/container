import native from "../../build/index.js";

export const start = async (): Promise<void> => {
  return native.systemStart();
};

export const stop = async (): Promise<void> => {
  return native.systemStop();
};

export const status = async (): Promise<string> => {
  return native.systemStatus();
};

export const version = async (): Promise<Record<string, string>> => {
  return native.systemVersion();
};

export const df = async (): Promise<Record<string, string>> => {
  return native.systemDf();
};

export const logs = async (follow?: boolean, last?: string): Promise<string[]> => {
  return native.systemLogs(follow, last);
};

export const propertyList = async (): Promise<Record<string, string>> => {
  return native.systemPropertyList();
};

export const dnsCreate = async (domain: string, ip?: string): Promise<string> => {
  return native.systemDnsCreate(domain, ip);
};

export const dnsList = async (): Promise<Array<Record<string, string>>> => {
  return native.systemDnsList();
};

export const dnsDelete = async (domain: string): Promise<void> => {
  return native.systemDnsDelete(domain);
};

export const kernelSet = async (path: string): Promise<string> => {
  return native.systemKernelSet(path);
};

export class ContainerSystemHandler {
  public async start(): Promise<void> {
    return start();
  }

  public async stop(): Promise<void> {
    return stop();
  }

  public async status(): Promise<string> {
    return status();
  }

  public async version(): Promise<Record<string, string>> {
    return version();
  }

  public async df(): Promise<Record<string, string>> {
    return df();
  }

  public async logs(follow?: boolean, last?: string): Promise<string[]> {
    return logs(follow, last);
  }

  public async propertyList(): Promise<Record<string, string>> {
    return propertyList();
  }
}

export const System = {
  start,
  stop,
  status,
  version,
  df,
  logs,
  propertyList,
  dnsCreate,
  dnsList,
  dnsDelete,
  kernelSet,
  SystemHandler: ContainerSystemHandler,
};

export const Container = {
  system: System,
  SystemHandler: ContainerSystemHandler,
};

export default System;
