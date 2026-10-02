use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct ContainerMachineCliHandler;

impl ContainerMachineCliHandler {
  pub fn new() -> Self {
    Self
  }

  pub fn create(&self, image: &str, name: Option<&str>) -> Result<String, String> {
    if image.is_empty() {
      return Err("image cannot be empty".to_string());
    }
    Ok(format!("machine_{}", name.unwrap_or(image)))
  }

  pub fn run(&self, _executable: Option<&str>, _args: &[String]) -> Result<i32, String> {
    Ok(0)
  }

  pub fn list(&self) -> Vec<String> {
    vec![]
  }

  pub fn stop(&self, id: &str) -> Result<(), String> {
    if id.is_empty() {
      return Err("machine id cannot be empty".to_string());
    }
    Ok(())
  }

  pub fn delete(&self, id: &str) -> Result<(), String> {
    if id.is_empty() {
      return Err("machine id cannot be empty".to_string());
    }
    Ok(())
  }

  pub fn inspect(&self, id: &str) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    map.insert("id".to_string(), id.to_string());
    map.insert("state".to_string(), "running".to_string());
    Ok(map)
  }

  pub fn logs(&self, _id: &str, _follow: bool, _tail: Option<i32>) -> Vec<String> {
    vec![]
  }

  pub fn set(&self, id: Option<&str>, _key_values: HashMap<String, String>) -> String {
    id.unwrap_or("default").to_string()
  }

  pub fn set_default(&self, id: &str) -> String {
    id.to_string()
  }

  pub fn capabilities(&self) -> HashMap<String, bool> {
    let mut map = HashMap::new();
    map.insert("nestedVirtualization".to_string(), true);
    map
  }
}
