use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::mpsc::UnboundedSender;

use core_transport::SessionCmd;

/// A live terminal session (local PTY or SSH).
pub struct ActiveSession {
    /// Send keyboard input or resize events to the session's I/O task.
    pub cmd_tx: UnboundedSender<SessionCmd>,
    pub cols: u16,
    pub rows: u16,
}

/// Shared application state injected into Tauri commands via [`tauri::State`].
pub struct AppState {
    /// Live terminal sessions keyed by UUID.
    pub sessions: Mutex<HashMap<String, ActiveSession>>,
    /// SQLite database (folders, saved sessions, credential references).
    pub db: core_persistence::Database,
    /// Keyword highlighter — rebuilt whenever rules change.
    pub highlighter: Arc<RwLock<core_highlight::Highlighter>>,
}
