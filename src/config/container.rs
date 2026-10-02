use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildConfig {
  pub rosetta: bool,
  pub cpus: u32,
  pub memory: String,
  pub image: String,
}

impl Default for BuildConfig {
  fn default() -> Self {
    Self {
      rosetta: true,
      cpus: 2,
      memory: "2048mb".to_string(),
      image: "ghcr.io/apple/container-builder-shim/builder:latest".to_string(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerConfig {
  pub cpus: u32,
  pub memory: String,
}

impl Default for ContainerConfig {
  fn default() -> Self {
    Self {
      cpus: 4,
      memory: "1gb".to_string(),
    }
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DNSConfig {
  pub domain: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelConfig {
  #[serde(rename = "binaryPath")]
  pub binary_path: String,
  pub url: String,
  pub digest: String,
}

impl Default for KernelConfig {
  fn default() -> Self {
    Self {
      binary_path: "opt/kata/share/kata-containers/vmlinux-6.18.35-197-debug".to_string(),
      url: "https://github.com/kata-containers/kata-containers/releases/download/3.32.0/kata-static-3.32.0-arm64.tar.zst".to_string(),
      digest: "sha256:8736c054d9223974735394f822000823baef509e1c33405ec798240fa9b6e4b5".to_string(),
    }
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
  pub subnet: Option<String>,
  pub subnetv6: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
  pub domain: String,
}

impl Default for RegistryConfig {
  fn default() -> Self {
    Self {
      domain: "docker.io".to_string(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VminitConfig {
  pub image: String,
}

impl Default for VminitConfig {
  fn default() -> Self {
    Self {
      image: "ghcr.io/apple/containerization/vminit:latest".to_string(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineConfig {
  pub cpus: u32,
  pub memory: String,
  #[serde(rename = "homeMount")]
  pub home_mount: String,
  pub virtualization: bool,
  #[serde(rename = "kernelPath")]
  pub kernel_path: Option<String>,
}

impl Default for MachineConfig {
  fn default() -> Self {
    Self {
      cpus: 4,
      memory: "2gb".to_string(),
      home_mount: "rw".to_string(),
      virtualization: false,
      kernel_path: None,
    }
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContainerSystemConfig {
  #[serde(default)]
  pub build: BuildConfig,
  #[serde(default)]
  pub container: ContainerConfig,
  #[serde(default)]
  pub dns: DNSConfig,
  #[serde(default)]
  pub kernel: KernelConfig,
  #[serde(default)]
  pub machine: MachineConfig,
  #[serde(default)]
  pub network: NetworkConfig,
  #[serde(default)]
  pub registry: RegistryConfig,
  #[serde(default)]
  pub vminit: VminitConfig,
}

impl ContainerSystemConfig {
  pub fn new() -> Self {
    Self::default()
  }
}
