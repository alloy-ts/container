use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerSystemCliHandler;

#[napi]
impl ContainerSystemCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self
  }

  #[napi]
  pub fn start(&self) -> napi::Result<()> {
    Ok(())
  }

  #[napi]
  pub fn stop(&self) -> napi::Result<()> {
    Ok(())
  }

  #[napi]
  pub fn status(&self) -> napi::Result<String> {
    Ok("running".to_string())
  }

  #[napi]
  pub fn version(&self) -> napi::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("version".to_string(), "1.0.0".to_string());
    map.insert("component".to_string(), "@lib/container".to_string());
    Ok(map)
  }

  #[napi]
  pub fn df(&self) -> napi::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("reclaimable".to_string(), "0B".to_string());
    Ok(map)
  }

  #[napi]
  pub fn logs(&self, follow: Option<bool>, last: Option<String>) -> napi::Result<Vec<String>> {
    let _ = (follow, last);
    Ok(vec![])
  }

  #[napi]
  pub fn list_properties(&self) -> napi::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("log.level".to_string(), "info".to_string());
    Ok(map)
  }
}
