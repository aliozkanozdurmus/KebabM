#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires access to the logged-in user's macOS Keychain"]
fn native_keychain_keeps_legacy_service_lookup_and_persists_new_entries() {
    use nexq_lib::credentials::CredentialManager;
    let provider = format!("meetinghelper-compat-{}", uuid::Uuid::new_v4());
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = CredentialManager::new().delete_key(&self.0);
        }
    }
    let _cleanup = Cleanup(provider.clone());
    // Create an old-identity fixture with this process's native access control.
    // Existing user entries and their access-control lists are never modified.
    keyring::Entry::new("NexQ", &provider)
        .unwrap()
        .set_password("synthetic-legacy-value")
        .unwrap();
    let manager = CredentialManager::new();
    assert!(manager.has_key(&provider).unwrap());
    assert!(!manager.has_key(&format!("{provider}-missing")).unwrap());
    assert_eq!(
        manager.get_key(&provider).unwrap().as_deref(),
        Some("synthetic-legacy-value")
    );
    manager
        .store_key(&provider, "synthetic-current-value")
        .unwrap();
    assert_eq!(
        manager.get_key(&provider).unwrap().as_deref(),
        Some("synthetic-current-value")
    );
    let external = std::process::Command::new("security")
        .args(["find-generic-password", "-s", "zaiqoM", "-a", &provider])
        .output()
        .unwrap();
    assert!(
        external.status.success(),
        "new entry was not persisted to the native Keychain"
    );
    manager.delete_key(&provider).unwrap();
    assert!(!manager.has_key(&provider).unwrap());
    assert!(manager.get_key(&provider).unwrap().is_none());
}
