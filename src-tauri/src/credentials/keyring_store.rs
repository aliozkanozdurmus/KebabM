use keyring::Entry;
use keyring::Error;

const SERVICE: &str = "zaiqoM";
const PREVIOUS_SERVICE: &str = concat!("Zai", "qo");
const LEGACY_SERVICE: &str = "NexQ";

fn entry(service: &str, provider: &str) -> Result<Entry, String> {
    Entry::new(service, provider).map_err(|e| e.to_string())
}

/// Query metadata only: status badges must never decrypt the password.
#[cfg(target_os = "macos")]
pub fn exists(provider: &str) -> Result<bool, String> {
    use security_framework::{
        item::{ItemClass, ItemSearchOptions},
        os::macos::keychain::{SecKeychain, SecPreferencesDomain},
    };
    if provider.is_empty() {
        return Err("Credential provider must not be empty".into());
    }
    let keychain = SecKeychain::default_for_domain(SecPreferencesDomain::User)
        .map_err(|e| e.to_string())?;
    for service in [SERVICE, PREVIOUS_SERVICE, LEGACY_SERVICE] {
        match ItemSearchOptions::new()
            .keychains(std::slice::from_ref(&keychain))
            .class(ItemClass::generic_password())
            .service(service)
            .account(provider)
            .load_attributes(true)
            .load_data(false)
            .skip_authenticated_items(true)
            .limit(1)
            .search()
        {
            Ok(items) if !items.is_empty() => return Ok(true),
            Ok(_) => {}
            Err(e) if e.code() == -25300 => {} // errSecItemNotFound
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(false)
}

pub fn write(provider: &str, key: &str) -> Result<(), String> {
    entry(SERVICE, provider)?
        .set_password(key)
        .map_err(|e| e.to_string())
}

pub fn read(provider: &str) -> Result<Option<String>, String> {
    match entry(SERVICE, provider)?.get_password() {
        Ok(value) => return Ok(Some(value)),
        Err(Error::NoEntry) => {}
        Err(e) => return Err(e.to_string()),
    }
    match entry(PREVIOUS_SERVICE, provider)?.get_password() {
        Ok(value) => return Ok(Some(value)),
        Err(Error::NoEntry) => {}
        Err(e) => return Err(e.to_string()),
    }
    match entry(LEGACY_SERVICE, provider)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn delete(provider: &str) -> Result<(), String> {
    delete_one(SERVICE, provider)?;
    delete_one(PREVIOUS_SERVICE, provider)?;
    delete_one(LEGACY_SERVICE, provider)
}

fn delete_one(service: &str, provider: &str) -> Result<(), String> {
    match entry(service, provider)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
