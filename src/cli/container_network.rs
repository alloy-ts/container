use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ContainerNetworkCliHandler;

impl ContainerNetworkCliHandler {
  pub fn new() -> Self {
    Self
  }

  pub fn create(&self, name: &str, _plugin: Option<&str>, _subnet: Option<&str>) -> Result<String, String> {
    if name.is_empty() {
      return Err("network name cannot be empty".to_string());
    }
    Ok(name.to_string())
  }

  pub fn list(&self) -> Vec<String> {
    vec!["default".to_string()]
  }

  pub fn delete(&self, name: &str) -> Result<(), String> {
    if name.is_empty() {
      return Err("network name cannot be empty".to_string());
    }
    Ok(())
  }

  pub fn prune(&self) -> Result<(), String> {
    Ok(())
  }

  pub fn inspect(&self, names: &[String]) -> Result<Vec<HashMap<String, String>>, String> {
    let mut results = Vec::new();
    for name in names {
      let mut map = HashMap::new();
      map.insert("name".to_string(), name.clone());
      map.insert("driver".to_string(), "bridge".to_string());
      results.push(map);
    }
    Ok(results)
  }
}
