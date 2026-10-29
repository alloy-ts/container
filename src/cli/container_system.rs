use crate::config::container::ContainerSystemConfig;
use napi_derive::napi;

#[napi]
#[derive(Clone, Debug)]
pub struct SystemCliHandler {
  pub config_json: String,
}

#[napi]
impl SystemCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    let config = ContainerSystemConfig::default();
    let json = serde_json::to_string(&config).unwrap_or_default();
    Self { config_json: json }
  }

  #[napi]
  pub fn get_config_json(&self) -> String {
    self.config_json.clone()
  }
}
