//! SFTP browser Tauri commands. Each looks up the SSH session by id, lazily
//! opens an SFTP subsystem on it the first time, then delegates to
//! [`core_transport::SftpClient`].
//!
//! Large transfers are streamed and emit `sftp-progress` events every ~100 ms
//! so the frontend can render a progress bar and offer a cancel button.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use core_transport::{new_sftp_cancel_token, SftpClient, SftpEntry, SftpSlot};

use crate::error::AppError;
use crate::state::AppState;

/// Progress event shipped to the frontend.
#[derive(serde::Serialize, Clone)]
struct SftpProgress {
    transfer_id: String,
    /// Bytes transferred so far.
    done: u64,
    /// Total file size, or `None` if not known up front.
    total: Option<u64>,
    /// `"download"` or `"upload"`.
    direction: &'static str,
}

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
    // First-use: open the SFTP subsystem. Lock the Mutex briefly — only the
    // `channel_open_session` + `request_subsystem` calls happen under the
    // lock; the returned channel runs independently after that.
    let client = {
        let guard = handle.lock().await;
        Arc::new(
            SftpClient::open(&guard)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?,
        )
    };
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

/// Throttle helper: only emit when at least 100 ms have passed since the
/// last emission for this transfer. Last-progress events fire unconditionally
/// (the streaming function calls `progress` once after the loop exits).
const PROGRESS_INTERVAL_MS: u64 = 100;

/// Build a progress callback that emits `sftp-progress` over Tauri while
/// rate-limiting bursts to ~10 Hz. Always emits the first (done=0) and
/// final calls.
fn make_progress(
    app: AppHandle,
    transfer_id: String,
    direction: &'static str,
) -> core_transport::ProgressFn {
    let last_emit = Arc::new(AtomicU64::new(0));
    Arc::new(move |done: u64, total: Option<u64>| {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let prev = last_emit.load(Ordering::Relaxed);
        let final_call = total.map(|t| done >= t).unwrap_or(false);
        if done > 0 && !final_call && now.saturating_sub(prev) < PROGRESS_INTERVAL_MS {
            return;
        }
        last_emit.store(now, Ordering::Relaxed);
        let _ = app.emit(
            "sftp-progress",
            SftpProgress {
                transfer_id: transfer_id.clone(),
                done,
                total,
                direction,
            },
        );
    })
}

/// Streaming download with per-chunk progress + cancel. The frontend supplies
/// `transfer_id` (any uuid will do) and listens for `sftp-progress` filtered
/// by that id; calling `sftp_cancel_transfer(transfer_id)` flips the cancel
/// flag the streaming loop checks after each chunk.
#[tauri::command]
pub async fn sftp_download_file(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    remote_path: String,
    local_path: String,
    transfer_id: String,
) -> Result<u64, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    let cancel = new_sftp_cancel_token();
    state
        .sftp_cancellations
        .lock()
        .unwrap()
        .insert(transfer_id.clone(), Arc::clone(&cancel));

    let progress = make_progress(app, transfer_id.clone(), "download");
    let result = sftp
        .download_streaming(&remote_path, &PathBuf::from(local_path), cancel, progress)
        .await;

    // Always remove the token, success or failure, so the map doesn't leak.
    state.sftp_cancellations.lock().unwrap().remove(&transfer_id);
    result.map_err(|e| AppError::Internal(e.to_string()))
}

/// Streaming upload — mirrors [`sftp_download_file`] for the opposite direction.
#[tauri::command]
pub async fn sftp_upload_file(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    local_path: String,
    remote_path: String,
    transfer_id: String,
) -> Result<u64, AppError> {
    let sftp = sftp_for(&state, &session_id).await?;
    let cancel = new_sftp_cancel_token();
    state
        .sftp_cancellations
        .lock()
        .unwrap()
        .insert(transfer_id.clone(), Arc::clone(&cancel));

    let progress = make_progress(app, transfer_id.clone(), "upload");
    let result = sftp
        .upload_streaming(&PathBuf::from(local_path), &remote_path, cancel, progress)
        .await;

    state.sftp_cancellations.lock().unwrap().remove(&transfer_id);
    result.map_err(|e| AppError::Internal(e.to_string()))
}

/// Cooperatively cancel an in-flight SFTP transfer. The streaming loop
/// checks the flag after every chunk, so cancellation happens within at
/// most one chunk's wall-clock time (~ STREAM_CHUNK ÷ link bandwidth).
/// No-op if `transfer_id` isn't active (e.g. it already finished).
#[tauri::command]
pub async fn sftp_cancel_transfer(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<(), AppError> {
    if let Some(token) = state.sftp_cancellations.lock().unwrap().get(&transfer_id) {
        token.store(true, Ordering::Relaxed);
    }
    Ok(())
}
