// macOS must exercise the real local adapter, with no access to OS Keychain.
#[cfg(target_os = "macos")]
#[test]
fn local_credentials_persist_across_restarts_with_private_permissions() {
    use nexq_lib::credentials::CredentialManager;
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("zaiqom-credentials-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let manager = CredentialManager::with_data_dir(root.clone());
    assert!(!manager.has_key("gemini").unwrap());
    manager.store_key(" Gemini ", "synthetic-first").unwrap();
    manager.store_key("anthropic", "synthetic-other").unwrap();
    drop(manager);
    let restarted = CredentialManager::with_data_dir(root.clone());
    assert_eq!(restarted.get_key("gemini").unwrap().as_deref(), Some("synthetic-first"));
    assert_eq!(std::fs::metadata(root.join("credentials")).unwrap().permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(root.join("credentials/credentials.json")).unwrap().permissions().mode() & 0o777, 0o600);
    restarted.delete_key("gemini").unwrap();
    drop(restarted);
    let third = CredentialManager::with_data_dir(root.clone());
    assert!(!third.has_key("gemini").unwrap());
    assert_eq!(third.get_key("anthropic").unwrap().as_deref(), Some("synthetic-other"));
    // A damaged store must never be silently replaced by an empty one.
    std::fs::write(root.join("credentials/credentials.json"), "invalid-json").unwrap();
    assert!(third.store_key("gemini", "replacement").is_err());
    assert_eq!(std::fs::read_to_string(root.join("credentials/credentials.json")).unwrap(), "invalid-json");
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn credential_store_rejects_symbolic_links_and_public_permissions() {
    use nexq_lib::credentials::CredentialManager;
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = std::env::temp_dir().join(format!("zaiqom-credentials-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let target = root.join("target");
    std::fs::create_dir(&target).unwrap();
    symlink(&target, root.join("credentials")).unwrap();
    let manager = CredentialManager::with_data_dir(root.clone());
    assert!(manager.store_key("gemini", "synthetic").is_err());
    assert!(!target.join("credentials.json").exists());
    std::fs::remove_file(root.join("credentials")).unwrap();
    std::fs::create_dir(root.join("credentials")).unwrap();
    std::fs::set_permissions(root.join("credentials"), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(manager.store_key("gemini", "synthetic").is_err());
    std::fs::remove_dir_all(root).unwrap();
}
