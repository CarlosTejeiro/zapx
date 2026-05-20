use std::io::{Read, Write as _};
use std::sync::mpsc;

use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use core_persistence::SavedSession;
use core_transport::{SerialTransport, SessionCmd, TelnetTransport};

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

// ---------------------------------------------------------------------------
// Saved session CRUD (persisted in SQLite via core-persistence)
// ---------------------------------------------------------------------------

/// Save a new SSH session (stores password in keyring, never in SQLite).
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_saved_session(
    state: State<'_, AppState>,
    name: String,
    folder_id: Option<i64>,
    host: String,
    port: u16,
    username: String,
    password: String,
) -> Result<i64, AppError> {
    // Generate a unique, opaque keyring key — never derived from user data.
    let keyring_key = format!("ssh:{}", Uuid::new_v4());

    core_vault::Vault::store(&keyring_key, &password)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let cred_id = state
        .db
        .create_credential(&name, "ssh_password", Some(&username), &keyring_key)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    state
        .db
        .create_session(
            folder_id,
            &name,
            "ssh",
            Some(&host),
            Some(port),
            Some(&username),
            Some(cred_id),
            "{}",
        )
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// List all saved sessions.
#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SavedSession>, AppError> {
    state
        .db
        .list_sessions()
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Delete a saved session (and its credential from the keyring).
#[tauri::command]
pub async fn delete_saved_session(state: State<'_, AppState>, id: i64) -> Result<(), AppError> {
    let session = state
        .db
        .get_session(id)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(cred_id) = session.credential_id {
        if let Ok(key) = state.db.get_credential_keyring_key(cred_id) {
            core_vault::Vault::delete(&key).ok();
            state.db.delete_credential(cred_id).ok();
        }
    }

    state
        .db
        .delete_session(id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Open a saved session: load config from DB, retrieve password from keyring,
/// establish the connection, and return the live session UUID.
#[tauri::command]
pub async fn open_saved_session(
    app: AppHandle,
    state: State<'_, AppState>,
    saved_session_id: i64,
    cols: u16,
    rows: u16,
) -> Result<String, AppError> {
    let session = state
        .db
        .get_session(saved_session_id)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    match session.protocol.as_str() {
        "ssh" => {
            let host = session
                .host
                .ok_or_else(|| AppError::Internal("missing host".into()))?;
            let port = session.port.unwrap_or(22);
            let username = session
                .username
                .ok_or_else(|| AppError::Internal("missing username".into()))?;

            let password = {
                let cred_id = session
                    .credential_id
                    .ok_or_else(|| AppError::Internal("missing credential".into()))?;
                let key = state
                    .db
                    .get_credential_keyring_key(cred_id)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                core_vault::Vault::retrieve(&key).map_err(|e| AppError::Internal(e.to_string()))?
            };

            let session_id = Uuid::new_v4().to_string();
            let transport = core_transport::SshTransport::open_shell(
                host, port, username, password, cols, rows,
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

            state.db.touch_session(saved_session_id).ok();
            tracing::debug!(session_id, saved_session_id, "saved session opened");
            Ok(session_id)
        }
        "telnet" => {
            let host = session
                .host
                .ok_or_else(|| AppError::Internal("missing host".into()))?;
            let port = session.port.unwrap_or(23);

            let session_id = Uuid::new_v4().to_string();
            let transport = TelnetTransport::connect(host.clone(), port)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let sid = session_id.clone();
            let app_handle = app.clone();
            let cmd_tx = transport.start_io_loop(cols, rows, move |data| {
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

            state.db.touch_session(saved_session_id).ok();
            tracing::debug!(
                session_id,
                saved_session_id,
                host,
                port,
                "telnet session opened"
            );
            Ok(session_id)
        }
        "serial" => {
            let opts: serde_json::Value =
                serde_json::from_str(&session.options_json).unwrap_or(serde_json::json!({}));
            let device = opts["device"]
                .as_str()
                .ok_or_else(|| AppError::Internal("missing serial device".into()))?
                .to_string();
            let baud_rate = opts["baud_rate"].as_u64().unwrap_or(9600) as u32;

            let session_id = Uuid::new_v4().to_string();
            let transport = tokio::task::spawn_blocking({
                let device = device.clone();
                move || SerialTransport::open(&device, baud_rate)
            })
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
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

            state.db.touch_session(saved_session_id).ok();
            tracing::debug!(
                session_id,
                saved_session_id,
                device,
                baud_rate,
                "serial session opened"
            );
            Ok(session_id)
        }
        p => Err(AppError::Internal(format!(
            "protocol '{p}' not yet supported"
        ))),
    }
}

/// Open a Telnet session directly (without saving).
#[tauri::command]
pub async fn open_telnet_session(
    app: AppHandle,
    state: State<'_, AppState>,
    host: String,
    port: u16,
    cols: u16,
    rows: u16,
) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();
    let transport = TelnetTransport::connect(host.clone(), port)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let sid = session_id.clone();
    let app_handle = app.clone();
    let cmd_tx = transport.start_io_loop(cols, rows, move |data| {
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

    tracing::debug!(session_id, host, port, "telnet session opened");
    Ok(session_id)
}

/// Open a Serial session directly (without saving).
#[tauri::command]
pub async fn open_serial_session(
    app: AppHandle,
    state: State<'_, AppState>,
    device: String,
    baud_rate: u32,
) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();
    let transport = tokio::task::spawn_blocking({
        let device = device.clone();
        move || SerialTransport::open(&device, baud_rate)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
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

    // Serial sessions get a default size; resize is ignored by the transport.
    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        ActiveSession {
            cmd_tx,
            cols: 80,
            rows: 24,
        },
    );

    tracing::debug!(session_id, device, baud_rate, "serial session opened");
    Ok(session_id)
}

/// List available serial ports on this machine.
#[tauri::command]
pub async fn list_serial_ports() -> Result<Vec<String>, AppError> {
    Ok(core_transport::list_serial_ports())
}

/// Save a new Telnet session.
#[tauri::command]
pub async fn create_telnet_session(
    state: State<'_, AppState>,
    name: String,
    folder_id: Option<i64>,
    host: String,
    port: u16,
) -> Result<i64, AppError> {
    state
        .db
        .create_session(
            folder_id,
            &name,
            "telnet",
            Some(&host),
            Some(port),
            None,
            None,
            "{}",
        )
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Save a new Serial session (device and baud stored in options_json).
#[tauri::command]
pub async fn create_serial_session(
    state: State<'_, AppState>,
    name: String,
    folder_id: Option<i64>,
    device: String,
    baud_rate: u32,
) -> Result<i64, AppError> {
    let opts = serde_json::json!({ "device": device, "baud_rate": baud_rate }).to_string();
    state
        .db
        .create_session(folder_id, &name, "serial", None, None, None, None, &opts)
        .map_err(|e| AppError::Internal(e.to_string()))
}

#[derive(serde::Serialize, Clone)]
struct TerminalDataPayload {
    session_id: String,
    data: Vec<u8>,
}
