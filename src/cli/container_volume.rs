use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct VolumeCreateOptions {
  pub name: String,
  pub size: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct VolumeDeleteOptions {
  pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct VolumeInspectOptions {
  pub names: Vec<String>,
}

pub fn handle_volume_list() -> napi::Result<Vec<String>> {
  Ok(vec![])
}

pub fn handle_volume_create(opts: VolumeCreateOptions) -> napi::Result<String> {
  Ok(opts.name)
}

pub fn handle_volume_delete(opts: VolumeDeleteOptions) -> napi::Result<()> {
  let _ = opts;
  Ok(())
}

pub fn handle_volume_prune() -> napi::Result<()> {
  Ok(())
}

pub fn handle_volume_inspect(opts: VolumeInspectOptions) -> napi::Result<Vec<HashMap<String, String>>> {
  let mut results = Vec::new();
  for name in opts.names {
    let mut map = HashMap::new();
    map.insert("name".to_string(), name);
    map.insert("driver".to_string(), "local".to_string());
    results.push(map);
  }
  Ok(results)
}
