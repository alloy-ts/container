use napi::bindgen_prelude::*;
use std::collections::HashMap;

pub struct BuildCommand {
    pub context_dir: String,
    pub dockerfile: String,
    pub target_image_names: Vec<String>,
    pub build_args: HashMap<String, String>,
}

impl BuildCommand {
    pub fn new(context_dir: String, dockerfile: Option<String>, tags: Vec<String>) -> Self {
        Self {
            context_dir,
            dockerfile: dockerfile.unwrap_or_else(|| "-".to_string()),
            target_image_names: tags,
            build_args: HashMap::new(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.context_dir.is_empty() {
            return Err(Error::from_reason("Context directory cannot be empty"));
        }
        Ok(())
    }

    pub fn run(&self) -> Result<String> {
        self.validate()?;
        Ok(self
            .target_image_names
            .first()
            .cloned()
            .unwrap_or_else(|| "built-image:latest".to_string()))
    }
}
