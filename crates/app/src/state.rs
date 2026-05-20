use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::mpsc::UnboundedSender;

use core_transport::SessionCmd;

/// A live terminal session (local PTY or SSH).
///
/// The underlying I/O loop is owned by a background task; this struct holds
/// only what the Tauri command handlers need to interact with it.
pub struct ActiveSession {
    /// Send keyboard input or resize events to the session's I/O task.
    pub cmd_tx: UnboundedSender<SessionCmd>,
    pub cols: u16,
    pub rows: u16,
}

/// Shared application state injected into Tauri commands via [`tauri::State`].
#[derive(Default)]
pub struct AppState {
    pub sessions: Mutex<HashMap<String, ActiveSession>>,
}
