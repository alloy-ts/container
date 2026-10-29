use napi_derive::napi;
use std::collections::HashMap;

#[napi]
#[derive(Clone, Debug, Default)]
pub struct ContainerNetwork {
  networks: HashMap<String, HashMap<String, String>>,
}

#[napi]
impl ContainerNetwork {
  #[napi(constructor)]
  pub fn new() -> Self {
    let mut default_nets = HashMap::new();
    let mut bridge_net = HashMap::new();
    bridge_net.insert("name".to_string(), "bridge".to_string());
    bridge_net.insert("driver".to_string(), "bridge".to_string());
    default_nets.insert("bridge".to_string(), bridge_net);

    Self { networks: default_nets }
  }

  #[napi]
  pub fn create(&mut self, name: String, _plugin: Option<String>, _subnet: Option<String>) -> napi::Result<String> {
    if name.is_empty() {
      return Err(napi::Error::from_reason("network name cannot be empty"));
    }
    let mut map = HashMap::new();
    map.insert("name".to_string(), name.clone());
    map.insert("driver".to_string(), "bridge".to_string());
    self.networks.insert(name.clone(), map);
    Ok(name)
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    self.networks.keys().cloned().collect()
  }

  #[napi]
  pub fn delete(&mut self, name: String) -> napi::Result<()> {
    self.networks.remove(&name);
    Ok(())
  }

  #[napi]
  pub fn prune(&mut self) -> napi::Result<()> {
    self.networks.retain(|k, _| k == "bridge");
    Ok(())
  }

  #[napi]
  pub fn inspect(&self, names: Vec<String>) -> Vec<HashMap<String, String>> {
    let mut results = Vec::new();
    for name in names {
      if let Some(net) = self.networks.get(&name) {
        results.push(net.clone());
      } else {
        let mut fallback = HashMap::new();
        fallback.insert("name".to_string(), name);
        fallback.insert("driver".to_string(), "bridge".to_string());
        results.push(fallback);
      }
    }
    results
  }
}

pub type ContainerNetworkCliHandler = ContainerNetwork;
