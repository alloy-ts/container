use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct RegistryResource {
  pub name: String,
  pub username: String,
  pub modification_date: String,
  pub creation_date: String,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct RegistryLoginOptions {
  pub server: String,
  pub username: Option<String>,
  pub password: Option<String>,
  pub password_stdin: Option<bool>,
  pub scheme: Option<String>,
}

#[derive(Default)]
struct InnerRegistryState {
  registries: HashMap<String, RegistryResource>,
}

static REGISTRY_STATE: Mutex<Option<Arc<Mutex<InnerRegistryState>>>> = Mutex::new(None);

fn get_registry_state() -> Arc<Mutex<InnerRegistryState>> {
  let mut lock = REGISTRY_STATE.lock().unwrap();
  if lock.is_none() {
    *lock = Some(Arc::new(Mutex::new(InnerRegistryState::default())));
  }
  Arc::clone(lock.as_ref().unwrap())
}

#[napi(js_name = "registryLogin")]
pub fn registry_login(options: RegistryLoginOptions) -> Result<String> {
  if options.server.trim().is_empty() {
    return Err(Error::from_reason("Server name cannot be empty"));
  }

  let username = options.username.unwrap_or_else(|| "anonymous".to_string());
  if options.password_stdin.unwrap_or(false) && username.is_empty() {
    return Err(Error::from_reason("must provide username with password_stdin"));
  }

  let state = get_registry_state();
  let mut lock = state.lock().map_err(|e| Error::from_reason(e.to_string()))?;
  let resource = RegistryResource {
    name: options.server.clone(),
    username: username.clone(),
    modification_date: "1970-01-01T00:00:00Z".to_string(),
    creation_date: "1970-01-01T00:00:00Z".to_string(),
  };
  lock.registries.insert(options.server.clone(), resource);

  Ok(options.server)
}

#[napi(js_name = "registryLogout")]
pub fn registry_logout(server: String) -> Result<()> {
  let state = get_registry_state();
  let mut lock = state.lock().map_err(|e| Error::from_reason(e.to_string()))?;
  lock.registries.remove(&server);
  Ok(())
}

#[napi(js_name = "registryList")]
pub fn registry_list() -> Result<Vec<RegistryResource>> {
  let state = get_registry_state();
  let lock = state.lock().map_err(|e| Error::from_reason(e.to_string()))?;
  Ok(lock.registries.values().cloned().collect())
}
