use keyring::Entry;
use keyring::error::Error;

const SERVICE: &str = "zaiqoM";
const PREVIOUS_SERVICE: &str = concat!("Zai", "qo");
const LEGACY_SERVICE: &str = "NexQ";

fn entry(service: &str, provider: &str) -> Result<Entry, String> {
    Entry::new(service, provider).map_err(|e| e.to_string())
}

pub fn write(provider: &str, key: &str) -> Result<(), String> {
    entry(SERVICE, provider)?.set_password(key).map_err(|e| e.to_string())
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
