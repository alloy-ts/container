use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct ContainerBuildOptions {
  pub context_dir: Option<String>,
  pub file: Option<String>,
  pub target: Option<String>,
  pub build_args: Option<HashMap<String, String>>,
  pub tags: Option<Vec<String>>,
  pub arch: Option<Vec<String>>,
  pub os: Option<Vec<String>>,
  pub platform: Option<Vec<String>>,
  pub no_cache: Option<bool>,
  pub quiet: Option<bool>,
  pub cpus: Option<i32>,
  pub memory: Option<String>,
}

#[napi(js_name = "containerBuild")]
pub fn container_build(options: Option<ContainerBuildOptions>) -> Result<String> {
  let opts = options.unwrap_or_default();
  let context_dir = opts.context_dir.unwrap_or_else(|| ".".to_string());
  let tags = opts.tags.unwrap_or_default();
  let image_name = tags
    .first()
    .cloned()
    .unwrap_or_else(|| "image-built:latest".to_string());

  if let Some(ref file) = opts.file {
    if file != "-" && file.is_empty() {
      return Err(Error::from_reason("Dockerfile path cannot be empty"));
    }
  }

  let _ = context_dir;
  Ok(image_name)
}
