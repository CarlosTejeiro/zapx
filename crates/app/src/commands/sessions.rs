use std::io::{Read, Write as _};
use std::sync::mpsc;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use core_transport::SessionCmd;

use crate::error::AppError;
use crate::state::{ActiveSession, AppState};

// ---------------------------------------------------------------------------
// Local PTY session
// ---------------------------------------------------------------------------

/// Spawn a local PTY session and return its UUID.
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

    let pty_writer = pty.writer();
    let (resize_tx, resize_rx) = mpsc::channel::<(u16, u16)>();

    // Bridge: tokio UnboundedReceiver → sync PTY writer / resize channel.
    // Small PTY writes (keystrokes) are fast enough to do synchronously here.
    let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<SessionCmd>();
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                SessionCmd::Data(data) => {
                    if let Ok(mut w) = pty_writer.lock() {
                        let _ = w.write_all(&data);
                    }
                }
                SessionCmd::Resize { cols, rows } => {
                    let _ = resize_tx.send((cols, rows));
                }
            }
        }
    });

    // Reader loop runs on a blocking thread (PTY reads are synchronous).
    let sid = session_id.clone();
    let app_handle = app.clone();
    tokio::task::spawn_blocking(move || {
        reader_loop(reader, pty, resize_rx, sid, app_handle);
    });

    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        ActiveSession {
            cmd_tx,
            cols: 80,
            rows: 24,
        },
    );

    tracing::debug!(session_id, "local session opened");
    Ok(session_id)
}

// ---------------------------------------------------------------------------
// SSH session
// ---------------------------------------------------------------------------

/// Connect to an SSH host, open a PTY shell, and return the session UUID.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn open_ssh_session(
    app: AppHandle,
    state: State<'_, AppState>,
    host: String,
    port: u16,
    user: String,
    password: String,
    cols: u16,
    rows: u16,
) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();

    let transport = core_transport::SshTransport::open_shell(
        host.clone(),
        port,
        user.clone(),
        password,
        cols,
        rows,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let sid = session_id.clone();
    let app_handle = app.clone();
    let cmd_tx = transport.start_io_loop(move |data| {
        let _ = app_handle.emit(
            "terminal-data",
            TerminalDataPayload {
                session_id: sid.clone(),
                data,
            },
        );
    });

    state
        .sessions
        .lock()
        .unwrap()
        .insert(session_id.clone(), ActiveSession { cmd_tx, cols, rows });

    tracing::debug!(session_id, host, port, user, "ssh session opened");
    Ok(session_id)
}

// ---------------------------------------------------------------------------
// Shared session commands (work for both local and SSH)
// ---------------------------------------------------------------------------

/// Write keyboard input to the session.
#[tauri::command]
pub async fn send_input(
    state: State<'_, AppState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), AppError> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;
    session.cmd_tx.send(SessionCmd::Data(data)).ok();
    Ok(())
}

/// Notify the session of a terminal resize.
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
    session.cmd_tx.send(SessionCmd::Resize { cols, rows }).ok();
    session.cols = cols;
    session.rows = rows;
    Ok(())
}

/// Terminate a session and clean up its resources.
///
/// Dropping `cmd_tx` signals the I/O task to exit cleanly.
#[tauri::command]
pub async fn close_session(state: State<'_, AppState>, session_id: String) -> Result<(), AppError> {
    let removed = state.sessions.lock().unwrap().remove(&session_id);
    if removed.is_some() {
        tracing::debug!(session_id, "session closed");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal — PTY reader loop (blocking thread)
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
