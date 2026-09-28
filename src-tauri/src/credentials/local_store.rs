//! Owner-only local credentials on macOS. Never probes or migrates Keychain.
//! The file is plaintext protected by filesystem permissions, not an OS vault.
use std::{collections::BTreeMap, fs::{self, OpenOptions}, io::Write, path::Path};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    keys: BTreeMap<String, String>,
}
impl Default for Document {
    fn default() -> Self { Self { version: 1, keys: BTreeMap::new() } }
}

fn verify_directory(directory: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Ok(meta) if meta.is_dir() && meta.permissions().mode() & 0o077 == 0 => Ok(true),
        _ => Err("Local credential directory must be private (0700) and must not be a symbolic link".into()),
    }
}

fn load(directory: &Path) -> Result<Document, String> {
    if !verify_directory(directory)? { return Ok(Document::default()); }
    let path = directory.join("credentials.json");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Document::default()),
        Ok(meta) if meta.is_file() && meta.permissions().mode() & 0o077 == 0 => {},
        _ => return Err("Local credential file must be private (0600) and must not be a symbolic link".into()),
    }
    let text = zeroize::Zeroizing::new(fs::read_to_string(path).map_err(|_| "Cannot read local credentials")?);
    let document: Document = serde_json::from_str(&text).map_err(|_| "Local credential file is invalid; it has not been overwritten")?;
    if document.version != 1 { return Err("Unsupported local credential format; file has not been overwritten".into()); }
    Ok(document)
}

fn save(directory: &Path, document: &Document) -> Result<(), String> {
    if !verify_directory(directory)? {
        // Parent is the app data directory, already created during application setup.
        std::fs::DirBuilder::new().mode(0o700).create(directory)
            .map_err(|_| "Cannot create private credential directory")?;
    }
    let temporary = directory.join(format!(".credentials-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary)
            .map_err(|_| "Cannot create local credential file")?;
        let encoded = zeroize::Zeroizing::new(serde_json::to_vec(document).map_err(|_| "Cannot encode local credentials")?);
        file.write_all(&encoded).and_then(|_| file.sync_all()).map_err(|_| "Cannot save local credentials")?;
        fs::rename(&temporary, directory.join("credentials.json")).map_err(|_| "Cannot replace local credentials")?;
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(temporary); }
    result
}

pub fn read(directory: &Path, provider: &str) -> Result<Option<String>, String> {
    Ok(load(directory)?.keys.remove(provider))
}
pub fn write(directory: &Path, provider: &str, key: &str) -> Result<(), String> {
    if provider.trim().is_empty() || key.trim().is_empty() { return Err("Provider and API key must not be empty".into()); }
    let mut document = load(directory)?;
    document.keys.insert(provider.into(), key.into());
    save(directory, &document)
}
pub fn delete(directory: &Path, provider: &str) -> Result<(), String> {
    let mut document = load(directory)?;
    if document.keys.remove(provider).is_some() { save(directory, &document)?; }
    Ok(())
}
