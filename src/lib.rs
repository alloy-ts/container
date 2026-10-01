use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use virtfw_varstore::store::EfiVarStore;

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsOptions {
  pub home_dir: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsContainerOptions {
  pub image: Option<String>,
  pub memory_mib: Option<i32>,
  pub cpus: Option<i32>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsContainerRestOptions {
  pub endpoint: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsContainerState {
  pub status: String,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsContainerInfo {
  pub id: String,
  pub name: Option<String>,
  pub state: JsContainerState,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsRuntimeMetrics {
  pub containeres_created_total: i64,
  pub num_running_containeres: i64,
}

#[napi]
pub struct JsImageHandle {}

#[napi]
impl JsImageHandle {
  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }
}

#[napi]
pub struct JsVolumeHandle {}

#[napi]
impl JsVolumeHandle {
  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }
}

#[napi]
pub struct JsEfiVarStore {
  inner: EfiVarStore,
}

#[napi]
impl JsEfiVarStore {
  #[napi(constructor)]
  pub fn new() -> Self {
    Self {
      inner: EfiVarStore::new(),
    }
  }

  #[napi]
  pub fn get_setup_mode(&self) -> bool {
    self.inner.get_setup_mode()
  }

  #[napi]
  pub fn get_secure_boot_enable(&mut self) -> Option<bool> {
    self.inner.get_secure_boot_enable()
  }

  #[napi]
  pub fn set_secure_boot_enable(&mut self, enabled: bool) {
    self.inner.set_secure_boot_enable(enabled);
  }

  #[napi]
  pub fn enroll_pk_mgmt(&mut self) {
    self.inner.enroll_pk_mgmt();
  }

  #[napi]
  pub fn enroll_pk_redhat(&mut self) {
    self.inner.enroll_pk_redhat();
  }

  #[napi]
  pub fn enroll_pk_microsoft(&mut self) {
    self.inner.enroll_pk_microsoft();
  }

  #[napi]
  pub fn enroll_kek_microsoft(&mut self) {
    self.inner.enroll_kek_microsoft();
  }

  #[napi]
  pub fn enroll_db_microsoft_uefi(&mut self) {
    self.inner.enroll_db_microsoft_uefi();
  }

  #[napi]
  pub fn enroll_dbx_native(&mut self) {
    self.inner.enroll_dbx_native();
  }

  #[napi]
  pub fn fs_inode_index(&self) -> u32 {
    self.inner.fs_inode_index()
  }

  #[napi]
  pub fn fs_inode_is_used(&self, inode: u32) -> bool {
    self.inner.fs_inode_is_used(inode)
  }

  #[napi]
  pub fn fs_clear_modified(&mut self) {
    self.inner.fs_clear_modified();
  }

  #[napi]
  pub fn policy_lock(&mut self) {
    self.inner.policy_lock();
  }

  #[napi]
  pub fn quirk_disable_shim_reboot(&mut self, enabled: bool) {
    self.inner.quirk_disable_shim_reboot(enabled);
  }

  #[napi]
  pub fn quirk_fallback_verbose(&mut self, enabled: bool) {
    self.inner.quirk_fallback_verbose(enabled);
  }

  #[napi]
  pub fn quirk_shim_verbose(&mut self, enabled: bool) {
    self.inner.quirk_shim_verbose(enabled);
  }

  #[napi]
  pub fn reset(&mut self) {
    self.inner.reset();
  }

  #[napi]
  pub fn end_of_dxe(&mut self) {
    self.inner.end_of_dxe();
  }

  #[napi]
  pub fn ready_to_boot(&mut self) {
    self.inner.ready_to_boot();
  }

  #[napi]
  pub fn exit_boot_service(&mut self) {
    self.inner.exit_boot_service();
  }
}

#[derive(Default)]
struct InnerContainerState {
  _home_dir: String,
  containers: HashMap<String, JsContainerInfo>,
  created_total: i64,
}

static DEFAULT_RUNTIME_STATE: Mutex<Option<Arc<Mutex<InnerContainerState>>>> = Mutex::new(None);

#[napi]
pub struct JsContainer {
  inner: Arc<Mutex<InnerContainerState>>,
  _id: Option<String>,
}

#[napi]
impl JsContainer {
  #[napi(constructor)]
  pub fn new(options: Option<JsOptions>) -> Result<Self> {
    let home_dir = options
      .and_then(|o| o.home_dir)
      .unwrap_or_else(|| "~/.container".to_string());

    Ok(Self {
      inner: Arc::new(Mutex::new(InnerContainerState {
        _home_dir: home_dir,
        containers: HashMap::new(),
        created_total: 0,
      })),
      _id: None,
    })
  }

  #[napi(factory)]
  pub fn with_default_config() -> Result<Self> {
    let mut lock = DEFAULT_RUNTIME_STATE
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    if lock.is_none() {
      let state = Arc::new(Mutex::new(InnerContainerState {
        _home_dir: "~/.container".to_string(),
        containers: HashMap::new(),
        created_total: 0,
      }));
      *lock = Some(state);
    }

    Ok(Self {
      inner: Arc::clone(lock.as_ref().unwrap()),
      _id: None,
    })
  }

  #[napi]
  pub fn init_default(options: JsOptions) -> Result<()> {
    let home_dir = options
      .home_dir
      .unwrap_or_else(|| "~/.container".to_string());
    let mut lock = DEFAULT_RUNTIME_STATE
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    let state = Arc::new(Mutex::new(InnerContainerState {
      _home_dir: home_dir,
      containers: HashMap::new(),
      created_total: 0,
    }));
    *lock = Some(state);
    Ok(())
  }

  #[napi(factory)]
  pub fn rest(options: JsContainerRestOptions) -> Result<Self> {
    let endpoint = options.endpoint.unwrap_or_else(|| "http://localhost".to_string());
    Ok(Self {
      inner: Arc::new(Mutex::new(InnerContainerState {
        _home_dir: endpoint,
        containers: HashMap::new(),
        created_total: 0,
      })),
      _id: None,
    })
  }

  #[napi(js_name = "importContainer")]
  pub async fn import_container(&self, _archive_path: String, name: Option<String>) -> Result<JsContainer> {
    let container_id = format!("cnt_{}", name.as_deref().unwrap_or("imported"));
    let mut state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    state.created_total += 1;
    let info = JsContainerInfo {
      id: container_id.clone(),
      name: name.clone(),
      state: JsContainerState {
        status: "running".to_string(),
      },
    };
    state.containers.insert(container_id.clone(), info.clone());
    if let Some(ref n) = name {
      state.containers.insert(n.clone(), info);
    }
    Ok(JsContainer {
      inner: Arc::clone(&self.inner),
      _id: Some(container_id),
    })
  }

  #[napi]
  pub async fn create(&self, _options: JsContainerOptions, name: Option<String>) -> Result<JsContainer> {
    let container_id = format!("cnt_{}", name.as_deref().unwrap_or("default"));
    let mut state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    state.created_total += 1;
    let info = JsContainerInfo {
      id: container_id.clone(),
      name: name.clone(),
      state: JsContainerState {
        status: "running".to_string(),
      },
    };
    state.containers.insert(container_id.clone(), info.clone());
    if let Some(ref n) = name {
      state.containers.insert(n.clone(), info);
    }
    Ok(JsContainer {
      inner: Arc::clone(&self.inner),
      _id: Some(container_id),
    })
  }

  #[napi]
  pub async fn get_or_create(
    &self,
    options: JsContainerOptions,
    name: Option<String>,
  ) -> Result<JsGetOrCreateResult> {
    let existing_id = {
      let state = self
        .inner
        .lock()
        .map_err(|e| Error::from_reason(e.to_string()))?;
      let key = name.as_deref().unwrap_or("default");
      state.containers.get(key).map(|info| info.id.clone())
    };

    if let Some(id) = existing_id {
      let container = JsContainer {
        inner: Arc::clone(&self.inner),
        _id: Some(id),
      };
      return Ok(JsGetOrCreateResult {
        inner_container: container,
        created: false,
      });
    }

    let container = self.create(options, name).await?;
    Ok(JsGetOrCreateResult {
      inner_container: container,
      created: true,
    })
  }

  #[napi]
  pub async fn list_info(&self) -> Result<Vec<JsContainerInfo>> {
    let state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    let mut infos = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for info in state.containers.values() {
      if seen.insert(info.id.clone()) {
        infos.push(info.clone());
      }
    }
    Ok(infos)
  }

  #[napi]
  pub async fn get_info(&self, id_or_name: String) -> Result<Option<JsContainerInfo>> {
    let state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    Ok(state.containers.get(&id_or_name).cloned())
  }

  #[napi]
  pub async fn get(&self, id_or_name: String) -> Result<Option<JsContainer>> {
    let state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    if let Some(info) = state.containers.get(&id_or_name) {
      Ok(Some(JsContainer {
        inner: Arc::clone(&self.inner),
        _id: Some(info.id.clone()),
      }))
    } else {
      Ok(None)
    }
  }

  #[napi]
  pub async fn metrics(&self) -> Result<JsRuntimeMetrics> {
    let state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    let running = state
      .containers
      .values()
      .filter(|c| c.state.status == "running")
      .count() as i64;
    Ok(JsRuntimeMetrics {
      containeres_created_total: state.created_total,
      num_running_containeres: running,
    })
  }

  #[napi(getter)]
  pub fn images(&self) -> Result<JsImageHandle> {
    Ok(JsImageHandle {})
  }

  #[napi(getter)]
  pub fn volumes(&self) -> Result<JsVolumeHandle> {
    Ok(JsVolumeHandle {})
  }

  #[napi]
  pub async fn remove(&self, id_or_name: String, _force: Option<bool>) -> Result<()> {
    let mut state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    if let Some(info) = state.containers.remove(&id_or_name) {
      if let Some(ref name) = info.name {
        state.containers.remove(name);
      }
    }
    Ok(())
  }

  #[napi]
  pub fn close(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub async fn shutdown(&self, _timeout: Option<i32>) -> Result<()> {
    let mut state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    state.containers.clear();
    Ok(())
  }
}

#[napi]
pub struct JsGetOrCreateResult {
  inner_container: JsContainer,
  created: bool,
}

#[napi]
impl JsGetOrCreateResult {
  #[napi(getter)]
  pub fn created(&self) -> bool {
    self.created
  }

  #[napi(getter, js_name = "container")]
  pub fn get_container(&self) -> JsContainer {
    JsContainer {
      inner: Arc::clone(&self.inner_container.inner),
      _id: self.inner_container._id.clone(),
    }
  }
}
