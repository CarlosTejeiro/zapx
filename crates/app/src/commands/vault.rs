//! Credential vault commands (the reusable-credentials feature).
//!
//! A vault entry is a reusable credential row (`credentials.reusable=1`). Its
//! secret lives only in the OS keyring; the bridge only ever carries opaque
//! references (`VaultEntryDto`, never a password field). The single
//! security-critical path, [`send_vault_secret`], retrieves the secret and
//! writes it straight to the live session's PTY without ever returning it.

use tauri::State;
use uuid::Uuid;

use core_transport::SessionCmd;

use crate::error::AppError;
use crate::state::AppState;

/// A vault entry as seen by the frontend. Carries NO secret — only the metadata
/// needed to list and reference it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VaultEntryDto {
    pub id: i64,
    pub name: String,
    pub username: Option<String>,
}

/// Create a reusable vault entry. The password goes to the OS keyring; only the
/// keyring reference + metadata are persisted. Returns the new credential id.
#[tauri::command]
pub async fn create_vault_entry(
    state: State<'_, AppState>,
    name: String,
    username: Option<String>,
    password: String,
) -> Result<i64, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("vault entry name is required".into()));
    }
    let keyring_key = format!("vault:{}", Uuid::new_v4());
    core_vault::Vault::store(&keyring_key, &password)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    state
        .db
        .create_vault_entry(name.trim(), username.as_deref(), &keyring_key)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// List all reusable vault entries (no secrets).
#[tauri::command]
pub async fn list_vault_entries(
    state: State<'_, AppState>,
) -> Result<Vec<VaultEntryDto>, AppError> {
    let entries = state
        .db
        .list_vault_entries()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(entries
        .into_iter()
        .map(|c| VaultEntryDto {
            id: c.id,
            name: c.name,
            username: c.username,
        })
        .collect())
}

/// Update a vault entry's metadata, and optionally rotate its secret. When
/// `password` is `Some`, the new secret is stored over the existing keyring key.
#[tauri::command]
pub async fn update_vault_entry(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("vault entry name is required".into()));
    }
    state
        .db
        .update_credential_meta(id, name.trim(), username.as_deref())
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if let Some(pw) = password {
        let key = state
            .db
            .get_credential_keyring_key(id)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        core_vault::Vault::store(&key, &pw).map_err(|e| AppError::Internal(e.to_string()))?;
    }
    Ok(())
}

/// Delete a vault entry: remove its keyring secret and its credential row.
/// Sessions referencing it have their `credential_id` set to NULL by the FK.
#[tauri::command]
pub async fn delete_vault_entry(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    if let Ok(key) = state.db.get_credential_keyring_key(id) {
        core_vault::Vault::delete(&key).ok();
    }
    state
        .db
        .delete_credential(id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Security-critical: send a vault entry's secret to a live session's PTY.
///
/// The secret is read from the keyring and written straight to the session's
/// input channel (the SAME path `send_input` uses) — it is never returned over
/// the bridge. `\r` is appended when `enter` is set.
#[tauri::command]
pub async fn send_vault_secret(
    state: State<'_, AppState>,
    session_id: String,
    vault_entry_id: i64,
    enter: bool,
) -> Result<(), AppError> {
    let key = state
        .db
        .get_credential_keyring_key(vault_entry_id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let secret =
        core_vault::Vault::retrieve(&key).map_err(|e| AppError::Internal(e.to_string()))?;

    let mut bytes = secret.into_bytes();
    if enter {
        bytes.push(b'\r');
    }

    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;
    session.cmd_tx.send(SessionCmd::Data(bytes)).ok();
    Ok(())
}
