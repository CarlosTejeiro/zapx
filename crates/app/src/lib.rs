mod commands;
mod error;
mod events;
mod menu;
mod state;

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::Manager;

pub use error::AppError;
pub use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("app=debug".parse().expect("valid directive")),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("zapx.db");
            let db = core_persistence::Database::open(&db_path)
                .map_err(|e| format!("DB init failed: {e}"))?;

            app.manage(AppState {
                sessions: Mutex::new(HashMap::new()),
                db,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sessions::open_local_session,
            commands::sessions::open_ssh_session,
            commands::sessions::send_input,
            commands::sessions::resize_terminal,
            commands::sessions::close_session,
            commands::sessions::create_saved_session,
            commands::sessions::list_sessions,
            commands::sessions::delete_saved_session,
            commands::sessions::open_saved_session,
            commands::folders::list_folders,
            commands::folders::create_folder,
            commands::folders::rename_folder,
            commands::folders::delete_folder,
            commands::settings::get_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running zapx");
}
