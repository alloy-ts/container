use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct RegistryLoginOptions {
  pub server: String,
  pub username: String,
  pub password_stdin: bool,
  pub scheme: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct RegistryLogoutOptions {
  pub registry: String,
}

#[derive(Clone, Debug, Default)]
pub struct RegistryListOptions {
  pub format: String,
  pub quiet: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ContainerRegistryCliHandler;

impl ContainerRegistryCliHandler {
  pub fn new() -> Self {
    Self
  }

  pub fn login(&self, server: &str, username: Option<&str>, password_stdin: bool) -> Result<(), String> {
    if password_stdin && (username.is_none() || username.unwrap().is_empty()) {
      return Err("must provide --username with --password-stdin".to_string());
    }
    if server.is_empty() {
      return Err("registry server cannot be empty".to_string());
    }
    Ok(())
  }

  pub fn logout(&self, registry: &str) -> Result<(), String> {
    if registry.is_empty() {
      return Err("registry server cannot be empty".to_string());
    }
    Ok(())
  }

  pub fn list(&self, _quiet: bool) -> Vec<HashMap<String, String>> {
    vec![]
  }
}
