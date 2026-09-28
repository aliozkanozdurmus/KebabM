use tauri::{command, State};
use crate::{credentials::CredentialManager, state::AppState};
use std::sync::{Arc, Mutex};

// OS credential dialogs may wait for user input indefinitely. Never occupy a
// Tokio worker with either that call or the mutex behind another pending dialog.
async fn credential_operation<T: Send + 'static>(
    credentials: Arc<Mutex<CredentialManager>>,
    operation: impl FnOnce(&CredentialManager) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = credentials.lock().map_err(|_| "Credential manager unavailable")?;
        operation(&manager)
    }).await.map_err(|_| "Credential operation interrupted".to_string())?
}

fn credentials(state: &AppState) -> Result<Arc<Mutex<CredentialManager>>, String> {
    state.credentials.clone().ok_or_else(|| "Credential manager not initialized".into())
}

#[command]
pub async fn store_api_key(provider: String, key: String, state: State<'_, AppState>) -> Result<(), String> {
    credential_operation(credentials(&state)?, move |manager| manager.store_key(&provider, &key)).await
}

#[command]
pub async fn get_api_key(provider: String, state: State<'_, AppState>) -> Result<Option<String>, String> {
    credential_operation(credentials(&state)?, move |manager| manager.get_key(&provider)).await
}

#[command]
pub async fn delete_api_key(provider: String, state: State<'_, AppState>) -> Result<(), String> {
    credential_operation(credentials(&state)?, move |manager| manager.delete_key(&provider)).await
}

#[command]
pub async fn has_api_key(provider: String, state: State<'_, AppState>) -> Result<bool, String> {
    credential_operation(credentials(&state)?, move |manager| manager.has_key(&provider)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread")]
    async fn waiting_for_os_credentials_does_not_block_async_requests() {
        let credentials = Arc::new(Mutex::new(CredentialManager::new()));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let pending = tokio::spawn(credential_operation(credentials, move |_| {
            let _ = started_tx.send(());
            // A finite bound also makes a regression fail rather than hang CI.
            release_rx.recv_timeout(std::time::Duration::from_secs(2))
                .map_err(|_| "Async worker was blocked".to_string())?;
            Ok(())
        }));
        started_rx.await.unwrap();
        tokio::task::yield_now().await;
        release_tx.send(()).unwrap();
        pending.await.unwrap().unwrap();
    }
}
