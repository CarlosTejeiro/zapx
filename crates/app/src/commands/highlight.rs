use tauri::State;

use core_persistence::HighlightRule;

use crate::error::AppError;
use crate::state::AppState;

fn rebuild_highlighter(state: &AppState) {
    let rules = state.db.list_highlight_rules().unwrap_or_default();
    let converted: Vec<core_highlight::HighlightRule> = rules
        .into_iter()
        .map(|r| core_highlight::HighlightRule {
            id: r.id,
            name: r.name,
            pattern: r.pattern,
            is_regex: r.is_regex,
            fg_color: r.fg_color,
            bg_color: r.bg_color,
            bold: r.bold,
            underline: r.underline,
            enabled: r.enabled,
            sort_order: r.sort_order,
        })
        .collect();
    if let Ok(mut h) = state.highlighter.write() {
        *h = core_highlight::Highlighter::new(converted);
    }
}

#[tauri::command]
pub async fn list_highlight_rules(
    state: State<'_, AppState>,
) -> Result<Vec<HighlightRule>, AppError> {
    state
        .db
        .list_highlight_rules()
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_highlight_rule(
    state: State<'_, AppState>,
    name: String,
    pattern: String,
    is_regex: bool,
    fg_color: Option<String>,
    bg_color: Option<String>,
    bold: bool,
    underline: bool,
) -> Result<i64, AppError> {
    let id = state
        .db
        .create_highlight_rule(
            &name,
            &pattern,
            is_regex,
            fg_color.as_deref(),
            bg_color.as_deref(),
            bold,
            underline,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;
    rebuild_highlighter(&state);
    Ok(id)
}

#[tauri::command]
pub async fn toggle_highlight_rule(
    state: State<'_, AppState>,
    id: i64,
    enabled: bool,
) -> Result<(), AppError> {
    state
        .db
        .toggle_highlight_rule(id, enabled)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    rebuild_highlighter(&state);
    Ok(())
}

#[tauri::command]
pub async fn delete_highlight_rule(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    state
        .db
        .delete_highlight_rule(id)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    rebuild_highlighter(&state);
    Ok(())
}
