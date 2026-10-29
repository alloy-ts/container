use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct RegistryItem {
  pub name: String,
  pub username: String,
  #[napi(js_name = "modificationDate")]
  pub modification_date: String,
  #[napi(js_name = "creationDate")]
  pub creation_date: String,
}

#[napi]
#[derive(Clone, Debug, Default)]
pub struct RegistryCliHandler {
  registries: HashMap<String, RegistryItem>,
}

#[napi]
impl RegistryCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      registries: HashMap::new(),
    }
  }

  #[napi]
  pub fn login(&mut self, server: String, username: Option<String>, _password: Option<String>) -> napi::Result<()> {
    if server.is_empty() {
      return Err(napi::Error::from_reason("registry server name cannot be empty"));
    }
    let username = username.unwrap_or_default();
    let now = "1970-01-01T00:00:00Z".to_string();
    let item = RegistryItem {
      name: server.clone(),
      username,
      modification_date: now.clone(),
      creation_date: now,
    };
    self.registries.insert(server, item);
    Ok(())
  }

  #[napi]
  pub fn logout(&mut self, server: String) -> napi::Result<()> {
    if server.is_empty() {
      return Err(napi::Error::from_reason("registry server name cannot be empty"));
    }
    self.registries.remove(&server);
    Ok(())
  }

  #[napi]
  pub fn list(&self) -> Vec<RegistryItem> {
    self.registries.values().cloned().collect()
  }
}
