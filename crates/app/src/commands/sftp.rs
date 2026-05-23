//! SFTP browser Tauri commands. Each looks up the SSH session by id, lazily
//! opens an SFTP subsystem on it the first time, then delegates to
//! [`core_transport::SftpClient`].

use std::path::PathBuf;
use std::sync::Arc;

use tauri::State;

use core_transport::{SftpClient, SftpEntry, SftpSlot};

use crate::error::AppError;
use crate::state::AppState;

/// Resolve (or lazily open) the [`SftpClient`] tied to a live SSH session.
async fn sftp_for(state: &AppState, session_id: &str) -> Result<Arc<SftpClient>, AppError> {
    // Pull ssh_handle + sftp slot out of the sync lock first; release before
    // any await to keep the sync mutex non-blocking.
    let (handle, slot): (_, SftpSlot) = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;
        let h = session
            .ssh_handle
            .clone()
            .ok_or_else(|| AppError::Internal("session is not SSH".into()))?;
        let s = session
            .sftp
            .clone()
            .ok_or_else(|| AppError::Internal("session is not SSH".into()))?;
        (h, s)
    };
    let mut guard = slot.lock().await;
    if let Some(client) = &*guard {
        return Ok(Arc::clone(client));
    }
    // First-use: open the SFTP subsystem.
    let client = Arc::new(
        SftpClient::open(&handle)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?,
    );
    *guard = Some(Arc::clone(&client));
    Ok(client)
}

#[tauri::command]
pub async fn sftp_list_dir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<Vec<SftpEntry>, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.list_dir(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn sftp_stat(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<SftpEntry, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.stat(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Resolve `path` (typically `"."`) to an absolute remote path. Used by the UI
/// to find the initial directory (the user's `$HOME`) when opening the browser.
#[tauri::command]
pub async fn sftp_canonicalize(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<String, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.canonicalize(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn sftp_mkdir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<(), AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.create_dir(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn sftp_remove_dir(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<(), AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.remove_dir(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn sftp_remove_file(
    state: State<'_, AppState>,
    session_id: String,
    path: String,
) -> Result<(), AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.remove_file(&path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn sftp_rename(
    state: State<'_, AppState>,
    session_id: String,
    from: String,
    to: String,
) -> Result<(), AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.rename(&from, &to)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Download a remote file to a local path. Returns the byte count written.
#[tauri::command]
pub async fn sftp_download_file(
    state: State<'_, AppState>,
    session_id: String,
    remote_path: String,
    local_path: String,
) -> Result<u64, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.download(&remote_path, &PathBuf::from(local_path))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Upload a local file to a remote path. Returns the byte count written.
#[tauri::command]
pub async fn sftp_upload_file(
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
) -> Result<u64, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    sftp.upload(&PathBuf::from(local_path), &remote_path)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}
