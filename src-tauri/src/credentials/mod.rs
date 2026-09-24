#[cfg(target_os = "windows")]
pub mod windows_cred;
#[cfg(not(target_os = "windows"))]
mod keyring_store;

/// Credential manager for secure API key storage.
/// Windows stores new secrets as `zaiqoM:{provider}` and still reads `NexQ:{provider}`.
/// macOS uses the login keychain and Linux uses the Secret Service.
pub struct CredentialManager;

impl CredentialManager {
    pub fn new() -> Self {
        Self
    }

    /// Store an API key for the given provider.
    pub fn store_key(&self, provider: &str, key: &str) -> Result<(), String> {
        let provider = Self::normalize_provider(provider);
        store(&provider, key)
    }

    /// Retrieve an API key for the given provider. Returns None if not stored.
    pub fn get_key(&self, provider: &str) -> Result<Option<String>, String> {
        let provider = Self::normalize_provider(provider);
        read(&provider)
    }

    /// Delete the API key for the given provider.
    pub fn delete_key(&self, provider: &str) -> Result<(), String> {
        let provider = Self::normalize_provider(provider);
        delete(&provider)
    }

    /// Check whether an API key exists for the given provider.
    pub fn has_key(&self, provider: &str) -> Result<bool, String> {
        let provider = Self::normalize_provider(provider);
        Ok(read(&provider)?.is_some())
    }

    /// Normalize provider name to lowercase for consistent key naming.
    fn normalize_provider(provider: &str) -> String {
        provider.to_lowercase().trim().to_string()
    }
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

#[cfg(not(target_os = "windows"))]
fn store(provider: &str, key: &str) -> Result<(), String> {
    keyring_store::write(provider, key)
}

#[cfg(not(target_os = "windows"))]
fn read(provider: &str) -> Result<Option<String>, String> {
    keyring_store::read(provider)
}

#[cfg(not(target_os = "windows"))]
fn delete(provider: &str) -> Result<(), String> {
    keyring_store::delete(provider)
}
