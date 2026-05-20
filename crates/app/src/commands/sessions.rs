use std::io::Read;
use std::sync::mpsc;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::{ActiveSession, AppState};

/// Spawn a local PTY session and return its UUID.
///
/// The PTY reader runs in a background thread that emits `terminal-data` events
/// to the frontend. The session is cleaned up when [`close_session`] is called
/// or when the shell process exits naturally.
#[tauri::command]
pub async fn open_local_session(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();

    let pty =
        core_transport::LocalPty::spawn(80, 24).map_err(|e| AppError::Internal(e.to_string()))?;

    let reader = pty
        .take_reader()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let writer = pty.writer();
    let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>();

    let sid = session_id.clone();
    let app_handle = app.clone();

    // The reader loop runs on a blocking thread (PTY reads are synchronous).
    // It owns the PTY for the duration of the session.
    tokio::task::spawn_blocking(move || {
        reader_loop(reader, pty, resize_rx, sid, app_handle);
    });

    let session = ActiveSession {
        writer,
        resize_tx,
        cols: 80,
        rows: 24,
    };

    state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id.clone(), session);

    tracing::debug!(session_id, "local session opened");
    Ok(session_id)
}

/// Write keyboard input to the PTY.
#[tauri::command]
pub async fn send_input(
    state: State<'_, AppState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), AppError> {
    let writer = {
        let sessions = state.sessions.lock().unwrap();
        let session = sessions
            .get(&session_id)
            .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;
        std::sync::Arc::clone(&session.writer)
    };

    use std::io::Write as _;
    let result = writer
        .lock()
        .unwrap()
        .write_all(&data)
        .map_err(|e| AppError::Internal(e.to_string()));
    result
}

/// Notify the PTY of a terminal resize.
#[tauri::command]
pub async fn resize_terminal(
    state: State<'_, AppState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), AppError> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;

    session
        .resize_tx
        .send((cols, rows))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    session.cols = cols;
    session.rows = rows;
    Ok(())
}

/// Terminate a session and clean up its resources.
///
/// Dropping the writer closes the PTY's stdin, which causes the shell to
/// receive EOF and exit. The reader task will finish when it sees the EOF.
#[tauri::command]
pub async fn close_session(state: State<'_, AppState>, session_id: String) -> Result<(), AppError> {
    let removed = state.sessions.lock().unwrap().remove(&session_id);
    if removed.is_some() {
        tracing::debug!(session_id, "session closed");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal — PTY reader loop running on a blocking thread
// ---------------------------------------------------------------------------

fn reader_loop(
    mut reader: Box<dyn Read + Send>,
    mut pty: core_transport::LocalPty,
    resize_rx: mpsc::Receiver<(u16, u16)>,
    session_id: String,
    app: AppHandle,
) {
    let mut buf = vec![0u8; 4096];

    loop {
        // Apply any pending resize requests before blocking on read.
        while let Ok((cols, rows)) = resize_rx.try_recv() {
            if let Err(e) = pty.resize(cols, rows) {
                tracing::warn!(session_id, "resize failed: {e}");
            }
        }

        if pty.is_done() {
            break;
        }

        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let payload = TerminalDataPayload {
                    session_id: session_id.clone(),
                    data: buf[..n].to_vec(),
                };
                if app.emit("terminal-data", payload).is_err() {
                    // App window was closed; stop the loop.
                    break;
                }
            }
            Err(e) => {
                tracing::debug!(session_id, "PTY read error: {e}");
                break;
            }
        }
    }

    pty.kill();
    tracing::debug!(session_id, "reader loop exited");
}

#[derive(serde::Serialize, Clone)]
struct TerminalDataPayload {
    session_id: String,
    data: Vec<u8>,
}
