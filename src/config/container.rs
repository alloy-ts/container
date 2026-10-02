use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySize(pub u64);

impl MemorySize {
  pub fn parse(input: &str) -> Result<Self, String> {
    let trimmed = input.trim().to_lowercase();
    if trimmed.is_empty() {
      return Err("empty memory string".to_string());
    }

    let mut num_str = String::new();
    let mut unit_str = String::new();

    for ch in trimmed.chars() {
      if ch.is_ascii_digit() {
        if !unit_str.is_empty() {
          return Err(format!("invalid memory format: {input}"));
        }
        num_str.push(ch);
      } else {
        unit_str.push(ch);
      }
    }

    let val: u64 = num_str.parse().map_err(|e| format!("invalid number in memory: {e}"))?;

    let multiplier: u64 = match unit_str.as_str() {
      "" | "b" => 1,
      "k" | "kb" | "kib" => 1024,
      "m" | "mb" | "mib" => 1024 * 1024,
      "g" | "gb" | "gib" => 1024 * 1024 * 1024,
      "t" | "tb" | "tib" => 1024 * 1024 * 1024 * 1024,
      "p" | "pb" | "pib" => 1024 * 1024 * 1024 * 1024 * 1024,
      _ => return Err(format!("unknown memory unit: {unit_str}")),
    };

    Ok(MemorySize(val * multiplier))
  }
}

impl Default for MemorySize {
  fn default() -> Self {
    MemorySize(2 * 1024 * 1024 * 1024) // 2GB
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
  pub rosetta: bool,
  pub cpus: usize,
  pub memory: MemorySize,
  pub image: String,
}

impl Default for BuildConfig {
  fn default() -> Self {
    Self {
      rosetta: true,
      cpus: 2,
      memory: MemorySize(2 * 1024 * 1024 * 1024),
      image: "ghcr.io/apple/container-builder-shim/builder:latest".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
  pub cpus: usize,
  pub memory: MemorySize,
}

impl Default for ContainerConfig {
  fn default() -> Self {
    Self {
      cpus: 4,
      memory: MemorySize(1024 * 1024 * 1024),
    }
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DNSConfig {
  pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
  pub subnet: Option<String>,
  pub subnetv6: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HomeMountOption {
  Ro,
  Rw,
  None,
}

impl Default for HomeMountOption {
  fn default() -> Self {
    Self::Rw
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
  pub cpus: usize,
  pub memory: MemorySize,
  pub home_mount: HomeMountOption,
  pub virtualization: bool,
  pub kernel_path: Option<String>,
}

impl Default for MachineConfig {
  fn default() -> Self {
    Self {
      cpus: 4,
      memory: MemorySize(2 * 1024 * 1024 * 1024),
      home_mount: HomeMountOption::Rw,
      virtualization: false,
      kernel_path: None,
    }
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
  pub fn load_default() -> Self {
    Self::default()
  }

  pub fn parse_toml(toml_str: &str) -> Result<Self, String> {
    let mut config = Self::default();
    let mut current_section = "";

    for line in toml_str.lines() {
      let line = line.trim();
      if line.is_empty() || line.starts_with('#') {
        continue;
      }

      if line.starts_with('[') && line.ends_with(']') {
        current_section = &line[1..line.len() - 1];
        continue;
      }

      if let Some((key, val)) = line.split_once('=') {
        let key = key.trim();
        let val = val.trim().trim_matches('"').trim_matches('\'');

        match (current_section, key) {
          ("build", "rosetta") => config.build.rosetta = val.parse().unwrap_or(true),
          ("build", "cpus") => config.build.cpus = val.parse().unwrap_or(2),
          ("build", "memory") => {
            if let Ok(m) = MemorySize::parse(val) {
              config.build.memory = m;
            }
          }
          ("build", "image") => config.build.image = val.to_string(),

          ("container", "cpus") => config.container.cpus = val.parse().unwrap_or(4),
          ("container", "memory") => {
            if let Ok(m) = MemorySize::parse(val) {
              config.container.memory = m;
            }
          }

          ("dns", "domain") => config.dns.domain = Some(val.to_string()),

          ("kernel", "binaryPath") => config.kernel.binary_path = val.to_string(),
          ("kernel", "url") => config.kernel.url = val.to_string(),
          ("kernel", "digest") => config.kernel.digest = val.to_string(),

          ("network", "subnet") => config.network.subnet = Some(val.to_string()),
          ("network", "subnetv6") => config.network.subnetv6 = Some(val.to_string()),

          ("registry", "domain") => config.registry.domain = val.to_string(),

          ("vminit", "image") => config.vminit.image = val.to_string(),

          _ => {}
        }
      }
    }

    Ok(config)
  }
}
