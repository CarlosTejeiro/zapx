mod commands;
mod error;
mod events;
mod login_script;
mod menu;
mod state;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("zapx.db");
            let log_dir = data_dir.join("session_logs");
            std::fs::create_dir_all(&log_dir)?;
            let db = core_persistence::Database::open(&db_path)
                .map_err(|e| format!("DB init failed: {e}"))?;

            let rules = db.list_highlight_rules().unwrap_or_default();
            let highlighter = Arc::new(RwLock::new(core_highlight::Highlighter::new(
                rules
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
                    .collect(),
            )));

            app.manage(AppState {
                sessions: Mutex::new(HashMap::new()),
                db,
                highlighter,
                loggers: Arc::new(Mutex::new(HashMap::new())),
                log_dir,
                ki_pending: Arc::new(Mutex::new(HashMap::new())),
                forwards: Mutex::new(HashMap::new()),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sessions::open_local_session,
            commands::sessions::open_ssh_session,
            commands::sessions::ssh_preflight_host_key,
            commands::sessions::ssh_trust_host_key,
            commands::sessions::respond_keyboard_interactive,
            commands::forwards::add_local_forward,
            commands::forwards::add_dynamic_forward,
            commands::forwards::list_forwards,
            commands::forwards::remove_forward,
            commands::sftp::sftp_list_dir,
            commands::sftp::sftp_stat,
            commands::sftp::sftp_canonicalize,
            commands::sftp::sftp_mkdir,
            commands::sftp::sftp_remove_dir,
            commands::sftp::sftp_remove_file,
            commands::sftp::sftp_rename,
            commands::sftp::sftp_download_file,
            commands::sftp::sftp_upload_file,
            commands::snippets::list_snippets,
            commands::snippets::create_snippet,
            commands::snippets::update_snippet,
            commands::snippets::delete_snippet,
            commands::login_scripts::get_login_script,
            commands::login_scripts::set_login_script,
            commands::sessions::send_input,
            commands::sessions::resize_terminal,
            commands::sessions::close_session,
            commands::sessions::create_saved_session,
            commands::sessions::update_saved_session,
            commands::sessions::move_saved_session,
            commands::sessions::list_sessions,
            commands::sessions::delete_saved_session,
            commands::sessions::open_saved_session,
            commands::sessions::open_telnet_session,
            commands::sessions::open_serial_session,
            commands::sessions::list_serial_ports,
            commands::sessions::create_telnet_session,
            commands::sessions::create_serial_session,
            commands::folders::list_folders,
            commands::folders::create_folder,
            commands::folders::rename_folder,
            commands::folders::delete_folder,
            commands::highlight::list_highlight_rules,
            commands::highlight::create_highlight_rule,
            commands::highlight::toggle_highlight_rule,
            commands::highlight::delete_highlight_rule,
            commands::logging::start_session_logging,
            commands::logging::stop_session_logging,
            commands::logging::list_session_logs,
            commands::logging::list_all_session_logs,
            commands::settings::get_settings,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::list_color_schemes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running zapx");
}
