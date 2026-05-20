use std::collections::HashMap;
use std::path::PathBuf;
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

/// A running session log: the logger plus the DB row id for later finalisation.
pub struct ActiveLog {
    pub logger: core_logging::SessionLogger,
    pub log_db_id: i64,
}

/// Shared application state injected into Tauri commands via [`tauri::State`].
pub struct AppState {
    /// Live terminal sessions keyed by UUID.
    pub sessions: Mutex<HashMap<String, ActiveSession>>,
    /// SQLite database (folders, saved sessions, credential references).
    pub db: core_persistence::Database,
    /// Keyword highlighter — rebuilt whenever rules change.
    pub highlighter: Arc<RwLock<core_highlight::Highlighter>>,
    /// Active session loggers keyed by session UUID.
    /// Wrapped in Arc so callbacks can hold a reference without borrowing AppState.
    pub loggers: Arc<Mutex<HashMap<String, ActiveLog>>>,
    /// Directory where session log files are written.
    pub log_dir: PathBuf,
}
