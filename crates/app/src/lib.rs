mod commands;
mod error;
mod events;
mod menu;
mod state;

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
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::sessions::open_local_session,
            commands::sessions::send_input,
            commands::sessions::resize_terminal,
            commands::sessions::close_session,
            commands::folders::list_folders,
            commands::settings::get_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running zapx");
}
