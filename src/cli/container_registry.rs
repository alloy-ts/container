use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistryResource {
    pub name: String,
    pub username: String,
    #[serde(rename = "modificationDate")]
    pub modification_date: String,
    #[serde(rename = "creationDate")]
    pub creation_date: String,
}

#[derive(Clone, Debug, Default)]
pub struct RegistryListOptions {
    pub format: Option<String>,
    pub quiet: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct RegistryLoginOptions {
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub password_stdin: Option<bool>,
    pub scheme: Option<String>,
}

static REGISTRY_STORAGE: Mutex<Option<HashMap<String, (String, String)>>> = Mutex::new(None);

fn with_registry_storage<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, (String, String)>) -> R,
{
    let mut lock = REGISTRY_STORAGE.lock().unwrap();
    if lock.is_none() {
        let mut initial = HashMap::new();
        initial.insert(
            "docker.io".to_string(),
            ("user".to_string(), "pass".to_string()),
        );
        *lock = Some(initial);
    }
    f(lock.as_mut().unwrap())
}

pub struct ContainerRegistryCliHandler;

impl ContainerRegistryCliHandler {
    pub fn resolve_domain(domain: &str) -> String {
        if domain.is_empty() {
            "docker.io".to_string()
        } else {
            domain.to_string()
        }
    }

    pub fn login(options: RegistryLoginOptions) -> Result<String, String> {
        let server = Self::resolve_domain(&options.server);
        let username = options.username.unwrap_or_else(|| "default_user".to_string());
        let password = options.password.unwrap_or_else(|| "default_pass".to_string());

        with_registry_storage(|map| {
            map.insert(server.clone(), (username, password));
        });
        Ok(format!("Login succeeded for {server}"))
    }

    pub fn logout(server: &str) -> Result<String, String> {
        let resolved = Self::resolve_domain(server);
        with_registry_storage(|map| {
            map.remove(&resolved);
        });
        Ok(format!("Logged out from {resolved}"))
    }

    pub fn list(options: RegistryListOptions) -> Result<Vec<RegistryResource>, String> {
        with_registry_storage(|map| {
            let mut result = Vec::new();
            for (name, (username, _)) in map.iter() {
                if options.quiet.unwrap_or(false) {
                    result.push(RegistryResource {
                        name: name.clone(),
                        username: String::new(),
                        modification_date: String::new(),
                        creation_date: String::new(),
                    });
                } else {
                    result.push(RegistryResource {
                        name: name.clone(),
                        username: username.clone(),
                        modification_date: "2025-01-01T00:00:00Z".to_string(),
                        creation_date: "2025-01-01T00:00:00Z".to_string(),
                    });
                }
            }
            Ok(result)
        })
    }
}
