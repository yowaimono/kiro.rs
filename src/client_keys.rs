//! Managed client credentials. Only SHA-256 digests persist; raw keys are returned once.
use crate::common::auth::constant_time_eq;
use chrono::Utc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_KEYS: usize = 1000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoredKey {
    id: Uuid,
    name: String,
    prefix: String,
    suffix: String,
    hash: String,
    disabled: bool,
    created_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyFile {
    version: u32,
    keys: Vec<StoredKey>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientKeyInfo {
    pub id: Uuid,
    pub name: String,
    pub masked_key: String,
    pub disabled: bool,
    pub created_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedClientKey {
    pub key: String,
    pub item: ClientKeyInfo,
}

#[derive(Debug)]
pub enum KeyError {
    InvalidName,
    NotFound,
    LimitReached,
    Persistence,
}

pub struct ClientKeyStore {
    path: PathBuf,
    state: RwLock<KeyFile>,
}

fn digest(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}
impl From<&StoredKey> for ClientKeyInfo {
    fn from(key: &StoredKey) -> Self {
        Self {
            id: key.id,
            name: key.name.clone(),
            masked_key: format!("{}••••{}", key.prefix, key.suffix),
            disabled: key.disabled,
            created_at: key.created_at.clone(),
        }
    }
}

impl ClientKeyStore {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let state = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<KeyFile>(&bytes)
                .map_err(|_| anyhow::anyhow!("Invalid client key store format"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => KeyFile {
                version: 1,
                keys: Vec::new(),
            },
            Err(_) => anyhow::bail!("Cannot read client key store"),
        };
        let mut ids = std::collections::HashSet::new();
        if state.version != 1
            || state.keys.len() > MAX_KEYS
            || state.keys.iter().any(|key| {
                !ids.insert(key.id)
                    || key.hash.len() != 64
                    || !key.hash.bytes().all(|b| b.is_ascii_hexdigit())
                    || key.name.trim().is_empty()
                    || key.name.chars().count() > 80
                    || key.prefix.len() > 20
                    || key.suffix.len() > 4
            })
        {
            anyhow::bail!("Invalid client key store data");
        }
        Ok(Self {
            path,
            state: RwLock::new(state),
        })
    }
    pub fn list(&self) -> Vec<ClientKeyInfo> {
        self.state
            .read()
            .keys
            .iter()
            .map(ClientKeyInfo::from)
            .collect()
    }
    pub fn accepts(&self, raw: &str) -> bool {
        if raw.len() > 256 || raw.is_empty() {
            return false;
        }
        let hash = digest(raw);
        self.state
            .read()
            .keys
            .iter()
            .any(|key| !key.disabled && constant_time_eq(&key.hash, &hash))
    }
    pub fn create(&self, name: &str) -> Result<CreatedClientKey, KeyError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err(KeyError::InvalidName);
        }
        // UUID v4 uses OS-backed randomness; two UUIDs provide 244 random bits.
        let raw = format!(
            "sk-kiro-{}{}",
            Uuid::new_v4().simple(),
            Uuid::new_v4().simple()
        );
        let key = StoredKey {
            id: Uuid::new_v4(),
            name: name.to_owned(),
            prefix: raw[..16].to_owned(),
            suffix: raw[raw.len() - 4..].to_owned(),
            hash: digest(&raw),
            disabled: false,
            created_at: Utc::now().to_rfc3339(),
        };
        let info = ClientKeyInfo::from(&key);
        let mut state = self.state.write();
        if state.keys.len() >= MAX_KEYS {
            return Err(KeyError::LimitReached);
        }
        let mut next = state.clone();
        next.keys.insert(0, key);
        self.persist(&next)?;
        *state = next;
        Ok(CreatedClientKey {
            key: raw,
            item: info,
        })
    }
    pub fn set_disabled(&self, id: Uuid, disabled: bool) -> Result<ClientKeyInfo, KeyError> {
        let mut state = self.state.write();
        let mut next = state.clone();
        let key = next
            .keys
            .iter_mut()
            .find(|key| key.id == id)
            .ok_or(KeyError::NotFound)?;
        key.disabled = disabled;
        let info = ClientKeyInfo::from(&*key);
        self.persist(&next)?;
        *state = next;
        Ok(info)
    }
    pub fn delete(&self, id: Uuid) -> Result<(), KeyError> {
        let mut state = self.state.write();
        let mut next = state.clone();
        let index = next
            .keys
            .iter()
            .position(|key| key.id == id)
            .ok_or(KeyError::NotFound)?;
        next.keys.remove(index);
        self.persist(&next)?;
        *state = next;
        Ok(())
    }
    fn persist(&self, state: &KeyFile) -> Result<(), KeyError> {
        let content = serde_json::to_vec_pretty(state).map_err(|_| KeyError::Persistence)?;
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        let temporary = parent.join(format!(".client-keys-{}.tmp", Uuid::new_v4()));
        let result = (|| -> std::io::Result<()> {
            let mut options = OpenOptions::new();
            options.create_new(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&content)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &self.path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(KeyError::Persistence);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        dir: PathBuf,
        path: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("kiro-client-keys-{}", Uuid::new_v4()));
            fs::create_dir(&dir).unwrap();
            let path = dir.join("keys.json");
            Self { dir, path }
        }
        fn store(&self) -> ClientKeyStore {
            ClientKeyStore::load(&self.path).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
    #[test]
    fn lifecycle_is_persistent_and_never_lists_or_stores_raw_secret() {
        let f = Fixture::new();
        let store = f.store();
        let created = store.create("  Claude Code  ").unwrap();
        assert_eq!(created.item.name, "Claude Code");
        assert!(store.accepts(&created.key));
        assert!(!store.accepts("wrong"));
        let disk = fs::read_to_string(&f.path).unwrap();
        assert!(!disk.contains(&created.key));
        let listed = serde_json::to_string(&store.list()).unwrap();
        assert!(!listed.contains(&created.key));
        assert!(!listed.contains("hash"));
        assert!(f.store().accepts(&created.key));
        store.set_disabled(created.item.id, true).unwrap();
        assert!(!store.accepts(&created.key));
        assert!(!f.store().accepts(&created.key));
        store.set_disabled(created.item.id, false).unwrap();
        assert!(store.accepts(&created.key));
        store.delete(created.item.id).unwrap();
        assert!(!store.accepts(&created.key));
        assert!(f.store().list().is_empty());
    }
    #[test]
    fn invalid_names_and_missing_ids_are_rejected() {
        let f = Fixture::new();
        let store = f.store();
        for name in [
            "".to_owned(),
            " ".to_owned(),
            "x".repeat(81),
            "bad\nname".to_owned(),
        ] {
            assert!(matches!(store.create(&name), Err(KeyError::InvalidName)));
        }
        assert!(matches!(
            store.delete(Uuid::new_v4()),
            Err(KeyError::NotFound)
        ));
        assert!(!f.path.exists());
    }
    #[test]
    fn failed_persistence_does_not_mutate_runtime_auth() {
        let f = Fixture::new();
        let store = f.store();
        let key = store.create("test").unwrap();
        fs::remove_file(&f.path).unwrap();
        fs::create_dir(&f.path).unwrap();
        assert!(matches!(
            store.set_disabled(key.item.id, true),
            Err(KeyError::Persistence)
        ));
        assert!(store.accepts(&key.key));
        assert!(matches!(
            store.delete(key.item.id),
            Err(KeyError::Persistence)
        ));
        assert!(store.create("another").is_err());
        assert_eq!(store.list().len(), 1);
    }
    #[test]
    fn malformed_store_fails_closed() {
        let f = Fixture::new();
        fs::write(&f.path, b"not json").unwrap();
        assert!(ClientKeyStore::load(&f.path).is_err());
        fs::write(&f.path, br#"{"version":2,"keys":[]}"#).unwrap();
        assert!(ClientKeyStore::load(&f.path).is_err());
    }
    #[test]
    fn concurrent_mutations_preserve_all_keys() {
        let f = Fixture::new();
        let store = std::sync::Arc::new(f.store());
        let workers: Vec<_> = (0..12)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || store.create(&format!("key {i}")).unwrap())
            })
            .collect();
        let created: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
        assert_eq!(f.store().list().len(), 12);
        for key in created {
            assert!(store.accepts(&key.key));
        }
    }
    #[cfg(unix)]
    #[test]
    fn store_file_permissions_are_private() {
        use std::os::unix::fs::PermissionsExt;
        let f = Fixture::new();
        f.store().create("test").unwrap();
        assert_eq!(
            fs::metadata(&f.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    #[tokio::test]
    async fn real_http_lifecycle_and_admin_boundary() {
        use crate::{
            admin::{AdminService, AdminState},
            kiro::token_manager::MultiTokenManager,
            model::config::Config,
        };
        use std::sync::Arc;
        let f = Fixture::new();
        let store = Arc::new(f.store());
        let manager = Arc::new(
            MultiTokenManager::new(Config::default(), Vec::new(), None, None, true).unwrap(),
        );
        let admin = crate::admin::create_admin_router(
            AdminState::new(
                "admin-secret",
                AdminService::new(manager, ["ide".to_owned()]),
            )
            .with_client_keys(store.clone(), true),
        );
        let app = crate::anthropic::create_router_with_provider(
            "legacy-client",
            None,
            true,
            Some(store.clone()),
        )
        .nest("/api/admin", admin);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        let url = format!("{base}/api/admin/client-keys");
        for auth in ["", "wrong", "legacy-client"] {
            assert_eq!(
                client
                    .get(&url)
                    .header("x-api-key", auth)
                    .send()
                    .await
                    .unwrap()
                    .status(),
                401
            );
        }
        assert_eq!(
            client
                .post(&url)
                .header("x-api-key", "admin-secret")
                .json(&serde_json::json!({"name":" "}))
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
        let response = client
            .post(&url)
            .header("x-api-key", "admin-secret")
            .json(&serde_json::json!({"name":"Integration"}))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 201);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let created: serde_json::Value = response.json().await.unwrap();
        let raw = created["key"].as_str().unwrap();
        let id = created["item"]["id"].as_str().unwrap();
        let list = client
            .get(&url)
            .bearer_auth("admin-secret")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(!list.contains(raw));
        assert!(!list.contains("hash"));
        assert_eq!(
            client
                .get(&url)
                .header("x-api-key", raw)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            client
                .get(format!("{base}/v1/models"))
                .header("x-api-key", "admin-secret")
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        for route in [
            "/v1/models",
            "/v1/messages/count_tokens",
            "/cc/v1/messages/count_tokens",
        ] {
            let response = if route.ends_with("models") {
                client
                    .get(format!("{base}{route}"))
                    .bearer_auth(raw)
                    .send()
                    .await
                    .unwrap()
            } else {
                client.post(format!("{base}{route}")).header("x-api-key",raw).json(&serde_json::json!({"model":"claude-haiku-4-5-20251001","messages":[{"role":"user","content":"test"}]})).send().await.unwrap()
            };
            assert_eq!(response.status(), 200);
        }
        let disable = format!("{url}/{id}/disabled");
        assert_eq!(
            client
                .post(&disable)
                .header("x-api-key", raw)
                .json(&serde_json::json!({"disabled":true}))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert_eq!(
            client
                .post(&disable)
                .header("x-api-key", "admin-secret")
                .json(&serde_json::json!({"disabled":true}))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            client
                .get(format!("{base}/v1/models"))
                .bearer_auth(raw)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert!(!f.store().accepts(raw));
        assert_eq!(
            client
                .post(&disable)
                .header("x-api-key", "admin-secret")
                .json(&serde_json::json!({"disabled":false}))
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            client
                .get(format!("{base}/v1/models"))
                .bearer_auth(raw)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        assert_eq!(
            client
                .delete(format!("{url}/{id}"))
                .header("x-api-key", "admin-secret")
                .send()
                .await
                .unwrap()
                .status(),
            204
        );
        assert_eq!(
            client
                .get(format!("{base}/v1/models"))
                .bearer_auth(raw)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
        assert!(f.store().list().is_empty());
        assert_eq!(
            client
                .get(format!("{base}/v1/models"))
                .bearer_auth("legacy-client")
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
        task.abort();
    }
}
