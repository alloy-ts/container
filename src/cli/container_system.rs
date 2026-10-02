use crate::config::ContainerSystemConfig;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug)]
pub struct ContainerSystemCliHandler {
  config: ContainerSystemConfig,
}

impl Default for ContainerSystemCliHandler {
  fn default() -> Self {
    Self::new()
  }
}

#[napi]
impl ContainerSystemCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      config: ContainerSystemConfig::load(),
    }
  }

  #[napi]
  pub fn start(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn stop(&self) -> Result<()> {
    Ok(())
  }

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
  pub fn logs(&self) -> Vec<String> {
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
    map
  }

  #[napi]
  pub fn dns_create(&self, domain: String) -> Result<String> {
    if domain.is_empty() {
      return Err(Error::from_reason("domain name cannot be empty"));
    }
    Ok(domain)
  }

  #[napi]
  pub fn dns_list(&self) -> Vec<HashMap<String, String>> {
    vec![]
  }

  #[napi]
  pub fn dns_delete(&self, _domain: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn kernel_set(&self, path: String) -> Result<String> {
    if path.is_empty() {
      return Err(Error::from_reason("kernel path cannot be empty"));
    }
    Ok(path)
  }
}
