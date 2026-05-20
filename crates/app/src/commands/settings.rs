use std::collections::HashMap;

use tauri::State;

use core_persistence::ColorScheme;

use crate::error::AppError;
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<HashMap<String, String>, AppError> {
    state
        .db
        .get_all_settings()
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn get_setting(
    state: State<'_, AppState>,
    key: String,
) -> Result<Option<String>, AppError> {
    state
        .db
        .get_setting(&key)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), AppError> {
    state
        .db
        .set_setting(&key, &value)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
pub async fn list_color_schemes(state: State<'_, AppState>) -> Result<Vec<ColorScheme>, AppError> {
    state
        .db
        .list_color_schemes()
        .map_err(|e| AppError::Internal(e.to_string()))
}
