use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct NetworkCreateOptions {
  pub name: String,
  pub plugin: Option<String>,
  pub subnet: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkDeleteOptions {
  pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct NetworkInspectOptions {
  pub names: Vec<String>,
}

pub fn handle_network_create(opts: NetworkCreateOptions) -> napi::Result<String> {
  Ok(opts.name)
}

pub fn handle_network_list() -> napi::Result<Vec<String>> {
  Ok(vec!["default".to_string()])
}

pub fn handle_network_delete(opts: NetworkDeleteOptions) -> napi::Result<()> {
  let _ = opts;
  Ok(())
}

pub fn handle_network_prune() -> napi::Result<()> {
  Ok(())
}

pub fn handle_network_inspect(opts: NetworkInspectOptions) -> napi::Result<Vec<HashMap<String, String>>> {
  let mut results = Vec::new();
  for name in opts.names {
    let mut map = HashMap::new();
    map.insert("name".to_string(), name);
    map.insert("driver".to_string(), "bridge".to_string());
    results.push(map);
  }
  Ok(results)
}
