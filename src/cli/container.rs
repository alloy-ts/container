use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ContainerCliCommand {
  pub name: String,
  pub options: HashMap<String, String>,
}

impl ContainerCliCommand {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      options: HashMap::new(),
    }
  }

  pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.options.insert(key.into(), value.into());
    self
  }
}
