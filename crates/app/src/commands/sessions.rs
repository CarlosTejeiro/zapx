/// List all saved sessions.
///
/// Returns an empty list until `core-persistence` is wired up in Bloque 3.
#[tauri::command]
pub async fn list_sessions() -> Vec<String> {
    vec![]
}
