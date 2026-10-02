use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CliContainerBuildOptions {
  pub arch: Option<Vec<String>>,
  pub build_arg: Option<Vec<String>>,
  pub cpus: Option<i64>,
  pub file: Option<String>,
  pub label: Option<Vec<String>>,
  pub memory: Option<String>,
  pub no_cache: Option<bool>,
  pub output: Option<Vec<String>>,
  pub os: Option<Vec<String>>,
  pub platform: Option<Vec<String>>,
  pub progress: Option<String>,
  pub quiet: Option<bool>,
  pub secret: Option<Vec<String>>,
  pub ssh: Option<String>,
  pub target_image_names: Option<Vec<String>>,
  pub target: Option<String>,
  pub vsock_port: Option<u32>,
  pub context_dir: Option<String>,
  pub pull: Option<bool>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CliContainerCommand {
  pub command_name: String,
  pub args: Vec<String>,
  pub options: HashMap<String, String>,
}

#[napi]
pub struct JsCliContainerHandler {}

#[napi]
impl JsCliContainerHandler {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {}
  }

  #[napi]
  pub fn parse_args(&self, args: Vec<String>) -> Result<CliContainerCommand> {
    let command_name = args.first().cloned().unwrap_or_else(|| "help".to_string());
    Ok(CliContainerCommand {
      command_name,
      args: args.into_iter().skip(1).collect(),
      options: HashMap::new(),
    })
  }

  #[napi]
  pub fn build_config(&self, options: CliContainerBuildOptions) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("context_dir".to_string(), options.context_dir.unwrap_or_else(|| ".".to_string()));
    map.insert(
      "target_image_names".to_string(),
      options.target_image_names.unwrap_or_default().join(","),
    );
    map.insert("no_cache".to_string(), options.no_cache.unwrap_or(false).to_string());
    map.insert("quiet".to_string(), options.quiet.unwrap_or(false).to_string());
    map.insert("pull".to_string(), options.pull.unwrap_or(false).to_string());
    Ok(map)
  }
}
