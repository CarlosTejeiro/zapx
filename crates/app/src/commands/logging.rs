use tauri::State;

use core_persistence::SessionLog;

use crate::error::AppError;
use crate::state::{ActiveLog, AppState};

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

    let logger = core_logging::SessionLogger::open(&state.log_dir, &session_name)
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
