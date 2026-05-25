use tauri::State;

use core_persistence::SessionLog;

use crate::error::AppError;
use crate::state::{ActiveLog, AppState};

/// Reveal the session-log directory in the host's file explorer
/// (Finder / Explorer / file manager). Best-effort: silently no-ops if
/// the platform opener is unavailable. Returns the directory path so the
/// frontend can show it in a toast.
#[tauri::command]
pub async fn open_logs_dir(state: State<'_, AppState>) -> Result<String, AppError> {
    let path = state.log_dir.clone();
    let path_str = path.to_string_lossy().into_owned();
    let _ = std::fs::create_dir_all(&path);
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(e) = std::process::Command::new(opener).arg(&path).spawn() {
        tracing::warn!("open_logs_dir spawn failed: {e}");
    }
    Ok(path_str)
}

#[tauri::command]
pub async fn start_session_logging(
    state: State<'_, AppState>,
    session_id: String,
    saved_session_id: Option<i64>,
    session_name: String,
) -> Result<String, AppError> {
    {
        let map = state.loggers.lock().unwrap();
        if map.contains_key(&session_id) {
            return Err(AppError::Internal("logging already active".into()));
        }
    }

    // Honour the user's "logging.format" setting (plain | raw). Plain is the
    // default and strips ANSI/VT escape sequences so the file reads as text.
    let format = state
        .db
        .get_setting("logging.format")
        .ok()
        .flatten()
        .as_deref()
        .map(core_logging::LogFormat::parse)
        .unwrap_or(core_logging::LogFormat::Plain);

    let logger = core_logging::SessionLogger::open_with_format(&state.log_dir, &session_name, format)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let path_str = logger
        .path()
        .to_str()
        .ok_or_else(|| AppError::Internal("non-UTF-8 log path".into()))?
        .to_owned();

    let started_at = logger.started_at().to_rfc3339();

    let log_db_id = state
        .db
        .create_session_log(saved_session_id, &path_str, &started_at)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    state
        .loggers
        .lock()
        .unwrap()
        .insert(session_id.clone(), ActiveLog { logger, log_db_id });

    tracing::debug!(session_id, log_db_id, path = path_str, "logging started");
    Ok(path_str)
}

#[tauri::command]
pub async fn stop_session_logging(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), AppError> {
    let active = state
        .loggers
        .lock()
        .unwrap()
        .remove(&session_id)
        .ok_or_else(|| AppError::Internal("no active log for this session".into()))?;

    let log_db_id = active.log_db_id;
    let (_, bytes, _) = active
        .logger
        .close()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let ended_at = chrono::Utc::now().to_rfc3339();
    state
        .db
        .end_session_log(log_db_id, &ended_at, bytes as i64)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    tracing::debug!(session_id, log_db_id, bytes, "logging stopped");
    Ok(())
}

#[tauri::command]
pub async fn list_session_logs(
    state: State<'_, AppState>,
    saved_session_id: i64,
) -> Result<Vec<SessionLog>, AppError> {
    state
        .db
        .list_session_logs(saved_session_id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn list_all_session_logs(
    state: State<'_, AppState>,
) -> Result<Vec<SessionLog>, AppError> {
    state
        .db
        .list_all_session_logs()
        .map_err(|e| AppError::Internal(e.to_string()))
}
