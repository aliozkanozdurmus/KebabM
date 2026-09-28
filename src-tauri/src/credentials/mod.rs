#[cfg(target_os = "linux")]
mod keyring_store;
#[cfg(target_os = "macos")]
mod local_store;
mod session_cache;
#[cfg(target_os = "windows")]
pub mod windows_cred;

use session_cache::SessionCache;

/// Credential manager for secure API key storage.
/// Windows stores new secrets as `zaiqoM:{provider}` and still reads `NexQ:{provider}`.
/// macOS uses an owner-only local file without Keychain access; Linux uses Secret Service.
pub struct CredentialManager {
    cache: SessionCache,
    #[cfg(target_os = "macos")]
    local_directory: Option<std::path::PathBuf>,
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            cache: SessionCache::default(),
            #[cfg(target_os = "macos")]
            local_directory: std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join("Library/Application Support/com.nexq.app/credentials")),
        }
    }

    pub fn with_data_dir(_directory: std::path::PathBuf) -> Self {
        #[cfg(target_os = "macos")]
        return Self { cache: SessionCache::default(), local_directory: Some(_directory.join("credentials")) };
        #[cfg(not(target_os = "macos"))]
        Self::new()
    }

    #[cfg(target_os = "macos")]
    fn local(&self) -> Result<&std::path::Path, String> {
        self.local_directory.as_deref().ok_or_else(|| "Local credential directory unavailable".into())
    }

    /// Store an API key for the given provider.
    pub fn store_key(&self, provider: &str, key: &str) -> Result<(), String> {
        let provider = Self::normalize_provider(provider);
        self.cache.store(&provider, key, || {
            #[cfg(target_os = "macos")]
            return local_store::write(self.local()?, &provider, key);
            #[cfg(not(target_os = "macos"))]
            store(&provider, key)
        })
    }

    /// Retrieve an API key for the given provider. Returns None if not stored.
    pub fn get_key(&self, provider: &str) -> Result<Option<String>, String> {
        let provider = Self::normalize_provider(provider);
        self.cache.read(&provider, || {
            #[cfg(target_os = "macos")]
            return local_store::read(self.local()?, &provider);
            #[cfg(not(target_os = "macos"))]
            read(&provider)
        })
    }

    /// Delete the API key for the given provider.
    pub fn delete_key(&self, provider: &str) -> Result<(), String> {
        let provider = Self::normalize_provider(provider);
        self.cache.delete(&provider, || {
            #[cfg(target_os = "macos")]
            return local_store::delete(self.local()?, &provider);
            #[cfg(not(target_os = "macos"))]
            delete(&provider)
        })
    }

    /// Check whether an API key exists for the given provider.
    pub fn has_key(&self, provider: &str) -> Result<bool, String> {
        let provider = Self::normalize_provider(provider);
        self.cache.contains(&provider, || {
            #[cfg(target_os = "macos")]
            return local_store::read(self.local()?, &provider).map(|key| key.is_some());
            #[cfg(not(target_os = "macos"))]
            exists(&provider)
        })
    }

    /// Normalize provider name to lowercase for consistent key naming.
    fn normalize_provider(provider: &str) -> String {
        provider.to_lowercase().trim().to_string()
    }
}

#[cfg(not(target_os = "macos"))]
fn exists(provider: &str) -> Result<bool, String> {
    read(provider).map(|value| value.is_some())
}

#[cfg(target_os = "windows")]
fn store(provider: &str, key: &str) -> Result<(), String> {
    windows_cred::credential_write(provider, key)
}

#[cfg(target_os = "windows")]
fn read(provider: &str) -> Result<Option<String>, String> {
    windows_cred::credential_read(provider)
}

#[cfg(target_os = "windows")]
fn delete(provider: &str) -> Result<(), String> {
    windows_cred::credential_delete(provider)
}

#[cfg(target_os = "linux")]
fn store(provider: &str, key: &str) -> Result<(), String> {
    keyring_store::write(provider, key)
}

#[cfg(target_os = "linux")]
fn read(provider: &str) -> Result<Option<String>, String> {
    keyring_store::read(provider)
}

#[cfg(target_os = "linux")]
fn delete(provider: &str) -> Result<(), String> {
    keyring_store::delete(provider)
}
