use keyring::v1::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const KEYRING_SERVICE: &str = "com.machfudin.forge-desktop.github";
pub const GITHUB_PUBLIC_HOST: &str = "github.com";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    Pat,
    DeviceFlow,
    GhCliImport,
    GitCredentialsImport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountProfile {
    pub id: String,
    pub host: String,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
    pub label: Option<String>,
    pub auth_method: AuthMethod,
    pub added_at: u64,
    #[serde(default)]
    pub memory_only: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountRegistry {
    #[serde(default)]
    pub accounts: Vec<AccountProfile>,
    #[serde(default)]
    pub default_by_host: HashMap<String, String>,
    #[serde(default)]
    pub repo_bindings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountOverview {
    pub accounts: Vec<AccountProfile>,
    pub default_by_host: HashMap<String, String>,
    pub active_repo_account_id: Option<String>,
    pub primary_account_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AccountHealth {
    Valid { profile: AccountProfile },
    Expired { message: String },
    SsoRequired { message: String, sso_url: Option<String> },
    Offline { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretStoreError {
    Unavailable(String),
    Other(String),
}

impl SecretStoreError {
    pub fn message(&self) -> &str {
        match self {
            SecretStoreError::Unavailable(m) | SecretStoreError::Other(m) => m,
        }
    }
}

pub trait SecretStore: Send + Sync {
    fn set_token(&self, account_id: &str, token: &str) -> Result<(), SecretStoreError>;
    fn get_token(&self, account_id: &str) -> Result<Option<String>, SecretStoreError>;
    fn delete_token(&self, account_id: &str) -> Result<(), SecretStoreError>;
}

pub struct KeyringSecretStore;

fn map_keyring_error(err: KeyringError, op: &str) -> SecretStoreError {
    match err {
        KeyringError::NoStorageAccess(inner) => SecretStoreError::Unavailable(format!(
            "Penyimpanan kredensial OS tidak tersedia saat {op}: {inner}"
        )),
        KeyringError::NoDefaultStore => SecretStoreError::Unavailable(format!(
            "Tidak ada OS Keyring / Secret Service aktif saat {op}"
        )),
        KeyringError::NotSupportedByStore(_) => SecretStoreError::Unavailable(format!(
            "OS Keyring tidak mendukung operasi {op}"
        )),
        KeyringError::PlatformFailure(inner) => {
            let msg = inner.to_string();
            let lower = msg.to_ascii_lowercase();
            if lower.contains("dbus")
                || lower.contains("secret")
                || lower.contains("keyring")
                || lower.contains("session")
            {
                SecretStoreError::Unavailable(format!(
                    "OS Secret Service tidak terjangkau saat {op}: {msg}"
                ))
            } else {
                SecretStoreError::Other(format!("Gagal mengakses OS Keyring saat {op}: {msg}"))
            }
        }
        other => SecretStoreError::Other(format!("Error OS Keyring saat {op}: {other}")),
    }
}

impl SecretStore for KeyringSecretStore {
    fn set_token(&self, account_id: &str, token: &str) -> Result<(), SecretStoreError> {
        let entry = Entry::new(KEYRING_SERVICE, account_id)
            .map_err(|e| map_keyring_error(e, "membuka entri"))?;
        entry
            .set_password(token)
            .map_err(|e| map_keyring_error(e, "menyimpan token"))
    }

    fn get_token(&self, account_id: &str) -> Result<Option<String>, SecretStoreError> {
        let entry = Entry::new(KEYRING_SERVICE, account_id)
            .map_err(|e| map_keyring_error(e, "membuka entri"))?;
        match entry.get_password() {
            Ok(pw) => Ok(Some(pw)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(e) => Err(map_keyring_error(e, "membaca token")),
        }
    }

    fn delete_token(&self, account_id: &str) -> Result<(), SecretStoreError> {
        let entry = Entry::new(KEYRING_SERVICE, account_id)
            .map_err(|e| map_keyring_error(e, "membuka entri"))?;
        match entry.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(e) => Err(map_keyring_error(e, "menghapus token")),
        }
    }
}

pub fn normalize_github_host(input: &str) -> Result<String, String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Ok(GITHUB_PUBLIC_HOST.to_string());
    }
    if raw.to_ascii_lowercase().starts_with("http://") {
        return Err("Host GitHub Enterprise wajib menggunakan HTTPS".into());
    }
    let without_scheme = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("HTTPS://"))
        .unwrap_or(raw);

    if without_scheme.contains("://") {
        return Err("Skema URL host tidak valid".into());
    }

    let trimmed = without_scheme
        .trim_end_matches('/')
        .strip_suffix("/api/v3")
        .unwrap_or(without_scheme.trim_end_matches('/'))
        .trim_end_matches('/')
        .to_ascii_lowercase();

    if trimmed.is_empty()
        || trimmed.contains(['/', '\\', '?', '#', '@', ' ', '\t', '\r', '\n'])
        || trimmed.starts_with('.')
        || trimmed.ends_with('.')
        || trimmed.contains("..")
    {
        return Err("Nama host GitHub tidak valid (contoh: git.perusahaan.com)".into());
    }

    if !trimmed.contains('.') {
        return Err("Host Enterprise harus berupa domain valid (contoh: git.perusahaan.com)".into());
    }

    Ok(trimmed)
}

pub fn github_api_base(host: &str) -> Result<String, String> {
    let norm = normalize_github_host(host)?;
    if norm == GITHUB_PUBLIC_HOST {
        Ok("https://api.github.com".to_string())
    } else {
        Ok(format!("https://{norm}/api/v3"))
    }
}

pub fn github_web_base(host: &str) -> Result<String, String> {
    let norm = normalize_github_host(host)?;
    Ok(format!("https://{norm}"))
}

pub fn make_account_id(host: &str, login: &str) -> String {
    format!("{}/{}", host.to_ascii_lowercase(), login.to_ascii_lowercase())
}

pub fn canonical_repo_key(repo_path: &str) -> String {
    let trimmed = repo_path.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let resolved = fs::canonicalize(Path::new(trimmed))
        .unwrap_or_else(|_| PathBuf::from(trimmed));
    let mut s = resolved.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    if cfg!(windows) {
        s.to_ascii_lowercase()
    } else {
        s
    }
}

pub struct AccountManager {
    registry_path: PathBuf,
    registry: Mutex<AccountRegistry>,
    secret_store: Box<dyn SecretStore>,
    session_tokens: Mutex<HashMap<String, String>>,
}

impl AccountManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let registry_path = app_data_dir.join("accounts.json");
        Self::with_store(registry_path, Box::new(KeyringSecretStore))
    }

    pub fn with_store(registry_path: PathBuf, secret_store: Box<dyn SecretStore>) -> Self {
        let registry = Self::load_registry(&registry_path);
        Self {
            registry_path,
            registry: Mutex::new(registry),
            secret_store,
            session_tokens: Mutex::new(HashMap::new()),
        }
    }

    fn load_registry(path: &Path) -> AccountRegistry {
        let Ok(content) = fs::read_to_string(path) else {
            return AccountRegistry::default();
        };
        serde_json::from_str::<AccountRegistry>(&content).unwrap_or_default()
    }

    fn save_registry(&self, reg: &AccountRegistry) -> Result<(), String> {
        if let Some(parent) = self.registry_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Gagal membuat direktori konfigurasi akun: {e}"))?;
        }
        let json = serde_json::to_string_pretty(reg)
            .map_err(|e| format!("Gagal serialisasi registry akun: {e}"))?;
        fs::write(&self.registry_path, json)
            .map_err(|e| format!("Gagal menyimpan registry akun: {e}"))
    }

    pub fn upsert_account(
        &self,
        mut profile: AccountProfile,
        token: &str,
        allow_memory_fallback: bool,
    ) -> Result<AccountProfile, String> {
        let norm_host = normalize_github_host(&profile.host)?;
        profile.host = norm_host.clone();
        profile.id = make_account_id(&norm_host, &profile.login);

        match self.secret_store.set_token(&profile.id, token) {
            Ok(()) => {
                profile.memory_only = false;
                if let Ok(mut mem) = self.session_tokens.lock() {
                    mem.remove(&profile.id);
                }
            }
            Err(SecretStoreError::Unavailable(msg)) => {
                if allow_memory_fallback {
                    profile.memory_only = true;
                    let mut mem = self
                        .session_tokens
                        .lock()
                        .map_err(|_| "Gagal mengunci penyimpanan sesi".to_string())?;
                    mem.insert(profile.id.clone(), token.to_string());
                } else {
                    return Err(format!("[KEYRING_UNAVAILABLE] {msg}"));
                }
            }
            Err(SecretStoreError::Other(msg)) => return Err(msg),
        }

        let mut reg = self
            .registry
            .lock()
            .map_err(|_| "Gagal mengunci registry akun".to_string())?;

        if let Some(existing) = reg.accounts.iter_mut().find(|a| a.id == profile.id) {
            if profile.label.is_none() {
                profile.label = existing.label.clone();
            }
            *existing = profile.clone();
        } else {
            reg.accounts.push(profile.clone());
        }

        reg.default_by_host
            .entry(norm_host)
            .or_insert_with(|| profile.id.clone());

        self.save_registry(&reg)?;
        Ok(profile)
    }

    pub fn get_token(&self, account_id: &str) -> Result<Option<String>, String> {
        if let Ok(mem) = self.session_tokens.lock() {
            if let Some(tok) = mem.get(account_id) {
                return Ok(Some(tok.clone()));
            }
        }
        match self.secret_store.get_token(account_id) {
            Ok(tok) => Ok(tok),
            Err(SecretStoreError::Unavailable(msg)) | Err(SecretStoreError::Other(msg)) => {
                Err(msg)
            }
        }
    }

    pub fn get_profile(&self, account_id: &str) -> Option<AccountProfile> {
        let reg = self.registry.lock().ok()?;
        reg.accounts.iter().find(|a| a.id == account_id).cloned()
    }

    pub fn update_profile_metadata(&self, updated: &AccountProfile) -> Result<(), String> {
        let mut reg = self
            .registry
            .lock()
            .map_err(|_| "Gagal mengunci registry akun".to_string())?;
        if let Some(existing) = reg.accounts.iter_mut().find(|a| a.id == updated.id) {
            existing.name = updated.name.clone();
            existing.email = updated.email.clone();
            existing.avatar_url = updated.avatar_url.clone();
            existing.html_url = updated.html_url.clone();
            self.save_registry(&reg)?;
        }
        Ok(())
    }

    pub fn list_overview(&self, repo: Option<&str>) -> AccountOverview {
        let Ok(reg) = self.registry.lock() else {
            return AccountOverview {
                accounts: vec![],
                default_by_host: HashMap::new(),
                active_repo_account_id: None,
                primary_account_id: None,
            };
        };
        Self::build_overview(&reg, repo)
    }

    fn build_overview(reg: &AccountRegistry, repo: Option<&str>) -> AccountOverview {
        let active_repo_account_id = repo
            .map(canonical_repo_key)
            .filter(|k| !k.is_empty())
            .and_then(|k| reg.repo_bindings.get(&k).cloned())
            .filter(|id| reg.accounts.iter().any(|a| &a.id == id));

        let primary_account_id = active_repo_account_id
            .clone()
            .or_else(|| reg.default_by_host.get(GITHUB_PUBLIC_HOST).cloned())
            .or_else(|| reg.default_by_host.values().next().cloned())
            .or_else(|| reg.accounts.first().map(|a| a.id.clone()));

        AccountOverview {
            accounts: reg.accounts.clone(),
            default_by_host: reg.default_by_host.clone(),
            active_repo_account_id,
            primary_account_id,
        }
    }

    pub fn bind_repo(&self, repo: &str, account_id: &str) -> Result<AccountOverview, String> {
        let repo_key = canonical_repo_key(repo);
        if repo_key.is_empty() {
            return Err("Path repositori tidak valid".into());
        }
        let mut reg = self
            .registry
            .lock()
            .map_err(|_| "Gagal mengunci registry akun".to_string())?;
        if !reg.accounts.iter().any(|a| a.id == account_id) {
            return Err("Akun GitHub tidak ditemukan".into());
        }
        reg.repo_bindings.insert(repo_key, account_id.to_string());
        self.save_registry(&reg)?;
        Ok(Self::build_overview(&reg, Some(repo)))
    }

    pub fn set_default(&self, account_id: &str, repo: Option<&str>) -> Result<AccountOverview, String> {
        let mut reg = self
            .registry
            .lock()
            .map_err(|_| "Gagal mengunci registry akun".to_string())?;
        let Some(acc) = reg.accounts.iter().find(|a| a.id == account_id) else {
            return Err("Akun GitHub tidak ditemukan".into());
        };
        let host = acc.host.clone();
        reg.default_by_host.insert(host, account_id.to_string());
        self.save_registry(&reg)?;
        Ok(Self::build_overview(&reg, repo))
    }

    pub fn update_label(
        &self,
        account_id: &str,
        label: Option<&str>,
        repo: Option<&str>,
    ) -> Result<AccountOverview, String> {
        let clean = label
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let mut reg = self
            .registry
            .lock()
            .map_err(|_| "Gagal mengunci registry akun".to_string())?;
        let Some(acc) = reg.accounts.iter_mut().find(|a| a.id == account_id) else {
            return Err("Akun GitHub tidak ditemukan".into());
        };
        acc.label = clean;
        self.save_registry(&reg)?;
        Ok(Self::build_overview(&reg, repo))
    }

    pub fn remove_account(
        &self,
        account_id: &str,
        repo: Option<&str>,
    ) -> Result<AccountOverview, String> {
        if let Ok(mut mem) = self.session_tokens.lock() {
            mem.remove(account_id);
        }
        match self.secret_store.delete_token(account_id) {
            Ok(()) | Err(SecretStoreError::Unavailable(_)) => {}
            Err(SecretStoreError::Other(msg)) => return Err(msg),
        }

        let mut reg = self
            .registry
            .lock()
            .map_err(|_| "Gagal mengunci registry akun".to_string())?;

        let removed_host = reg
            .accounts
            .iter()
            .find(|a| a.id == account_id)
            .map(|a| a.host.clone());

        reg.accounts.retain(|a| a.id != account_id);
        reg.repo_bindings.retain(|_, v| v != account_id);

        if let Some(host) = removed_host {
            if reg.default_by_host.get(&host).map(|v| v == account_id).unwrap_or(false) {
                reg.default_by_host.remove(&host);
                let next_id = reg
                    .accounts
                    .iter()
                    .find(|a| a.host == host)
                    .map(|a| a.id.clone());
                if let Some(id) = next_id {
                    reg.default_by_host.insert(host, id);
                }
            }
        }

        self.save_registry(&reg)?;
        Ok(Self::build_overview(&reg, repo))
    }

    pub fn registered_hosts(&self) -> Vec<String> {
        let mut hosts = vec![GITHUB_PUBLIC_HOST.to_string()];
        if let Ok(reg) = self.registry.lock() {
            for acc in &reg.accounts {
                if !hosts.contains(&acc.host) {
                    hosts.push(acc.host.clone());
                }
            }
        }
        hosts
    }

    pub fn resolve_account_for_repo(
        &self,
        repo: Option<&str>,
        host: &str,
    ) -> Option<(AccountProfile, String)> {
        self.resolve_account(None, repo, host)
    }

    pub fn resolve_account(
        &self,
        account_id: Option<&str>,
        repo: Option<&str>,
        host: &str,
    ) -> Option<(AccountProfile, String)> {
        let norm_host = normalize_github_host(host).ok()?;
        let profile = {
            let reg = self.registry.lock().ok()?;
            if let Some(explicit_id) = account_id {
                reg.accounts.iter().find(|a| a.id == explicit_id).cloned()
            } else {
                let bound = repo
                    .map(canonical_repo_key)
                    .filter(|k| !k.is_empty())
                    .and_then(|k| reg.repo_bindings.get(&k))
                    .and_then(|id| reg.accounts.iter().find(|a| &a.id == id && a.host == norm_host))
                    .cloned();

                bound
                    .or_else(|| {
                        reg.default_by_host
                            .get(&norm_host)
                            .and_then(|id| reg.accounts.iter().find(|a| &a.id == id))
                            .cloned()
                    })
                    .or_else(|| {
                        let matching: Vec<&AccountProfile> =
                            reg.accounts.iter().filter(|a| a.host == norm_host).collect();
                        if matching.len() == 1 {
                            Some(matching[0].clone())
                        } else {
                            None
                        }
                    })
            }
        }?;

        let tok = self.get_token(&profile.id).ok()??;
        Some((profile, tok))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemorySecretStore {
        data: Mutex<HashMap<String, String>>,
        fail_unavailable: bool,
    }

    impl InMemorySecretStore {
        fn new() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
                fail_unavailable: false,
            }
        }

        fn unavailable() -> Self {
            Self {
                data: Mutex::new(HashMap::new()),
                fail_unavailable: true,
            }
        }
    }

    impl SecretStore for InMemorySecretStore {
        fn set_token(&self, account_id: &str, token: &str) -> Result<(), SecretStoreError> {
            if self.fail_unavailable {
                return Err(SecretStoreError::Unavailable("No DBus".into()));
            }
            self.data
                .lock()
                .unwrap()
                .insert(account_id.to_string(), token.to_string());
            Ok(())
        }

        fn get_token(&self, account_id: &str) -> Result<Option<String>, SecretStoreError> {
            if self.fail_unavailable {
                return Err(SecretStoreError::Unavailable("No DBus".into()));
            }
            Ok(self.data.lock().unwrap().get(account_id).cloned())
        }

        fn delete_token(&self, account_id: &str) -> Result<(), SecretStoreError> {
            if self.fail_unavailable {
                return Err(SecretStoreError::Unavailable("No DBus".into()));
            }
            self.data.lock().unwrap().remove(account_id);
            Ok(())
        }
    }

    fn temp_registry_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        let pid = std::process::id();
        p.push(format!("forge-desktop-acc-test-{pid}-{name}"));
        let _ = fs::remove_dir_all(&p);
        p.join("accounts.json")
    }

    fn sample_profile(host: &str, login: &str, label: Option<&str>) -> AccountProfile {
        AccountProfile {
            id: make_account_id(host, login),
            host: host.to_string(),
            login: login.to_string(),
            name: Some(login.to_string()),
            email: Some(format!("{login}@example.com")),
            avatar_url: None,
            html_url: Some(format!("https://{host}/{login}")),
            label: label.map(|s| s.to_string()),
            auth_method: AuthMethod::Pat,
            added_at: 1000,
            memory_only: false,
        }
    }

    #[test]
    fn test_normalize_host_and_urls() {
        assert_eq!(normalize_github_host("").unwrap(), "github.com");
        assert_eq!(normalize_github_host("https://GitHub.com/").unwrap(), "github.com");
        assert_eq!(
            normalize_github_host("https://git.kantor.id/api/v3/").unwrap(),
            "git.kantor.id"
        );
        assert!(normalize_github_host("http://git.kantor.id").is_err());
        assert!(normalize_github_host("git.kantor.id/foo").is_err());
        assert!(normalize_github_host("localhost").is_err());

        assert_eq!(github_api_base("github.com").unwrap(), "https://api.github.com");
        assert_eq!(
            github_api_base("git.kantor.id").unwrap(),
            "https://git.kantor.id/api/v3"
        );
        assert_eq!(github_web_base("git.kantor.id").unwrap(), "https://git.kantor.id");
    }

    #[test]
    fn test_account_crud_binding_and_precedence() {
        let reg_path = temp_registry_path("crud");
        let mgr = AccountManager::with_store(reg_path.clone(), Box::new(InMemorySecretStore::new()));

        let personal = mgr
            .upsert_account(sample_profile("github.com", "alice", Some("Pribadi")), "tok_alice", false)
            .unwrap();
        let work = mgr
            .upsert_account(sample_profile("github.com", "alice-work", Some("Kantor")), "tok_work", false)
            .unwrap();
        let ghe = mgr
            .upsert_account(sample_profile("git.kantor.id", "budi", None), "tok_ghe", false)
            .unwrap();

        // First account on host becomes default automatically
        let overview = mgr.list_overview(None);
        assert_eq!(overview.default_by_host.get("github.com"), Some(&personal.id));
        assert_eq!(overview.default_by_host.get("git.kantor.id"), Some(&ghe.id));

        // Repo binding overrides default for same host
        let repo_a = "/tmp/repo-kantor";
        mgr.bind_repo(repo_a, &work.id).unwrap();
        let resolved_work = mgr.resolve_account_for_repo(Some(repo_a), "github.com").unwrap();
        assert_eq!(resolved_work.0.login, "alice-work");
        assert_eq!(resolved_work.1, "tok_work");

        // Re-auth preserves repo_bindings & label when omitted
        mgr.upsert_account(sample_profile("github.com", "alice-work", None), "tok_work_v2", false)
            .unwrap();
        let resolved_v2 = mgr.resolve_account_for_repo(Some(repo_a), "github.com").unwrap();
        assert_eq!(resolved_v2.1, "tok_work_v2");
        assert_eq!(resolved_v2.0.label.as_deref(), Some("Kantor"));

        // Removing work account cleans repo_binding & falls back to default
        mgr.remove_account(&work.id, Some(repo_a)).unwrap();
        let overview_after = mgr.list_overview(Some(repo_a));
        assert_eq!(overview_after.active_repo_account_id, None);
        let fallback = mgr.resolve_account_for_repo(Some(repo_a), "github.com").unwrap();
        assert_eq!(fallback.0.login, "alice");

        let _ = fs::remove_dir_all(reg_path.parent().unwrap());
    }

    #[test]
    fn test_keyring_unavailable_requires_opt_in_memory_fallback() {
        let reg_path = temp_registry_path("fallback");
        let mgr = AccountManager::with_store(
            reg_path.clone(),
            Box::new(InMemorySecretStore::unavailable()),
        );

        let err = mgr
            .upsert_account(sample_profile("github.com", "headless", None), "tok_1", false)
            .unwrap_err();
        assert!(err.starts_with("[KEYRING_UNAVAILABLE]"));

        let saved = mgr
            .upsert_account(sample_profile("github.com", "headless", None), "tok_1", true)
            .unwrap();
        assert!(saved.memory_only);
        assert_eq!(mgr.get_token(&saved.id).unwrap(), Some("tok_1".into()));

        let _ = fs::remove_dir_all(reg_path.parent().unwrap());
    }
}
