use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsBuilderStartOptions {
  pub cpus: Option<i64>,
  pub memory: Option<String>,
  pub dns_nameservers: Option<Vec<String>>,
  pub ssh: Option<bool>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsBuilderStatusOptions {
  pub format: Option<String>,
  pub quiet: Option<bool>,
}

#[napi]
pub struct JsBuilderCommand {}

#[napi]
impl JsBuilderCommand {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {}
  }

  #[napi]
  pub fn start(&self, options: Option<JsBuilderStartOptions>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("status".to_string(), "started".to_string());
    if let Some(opts) = options {
      if let Some(cpus) = opts.cpus {
        map.insert("cpus".to_string(), cpus.to_string());
      }
      if let Some(mem) = opts.memory {
        map.insert("memory".to_string(), mem);
      }
    }
    Ok(map)
  }

  #[napi]
  pub fn status(&self, options: Option<JsBuilderStatusOptions>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    let quiet = options.as_ref().and_then(|o| o.quiet).unwrap_or(false);
    map.insert("id".to_string(), "buildkit".to_string());
    map.insert("status".to_string(), "running".to_string());
    map.insert("quiet".to_string(), quiet.to_string());
    Ok(map)
  }

  #[napi]
  pub fn stop(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn delete(&self) -> Result<()> {
    Ok(())
  }
}

#[napi]
pub struct JsBuildFile {}

#[napi]
impl JsBuildFile {
  #[napi]
  pub fn resolve_path(context_dir: String) -> Result<Option<String>> {
    let dockerfile_path = Path::new(&context_dir).join("Dockerfile");
    let containerfile_path = Path::new(&context_dir).join("Containerfile");

    if dockerfile_path.exists() {
      return Ok(dockerfile_path.to_str().map(|s| s.to_string()));
    }
    if containerfile_path.exists() {
      return Ok(containerfile_path.to_str().map(|s| s.to_string()));
    }
    Ok(None)
  }
}

#[napi]
pub struct JsBufferedCopyReader {
  file: Option<File>,
  chunk_size: usize,
  finished: bool,
}

#[napi]
impl JsBufferedCopyReader {
  #[napi(constructor)]
  pub fn new(filepath: String, chunk_size: Option<u32>) -> Result<Self> {
    let size = chunk_size.unwrap_or(4 * 1024 * 1024) as usize;
    let file = File::open(&filepath).ok();
    let finished = file.is_none();
    Ok(Self {
      file,
      chunk_size: size,
      finished,
    })
  }

  #[napi]
  pub fn next_chunk(&mut self) -> Result<Option<Buffer>> {
    if self.finished {
      return Ok(None);
    }
    if let Some(ref mut file) = self.file {
      let mut buf = vec![0u8; self.chunk_size];
      match file.read(&mut buf) {
        Ok(0) => {
          self.finished = true;
          Ok(None)
        }
        Ok(n) => {
          buf.truncate(n);
          if n < self.chunk_size {
            self.finished = true;
          }
          Ok(Some(Buffer::from(buf)))
        }
        Err(_) => {
          self.finished = true;
          Ok(None)
        }
      }
    } else {
      self.finished = true;
      Ok(None)
    }
  }

  #[napi(getter)]
  pub fn has_finished(&self) -> bool {
    self.finished
  }
}

#[napi(js_name = "JsBuildFSSync")]
pub struct JsBuildFSSync {
  context_dir: String,
}

#[napi]
impl JsBuildFSSync {
  #[napi(constructor)]
  pub fn new(context_dir: String) -> Self {
    Self { context_dir }
  }

  #[napi]
  pub fn read(&self, source: String, offset: Option<i64>, len: Option<i64>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("stage".to_string(), "fssync".to_string());
    map.insert("method".to_string(), "Read".to_string());
    map.insert("source".to_string(), source);
    map.insert("offset".to_string(), offset.unwrap_or(0).to_string());
    map.insert("len".to_string(), len.unwrap_or(0).to_string());
    Ok(map)
  }

  #[napi]
  pub fn info(&self, source: String) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("stage".to_string(), "fssync".to_string());
    map.insert("method".to_string(), "Info".to_string());
    map.insert("source".to_string(), source);
    Ok(map)
  }

  #[napi]
  pub fn walk(&self, follow_paths: Vec<String>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("stage".to_string(), "fssync".to_string());
    map.insert("method".to_string(), "Walk".to_string());
    map.insert("context_dir".to_string(), self.context_dir.clone());
    map.insert("follow_paths".to_string(), follow_paths.join(","));
    Ok(map)
  }
}
