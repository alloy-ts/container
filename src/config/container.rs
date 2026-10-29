use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ContainerSystemConfig {
  pub build: BuildConfig,
  pub container: ContainerConfig,
  pub dns: DNSConfig,
  pub kernel: KernelConfig,
  pub machine: MachineConfig,
  pub network: NetworkConfig,
  pub registry: RegistryConfig,
  pub vminit: VminitConfig,
}

impl Default for ContainerSystemConfig {
  fn default() -> Self {
    Self {
      build: BuildConfig::default(),
      container: ContainerConfig::default(),
      dns: DNSConfig::default(),
      kernel: KernelConfig::default(),
      machine: MachineConfig::default(),
      network: NetworkConfig::default(),
      registry: RegistryConfig::default(),
      vminit: VminitConfig::default(),
    }
  }
}

impl ContainerSystemConfig {
  pub fn load() -> Self {
    if let Some(config_path) = Self::default_config_path() {
      if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
          if let Ok(config) = toml::from_str::<ContainerSystemConfig>(&content) {
            return config;
          }
        }
      }
    }
    Self::default()
  }

  pub fn default_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("container").join("config.toml"))
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BuildConfig {
  pub rosetta: bool,
  pub cpus: i32,
  pub memory: String,
  pub image: String,
}

impl Default for BuildConfig {
  fn default() -> Self {
    Self {
      rosetta: true,
      cpus: 2,
      memory: "2048MB".to_string(),
      image: "ghcr.io/apple/container-builder-shim/builder:0.13.1".to_string(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ContainerConfig {
  pub cpus: i32,
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
#[serde(default)]
pub struct DNSConfig {
  pub domain: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct VminitConfig {
  pub image: String,
}

impl Default for VminitConfig {
  fn default() -> Self {
    Self {
      image: "ghcr.io/apple/containerization/vminit:0.34.0".to_string(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
pub struct NetworkConfig {
  pub subnet: Option<String>,
  pub subnetv6: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
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
#[serde(default)]
pub struct MachineConfig {
  pub cpus: i32,
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
      memory: "2GB".to_string(),
      home_mount: "rw".to_string(),
      virtualization: false,
      kernel_path: None,
    }
  }
}
