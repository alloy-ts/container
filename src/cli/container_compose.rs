use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ContainerComposeCliCommand {
  pub subcommand: String,
  pub options: HashMap<String, String>,
}

impl ContainerComposeCliCommand {
  pub fn new(subcommand: impl Into<String>) -> Self {
    Self {
      subcommand: subcommand.into(),
      options: HashMap::new(),
    }
  }

  pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
    self.options.insert(key.into(), value.into());
    self
  }
}
