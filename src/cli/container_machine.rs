use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct MachineCreateOptions {
  pub image: String,
  pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MachineRunOptions {
  pub executable: Option<String>,
  pub args: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MachineStopOptions {
  pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct MachineDeleteOptions {
  pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct MachineInspectOptions {
  pub id: String,
}

#[derive(Debug, Clone, Default)]
pub struct MachineLogsOptions {
  pub id: String,
  pub follow: Option<bool>,
  pub tail: Option<i32>,
  pub boot: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct MachineSetOptions {
  pub id: Option<String>,
  pub key_values: HashMap<String, String>,
}

pub fn handle_machine_create(opts: MachineCreateOptions) -> napi::Result<String> {
  Ok(format!("machine_{}", opts.image))
}

pub fn handle_machine_run(opts: MachineRunOptions) -> napi::Result<i32> {
  let _ = opts;
  Ok(0)
}

pub fn handle_machine_list() -> napi::Result<Vec<String>> {
  Ok(vec![])
}

pub fn handle_machine_stop(opts: MachineStopOptions) -> napi::Result<()> {
  let _ = opts;
  Ok(())
}

pub fn handle_machine_delete(opts: MachineDeleteOptions) -> napi::Result<()> {
  let _ = opts;
  Ok(())
}

pub fn handle_machine_inspect(opts: MachineInspectOptions) -> napi::Result<HashMap<String, String>> {
  let mut map = HashMap::new();
  map.insert("id".to_string(), opts.id);
  map.insert("state".to_string(), "running".to_string());
  map.insert("cpus".to_string(), "2".to_string());
  map.insert("memory".to_string(), "2048".to_string());
  Ok(map)
}

pub fn handle_machine_logs(opts: MachineLogsOptions) -> napi::Result<Vec<String>> {
  let _ = opts;
  Ok(vec![])
}

pub fn handle_machine_set(opts: MachineSetOptions) -> napi::Result<String> {
  Ok(opts.id.unwrap_or_else(|| "default".to_string()))
}

pub fn handle_machine_set_default(id: String) -> napi::Result<String> {
  Ok(id)
}

pub fn handle_machine_capabilities() -> napi::Result<HashMap<String, bool>> {
  let mut map = HashMap::new();
  map.insert("nestedVirtualization".to_string(), true);
  Ok(map)
}
