//! Tauri commands for the command-hint engine.
//!
//! Snippet CRUD lives in [`crate::commands::snippets`] — the hint engine just
//! reads from the same table so the dialog and the popup stay in sync.

use std::str::FromStr;

use tauri::State;

use core_hints::{Hint, HintEngine, Platform};

use crate::error::AppError;
use crate::state::AppState;

fn platform_from_opt(s: Option<&str>) -> Platform {
    s.and_then(|raw| Platform::from_str(raw).ok())
        .unwrap_or(Platform::Generic)
}

/// Resolve the platform for a saved session: explicit override beats the
/// global default in `settings`.
fn resolve_platform(state: &AppState, saved_session_id: Option<i64>) -> Platform {
    if let Some(id) = saved_session_id {
        if let Ok(Some(p)) = state.db.get_session_platform(id) {
            return platform_from_opt(Some(p.as_str()));
        }
    }
    let default = state
        .db
        .get_setting("hints.default_platform")
        .ok()
        .flatten();
    platform_from_opt(default.as_deref())
}

#[tauri::command]
pub async fn get_hints(
    state: State<'_, AppState>,
    saved_session_id: Option<i64>,
    prefix: String,
    limit: Option<usize>,
) -> Result<Vec<Hint>, AppError> {
    let platform = resolve_platform(&state, saved_session_id);
    let engine = HintEngine::new(&state.db);
    engine
        .suggest(saved_session_id, platform, &prefix, limit.unwrap_or(5))
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn record_command(
    state: State<'_, AppState>,
    saved_session_id: Option<i64>,
    command: String,
) -> Result<(), AppError> {
    let engine = HintEngine::new(&state.db);
    engine
        .record(saved_session_id, &command)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn set_session_platform(
    state: State<'_, AppState>,
    saved_session_id: i64,
    platform: String,
) -> Result<(), AppError> {
    Platform::from_str(&platform).map_err(|e| AppError::Internal(e.to_string()))?;
    state
        .db
        .set_session_platform(saved_session_id, &platform)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn get_session_platform(
    state: State<'_, AppState>,
    saved_session_id: i64,
) -> Result<Option<String>, AppError> {
    state
        .db
        .get_session_platform(saved_session_id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn list_platforms() -> Result<Vec<PlatformInfo>, AppError> {
    Ok(core_hints::catalog::all_platforms()
        .iter()
        .map(|p| PlatformInfo {
            id: p.as_str().to_string(),
            name: p.display_name().to_string(),
        })
        .collect())
}

#[tauri::command]
pub async fn clear_command_history(
    state: State<'_, AppState>,
    saved_session_id: Option<i64>,
) -> Result<(), AppError> {
    state
        .db
        .clear_history(saved_session_id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlatformInfo {
    pub id: String,
    pub name: String,
}
