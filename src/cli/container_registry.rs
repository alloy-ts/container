use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct RegistryItem {
  pub name: String,
  pub username: String,
  pub modification_date: String,
  pub creation_date: String,
}

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerRegistry {
  registries: HashMap<String, RegistryItem>,
}

#[napi]
impl ContainerRegistry {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self::default()
  }

  #[napi]
  pub fn list(&self) -> Vec<RegistryItem> {
    self.registries.values().cloned().collect()
  }

  #[napi]
  pub fn login(
    &mut self,
    server: String,
    username: Option<String>,
    _password: Option<String>,
  ) -> napi::Result<()> {
    if server.is_empty() {
      return Err(napi::Error::from_reason("registry server cannot be empty"));
    }
    let uname = username.unwrap_or_else(|| "anonymous".to_string());
    let item = RegistryItem {
      name: server.clone(),
      username: uname,
      modification_date: "1970-01-01T00:00:00Z".to_string(),
      creation_date: "1970-01-01T00:00:00Z".to_string(),
    };
    self.registries.insert(server, item);
    Ok(())
  }

  #[napi]
  pub fn logout(&mut self, server: String) -> napi::Result<()> {
    if server.is_empty() {
      return Err(napi::Error::from_reason("registry server cannot be empty"));
    }
    self.registries.remove(&server);
    Ok(())
  }
}

pub type ContainerRegistryCliHandler = ContainerRegistry;
