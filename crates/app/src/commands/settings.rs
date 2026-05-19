/// Retrieve global application settings.
#[tauri::command]
pub async fn get_settings() -> serde_json::Value {
    serde_json::json!({})
}
