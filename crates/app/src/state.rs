use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// A live terminal session.
///
/// The PTY is owned by the reader task; this struct holds only what the
/// Tauri command handlers need to interact with it.
pub struct ActiveSession {
    /// Write keyboard input to the PTY.
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// Send resize requests to the reader task.
    pub resize_tx: std::sync::mpsc::Sender<(u16, u16)>,
    pub cols: u16,
    pub rows: u16,
}

/// Shared application state injected into Tauri commands via [`tauri::State`].
#[derive(Default)]
pub struct AppState {
    pub sessions: Mutex<HashMap<String, ActiveSession>>,
}
