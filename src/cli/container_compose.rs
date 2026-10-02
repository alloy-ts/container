use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CliComposeCommandOptions {
  pub files: Option<Vec<String>>,
  pub project_name: Option<String>,
  pub env_file: Option<String>,
  pub detach: Option<bool>,
  pub build: Option<bool>,
}

#[napi]
pub struct JsCliComposeHandler {}

#[napi]
impl JsCliComposeHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {}
  }

  #[napi]
  pub fn parse_compose_args(&self, args: Vec<String>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("command".to_string(), args.first().cloned().unwrap_or_else(|| "ps".to_string()));
    map.insert("raw_args".to_string(), args.join(" "));
    Ok(map)
  }
}
