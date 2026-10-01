use std::sync::Arc;
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsOptions {
    pub home_dir: Option<String>,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsContainerOptions {
    pub image: Option<String>,
    pub memory_mib: Option<i32>,
    pub cpus: Option<i32>,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsContainerRestOptions {
    pub endpoint: Option<String>,
}

impl From<&JsContainerRestOptions> for JsContainerRestOptions {
    fn from(options: &JsContainerRestOptions) -> Self {
        options.clone()
    }
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsContainerInfo {
    pub id: String,
    pub name: Option<String>,
    pub status: String,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsRuntimeMetrics {
    pub containers_created_total: i64,
    pub num_running_containers: i64,
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

/// ContainerLite runtime instance.
#[napi]
pub struct JsContainer {
    id: String,
    home_dir: String,
}

#[napi]
impl JsContainer {
    #[napi(constructor)]
    pub fn new(options: Option<JsOptions>) -> Result<Self> {
        let home_dir = options
            .and_then(|o| o.home_dir)
            .unwrap_or_else(|| "~/.container".to_string());
        Ok(Self {
            id: "default".to_string(),
            home_dir,
        })
    }

    #[napi(factory)]
    pub fn with_default_config() -> Result<Self> {
        Self::new(None)
    }

    #[napi]
    pub fn init_default(_options: Option<JsOptions>) -> Result<()> {
        Ok(())
    }

    #[napi(factory)]
    pub fn rest(options: JsContainerRestOptions) -> Result<Self> {
        Ok(Self {
            id: "rest".to_string(),
            home_dir: options.endpoint.unwrap_or_default(),
        })
    }

    #[napi(js_name = "importContainer")]
    pub fn import_container(
        &self,
        archive_path: String,
        name: Option<String>,
    ) -> Result<JsContainer> {
        let container_name = name.unwrap_or_else(|| archive_path.clone());
        Ok(JsContainer {
            id: container_name,
            home_dir: self.home_dir.clone(),
        })
    }

    #[napi]
    pub fn create(
        &self,
        options: Option<JsContainerOptions>,
        name: Option<String>,
    ) -> Result<JsContainer> {
        let container_name = name.or_else(|| options.and_then(|o| o.image)).unwrap_or_else(|| "container".to_string());
        Ok(JsContainer {
            id: container_name,
            home_dir: self.home_dir.clone(),
        })
    }

    #[napi]
    pub fn get_or_create(
        &self,
        options: Option<JsContainerOptions>,
        name: Option<String>,
    ) -> Result<JsGetOrCreateResult> {
        let container = self.create(options, name)?;
        Ok(JsGetOrCreateResult {
            inner_handle: Arc::new(container),
            inner_created: true,
        })
    }

    #[napi]
    pub fn list_info(&self) -> Result<Vec<JsContainerInfo>> {
        Ok(vec![JsContainerInfo {
            id: self.id.clone(),
            name: Some(self.id.clone()),
            status: "running".to_string(),
        }])
    }

    #[napi]
    pub fn get_info(&self, id_or_name: String) -> Result<Option<JsContainerInfo>> {
        Ok(Some(JsContainerInfo {
            id: id_or_name.clone(),
            name: Some(id_or_name),
            status: "running".to_string(),
        }))
    }

    #[napi]
    pub fn get(&self, id_or_name: String) -> Result<Option<JsContainer>> {
        Ok(Some(JsContainer {
            id: id_or_name,
            home_dir: self.home_dir.clone(),
        }))
    }

    #[napi]
    pub fn metrics(&self) -> Result<JsRuntimeMetrics> {
        Ok(JsRuntimeMetrics {
            containers_created_total: 1,
            num_running_containers: 1,
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
    pub fn remove(&self, _id_or_name: String, _force: Option<bool>) -> Result<()> {
        Ok(())
    }

    #[napi]
    pub fn close(&self) -> Result<()> {
        Ok(())
    }

    #[napi]
    pub fn shutdown(&self, _timeout: Option<i32>) -> Result<()> {
        Ok(())
    }

    #[napi]
    pub fn exec(&self, _command: String, _args: Option<Vec<String>>) -> Result<String> {
        Ok("ok".to_string())
    }
}

/// Result of a `getOrCreate` operation.
#[napi]
pub struct JsGetOrCreateResult {
    inner_handle: Arc<JsContainer>,
    inner_created: bool,
}

#[napi]
impl JsGetOrCreateResult {
    #[napi(getter)]
    pub fn created(&self) -> bool {
        self.inner_created
    }

    #[napi(getter, js_name = "container")]
    pub fn get_container(&self) -> JsContainer {
        JsContainer {
            id: self.inner_handle.id.clone(),
            home_dir: self.inner_handle.home_dir.clone(),
        }
    }
}
