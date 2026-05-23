//! Snippet CRUD commands. Snippets are short named text blocks the frontend
//! can dispatch to the focused session via the existing `send_input` command.

use tauri::State;

use core_persistence::Snippet;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn list_snippets(state: State<'_, AppState>) -> Result<Vec<Snippet>, AppError> {
    state
        .db
        .list_snippets()
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn create_snippet(
    state: State<'_, AppState>,
    name: String,
    content: String,
) -> Result<i64, AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("snippet name is required".into()));
    }
    state
        .db
        .create_snippet(name.trim(), &content)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn update_snippet(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    content: String,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("snippet name is required".into()));
    }
    state
        .db
        .update_snippet(id, name.trim(), &content)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn delete_snippet(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    state
        .db
        .delete_snippet(id)
        .map_err(|e| AppError::Internal(e.to_string()))
}
