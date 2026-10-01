use crate::BuildOptions;

#[derive(Clone, Debug, Default)]
pub struct ContainerBuildCliHandler {
  pub options: BuildOptions,
}

impl ContainerBuildCliHandler {
  pub fn new(options: BuildOptions) -> Self {
    Self { options }
  }

  pub fn validate(&self) -> Result<(), String> {
    if let Some(ref context_dir) = self.options.context_dir {
      if context_dir.is_empty() {
        return Err("context directory cannot be empty".to_string());
      }
    }
    Ok(())
  }
}
