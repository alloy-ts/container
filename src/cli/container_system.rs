use crate::config::ContainerSystemConfig;
use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerSystem {
  config: ContainerSystemConfig,
}

#[napi]
impl ContainerSystem {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      config: ContainerSystemConfig::load(),
    }
  }

  #[napi]
  pub fn start(&self) {}

  #[napi]
  pub fn stop(&self) {}

  #[napi]
  pub fn status(&self) -> String {
    "running".to_string()
  }

  #[napi]
  pub fn version(&self) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("version".to_string(), "1.0.0".to_string());
    map.insert("component".to_string(), "@lib/container".to_string());
    map
  }

  #[napi]
  pub fn df(&self) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("reclaimable".to_string(), "0B".to_string());
    map
  }

  #[napi]
  pub fn logs(&self, _follow: Option<bool>, _last: Option<String>) -> Vec<String> {
    vec![]
  }

  #[napi]
  pub fn list_properties(&self) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("log.level".to_string(), "info".to_string());
    map.insert("build.cpus".to_string(), self.config.build.cpus.to_string());
    map.insert("build.memory".to_string(), self.config.build.memory.clone());
    map.insert("build.rosetta".to_string(), self.config.build.rosetta.to_string());
    map.insert("build.image".to_string(), self.config.build.image.clone());
    map.insert("container.cpus".to_string(), self.config.container.cpus.to_string());
    map.insert("container.memory".to_string(), self.config.container.memory.clone());
    map.insert("registry.domain".to_string(), self.config.registry.domain.clone());
    map.insert("vminit.image".to_string(), self.config.vminit.image.clone());
    map.insert("kernel.binary_path".to_string(), self.config.kernel.binary_path.clone());
    map.insert("kernel.url".to_string(), self.config.kernel.url.clone());
    map.insert("kernel.digest".to_string(), self.config.kernel.digest.clone());
    map
  }

  #[napi]
  pub fn dns_create(&self, domain: String, _ip: Option<String>) -> String {
    domain
  }

  #[napi]
  pub fn dns_list(&self) -> Vec<HashMap<String, String>> {
    vec![]
  }

  #[napi]
  pub fn dns_delete(&self, _domain: String) {}

  #[napi]
  pub fn kernel_set(&mut self, path: String) -> String {
    self.config.kernel.binary_path = path.clone();
    path
  }
}

pub type ContainerSystemCliHandler = ContainerSystem;
