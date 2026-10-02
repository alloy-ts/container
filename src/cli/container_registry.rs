use napi_derive::napi;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct RegistryResource {
  pub name: String,
  pub username: String,
  pub modification_date: String,
  pub creation_date: String,
}

#[derive(Clone, Debug)]
pub struct PrintableRegistry {
  pub registry: RegistryResource,
}

impl PrintableRegistry {
  pub fn new(registry: RegistryResource) -> Self {
    Self { registry }
  }

  pub fn table_header() -> Vec<&'static str> {
    vec!["HOSTNAME", "USERNAME", "MODIFIED", "CREATED"]
  }

  pub fn table_row(&self) -> Vec<String> {
    vec![
      self.registry.name.clone(),
      self.registry.username.clone(),
      self.registry.modification_date.clone(),
      self.registry.creation_date.clone(),
    ]
  }

  pub fn quiet_value(&self) -> String {
    self.registry.name.clone()
  }
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct RegistryListOptions {
  pub format: Option<String>,
  pub quiet: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct RegistryCommand {
  pub subcommand: String,
  pub options: HashMap<String, String>,
}

impl RegistryCommand {
  pub fn new(subcommand: impl Into<String>) -> Self {
    Self {
      subcommand: subcommand.into(),
      options: HashMap::new(),
    }
  }

  pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.options.insert(key.into(), value.into());
    self
  }
}

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerRegistryCliHandler;

#[napi]
impl ContainerRegistryCliHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self
  }

  #[napi]
  pub fn login(
    &self,
    server: String,
    username: Option<String>,
    _password: Option<String>,
    password_stdin: Option<bool>,
  ) -> napi::Result<()> {
    if password_stdin.unwrap_or(false) && username.as_deref().unwrap_or("").is_empty() {
      return Err(napi::Error::from_reason(
        "must provide --username with --password-stdin",
      ));
    }
    if server.is_empty() {
      return Err(napi::Error::from_reason("registry server name is required"));
    }
    Ok(())
  }

  #[napi]
  pub fn logout(&self, registry: String) -> napi::Result<()> {
    if registry.is_empty() {
      return Err(napi::Error::from_reason("registry server name is required"));
    }
    Ok(())
  }

  #[napi]
  pub fn list(&self, _opts: Option<RegistryListOptions>) -> Vec<String> {
    vec![]
  }
}
