use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

fn default_build_cpus() -> usize {
    2
}

fn default_build_memory() -> String {
    "2048mb".to_string()
}

fn default_build_image() -> String {
    "ghcr.io/apple/container-builder-shim/builder:0.13.1".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_true")]
    pub rosetta: bool,
    #[serde(default = "default_build_cpus")]
    pub cpus: usize,
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

fn default_container_cpus() -> usize {
    4
}

fn default_container_memory() -> String {
    "1gb".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContainerConfig {
    #[serde(default = "default_container_cpus")]
    pub cpus: usize,
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
    pub domain: Option<String>,
}

fn default_vminit_image() -> String {
    "ghcr.io/apple/containerization/vminit:0.34.0".to_string()
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

fn default_kernel_binary_path() -> String {
    "opt/kata/share/kata-containers/vmlinux-6.18.35-197-debug".to_string()
}

fn default_kernel_url() -> String {
    "https://github.com/kata-containers/kata-containers/releases/download/3.32.0/kata-static-3.32.0-arm64.tar.zst".to_string()
}

fn default_kernel_digest() -> String {
    "sha256:8736c054d9223974735394f822000823baef509e1c33405ec798240fa9b6e4b5".to_string()
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

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub subnet: Option<String>,
    pub subnetv6: Option<String>,
}

fn default_registry_domain() -> String {
    "docker.io".to_string()
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

fn default_machine_cpus() -> usize {
    4
}

fn default_machine_memory() -> String {
    "2gb".to_string()
}

fn default_machine_home_mount() -> String {
    "rw".to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineConfig {
    #[serde(default = "default_machine_cpus")]
    pub cpus: usize,
    #[serde(default = "default_machine_memory")]
    pub memory: String,
    #[serde(rename = "homeMount", default = "default_machine_home_mount")]
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
            home_mount: default_machine_home_mount(),
            virtualization: false,
            kernel_path: None,
        }
    }
}

impl MachineConfig {
    pub fn settable_keys() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("cpus", "<number>", "Number of virtual CPUs"),
            ("memory", "<size>", "Memory allocation (e.g., 2G, 1G). Default: half of system memory"),
            ("home-mount", "<string>", "User home directory mount option (ro, rw, none). Default: rw"),
            ("virtualization", "<bool>", "Enable nested virtualization (true|false)."),
            ("kernel", "<path>", "Path to a custom kernel binary. Empty value resets to default."),
        ]
    }

    pub fn help_text() -> String {
        Self::settable_keys()
            .into_iter()
            .map(|(key, val, desc)| {
                let label = format!("{key}={val}");
                format!("{:<24}{}", label, desc)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.cpus == 0 {
            return Err("invalid CPU count '0'. Must be a positive integer.".to_string());
        }
        if !["ro", "rw", "none"].contains(&self.home_mount.as_str()) {
            return Err(format!("invalid home mount option '{}'. Valid: ro, rw, none", self.home_mount));
        }
        Ok(())
    }

    pub fn with(&self, kwargs: &HashMap<String, String>) -> Result<Self, String> {
        let mut new_config = self.clone();
        for (k, v) in kwargs {
            match k.as_str() {
                "cpus" => {
                    new_config.cpus = v.parse::<usize>().map_err(|_| format!("failed to parse {v} for cpus"))?;
                }
                "memory" => {
                    new_config.memory = v.clone();
                }
                "home-mount" | "home_mount" => {
                    if !["ro", "rw", "none"].contains(&v.as_str()) {
                        return Err(format!("invalid home mount option '{v}'. Valid: ro, rw, none"));
                    }
                    new_config.home_mount = v.clone();
                }
                "virtualization" => {
                    new_config.virtualization = v.parse::<bool>().map_err(|_| format!("invalid value '{v}' for virtualization"))?;
                }
                "kernel" | "kernel_path" => {
                    if v.is_empty() {
                        new_config.kernel_path = None;
                    } else {
                        new_config.kernel_path = Some(v.clone());
                    }
                }
                _ => return Err(format!("unknown key '{k}'")),
            }
        }
        new_config.validate()?;
        Ok(new_config)
    }

    pub fn validate_kernel_path(path: &str) -> Result<String, String> {
        let p = PathBuf::from(path);
        if !p.exists() {
            return Err(format!("kernel binary not found at '{path}'"));
        }
        if p.is_dir() {
            return Err(format!("kernel path '{path}' is a directory, expected a file"));
        }
        Ok(path.to_string())
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
    pub fn from_toml_str(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse TOML config: {e}"))
    }

    pub fn to_toml_string(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    pub fn load() -> Self {
        if let Some(home) = std::env::var_os("HOME") {
            let config_path = PathBuf::from(home).join(".config/container/config.toml");
            if config_path.exists() {
                if let Ok(content) = fs::read_to_string(&config_path) {
                    if let Ok(cfg) = Self::from_toml_str(&content) {
                        return cfg;
                    }
                }
            }
        }
        Self::default()
    }
}
