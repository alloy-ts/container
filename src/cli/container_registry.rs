use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct RegistryLoginOptions {
  pub server: String,
  pub username: Option<String>,
  pub password: Option<String>,
  pub password_stdin: Option<bool>,
  pub scheme: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RegistryLogoutOptions {
  pub server: String,
}

#[derive(Debug, Clone, Default)]
pub struct RegistryListOptions {
  pub format: Option<String>,
  pub quiet: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RegistryInfo {
  pub hostname: String,
  pub username: String,
  pub modified_at: String,
  pub created_at: String,
}

pub fn handle_registry_login(opts: RegistryLoginOptions) -> napi::Result<()> {
  let _ = opts;
  Ok(())
}

pub fn handle_registry_logout(opts: RegistryLogoutOptions) -> napi::Result<()> {
  let _ = opts;
  Ok(())
}

pub fn handle_registry_list(opts: RegistryListOptions) -> napi::Result<Vec<HashMap<String, String>>> {
  let _ = opts;
  Ok(vec![])
}
