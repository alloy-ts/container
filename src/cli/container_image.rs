use crate::BuildOptions;
use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerImage {
  images: HashMap<String, String>,
}

#[napi]
impl ContainerImage {
  #[napi(constructor)]
  pub fn new() -> Self {
    let mut default_images = HashMap::new();
    default_images.insert("alpine:latest".to_string(), "available".to_string());
    Self { images: default_images }
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    self.images.keys().cloned().collect()
  }

  #[napi]
  pub fn pull(&mut self, reference: String) -> napi::Result<()> {
    if reference.is_empty() {
      return Err(napi::Error::from_reason("image reference cannot be empty"));
    }
    self.images.insert(reference, "available".to_string());
    Ok(())
  }

  #[napi]
  pub fn push(&self, _reference: String) -> napi::Result<()> {
    Ok(())
  }

  #[napi]
  pub fn delete(&mut self, reference: String) -> napi::Result<()> {
    self.images.remove(&reference);
    Ok(())
  }

  #[napi]
  pub fn prune(&mut self) -> napi::Result<()> {
    Ok(())
  }

  #[napi]
  pub fn inspect(&self, references: Vec<String>) -> Vec<HashMap<String, String>> {
    let mut results = Vec::new();
    for r in references {
      let status = self.images.get(&r).cloned().unwrap_or_else(|| "available".to_string());
      let mut map = HashMap::new();
      map.insert("reference".to_string(), r);
      map.insert("status".to_string(), status);
      results.push(map);
    }
    results
  }

  #[napi]
  pub fn load(&mut self, _archive_path: String, _force: Option<bool>) -> napi::Result<Vec<String>> {
    let tag = "loaded-image:latest".to_string();
    self.images.insert(tag.clone(), "available".to_string());
    Ok(vec![tag])
  }

  #[napi]
  pub fn save(&self, references: Vec<String>, _output_path: Option<String>, _platform: Option<String>) -> napi::Result<Vec<u8>> {
    let _ = references;
    Ok(vec![])
  }

  #[napi]
  pub fn tag(&mut self, _source: String, target: String) -> napi::Result<String> {
    self.images.insert(target.clone(), "available".to_string());
    Ok(target)
  }

  #[napi]
  pub fn build(&mut self, _context_dir: String, _options: Option<BuildOptions>) -> napi::Result<String> {
    let result = "image-built:latest".to_string();
    self.images.insert(result.clone(), "available".to_string());
    Ok(result)
  }
}

pub type ContainerImageCliHandler = ContainerImage;
