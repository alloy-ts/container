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
  pub name: Option<String>,
  pub env: Option<HashMap<String, String>>,
  pub workdir: Option<String>,
  pub user: Option<String>,
  pub detach: Option<bool>,
  pub interactive: Option<bool>,
  pub tty: Option<bool>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsContainerRestOptions {
  pub endpoint: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct JsPublishedPort {
  #[napi(js_name = "guestPort")]
  pub guest_port: u32,
  #[napi(js_name = "hostIp")]
  pub host_ip: String,
  #[napi(js_name = "hostPort")]
  pub host_port: u32,
  pub protocol: String,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct JsOutboundNetworkInfo {
  pub mode: String,
  #[napi(js_name = "allowNet")]
  pub allow_net: Vec<String>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct JsInboundNetworkInfo {
  pub mode: String,
  #[napi(js_name = "allowNet")]
  pub allow_net: Vec<String>,
}

#[napi(object, use_nullable = true)]
#[derive(Clone, Debug)]
pub struct JsNetworkInfo {
  pub outbound: JsOutboundNetworkInfo,
  pub inbound: JsInboundNetworkInfo,
  pub mode: String,
  #[napi(js_name = "allowNet")]
  pub allow_net: Vec<String>,
  #[napi(js_name = "publishedPorts")]
  pub published_ports: Option<Vec<JsPublishedPort>>,
}

#[napi(string_enum)]
#[derive(Clone, Debug)]
pub enum JsHealthState {
  None,
  Starting,
  Healthy,
  Unhealthy,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct JsHealthStatus {
  pub state: JsHealthState,
  pub failures: u32,
  pub last_check: Option<String>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsContainerStateInfo {
  pub status: String,
  pub running: bool,
  pub pid: Option<u32>,
  pub exit_code: Option<i32>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct JsContainerInfo {
  pub id: String,
  pub name: Option<String>,
  pub state: JsContainerStateInfo,
  pub created_at: String,
  pub started_at: Option<String>,
  pub last_activity_at: Option<String>,
  pub image: String,
  pub cpus: u32,
  pub memory_mib: u32,
  pub network: Either<JsNetworkInfo, Null>,
  #[napi(js_name = "autoStop")]
  pub auto_stop: u32,
  #[napi(js_name = "autoDelete")]
  pub auto_delete: u32,
  #[napi(js_name = "autoResume")]
  pub auto_resume: bool,
  pub health_status: JsHealthStatus,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsRuntimeMetrics {
  pub containeres_created_total: i64,
  pub num_running_containeres: i64,
}

#[napi]
#[derive(Clone, Debug, Default)]
pub struct JsBuildTransfer {
  metadata: HashMap<String, String>,
}

#[napi]
impl JsBuildTransfer {
  #[napi(constructor)]
  pub fn new(metadata: Option<HashMap<String, String>>) -> Self {
    Self {
      metadata: metadata.unwrap_or_default(),
    }
  }

  #[napi(getter)]
  pub fn metadata(&self) -> HashMap<String, String> {
    self.metadata.clone()
  }

  #[napi]
  pub fn stage(&self) -> Option<String> {
    let stage = self.metadata.get("stage")?;
    if stage.is_empty() {
      None
    } else {
      Some(stage.clone())
    }
  }

  #[napi]
  pub fn method(&self) -> Option<String> {
    let method = self.metadata.get("method")?;
    if method.is_empty() {
      None
    } else {
      Some(method.clone())
    }
  }

  #[napi]
  pub fn include_patterns(&self) -> Option<Vec<String>> {
    let s = self.metadata.get("include-patterns")?;
    if s.is_empty() {
      None
    } else {
      Some(s.split(',').map(|p| p.to_string()).collect())
    }
  }

  #[napi]
  pub fn follow_paths(&self) -> Option<Vec<String>> {
    let s = self.metadata.get("followpaths")?;
    if s.is_empty() {
      None
    } else {
      Some(s.split(',').map(|p| p.to_string()).collect())
    }
  }

  #[napi]
  pub fn mode(&self) -> Option<String> {
    self.metadata.get("mode").cloned()
  }

  #[napi]
  pub fn size(&self) -> Option<i64> {
    let s = self.metadata.get("size")?;
    if s.is_empty() {
      None
    } else {
      s.parse::<i64>().ok()
    }
  }

  #[napi]
  pub fn offset(&self) -> Option<i64> {
    let s = self.metadata.get("offset")?;
    if s.is_empty() {
      None
    } else {
      s.parse::<i64>().ok()
    }
  }

  #[napi]
  pub fn len(&self) -> Option<i64> {
    let s = self.metadata.get("length")?;
    if s.is_empty() {
      None
    } else {
      s.parse::<i64>().ok()
    }
  }
}

#[napi]
#[derive(Clone, Debug, Default)]
pub struct JsImageTransfer {
  metadata: HashMap<String, String>,
}

#[napi]
impl JsImageTransfer {
  #[napi(constructor)]
  pub fn new(metadata: Option<HashMap<String, String>>) -> Self {
    Self {
      metadata: metadata.unwrap_or_default(),
    }
  }

  #[napi(getter)]
  pub fn metadata(&self) -> HashMap<String, String> {
    self.metadata.clone()
  }

  #[napi]
  pub fn stage(&self) -> Option<String> {
    self.metadata.get("stage").cloned()
  }

  #[napi]
  pub fn method(&self) -> Option<String> {
    self.metadata.get("method").cloned()
  }

  #[napi]
  pub fn ref_name(&self) -> Option<String> {
    self.metadata.get("ref").cloned()
  }

  #[napi]
  pub fn platform(&self) -> Option<String> {
    self.metadata.get("platform").cloned()
  }

  #[napi]
  pub fn mode(&self) -> Option<String> {
    self.metadata.get("mode").cloned()
  }

  #[napi]
  pub fn size(&self) -> Option<i64> {
    let s = self.metadata.get("size")?;
    s.parse::<i64>().ok()
  }

  #[napi]
  pub fn len(&self) -> Option<i64> {
    let s = self.metadata.get("length")?;
    s.parse::<i64>().ok()
  }

  #[napi]
  pub fn offset(&self) -> Option<i64> {
    let s = self.metadata.get("offset")?;
    s.parse::<i64>().ok()
  }
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsIo {
  pub data: Vec<u8>,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsInfoRequest {
  pub id: String,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsInfoResponse {
  pub id: String,
  pub status: String,
}

#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct JsClientStream {
  pub data: Vec<u8>,
}

#[napi]
#[derive(Clone, Debug, Default)]
pub struct JsServerStream {
  image_transfer: Option<JsImageTransfer>,
  build_transfer: Option<JsBuildTransfer>,
  io: Option<JsIo>,
}

#[napi]
impl JsServerStream {
  #[napi(constructor)]
  pub fn new(
    image_transfer: Option<&JsImageTransfer>,
    build_transfer: Option<&JsBuildTransfer>,
    io: Option<JsIo>,
  ) -> Self {
    Self {
      image_transfer: image_transfer.cloned(),
      build_transfer: build_transfer.cloned(),
      io,
    }
  }

  #[napi]
  pub fn get_image_transfer(&self) -> Option<JsImageTransfer> {
    self.image_transfer.clone()
  }

  #[napi]
  pub fn get_build_transfer(&self) -> Option<JsBuildTransfer> {
    self.build_transfer.clone()
  }

  #[napi]
  pub fn get_io(&self) -> Option<JsIo> {
    self.io.clone()
  }
}

#[napi]
pub struct JsImageHandle {}

#[napi]
impl JsImageHandle {
  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }

  #[napi]
  pub fn pull(&self, _reference: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn push(&self, _reference: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn delete(&self, _reference: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn prune(&self) -> Result<()> {
    Ok(())
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

  #[napi]
  pub fn create(&self, name: String, _size: Option<String>) -> Result<String> {
    Ok(name)
  }

  #[napi]
  pub fn delete(&self, name: String) -> Result<()> {
    let _ = name;
    Ok(())
  }

  #[napi]
  pub fn prune(&self) -> Result<()> {
    Ok(())
  }
}

#[napi]
pub struct JsMachineHandle {}

#[napi]
impl JsMachineHandle {
  #[napi]
  pub fn create(&self, image: String, _name: Option<String>) -> Result<String> {
    Ok(format!("machine_{image}"))
  }

  #[napi]
  pub fn run(&self, _executable: Option<String>, _args: Option<Vec<String>>) -> Result<i32> {
    Ok(0)
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }

  #[napi]
  pub fn stop(&self, _id: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn delete(&self, _id: String) -> Result<()> {
    Ok(())
  }
}

#[napi(js_name = "JsK8sHandle")]
pub struct JsK8sHandle {}

#[napi]
impl JsK8sHandle {
  #[napi]
  pub fn create(&self, name: Option<String>, _cpus: Option<i32>, _memory: Option<String>) -> Result<String> {
    Ok(name.unwrap_or_else(|| "k8s-dev".to_string()))
  }

  #[napi]
  pub fn delete(&self, _name: Option<String>) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec!["k8s-dev".to_string()]
  }

  #[napi]
  pub fn load_image(&self, _image: String, _name: Option<String>) -> Result<()> {
    Ok(())
  }
}

#[napi]
pub struct JsNetworkHandle {}

#[napi]
impl JsNetworkHandle {
  #[napi]
  pub fn create(&self, name: String, _plugin: Option<String>, _subnet: Option<String>) -> Result<String> {
    Ok(name)
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec!["default".to_string()]
  }

  #[napi]
  pub fn delete(&self, _name: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn prune(&self) -> Result<()> {
    Ok(())
  }
}

#[napi]
pub struct JsRegistryHandle {}

#[napi]
impl JsRegistryHandle {
  #[napi]
  pub fn login(&self, _server: String, _username: Option<String>, _password: Option<String>) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn logout(&self, _server: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn list(&self) -> Vec<String> {
    vec![]
  }
}

#[napi]
pub struct JsSystemHandle {}

#[napi]
impl JsSystemHandle {
  #[napi]
  pub fn start(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn stop(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn status(&self) -> Result<String> {
    Ok("running".to_string())
  }

  #[napi]
  pub fn version(&self) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("version".to_string(), "1.0.0".to_string());
    map.insert("component".to_string(), "@lib/container".to_string());
    Ok(map)
  }

  #[napi]
  pub fn df(&self) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("reclaimable".to_string(), "0B".to_string());
    Ok(map)
  }
}

#[napi]
pub struct JsComposeSystemHandle {}

#[napi]
impl JsComposeSystemHandle {
  #[napi]
  pub fn status(&self, _socket: Option<String>, _address: Option<String>) -> Result<String> {
    Ok("container-compose daemon running".to_string())
  }

  #[napi]
  pub fn generate_key(&self, name: String, _auth_file: Option<String>) -> Result<String> {
    Ok(format!("key_{name}"))
  }

  #[napi]
  pub fn generate_cert(&self, _out_dir: Option<String>, _cn: Option<String>, _days: Option<i32>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("cert".to_string(), "cert.pem".to_string());
    map.insert("key".to_string(), "key.pem".to_string());
    Ok(map)
  }

  #[napi]
  pub fn list_keys(&self, _auth_file: Option<String>) -> Result<Vec<HashMap<String, String>>> {
    Ok(vec![])
  }

  #[napi]
  pub fn revoke_key(&self, _name: String, _auth_file: Option<String>) -> Result<()> {
    Ok(())
  }
}

#[napi]
pub struct JsComposeHandle {}

#[napi]
impl JsComposeHandle {
  #[napi]
  pub fn up(&self, _detach: Option<bool>, _build: Option<bool>) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn down(&self, _volumes: Option<bool>) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn start(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn stop(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn restart(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn create(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn kill(&self, _signal: Option<String>) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn rm(&self, _force: Option<bool>) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn ps(&self) -> Result<Vec<String>> {
    Ok(vec![])
  }

  #[napi]
  pub fn ls(&self) -> Result<Vec<String>> {
    Ok(vec![])
  }

  #[napi]
  pub fn logs(&self, _follow: Option<bool>) -> Result<Vec<String>> {
    Ok(vec![])
  }

  #[napi]
  pub fn top(&self) -> Result<Vec<String>> {
    Ok(vec![])
  }

  #[napi]
  pub fn port(&self, service: String, private_port: i32) -> Result<String> {
    Ok(format!("{service}:{private_port}"))
  }

  #[napi]
  pub fn events(&self) -> Result<Vec<String>> {
    Ok(vec![])
  }

  #[napi]
  pub fn config(&self) -> Result<String> {
    Ok("".to_string())
  }

  #[napi]
  pub fn build(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn run(&self, _service: String, _command: Option<Vec<String>>) -> Result<i32> {
    Ok(0)
  }

  #[napi]
  pub fn exec(&self, _service: String, _command: Vec<String>) -> Result<i32> {
    Ok(0)
  }

  #[napi]
  pub fn watch(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn pull(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn push(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn serve(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub fn version(&self) -> Result<String> {
    Ok("container-compose v1.0.0".to_string())
  }

  #[napi(getter)]
  pub fn system(&self) -> Result<JsComposeSystemHandle> {
    Ok(JsComposeSystemHandle {})
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
  id: Option<String>,
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
      id: None,
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
      id: None,
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
      id: None,
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
      state: JsContainerStateInfo {
        status: "running".to_string(),
        running: true,
        pid: Some(100),
        exit_code: None,
      },
      created_at: "1970-01-01T00:00:00Z".to_string(),
      started_at: Some("1970-01-01T00:00:00Z".to_string()),
      last_activity_at: None,
      image: "alpine:latest".to_string(),
      cpus: 1,
      memory_mib: 512,
      network: Either::B(Null),
      auto_stop: 0,
      auto_delete: 0,
      auto_resume: false,
      health_status: JsHealthStatus {
        state: JsHealthState::None,
        failures: 0,
        last_check: None,
      },
    };
    state.containers.insert(container_id.clone(), info.clone());
    if let Some(ref n) = name {
      state.containers.insert(n.clone(), info);
    }
    Ok(JsContainer {
      inner: Arc::clone(&self.inner),
      id: Some(container_id),
    })
  }

  #[napi]
  pub async fn create(&self, options: JsContainerOptions, name: Option<String>) -> Result<JsContainer> {
    let container_id = format!("cnt_{}", name.as_deref().unwrap_or("default"));
    let mut state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    state.created_total += 1;
    let info = JsContainerInfo {
      id: container_id.clone(),
      name: name.clone(),
      state: JsContainerStateInfo {
        status: "configured".to_string(),
        running: false,
        pid: None,
        exit_code: None,
      },
      created_at: "1970-01-01T00:00:00Z".to_string(),
      started_at: None,
      last_activity_at: None,
      image: options.image.unwrap_or_else(|| "alpine:latest".to_string()),
      cpus: options.cpus.unwrap_or(1) as u32,
      memory_mib: options.memory_mib.unwrap_or(512) as u32,
      network: Either::B(Null),
      auto_stop: 0,
      auto_delete: 0,
      auto_resume: false,
      health_status: JsHealthStatus {
        state: JsHealthState::None,
        failures: 0,
        last_check: None,
      },
    };
    state.containers.insert(container_id.clone(), info.clone());
    if let Some(ref n) = name {
      state.containers.insert(n.clone(), info);
    }
    Ok(JsContainer {
      inner: Arc::clone(&self.inner),
      id: Some(container_id),
    })
  }

  #[napi]
  pub async fn start(&self, _attach: Option<bool>, _interactive: Option<bool>) -> Result<i32> {
    if let Some(ref container_id) = self.id {
      let mut state = self
        .inner
        .lock()
        .map_err(|e| Error::from_reason(e.to_string()))?;
      if let Some(info) = state.containers.get_mut(container_id) {
        info.state.status = "running".to_string();
        info.state.running = true;
        info.state.pid = Some(101);
      }
    }
    Ok(0)
  }

  #[napi]
  pub async fn stop(&self, _signal: Option<String>, _time: Option<i32>) -> Result<()> {
    if let Some(ref container_id) = self.id {
      let mut state = self
        .inner
        .lock()
        .map_err(|e| Error::from_reason(e.to_string()))?;
      if let Some(info) = state.containers.get_mut(container_id) {
        info.state.status = "stopped".to_string();
        info.state.running = false;
        info.state.pid = None;
        info.state.exit_code = Some(0);
      }
    }
    Ok(())
  }

  #[napi]
  pub async fn kill(&self, _signal: Option<String>) -> Result<()> {
    self.stop(None, None).await
  }

  #[napi]
  pub async fn exec(
    &self,
    _cmd: Vec<String>,
    _env: Option<HashMap<String, String>>,
    _cwd: Option<String>,
    _user: Option<String>,
    _detach: Option<bool>,
    _interactive: Option<bool>,
    _tty: Option<bool>,
  ) -> Result<i32> {
    Ok(0)
  }

  #[napi]
  pub async fn inspect(&self) -> Result<JsContainerInfo> {
    let id = self.id.clone().unwrap_or_else(|| "default".to_string());
    let state = self
      .inner
      .lock()
      .map_err(|e| Error::from_reason(e.to_string()))?;
    state.containers.get(&id).cloned().ok_or_else(|| {
      Error::from_reason(format!("Container not found: {}", id))
    })
  }

  #[napi]
  pub async fn logs(&self, _follow: Option<bool>, _tail: Option<i32>, _boot: Option<bool>) -> Result<Vec<String>> {
    Ok(vec![])
  }

  #[napi]
  pub async fn stats(&self, _no_stream: Option<bool>) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("cpu".to_string(), "0.0%".to_string());
    map.insert("memory".to_string(), "0MB".to_string());
    Ok(map)
  }

  #[napi]
  pub async fn clean(&self) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub async fn copy(&self, _src: String, _dest: String) -> Result<()> {
    Ok(())
  }

  #[napi]
  pub async fn commit(&self, reference: String) -> Result<String> {
    Ok(reference)
  }

  #[napi]
  pub async fn export(&self, _output_path: Option<String>) -> Result<Vec<u8>> {
    Ok(vec![])
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
        id: Some(id),
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
        id: Some(info.id.clone()),
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

  #[napi(getter)]
  pub fn machines(&self) -> Result<JsMachineHandle> {
    Ok(JsMachineHandle {})
  }

  #[napi(getter, js_name = "k8s")]
  pub fn k8s(&self) -> Result<JsK8sHandle> {
    Ok(JsK8sHandle {})
  }

  #[napi(getter)]
  pub fn network(&self) -> Result<JsNetworkHandle> {
    Ok(JsNetworkHandle {})
  }

  #[napi(getter)]
  pub fn registry(&self) -> Result<JsRegistryHandle> {
    Ok(JsRegistryHandle {})
  }

  #[napi(getter)]
  pub fn system(&self) -> Result<JsSystemHandle> {
    Ok(JsSystemHandle {})
  }

  #[napi(getter)]
  pub fn compose(&self) -> Result<JsComposeHandle> {
    Ok(JsComposeHandle {})
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
      id: self.inner_container.id.clone(),
    }
  }
}
