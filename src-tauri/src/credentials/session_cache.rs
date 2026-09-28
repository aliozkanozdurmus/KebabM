use std::{collections::HashMap, sync::Mutex};
use zeroize::Zeroizing;

/// Shared by all credential consumers for the lifetime of the native app.
/// No disk persistence; entries are wiped when replaced, removed or dropped.
#[derive(Default)]
pub(super) struct SessionCache {
    values: Mutex<HashMap<String, Zeroizing<String>>>,
}

impl SessionCache {
    pub fn read(
        &self,
        provider: &str,
        load: impl FnOnce() -> Result<Option<String>, String>,
    ) -> Result<Option<String>, String> {
        // Hold the lock through the first native read so concurrent callers share it.
        let mut values = self
            .values
            .lock()
            .map_err(|_| "Credential cache unavailable")?;
        if let Some(value) = values.get(provider) {
            return Ok(Some(value.to_string()));
        }
        let value = load()?;
        if let Some(value) = &value {
            values.insert(provider.into(), Zeroizing::new(value.clone()));
        }
        // Missing entries and access errors stay retryable.
        Ok(value)
    }

    pub fn contains(
        &self,
        provider: &str,
        probe: impl FnOnce() -> Result<bool, String>,
    ) -> Result<bool, String> {
        let values = self
            .values
            .lock()
            .map_err(|_| "Credential cache unavailable")?;
        if values.contains_key(provider) {
            return Ok(true);
        }
        probe()
    }

    pub fn store(
        &self,
        provider: &str,
        value: &str,
        persist: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "Credential cache unavailable")?;
        values.remove(provider);
        persist()?;
        values.insert(provider.into(), Zeroizing::new(value.into()));
        Ok(())
    }

    pub fn delete(
        &self,
        provider: &str,
        remove: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|_| "Credential cache unavailable")?;
        // Invalidate even on failure: legacy deletion may have partially succeeded.
        values.remove(provider);
        remove()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn simultaneous_reads_unlock_once_and_status_does_not_read_again() {
        let cache = SessionCache::default();
        let reads = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..12 {
                scope.spawn(|| {
                    let value = cache
                        .read("gemini", || {
                            reads.fetch_add(1, Ordering::SeqCst);
                            Ok(Some("synthetic-key".into()))
                        })
                        .unwrap();
                    assert_eq!(value.as_deref(), Some("synthetic-key"));
                });
            }
        });
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert!(cache
            .contains("gemini", || panic!("must use session value"))
            .unwrap());
    }

    #[test]
    fn metadata_probe_does_not_authorize_or_cache_a_secret() {
        let cache = SessionCache::default();
        assert!(cache.contains("gemini", || Ok(true)).unwrap());
        assert!(cache
            .read("gemini", || Err("access denied".into()))
            .is_err());
        assert_eq!(
            cache
                .read("gemini", || Ok(Some("authorized".into())))
                .unwrap()
                .as_deref(),
            Some("authorized")
        );
    }

    #[test]
    fn missing_and_denied_reads_can_be_retried() {
        let cache = SessionCache::default();
        assert_eq!(cache.read("gemini", || Ok(None)).unwrap(), None);
        assert!(cache.read("gemini", || Err("denied".into())).is_err());
        assert_eq!(
            cache
                .read("gemini", || Ok(Some("new".into())))
                .unwrap()
                .as_deref(),
            Some("new")
        );
    }

    #[test]
    fn writes_replace_cache_only_after_success_and_deletes_invalidate_it() {
        let cache = SessionCache::default();
        cache.store("gemini", "first", || Ok(())).unwrap();
        cache.store("gemini", "second", || Ok(())).unwrap();
        assert_eq!(
            cache
                .read("gemini", || panic!("must not read after save"))
                .unwrap()
                .as_deref(),
            Some("second")
        );
        assert!(cache
            .store("gemini", "failed", || Err("write failed".into()))
            .is_err());
        assert_eq!(
            cache
                .read("gemini", || Ok(Some("disk".into())))
                .unwrap()
                .as_deref(),
            Some("disk")
        );
        assert!(cache
            .delete("gemini", || Err("partial deletion".into()))
            .is_err());
        assert_eq!(cache.read("gemini", || Ok(None)).unwrap(), None);
        cache.store("gemini", "last", || Ok(())).unwrap();
        cache.delete("gemini", || Ok(())).unwrap();
        assert!(!cache.contains("gemini", || Ok(false)).unwrap());
    }
}
