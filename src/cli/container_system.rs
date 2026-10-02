use crate::config::ContainerSystemConfig;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi(js_name = "systemStart")]
pub fn system_start() -> Result<()> {
  Ok(())
}

#[napi(js_name = "systemStop")]
pub fn system_stop() -> Result<()> {
  Ok(())
}

#[napi(js_name = "systemStatus")]
pub fn system_status() -> Result<String> {
  Ok("running".to_string())
}

#[napi(js_name = "systemVersion")]
pub fn system_version() -> Result<HashMap<String, String>> {
  let mut map = HashMap::new();
  map.insert("version".to_string(), "1.0.0".to_string());
  map.insert("component".to_string(), "container".to_string());
  Ok(map)
}

#[napi(js_name = "systemDf")]
pub fn system_df() -> Result<HashMap<String, String>> {
  let mut map = HashMap::new();
  map.insert("reclaimable".to_string(), "0B".to_string());
  Ok(map)
}

#[napi(js_name = "systemLogs")]
pub fn system_logs(_follow: Option<bool>, _last: Option<String>) -> Result<Vec<String>> {
  Ok(vec![])
}

#[napi(js_name = "systemPropertyList")]
pub fn system_property_list() -> Result<HashMap<String, String>> {
  let config = ContainerSystemConfig::load();
  let mut map = HashMap::new();
  map.insert("build.cpus".to_string(), config.build.cpus.to_string());
  map.insert("build.memory".to_string(), config.build.memory);
  map.insert("container.cpus".to_string(), config.container.cpus.to_string());
  map.insert("container.memory".to_string(), config.container.memory);
  map.insert("registry.domain".to_string(), config.registry.domain);
  map.insert("log.level".to_string(), "info".to_string());
  Ok(map)
}

#[napi(js_name = "systemDnsCreate")]
pub fn system_dns_create(domain: String, _ip: Option<String>) -> Result<String> {
  Ok(domain)
}

#[napi(js_name = "systemDnsList")]
pub fn system_dns_list() -> Result<Vec<HashMap<String, String>>> {
  Ok(vec![])
}

#[napi(js_name = "systemDnsDelete")]
pub fn system_dns_delete(_domain: String) -> Result<()> {
  Ok(())
}

#[napi(js_name = "systemKernelSet")]
pub fn system_kernel_set(path: String) -> Result<String> {
  Ok(path)
}
