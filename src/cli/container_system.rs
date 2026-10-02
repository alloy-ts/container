use crate::config::ContainerSystemConfig;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ContainerSystemCliHandler {
    pub config: ContainerSystemConfig,
}

impl ContainerSystemCliHandler {
    pub fn new() -> Self {
        Self {
            config: ContainerSystemConfig::load(),
        }
    }

    pub fn status(&self) -> String {
        "running".to_string()
    }

    pub fn version(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("version".to_string(), "1.0.0".to_string());
        map.insert("component".to_string(), "@lib/container".to_string());
        map.insert("builderShim".to_string(), self.config.build.image.clone());
        map.insert("vminit".to_string(), self.config.vminit.image.clone());
        map
    }

    pub fn df(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("reclaimable".to_string(), "0B".to_string());
        map.insert("totalSize".to_string(), "0B".to_string());
        map
    }

    pub fn logs(&self, _follow: Option<bool>, _last: Option<String>) -> Vec<String> {
        vec![]
    }

    pub fn list_properties(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
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

    pub fn property_list(&self, format: Option<String>) -> String {
        if format.as_deref() == Some("json") {
            serde_json::to_string_pretty(&self.config).unwrap_or_else(|_| self.config.to_toml_string())
        } else {
            self.config.to_toml_string()
        }
    }

    pub fn dns_create(&self, domain: String, _ip: Option<String>) -> String {
        domain
    }

    pub fn dns_list(&self) -> Vec<HashMap<String, String>> {
        vec![]
    }

    pub fn dns_delete(&self, _domain: String) -> Result<(), String> {
        Ok(())
    }

    pub fn kernel_set(&self, path: String) -> Result<String, String> {
        crate::config::MachineConfig::validate_kernel_path(&path)?;
        Ok(path)
    }
}
