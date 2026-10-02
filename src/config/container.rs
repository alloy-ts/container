use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

fn default_true() -> bool {
  true
}

fn default_build_cpus() -> i32 {
  2
}

fn default_build_memory() -> String {
  "2048mb".to_string()
}

fn default_build_image() -> String {
  "ghcr.io/apple/container-builder-shim/builder:latest".to_string()
}

fn default_container_cpus() -> i32 {
  4
}

fn default_container_memory() -> String {
  "1gb".to_string()
}

fn default_kernel_binary_path() -> String {
  "opt/kata/share/kata-containers/vmlinux-6.18.35-197-debug".to_string()
}

fn default_kernel_url() -> String {
  "https://github.com/kata-containers/kata-containers/releases/download/3.32.0/kata-static-3.32.0-arm64.tar.zst".to_string()
}

fn default_kernel_digest() -> String {
  "sha256:8736c054d9223974735394f822000823baef509e1c33405ec798240fa9b6e4b5".to_string()
}

fn default_machine_cpus() -> i32 {
  4
}

fn default_machine_memory() -> String {
  "2048mb".to_string()
}

fn default_home_mount() -> String {
  "rw".to_string()
}

fn default_registry_domain() -> String {
  "docker.io".to_string()
}

fn default_vminit_image() -> String {
  "ghcr.io/apple/containerization/vminit:latest".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildConfig {
  #[serde(default = "default_true")]
  pub rosetta: bool,
  #[serde(default = "default_build_cpus")]
  pub cpus: i32,
  #[serde(default = "default_build_memory")]
  pub memory: String,
  #[serde(default = "default_build_image")]
  pub image: String,
}

impl Default for BuildConfig {
  fn default() -> Self {
    Self {
      rosetta: default_true(),
      cpus: default_build_cpus(),
      memory: default_build_memory(),
      image: default_build_image(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerConfig {
  #[serde(default = "default_container_cpus")]
  pub cpus: i32,
  #[serde(default = "default_container_memory")]
  pub memory: String,
}

impl Default for ContainerConfig {
  fn default() -> Self {
    Self {
      cpus: default_container_cpus(),
      memory: default_container_memory(),
    }
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DNSConfig {
  #[serde(default)]
  pub domain: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelConfig {
  #[serde(rename = "binaryPath", default = "default_kernel_binary_path")]
  pub binary_path: String,
  #[serde(default = "default_kernel_url")]
  pub url: String,
  #[serde(default = "default_kernel_digest")]
  pub digest: String,
}

impl Default for KernelConfig {
  fn default() -> Self {
    Self {
      binary_path: default_kernel_binary_path(),
      url: default_kernel_url(),
      digest: default_kernel_digest(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineConfig {
  #[serde(default = "default_machine_cpus")]
  pub cpus: i32,
  #[serde(default = "default_machine_memory")]
  pub memory: String,
  #[serde(rename = "homeMount", default = "default_home_mount")]
  pub home_mount: String,
  #[serde(default)]
  pub virtualization: bool,
  #[serde(rename = "kernelPath", default)]
  pub kernel_path: Option<String>,
}

impl Default for MachineConfig {
  fn default() -> Self {
    Self {
      cpus: default_machine_cpus(),
      memory: default_machine_memory(),
      home_mount: default_home_mount(),
      virtualization: false,
      kernel_path: None,
    }
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
  #[serde(default)]
  pub subnet: Option<String>,
  #[serde(default)]
  pub subnetv6: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
  #[serde(default = "default_registry_domain")]
  pub domain: String,
}

impl Default for RegistryConfig {
  fn default() -> Self {
    Self {
      domain: default_registry_domain(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VminitConfig {
  #[serde(default = "default_vminit_image")]
  pub image: String,
}

impl Default for VminitConfig {
  fn default() -> Self {
    Self {
      image: default_vminit_image(),
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

  pub fn default_config_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
      PathBuf::from(home).join(".config").join("container").join("config.toml")
    } else {
      PathBuf::from(".config/container/config.toml")
    }
  }

  pub fn load() -> Self {
    Self::load_from_path(&Self::default_config_path())
  }

  pub fn load_from_path(path: &Path) -> Self {
    if !path.exists() {
      return Self::default();
    }
    let content = match fs::read_to_string(path) {
      Ok(c) => c,
      Err(_) => return Self::default(),
    };

    toml::from_str(&content)
      .unwrap_or_else(|_| serde_json::from_str(&content).unwrap_or_default())
  }
}
