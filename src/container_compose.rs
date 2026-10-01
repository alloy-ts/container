use napi_derive::napi;

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsComposeStatusOptions {
    pub socket: Option<String>,
    pub address: Option<String>,
    pub cacert: Option<String>,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsGenerateKeyOptions {
    pub name: Option<String>,
    pub auth_file: Option<String>,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsGenerateCertOptions {
    pub out_dir: Option<String>,
    pub cn: Option<String>,
    pub days: Option<i32>,
    pub san_dns: Option<Vec<String>>,
    pub san_ip: Option<Vec<String>>,
    pub force: Option<bool>,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsListKeysOptions {
    pub auth_file: Option<String>,
}

#[napi(object)]
#[derive(Default, Clone)]
pub struct JsRevokeKeyOptions {
    pub name: String,
    pub auth_file: Option<String>,
}

#[napi]
pub struct JsContainerCompose {}

#[napi]
impl JsContainerCompose {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    #[napi]
    pub fn status(&self, _options: Option<JsComposeStatusOptions>) -> bool {
        true
    }

    #[napi]
    pub fn generate_key(&self, options: Option<JsGenerateKeyOptions>) -> String {
        let name = options.and_then(|o| o.name).unwrap_or_else(|| "default".to_string());
        format!("key-{}", name)
    }

    #[napi]
    pub fn generate_cert(&self, _options: Option<JsGenerateCertOptions>) -> bool {
        true
    }

    #[napi]
    pub fn list_keys(&self, _options: Option<JsListKeysOptions>) -> Vec<String> {
        vec![]
    }

    #[napi]
    pub fn revoke_key(&self, options: JsRevokeKeyOptions) -> bool {
        !options.name.is_empty()
    }
}
