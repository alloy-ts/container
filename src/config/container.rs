use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct BuildConfig {
  pub rosetta: bool,
  pub cpus: usize,
  pub memory: String,
  pub image: String,
}

impl Default for BuildConfig {
  fn default() -> Self {
    Self {
      rosetta: true,
      cpus: 2,
      memory: "2048mb".to_string(),
      image: "ghcr.io/apple/container-builder-shim/builder:0.13.1".to_string(),
    }
  }
}

#[derive(Clone, Debug)]
pub struct ContainerConfig {
  pub cpus: usize,
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

#[derive(Clone, Debug, Default)]
pub struct DNSConfig {
  pub domain: Option<String>,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Default)]
pub struct NetworkConfig {
  pub subnet: Option<String>,
  pub subnetv6: Option<String>,
}

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, Default)]
pub struct MachineConfig {
  pub cpus: Option<usize>,
  pub memory: Option<String>,
  pub home_mount: Option<String>,
  pub virtualization: Option<bool>,
  pub kernel_path: Option<String>,
}

impl MachineConfig {
  pub fn default_cpus() -> usize {
    4
  }

  pub fn default_memory() -> &'static str {
    "1gb"
  }

  pub fn default_home_mount() -> &'static str {
    "rw"
  }

  pub fn validate_kernel_path(path: &str) -> Result<String, String> {
    if path.is_empty() {
      return Err("kernel path cannot be empty".to_string());
    }
    Ok(path.to_string())
  }

  pub fn with_options(&self, kwargs: HashMap<String, String>) -> Result<Self, String> {
    let mut config = self.clone();
    for (k, v) in kwargs {
      match k.as_str() {
        "cpus" => {
          let num = v.parse::<usize>().map_err(|_| format!("failed to parse {v} for cpus"))?;
          if num == 0 {
            return Err("cpus must be a positive integer".to_string());
          }
          config.cpus = Some(num);
        }
        "memory" => {
          config.memory = Some(v);
        }
        "home-mount" => {
          if !["ro", "rw", "none"].contains(&v.as_str()) {
            return Err(format!("invalid home mount option '{v}'"));
          }
          config.home_mount = Some(v);
        }
        "virtualization" => {
          let val = match v.as_str() {
            "true" => true,
            "false" => false,
            _ => return Err(format!("invalid boolean value '{v}' for virtualization")),
          };
          config.virtualization = Some(val);
        }
        "kernel" => {
          if v.is_empty() {
            config.kernel_path = None;
          } else {
            let path = Self::validate_kernel_path(&v)?;
            config.kernel_path = Some(path);
          }
        }
        _ => return Err(format!("unknown key '{k}'")),
      }
    }
    Ok(config)
  }
}

#[derive(Clone, Debug, Default)]
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

impl ContainerSystemConfig {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn load_default() -> Self {
    Self::default()
  }

  pub fn from_toml(content: &str) -> Result<Self, String> {
    let mut config = Self::default();
    let mut current_section = "";

    for line in content.lines() {
      let trimmed = line.trim();
      if trimmed.is_empty() || trimmed.starts_with('#') {
        continue;
      }

      if trimmed.starts_with('[') && trimmed.ends_with(']') {
        current_section = &trimmed[1..trimmed.len() - 1];
        continue;
      }

      if let Some((key, val)) = trimmed.split_once('=') {
        let k = key.trim();
        let v = val.trim().trim_matches('"').trim_matches('\'');

        match (current_section, k) {
          ("build", "rosetta") => config.build.rosetta = v == "true",
          ("build", "cpus") => {
            if let Ok(n) = v.parse() {
              config.build.cpus = n;
            }
          }
          ("build", "memory") => config.build.memory = v.to_string(),
          ("build", "image") => config.build.image = v.to_string(),

          ("container", "cpus") => {
            if let Ok(n) = v.parse() {
              config.container.cpus = n;
            }
          }
          ("container", "memory") => config.container.memory = v.to_string(),

          ("dns", "domain") => config.dns.domain = Some(v.to_string()),

          ("kernel", "binaryPath") | ("kernel", "binary_path") => config.kernel.binary_path = v.to_string(),
          ("kernel", "url") => config.kernel.url = v.to_string(),
          ("kernel", "digest") => config.kernel.digest = v.to_string(),

          ("network", "subnet") => config.network.subnet = Some(v.to_string()),
          ("network", "subnetv6") => config.network.subnetv6 = Some(v.to_string()),

          ("registry", "domain") => config.registry.domain = v.to_string(),

          ("vminit", "image") => config.vminit.image = v.to_string(),

          _ => {}
        }
      }
    }

    Ok(config)
  }
}
