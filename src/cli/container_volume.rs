use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerVolumeCliHandler;

#[napi]
impl ContainerVolumeCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self
  }

  #[napi]
  pub fn create(&self, name: String, _size: Option<String>) -> Result<String> {
    if name.is_empty() {
      return Err(Error::from_reason("volume name cannot be empty"));
    }
    Ok(name)
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }

  #[napi]
  pub fn delete(&self, name: String) -> Result<()> {
    if name.is_empty() {
      return Err(Error::from_reason("volume name cannot be empty"));
    }
    Ok(())
  }

  #[napi]
  pub fn prune(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn inspect(&self, names: Vec<String>) -> Result<Vec<HashMap<String, String>>> {
    let mut results = Vec::new();
    for name in names {
      let mut map = HashMap::new();
      map.insert("name".to_string(), name);
      map.insert("driver".to_string(), "local".to_string());
      results.push(map);
    }
    Ok(results)
  }
}
