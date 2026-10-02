use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerMachineCliHandler;

#[napi]
impl ContainerMachineCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self
  }

  #[napi]
  pub fn create(&self, image: String, name: Option<String>) -> Result<String> {
    if image.is_empty() {
      return Err(Error::from_reason("image cannot be empty"));
    }
    let machine_name = name.unwrap_or(image);
    Ok(format!("machine_{machine_name}"))
  }

  #[napi]
  pub fn run(&self, _executable: Option<String>, _args: Option<Vec<String>>) -> Result<i32> {
    Ok(0)
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }

  #[napi]
  pub fn stop(&self, id: String) -> Result<()> {
    if id.is_empty() {
      return Err(Error::from_reason("machine id cannot be empty"));
    }
    Ok(())
  }

  #[napi]
  pub fn delete(&self, id: String) -> Result<()> {
    if id.is_empty() {
      return Err(Error::from_reason("machine id cannot be empty"));
    }
    Ok(())
  }

  #[napi]
  pub fn inspect(&self, id: String) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("id".to_string(), id);
    map.insert("state".to_string(), "running".to_string());
    Ok(map)
  }

  #[napi]
  pub fn logs(&self, _id: String, _follow: Option<bool>, _tail: Option<i32>) -> Vec<String> {
    vec![]
  }

  #[napi]
  pub fn set(&self, id: Option<String>, _key_values: HashMap<String, String>) -> String {
    id.unwrap_or_else(|| "default".to_string())
  }

  #[napi]
  pub fn set_default(&self, id: String) -> String {
    id
  }

  #[napi]
  pub fn capabilities(&self) -> HashMap<String, bool> {
    let mut map = HashMap::new();
    map.insert("nestedVirtualization".to_string(), true);
    map
  }
}
