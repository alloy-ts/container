use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct RegistryLoginOptions {
  pub server: String,
  pub username: Option<String>,
  pub password: Option<String>,
  pub password_stdin: Option<bool>,
  pub scheme: Option<String>,
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
    password: Option<String>,
    password_stdin: Option<bool>,
  ) -> Result<()> {
    if password_stdin.unwrap_or(false) && (username.as_deref().unwrap_or("").is_empty()) {
      return Err(Error::from_reason(
        "must provide --username with --password-stdin",
      ));
    }
    if server.is_empty() {
      return Err(Error::from_reason("registry server cannot be empty"));
    }
    let _ = password;
    Ok(())
  }

  #[napi]
  pub fn logout(&self, registry: String) -> Result<()> {
    if registry.is_empty() {
      return Err(Error::from_reason("registry server cannot be empty"));
    }
    Ok(())
  }

  #[napi]
  pub fn list(&self, _quiet: Option<bool>) -> Vec<HashMap<String, String>> {
    vec![]
  }
}
