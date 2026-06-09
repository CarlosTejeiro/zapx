//! Port-forwarding Tauri commands. Each command operates on a live SSH
//! session (looked up by its in-memory session UUID); the `ssh_handle` field
//! of [`ActiveSession`] is the [`core_transport::SharedSshHandle`] reused for
//! `direct-tcpip` (`-L` / `-D`) and `tcpip-forward` (`-R`).

use tauri::State;

use core_transport::{ForwardInfo, RemoteForwardRegistry, SharedSshHandle};

use crate::error::AppError;
use crate::state::AppState;

/// Look up the [`SharedSshHandle`] for a live SSH session, or return a clear
/// error.
fn session_handle(state: &AppState, session_id: &str) -> Result<SharedSshHandle, AppError> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(session_id)
        .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;
    session
        .ssh_handle
        .clone()
        .ok_or_else(|| AppError::Internal("session is not SSH".into()))
}

/// Look up both the SSH handle and the remote-forward registry. Used by
/// [`add_remote_forward`] which needs to register a `(bind, target)` mapping
/// before issuing `tcpip-forward` so the handler can route incoming channels.
fn session_handle_and_registry(
    state: &AppState,
    session_id: &str,
) -> Result<(SharedSshHandle, RemoteForwardRegistry), AppError> {
    let sessions = state.sessions.lock().unwrap();
    let session = sessions
        .get(session_id)
        .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))?;
    let handle = session
        .ssh_handle
        .clone()
        .ok_or_else(|| AppError::Internal("session is not SSH".into()))?;
    let registry = session
        .remote_forwards
        .clone()
        .ok_or_else(|| AppError::Internal("session is not SSH".into()))?;
    Ok((handle, registry))
}

/// Bind a local TCP port and tunnel each connection to `(target_host,
/// target_port)` via `direct-tcpip` on the SSH session. Equivalent to
/// `ssh -L <bind_port>:<target_host>:<target_port>`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_local_forward(
    state: State<'_, AppState>,
    session_id: String,
    bind_addr: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<ForwardInfo, AppError> {
    let handle = session_handle(&state, &session_id)?;
    let controller =
        core_transport::open_local_forward(handle, bind_addr, bind_port, target_host, target_port)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    let info = controller.info.clone();
    state
        .forwards
        .lock()
        .unwrap()
        .entry(session_id)
        .or_default()
        .push(controller);
    Ok(info)
}

/// Bind a local TCP port as a SOCKS5 server; each accepted client's CONNECT
/// request determines the target. Equivalent to `ssh -D <bind_port>`.
#[tauri::command]
pub async fn add_dynamic_forward(
    state: State<'_, AppState>,
    session_id: String,
    bind_addr: String,
    bind_port: u16,
) -> Result<ForwardInfo, AppError> {
    let handle = session_handle(&state, &session_id)?;
    let controller = core_transport::open_dynamic_forward(handle, bind_addr, bind_port)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let info = controller.info.clone();
    state
        .forwards
        .lock()
        .unwrap()
        .entry(session_id)
        .or_default()
        .push(controller);
    Ok(info)
}

/// Ask the SSH server to listen on `(bind_addr, bind_port)` and tunnel every
/// inbound connection back to us; we bridge each to `(target_host,
/// target_port)` on this machine. Equivalent to
/// `ssh -R <bind_addr>:<bind_port>:<target_host>:<target_port>`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn add_remote_forward(
    state: State<'_, AppState>,
    session_id: String,
    bind_addr: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<ForwardInfo, AppError> {
    let (handle, registry) = session_handle_and_registry(&state, &session_id)?;
    let controller = core_transport::open_remote_forward(
        handle,
        registry,
        bind_addr,
        bind_port,
        target_host,
        target_port,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;
    let info = controller.info.clone();
    state
        .forwards
        .lock()
        .unwrap()
        .entry(session_id)
        .or_default()
        .push(controller);
    Ok(info)
}

/// Return the metadata for every active forward attached to `session_id`.
#[tauri::command]
pub async fn list_forwards(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<ForwardInfo>, AppError> {
    let map = state.forwards.lock().unwrap();
    Ok(map
        .get(&session_id)
        .map(|v| v.iter().map(|c| c.info.clone()).collect())
        .unwrap_or_default())
}

/// Stop a single forward (by id) attached to `session_id`. Dropping the
/// controller aborts the listener task.
#[tauri::command]
pub async fn remove_forward(
    state: State<'_, AppState>,
    session_id: String,
    forward_id: String,
) -> Result<(), AppError> {
    let mut map = state.forwards.lock().unwrap();
    if let Some(vec) = map.get_mut(&session_id) {
        vec.retain(|c| c.info.id != forward_id);
    }
    Ok(())
}
