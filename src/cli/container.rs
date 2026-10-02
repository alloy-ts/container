use napi::bindgen_prelude::*;
use std::collections::HashMap;

pub struct ContainerCli;

impl ContainerCli {
    pub fn build(context_dir: &str, dockerfile: Option<&str>, tags: Vec<String>) -> Result<String> {
        let _ = (context_dir, dockerfile);
        Ok(tags.first().cloned().unwrap_or_else(|| "image-built:latest".to_string()))
    }

    pub fn prune_info(pruned_ids: Vec<String>) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("reclaimed".to_string(), "0B".to_string());
        map.insert("pruned_count".to_string(), pruned_ids.len().to_string());
        map
    }
}

pub struct BuilderCli;

impl BuilderCli {
    pub fn start() -> Result<String> {
        Ok("buildkit".to_string())
    }

    pub fn stop() -> Result<()> {
        Ok(())
    }

    pub fn status() -> Result<HashMap<String, String>> {
        let mut map = HashMap::new();
        map.insert("id".to_string(), "buildkit".to_string());
        map.insert("status".to_string(), "running".to_string());
        Ok(map)
    }

    pub fn delete(_force: bool) -> Result<()> {
        Ok(())
    }
}
