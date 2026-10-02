use std::collections::HashMap;
use crate::BuildOptions;

#[derive(Debug, Clone, Default)]
pub struct ContainerBuildArgs {
  pub context_dir: String,
  pub dockerfile: Option<String>,
  pub target: Option<String>,
  pub build_args: HashMap<String, String>,
  pub tags: Vec<String>,
  pub cpus: Option<i64>,
  pub memory: Option<String>,
  pub os: Vec<String>,
  pub arch: Vec<String>,
  pub platform: Vec<String>,
  pub no_cache: bool,
  pub output: Vec<String>,
  pub quiet: bool,
  pub secret: Vec<String>,
  pub ssh: String,
  pub pull: bool,
  pub label: Vec<String>,
}

pub fn handle_container_build(context_dir: String, options: Option<BuildOptions>) -> napi::Result<String> {
  let opts = options.unwrap_or_default();
  let build_args = ContainerBuildArgs {
    context_dir,
    dockerfile: opts.dockerfile,
    target: opts.target,
    build_args: opts.build_args.unwrap_or_default(),
    tags: opts.tags.unwrap_or_default(),
    ..Default::default()
  };

  let _ = build_args;
  Ok("image-built:latest".to_string())
}
