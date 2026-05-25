use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

/// Map of in-flight keyboard-interactive interactions, keyed by UUID. The
/// transport-layer responder inserts a sender here and awaits the matching
/// receiver; the `respond_keyboard_interactive` Tauri command fires it.
pub type KiPending = Arc<Mutex<HashMap<String, oneshot::Sender<Vec<String>>>>>;

use std::sync::atomic::AtomicU64;

use core_transport::{ForwardController, SessionCmd};

/// A live terminal session (local PTY or SSH).
pub struct ActiveSession {
    /// Send keyboard input or resize events to the session's I/O task.
    pub cmd_tx: UnboundedSender<SessionCmd>,
    pub cols: u16,
    pub rows: u16,
    /// Shared SSH session handle. `Some` for SSH sessions (used by port-forwards
    /// to open `direct-tcpip` channels); `None` for local PTY / Telnet / Serial.
    pub ssh_handle: Option<Arc<core_transport::SshHandle>>,
    /// Lazy-initialised SFTP subsystem, opened on first use by an `sftp_*`
    /// Tauri command. `None` outer = no SFTP yet attempted; `Some(None)` = slot
    /// exists for an SSH session; `Some(Some(_))` = SFTP active.
    pub sftp: Option<core_transport::SftpSlot>,
    /// Cumulative bytes received from the remote since session start. The
    /// stats emitter task reads this once per second to compute the rolling
    /// throughput rate without holding the sessions lock.
    pub rx_total: Arc<AtomicU64>,
    /// Handle for the periodic stats task — aborted on session close.
    pub stats_task: Option<tokio::task::JoinHandle<()>>,
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
    /// In-flight keyboard-interactive prompts awaiting frontend responses,
    /// keyed by interaction id (UUID). The Tauri command `respond_keyboard_interactive`
    /// removes the sender for a given id and fires it with the user's answers.
    pub ki_pending: KiPending,
    /// Active port-forwards keyed by SSH session UUID. Dropping a
    /// [`ForwardController`] stops its listener task.
    pub forwards: Mutex<HashMap<String, Vec<ForwardController>>>,
}
