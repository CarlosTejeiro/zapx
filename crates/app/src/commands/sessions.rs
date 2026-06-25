use std::collections::HashMap;
use std::io::{Read, Write as _};
use std::sync::{mpsc, Arc, Mutex, RwLock};

use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use core_persistence::SavedSession;
use core_transport::{SerialTransport, SessionCmd, TelnetTransport};

use crate::error::AppError;
use crate::login_script::{LoginRunner, LoginStep};
use crate::state::{ActiveLog, ActiveSession, AppState, SharedPlatformDetector};

use core_hints::detector::PlatformDetector;
use core_hints::Platform;

/// Shared, late-filled slot so `make_on_data` can route bytes to a
/// [`LoginRunner`] that is only constructed AFTER `start_io_loop` yields its
/// command sender. For sessions without a script we use an always-empty slot.
type LoginRunnerSlot = Arc<Mutex<Option<Arc<LoginRunner>>>>;

fn empty_login_slot() -> LoginRunnerSlot {
    Arc::new(Mutex::new(None))
}

type TriggerRunnerSlot = Arc<Mutex<Option<Arc<crate::triggers::TriggerRunner>>>>;

fn empty_trigger_slot() -> TriggerRunnerSlot {
    Arc::new(Mutex::new(None))
}

/// Frontend payload for `session-platform-detected`.
#[derive(serde::Serialize, Clone)]
struct SessionPlatformDetected {
    session_id: String,
    platform: String,
    display_name: String,
}

/// Build the callback the platform detector fires the first time it latches.
/// Persists the platform onto the saved-session row IF (a) the session was
/// saved and (b) it doesn't already have an explicit platform — we never
/// override a user choice. Always emits `session-platform-detected` so the
/// frontend can react even for ephemeral sessions.
fn make_detection_callback(
    app: AppHandle,
    session_id: String,
    saved_session_id: Option<i64>,
) -> Arc<dyn Fn(Platform) + Send + Sync + 'static> {
    Arc::new(move |platform: Platform| {
        if let Some(id) = saved_session_id {
            let state = app.state::<AppState>();
            let current = state.db.get_session_platform(id).ok().flatten();
            let should_write = matches!(current.as_deref(), None | Some("") | Some("generic"));
            if should_write {
                if let Err(e) = state.db.set_session_platform(id, platform.as_str()) {
                    tracing::warn!(
                        saved_session_id = id,
                        "persist detected platform failed: {e}"
                    );
                }
            }
        }
        let _ = app.emit(
            "session-platform-detected",
            SessionPlatformDetected {
                session_id: session_id.clone(),
                platform: platform.as_str().to_string(),
                display_name: platform.display_name().to_string(),
            },
        );
    })
}

/// Allocate the per-session detector. Cheap; one allocation + a small Vec.
fn new_detector() -> SharedPlatformDetector {
    Arc::new(Mutex::new(PlatformDetector::new()))
}

fn parse_login_script(json: Option<&str>) -> Option<Vec<LoginStep>> {
    let raw = json?;
    let steps: Vec<LoginStep> = serde_json::from_str(raw).ok()?;
    if steps.is_empty() {
        None
    } else {
        Some(steps)
    }
}

/// Build and store a [`LoginRunner`] into `slot` (no-op if `script_json` is
/// `None` or parses to an empty array). The runner forwards progress events
/// to the frontend as `login-script-progress`.
fn maybe_install_login_runner(
    slot: &LoginRunnerSlot,
    script_json: Option<&str>,
    session_id: &str,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<core_transport::SessionCmd>,
    app: &AppHandle,
) {
    let Some(steps) = parse_login_script(script_json) else {
        return;
    };
    let app_for_progress = app.clone();
    let runner = Arc::new(LoginRunner::new(
        session_id.to_string(),
        steps,
        cmd_tx,
        Box::new(move |p| {
            let _ = app_for_progress.emit("login-script-progress", p);
        }),
    ));
    *slot.lock().unwrap() = Some(runner);
}

fn parse_triggers(json: Option<&str>) -> Option<Vec<crate::triggers::Trigger>> {
    let raw = json?;
    let triggers: Vec<crate::triggers::Trigger> = serde_json::from_str(raw).ok()?;
    if triggers.is_empty() {
        None
    } else {
        Some(triggers)
    }
}

/// Build and store a [`TriggerRunner`](crate::triggers::TriggerRunner) into
/// `slot` (no-op if `triggers_json` is `None`/empty). `notify`/`bell` fires are
/// forwarded to the frontend as the `trigger-fired` event.
fn maybe_install_trigger_runner(
    slot: &TriggerRunnerSlot,
    triggers_json: Option<&str>,
    session_id: &str,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<core_transport::SessionCmd>,
    app: &AppHandle,
) {
    let Some(triggers) = parse_triggers(triggers_json) else {
        return;
    };
    let app_for_fire = app.clone();
    let runner = Arc::new(crate::triggers::TriggerRunner::new(
        session_id.to_string(),
        triggers,
        cmd_tx,
        Box::new(move |e| {
            let _ = app_for_fire.emit("trigger-fired", e);
        }),
    ));
    *slot.lock().unwrap() = Some(runner);
}

/// Wrap an `on_data` callback to:
/// 1. Write raw bytes to the session log (if logging is active).
/// 2. Feed the (raw, pre-highlight) bytes to the login-script runner, if any.
/// 3. Feed the bytes to the platform detector (SSH sessions only); fire the
///    `on_platform_detected` callback the first time it latches.
/// 4. Apply keyword highlighting.
/// 5. Emit highlighted bytes to the frontend.
#[allow(clippy::too_many_arguments)]
fn make_on_data<F>(
    session_id: String,
    loggers: Arc<Mutex<HashMap<String, ActiveLog>>>,
    highlighter: Arc<RwLock<core_highlight::Highlighter>>,
    login_slot: LoginRunnerSlot,
    trigger_slot: TriggerRunnerSlot,
    rx_total: Arc<std::sync::atomic::AtomicU64>,
    detector: Option<crate::state::SharedPlatformDetector>,
    on_platform_detected: Option<Arc<dyn Fn(core_hints::Platform) + Send + Sync + 'static>>,
    inner: F,
) -> impl Fn(Vec<u8>) + Send + 'static
where
    F: Fn(Vec<u8>) + Send + 'static,
{
    move |data: Vec<u8>| {
        // Count raw bytes (pre-highlight) — the StatusBar shows the wire rate.
        rx_total.fetch_add(data.len() as u64, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut map) = loggers.lock() {
            if let Some(active) = map.get_mut(&session_id) {
                let _ = active.logger.write(&data);
            }
        }
        // Feed the login runner (if any) BEFORE highlight so pattern matching
        // sees the raw bytes the server actually sent.
        let runner = login_slot.lock().unwrap().clone();
        if let Some(r) = runner {
            r.feed(&data);
        }
        // Output triggers see the same raw bytes (pre-highlight).
        let trig = trigger_slot.lock().unwrap().clone();
        if let Some(t) = trig {
            t.feed(&data);
        }
        // Platform auto-detect: only SSH sessions wire this. `feed` returns
        // `Some` ONCE, at the moment of detection, then `None` forever; we
        // pipe that single hit to the host's persistence + emit callback.
        if let Some(d) = detector.as_ref() {
            let latched = d.lock().ok().and_then(|mut d| d.feed(&data));
            if let (Some(p), Some(cb)) = (latched, on_platform_detected.as_ref()) {
                cb(p);
            }
        }
        let highlighted = highlighter.read().unwrap().apply(&data);
        inner(highlighted);
    }
}

/// Spawn a task that emits `session-stats { session_id, bytes_per_sec }`
/// every second. Returns the JoinHandle so the session close path can abort
/// it (otherwise the task would keep ticking for a dead UUID).
fn spawn_stats_emitter(
    app: AppHandle,
    session_id: String,
    rx_total: Arc<std::sync::atomic::AtomicU64>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last = 0u64;
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
        tick.tick().await; // discard the immediate first tick
        loop {
            tick.tick().await;
            let total = rx_total.load(std::sync::atomic::Ordering::Relaxed);
            let delta = total.saturating_sub(last);
            last = total;
            let _ = app.emit(
                "session-stats",
                SessionStatsPayload {
                    session_id: session_id.clone(),
                    bytes_per_sec: delta,
                },
            );
        }
    })
}

#[derive(serde::Serialize, Clone)]
struct SessionStatsPayload {
    session_id: String,
    bytes_per_sec: u64,
}

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

    // Local PTY doesn't route through make_on_data, so we don't track
    // throughput for it yet. The StatusBar will simply show 0 B/s.
    let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let stats_task: Option<tokio::task::JoinHandle<()>> = None;

    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        ActiveSession {
            cmd_tx,
            cols: 80,
            rows: 24,
            ssh_handle: None,
            remote_forwards: None,
            platform_detector: None,
            tcp_mss: core_transport::TcpMss::default(),
            sftp: None,
            rx_total,
            stats_task,
        },
    );

    tracing::debug!(session_id, "local session opened");
    Ok(session_id)
}

// ---------------------------------------------------------------------------
// SSH session
// ---------------------------------------------------------------------------

/// How the frontend specifies SSH authentication. Discriminated by `type`.
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthMethodArg {
    Password {
        password: String,
    },
    Key {
        #[serde(rename = "keyPath")]
        key_path: String,
        passphrase: Option<String>,
    },
    #[serde(rename = "keyboard-interactive")]
    KeyboardInteractive,
    Agent,
}

/// Event payload for `ssh-ki-prompt`. The frontend renders the prompts and
/// then calls `respond_keyboard_interactive` with the matching `interactionId`.
#[derive(serde::Serialize, Clone)]
struct KiPromptPayload {
    #[serde(rename = "interactionId")]
    interaction_id: String,
    name: String,
    instructions: String,
    prompts: Vec<core_transport::KiPrompt>,
}

/// Build a keyboard-interactive responder that bridges to the UI: for each
/// `InfoRequest` from the server, emit a `ssh-ki-prompt` event with a fresh
/// interaction id and await a oneshot fired by [`respond_keyboard_interactive`].
fn build_ki_responder(
    app: AppHandle,
    ki_pending: crate::state::KiPending,
) -> core_transport::KiResponder {
    Arc::new(move |req: core_transport::KiRequest| {
        let app = app.clone();
        let ki_pending = Arc::clone(&ki_pending);
        Box::pin(async move {
            let interaction_id = Uuid::new_v4().to_string();
            let (tx, rx) = tokio::sync::oneshot::channel::<Vec<String>>();
            ki_pending
                .lock()
                .unwrap()
                .insert(interaction_id.clone(), tx);
            let _ = app.emit(
                "ssh-ki-prompt",
                KiPromptPayload {
                    interaction_id: interaction_id.clone(),
                    name: req.name,
                    instructions: req.instructions,
                    prompts: req.prompts,
                },
            );
            match rx.await {
                Ok(responses) => responses,
                Err(_) => {
                    // Sender dropped (e.g. session torn down) — clean up and
                    // return an empty Vec which will fail auth cleanly.
                    ki_pending.lock().unwrap().remove(&interaction_id);
                    Vec::new()
                }
            }
        })
    })
}

/// Convert an [`AuthMethodArg`] from the frontend into a [`core_transport::SshAuth`].
fn build_ssh_auth(
    arg: AuthMethodArg,
    app: &AppHandle,
    ki_pending: &crate::state::KiPending,
    state: &AppState,
) -> core_transport::SshAuth {
    match arg {
        AuthMethodArg::Password { password } => core_transport::SshAuth::Password(password),
        AuthMethodArg::Key {
            key_path,
            passphrase,
        } => core_transport::SshAuth::PublicKey {
            key_path,
            passphrase,
        },
        AuthMethodArg::KeyboardInteractive => core_transport::SshAuth::KeyboardInteractive {
            responder: build_ki_responder(app.clone(), Arc::clone(ki_pending)),
        },
        AuthMethodArg::Agent => core_transport::SshAuth::Agent {
            priority: read_agent_priority(state),
        },
    }
}

/// Read the persisted `ssh.agent_priority` setting and resolve to an enum.
/// Falls back to `Auto` (today's behaviour) if the row is missing or invalid.
fn read_agent_priority(state: &AppState) -> core_transport::AgentPriority {
    let raw = state
        .db
        .get_setting("ssh.agent_priority")
        .ok()
        .flatten()
        .unwrap_or_default();
    core_transport::AgentPriority::from_setting(&raw)
}

/// Fire the user's answers into the oneshot for an in-flight KI prompt.
#[tauri::command]
pub async fn respond_keyboard_interactive(
    state: State<'_, AppState>,
    interaction_id: String,
    responses: Vec<String>,
) -> Result<(), AppError> {
    let sender = state.ki_pending.lock().unwrap().remove(&interaction_id);
    if let Some(tx) = sender {
        let _ = tx.send(responses);
    }
    Ok(())
}

/// Fetch the server's host key and classify it against `~/.ssh/known_hosts`.
/// Run before [`open_ssh_session`] so the UI can prompt for TOFU/mismatch.
#[tauri::command]
pub async fn ssh_preflight_host_key(
    host: String,
    port: u16,
) -> Result<core_transport::HostKeyStatus, AppError> {
    core_transport::preflight_host_key(host, port)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Persist trust for a host key (writes `~/.ssh/known_hosts`).
#[tauri::command]
pub async fn ssh_trust_host_key(host: String, port: u16) -> Result<(), AppError> {
    core_transport::trust_host_key(host, port)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Connect to an SSH host, open a PTY shell, and return the session UUID.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn open_ssh_session(
    app: AppHandle,
    state: State<'_, AppState>,
    host: String,
    port: u16,
    user: String,
    auth: AuthMethodArg,
    cols: u16,
    rows: u16,
) -> Result<String, AppError> {
    let session_id = Uuid::new_v4().to_string();

    let auth = build_ssh_auth(auth, &app, &state.ki_pending, &state);
    let mut transport = core_transport::SshTransport::open_shell(
        host.clone(),
        port,
        user.clone(),
        auth,
        cols,
        rows,
    )
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?;

    let sid = session_id.clone();
    let app_handle = app.clone();
    let hl = Arc::clone(&state.highlighter);
    let lg = Arc::clone(&state.loggers);
    let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let detector = new_detector();
    // Ephemeral SSH (open_ssh_session, no saved id) — emit only, don't persist.
    let on_detected = make_detection_callback(app.clone(), session_id.clone(), None);
    let tcp_mss = transport.tcp_mss();
    let mss_watcher = transport.take_mss_watcher();
    let (cmd_tx, ssh_handle, remote_forwards) = transport.start_io_loop(
        make_on_data(
            session_id.clone(),
            lg,
            hl,
            empty_login_slot(),
            empty_trigger_slot(),
            Arc::clone(&rx_total),
            Some(Arc::clone(&detector)),
            Some(on_detected),
            move |data| {
                let _ = app_handle.emit(
                    "terminal-data",
                    TerminalDataPayload {
                        session_id: sid.clone(),
                        data,
                    },
                );
            },
        ),
        make_on_close(app.clone(), session_id.clone()),
    );
    let stats_task = Some(spawn_stats_emitter(
        app.clone(),
        session_id.clone(),
        Arc::clone(&rx_total),
    ));
    // Live MSS polling for the same session. Dropped automatically when the
    // session is closed (the task observes the watcher's empty streak and
    // exits within a few INTERVAL ticks).
    if let Some(w) = mss_watcher {
        std::mem::drop(spawn_mss_emitter(app.clone(), session_id.clone(), w));
    }

    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        ActiveSession {
            cmd_tx,
            cols,
            rows,
            ssh_handle: Some(ssh_handle),
            remote_forwards: Some(remote_forwards),
            platform_detector: Some(detector),
            tcp_mss,
            sftp: Some(core_transport::empty_sftp_slot()),
            rx_total,
            stats_task,
        },
    );

    tracing::debug!(session_id, host, port, user, "ssh session opened");
    Ok(session_id)
}

// ---------------------------------------------------------------------------
// Shared session commands (work for both local and SSH)
// ---------------------------------------------------------------------------

/// TCP MSS snapshot for `session_id`. Empty fields are surfaced as-is
/// (`null` over the wire) — the frontend renders a "n/a" badge for
/// platforms that don't expose a given direction.
#[tauri::command]
pub async fn get_session_tcp_mss(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<core_transport::TcpMss, AppError> {
    let sessions = state.sessions.lock().unwrap();
    sessions
        .get(&session_id)
        .map(|s| s.tcp_mss)
        .ok_or_else(|| AppError::Internal(format!("unknown session: {session_id}")))
}

/// Background task: poll the per-session MSS watcher every `INTERVAL` and
/// emit a `session-mss-updated { session_id, mss }` event when the value
/// changes. Aborts itself when:
///   * the value goes back to `default()` (socket closed or peer hung up),
///   * the underlying watcher rejected the query several times in a row,
///   * the host drops the JoinHandle (session close path).
fn spawn_mss_emitter(
    app: AppHandle,
    session_id: String,
    watcher: core_transport::MssWatcher,
) -> tokio::task::JoinHandle<()> {
    use core_transport::TcpMss;
    const INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);
    const MAX_EMPTY: u32 = 3;

    tokio::spawn(async move {
        let mut last: TcpMss = watcher.query();
        let mut empty_streak: u32 = 0;
        let mut tick = tokio::time::interval(INTERVAL);
        // First tick fires immediately; we already emitted via the connect
        // snapshot, so skip one.
        tick.tick().await;

        loop {
            tick.tick().await;
            let now = watcher.query();
            let empty = now.send.is_none() && now.recv.is_none();
            if empty {
                empty_streak += 1;
                if empty_streak >= MAX_EMPTY {
                    break;
                }
                continue;
            }
            empty_streak = 0;
            if now.send != last.send || now.recv != last.recv {
                last = now;
                let _ = app.emit(
                    "session-mss-updated",
                    SessionMssPayload {
                        session_id: session_id.clone(),
                        mss: now,
                    },
                );
            }
        }
    })
}

#[derive(serde::Serialize, Clone)]
struct SessionMssPayload {
    session_id: String,
    mss: core_transport::TcpMss,
}

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
    // Drop any port-forwards bound to this session — Drop on ForwardController
    // aborts each listener task.
    state.forwards.lock().unwrap().remove(&session_id);
    if let Some(active) = removed {
        if let Some(task) = active.stats_task {
            task.abort();
        }
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

/// Save a new SSH session. Secrets (password, key passphrase) go to the OS
/// keyring; the key file path is not a secret and is stored in `options_json`.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_saved_session(
    state: State<'_, AppState>,
    name: String,
    folder_id: Option<i64>,
    host: String,
    port: u16,
    username: String,
    auth: AuthMethodArg,
    via_session_id: Option<i64>,
) -> Result<i64, AppError> {
    // Sanity-check the bastion reference now so we fail fast on create, not
    // only when opening the connection.
    if let Some(via_id) = via_session_id {
        let bastion = state
            .db
            .get_session(via_id)
            .map_err(|e| AppError::Internal(format!("via_session_id {via_id}: {e}")))?;
        if bastion.protocol != "ssh" {
            return Err(AppError::Internal(format!(
                "via_session_id {via_id} is not an SSH session"
            )));
        }
    }
    // (auth_method, credential_id, options_json, password_to_cache) derived per type.
    let (auth_method, credential_id, options_json, password_to_cache): (
        &str,
        Option<i64>,
        String,
        Option<String>,
    ) = match auth {
        AuthMethodArg::Password { password } => {
            let keyring_key = format!("ssh:{}", Uuid::new_v4());
            core_vault::Vault::store(&keyring_key, &password)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            let cred_id = state
                .db
                .create_credential(&name, "ssh_password", Some(&username), &keyring_key)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            ("password", Some(cred_id), "{}".to_string(), Some(password))
        }
        AuthMethodArg::Key {
            key_path,
            passphrase,
        } => {
            // Only the passphrase is a secret; store it in the vault if present.
            let cred_id = match passphrase {
                Some(pp) if !pp.is_empty() => {
                    let keyring_key = format!("ssh:{}", Uuid::new_v4());
                    core_vault::Vault::store(&keyring_key, &pp)
                        .map_err(|e| AppError::Internal(e.to_string()))?;
                    Some(
                        state
                            .db
                            .create_credential(&name, "ssh_key", Some(&username), &keyring_key)
                            .map_err(|e| AppError::Internal(e.to_string()))?,
                    )
                }
                _ => None,
            };
            let options = serde_json::json!({ "key_path": key_path }).to_string();
            ("key", cred_id, options, None)
        }
        AuthMethodArg::KeyboardInteractive => {
            ("keyboard-interactive", None, "{}".to_string(), None)
        }
        AuthMethodArg::Agent => ("agent", None, "{}".to_string(), None),
    };

    let id = state
        .db
        .create_session_full(
            folder_id,
            &name,
            "ssh",
            Some(&host),
            Some(port),
            Some(&username),
            credential_id,
            &options_json,
            Some(auth_method),
            via_session_id,
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Seed both caches so the first reopen works even if the dev-mode
    // Keychain denies us read access later.
    if let Some(pw) = password_to_cache {
        state.password_cache.lock().unwrap().insert(id, pw.clone());
        if let Ok(blob) = core_vault::encrypt_with_seed(&state.vault_seed, &pw) {
            let _ = state.db.set_session_secret(id, &blob);
        }
    }

    Ok(id)
}

/// List all saved sessions.
#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SavedSession>, AppError> {
    state
        .db
        .list_sessions()
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Update editable fields of a saved session. Credentials (auth method,
/// password, key passphrase) are intentionally NOT touched — change them by
/// recreating the session.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_saved_session(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    folder_id: Option<i64>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    via_session_id: Option<i64>,
    // `options_json`: protocol-specific options blob (e.g. ssh `key_path`,
    // serial `baud_rate`). Passed through to the DB when `Some`, preserved
    // otherwise — see persistence::update_session_fields.
    options_json: Option<String>,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Internal("name is required".into()));
    }
    // Cycle / self-reference check for via_session_id.
    if let Some(vid) = via_session_id {
        if vid == id {
            return Err(AppError::Internal(
                "session cannot use itself as a jump host".into(),
            ));
        }
        let bastion = state
            .db
            .get_session(vid)
            .map_err(|e| AppError::Internal(format!("via_session_id {vid}: {e}")))?;
        if bastion.protocol != "ssh" {
            return Err(AppError::Internal(format!(
                "via_session_id {vid} is not an SSH session"
            )));
        }
    }
    state
        .db
        .update_session_fields(
            id,
            name.trim(),
            folder_id,
            host.as_deref(),
            port,
            username.as_deref(),
            via_session_id,
            options_json.as_deref(),
        )
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Move a saved session to a different folder (or to the root with `None`).
#[tauri::command]
pub async fn move_saved_session(
    state: State<'_, AppState>,
    id: i64,
    folder_id: Option<i64>,
) -> Result<(), AppError> {
    state
        .db
        .set_session_folder(id, folder_id)
        .map_err(|e| AppError::Internal(e.to_string()))
}

/// Reorder `id` to position `targetIndex` inside `folderId` (`null` for root).
/// Also reparents if `folderId` differs from the row's current value, so the
/// frontend can dispatch a single command for both drop-into-folder and
/// drag-to-reorder gestures.
#[tauri::command]
pub async fn reorder_saved_session(
    state: State<'_, AppState>,
    id: i64,
    folder_id: Option<i64>,
    target_index: usize,
) -> Result<(), AppError> {
    state
        .db
        .reorder_session(id, folder_id, target_index)
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

/// Build the [`core_transport::SshAuth`] for a saved SSH session, pulling its
/// secret from the vault if applicable. Shared by direct connects and every
/// hop of a ProxyJump chain.
fn resolve_ssh_auth_for_session(
    session: &SavedSession,
    app: &AppHandle,
    state: &AppState,
) -> Result<core_transport::SshAuth, AppError> {
    // Resolve the password through a fallback chain:
    //   1. OS keyring (primary; works in signed/production builds).
    //   2. Process-lifetime in-memory cache (survives Cmd+R).
    //   3. Encrypted local store in SQLite (survives app restart — used on
    //      macOS dev builds where the unsigned binary is denied keyring read).
    let secret = match session.credential_id {
        Some(cred_id) => {
            let from_keyring = state
                .db
                .get_credential_keyring_key(cred_id)
                .ok()
                .and_then(|key| core_vault::Vault::retrieve(&key).ok());
            from_keyring
                .or_else(|| {
                    state
                        .password_cache
                        .lock()
                        .unwrap()
                        .get(&session.id)
                        .cloned()
                })
                .or_else(|| {
                    state
                        .db
                        .get_session_secret(session.id)
                        .ok()
                        .flatten()
                        .and_then(|blob| {
                            core_vault::decrypt_with_seed(&state.vault_seed, &blob).ok()
                        })
                })
        }
        None => None,
    };
    // Warm the in-memory cache if we resolved via DB (saves a decrypt+SQL
    // round-trip on subsequent opens).
    if let Some(ref pw) = secret {
        state
            .password_cache
            .lock()
            .unwrap()
            .insert(session.id, pw.clone());
    }

    Ok(match session.auth_method.as_deref() {
        Some("key") => {
            let opts: serde_json::Value =
                serde_json::from_str(&session.options_json).unwrap_or(serde_json::json!({}));
            let key_path = opts["key_path"]
                .as_str()
                .ok_or_else(|| AppError::Internal("missing key_path".into()))?
                .to_string();
            core_transport::SshAuth::PublicKey {
                key_path,
                passphrase: secret,
            }
        }
        Some("agent") => core_transport::SshAuth::Agent {
            priority: read_agent_priority(state),
        },
        Some("keyboard-interactive") => core_transport::SshAuth::KeyboardInteractive {
            responder: build_ki_responder(app.clone(), Arc::clone(&state.ki_pending)),
        },
        // None / "password" — legacy behaviour.
        _ => core_transport::SshAuth::Password(
            secret.ok_or_else(|| AppError::Internal("missing credential".into()))?,
        ),
    })
}

/// Persist the user-typed password for `saved_session_id`.
///
/// Writes to BOTH:
///  * the in-memory cache (fast, survives Cmd+R),
///  * the encrypted `session_secrets` table (survives app restart).
///
/// Called by the reconnect dialog after a successful re-auth so the user
/// stops being prompted on every relaunch when the OS keyring is unusable
/// (typical for macOS dev builds without code-signing).
#[tauri::command]
pub async fn cache_session_password(
    state: State<'_, AppState>,
    saved_session_id: i64,
    password: String,
) -> Result<(), AppError> {
    state
        .password_cache
        .lock()
        .unwrap()
        .insert(saved_session_id, password.clone());

    // Best-effort encrypted persistence — surface failures as a warning,
    // not a hard error, so a missing aes-gcm dep on some platform doesn't
    // break the reconnect flow.
    match core_vault::encrypt_with_seed(&state.vault_seed, &password) {
        Ok(blob) => {
            if let Err(e) = state.db.set_session_secret(saved_session_id, &blob) {
                tracing::warn!(saved_session_id, "persist session_secret failed: {e}");
            }
        }
        Err(e) => tracing::warn!(saved_session_id, "encrypt session_secret failed: {e}"),
    }
    Ok(())
}

/// Walk the `via_session_id` chain starting at `start_id` and return the list
/// of saved-session ids from outermost (no `via`) to innermost (just before
/// the target). Detects cycles.
fn collect_chain_ids(start_id: i64, state: &AppState) -> Result<Vec<i64>, AppError> {
    let mut chain_ids = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut current = Some(start_id);
    while let Some(id) = current {
        if !visited.insert(id) {
            return Err(AppError::Internal(format!(
                "circular via_session_id chain at session {id}"
            )));
        }
        let saved = state
            .db
            .get_session(id)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if saved.protocol != "ssh" {
            return Err(AppError::Internal(format!(
                "via_session_id {id} points to non-SSH session"
            )));
        }
        current = saved.via_session_id;
        chain_ids.push(id);
    }
    chain_ids.reverse();
    Ok(chain_ids)
}

/// Build the chain of authenticated bastion handles required to reach a
/// target whose `via_session_id` is `Some(via_start)`. The last element of
/// the returned Vec is the bastion through which the target is tunneled.
async fn build_bastion_chain(
    via_start: i64,
    app: &AppHandle,
    state: &AppState,
) -> Result<Vec<core_transport::SshHandle>, AppError> {
    let chain_ids = collect_chain_ids(via_start, state)?;
    let mut handles: Vec<core_transport::SshHandle> = Vec::with_capacity(chain_ids.len());
    for id in chain_ids {
        let saved = state
            .db
            .get_session(id)
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let host = saved
            .host
            .clone()
            .ok_or_else(|| AppError::Internal(format!("bastion {id} missing host")))?;
        let port = saved.port.unwrap_or(22);
        let user = saved
            .username
            .clone()
            .ok_or_else(|| AppError::Internal(format!("bastion {id} missing username")))?;
        let auth = resolve_ssh_auth_for_session(&saved, app, state)?;

        let handle = if let Some(prev) = handles.last() {
            core_transport::connect_authenticated_via(prev, host, port, user, auth)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
        } else {
            core_transport::connect_authenticated(host, port, user, auth)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
        };
        handles.push(handle);
    }
    Ok(handles)
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

    // Pull the (optional) login script out once so the per-protocol arms can
    // freely move fields out of `session` without holding a borrow.
    let login_script_json = session.login_script_json.clone();
    // Triggers live in a dedicated column (fetched separately, not on the
    // SavedSession struct), so grab them here too.
    let triggers_json = state
        .db
        .get_session_triggers(saved_session_id)
        .ok()
        .flatten();

    match session.protocol.as_str() {
        "ssh" => {
            let host = session
                .host
                .clone()
                .ok_or_else(|| AppError::Internal("missing host".into()))?;
            let port = session.port.unwrap_or(22);
            let username = session
                .username
                .clone()
                .ok_or_else(|| AppError::Internal("missing username".into()))?;

            let auth = resolve_ssh_auth_for_session(&session, &app, &state)?;

            let session_id = Uuid::new_v4().to_string();
            let mut transport = if let Some(via_id) = session.via_session_id {
                // Build the bastion chain, then tunnel to the target through it.
                let chain = build_bastion_chain(via_id, &app, &state).await?;
                core_transport::SshTransport::open_shell_via(
                    chain, host, port, username, auth, cols, rows,
                )
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
            } else {
                core_transport::SshTransport::open_shell(host, port, username, auth, cols, rows)
                    .await
                    .map_err(|e| AppError::Internal(e.to_string()))?
            };

            let sid = session_id.clone();
            let app_handle = app.clone();
            let hl = Arc::clone(&state.highlighter);
            let lg = Arc::clone(&state.loggers);
            let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let login_slot: LoginRunnerSlot = empty_login_slot();
            let login_slot_for_closure = Arc::clone(&login_slot);
            let trigger_slot: TriggerRunnerSlot = empty_trigger_slot();
            let trigger_slot_for_closure = Arc::clone(&trigger_slot);
            let detector = new_detector();
            // Saved SSH — also persists the detected platform if the row
            // doesn't already have a user-chosen one.
            let on_detected =
                make_detection_callback(app.clone(), session_id.clone(), Some(saved_session_id));
            let tcp_mss = transport.tcp_mss();
            let mss_watcher = transport.take_mss_watcher();
            let (cmd_tx, ssh_handle, remote_forwards) = transport.start_io_loop(
                make_on_data(
                    session_id.clone(),
                    lg,
                    hl,
                    login_slot_for_closure,
                    trigger_slot_for_closure,
                    Arc::clone(&rx_total),
                    Some(Arc::clone(&detector)),
                    Some(on_detected),
                    move |data| {
                        let _ = app_handle.emit(
                            "terminal-data",
                            TerminalDataPayload {
                                session_id: sid.clone(),
                                data,
                            },
                        );
                    },
                ),
                make_on_close(app.clone(), session_id.clone()),
            );

            // Wire up the optional login automation (no-op if the session has no script).
            maybe_install_login_runner(
                &login_slot,
                login_script_json.as_deref(),
                &session_id,
                cmd_tx.clone(),
                &app,
            );
            maybe_install_trigger_runner(
                &trigger_slot,
                triggers_json.as_deref(),
                &session_id,
                cmd_tx.clone(),
                &app,
            );
            let stats_task =
                spawn_stats_emitter(app.clone(), session_id.clone(), Arc::clone(&rx_total));
            if let Some(w) = mss_watcher {
                // Detach the task — it polls until the watcher reports empty
                // for several ticks in a row, which is the post-close state.
                std::mem::drop(spawn_mss_emitter(app.clone(), session_id.clone(), w));
            }

            // Clone the shared handles before they move into ActiveSession so
            // saved forwards can be auto-started against the same connection.
            let fwd_handle = ssh_handle.clone();
            let fwd_registry = remote_forwards.clone();

            state.sessions.lock().unwrap().insert(
                session_id.clone(),
                ActiveSession {
                    cmd_tx,
                    cols,
                    rows,
                    ssh_handle: Some(ssh_handle),
                    remote_forwards: Some(remote_forwards),
                    platform_detector: Some(detector),
                    tcp_mss,
                    sftp: Some(core_transport::empty_sftp_slot()),
                    rx_total,
                    stats_task: Some(stats_task),
                },
            );

            // Auto-start saved port-forwards (best-effort; failures are logged,
            // never fatal to the session open).
            let saved_forwards = state
                .db
                .list_session_forwards(saved_session_id)
                .unwrap_or_default();
            if !saved_forwards.is_empty() {
                crate::commands::forwards::autostart_saved_forwards(
                    &state,
                    &session_id,
                    fwd_handle,
                    fwd_registry,
                    &saved_forwards,
                )
                .await;
            }

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
            let hl = Arc::clone(&state.highlighter);
            let lg = Arc::clone(&state.loggers);
            let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let login_slot: LoginRunnerSlot = empty_login_slot();
            let login_slot_for_closure = Arc::clone(&login_slot);
            let trigger_slot: TriggerRunnerSlot = empty_trigger_slot();
            let trigger_slot_for_closure = Arc::clone(&trigger_slot);
            let cmd_tx = transport.start_io_loop(
                cols,
                rows,
                make_on_data(
                    session_id.clone(),
                    lg,
                    hl,
                    login_slot_for_closure,
                    trigger_slot_for_closure,
                    Arc::clone(&rx_total),
                    None, // no platform detector for non-SSH
                    None,
                    move |data| {
                        let _ = app_handle.emit(
                            "terminal-data",
                            TerminalDataPayload {
                                session_id: sid.clone(),
                                data,
                            },
                        );
                    },
                ),
            );

            maybe_install_login_runner(
                &login_slot,
                login_script_json.as_deref(),
                &session_id,
                cmd_tx.clone(),
                &app,
            );
            maybe_install_trigger_runner(
                &trigger_slot,
                triggers_json.as_deref(),
                &session_id,
                cmd_tx.clone(),
                &app,
            );
            let stats_task =
                spawn_stats_emitter(app.clone(), session_id.clone(), Arc::clone(&rx_total));

            state.sessions.lock().unwrap().insert(
                session_id.clone(),
                ActiveSession {
                    cmd_tx,
                    cols,
                    rows,
                    ssh_handle: None,
                    remote_forwards: None,
                    platform_detector: None,
                    tcp_mss: core_transport::TcpMss::default(),
                    sftp: None,
                    rx_total,
                    stats_task: Some(stats_task),
                },
            );

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
            let hl = Arc::clone(&state.highlighter);
            let lg = Arc::clone(&state.loggers);
            let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let login_slot: LoginRunnerSlot = empty_login_slot();
            let login_slot_for_closure = Arc::clone(&login_slot);
            let trigger_slot: TriggerRunnerSlot = empty_trigger_slot();
            let trigger_slot_for_closure = Arc::clone(&trigger_slot);
            let cmd_tx = transport.start_io_loop(make_on_data(
                session_id.clone(),
                lg,
                hl,
                login_slot_for_closure,
                trigger_slot_for_closure,
                Arc::clone(&rx_total),
                None, // no platform detector for non-SSH
                None,
                move |data| {
                    let _ = app_handle.emit(
                        "terminal-data",
                        TerminalDataPayload {
                            session_id: sid.clone(),
                            data,
                        },
                    );
                },
            ));

            maybe_install_login_runner(
                &login_slot,
                login_script_json.as_deref(),
                &session_id,
                cmd_tx.clone(),
                &app,
            );
            maybe_install_trigger_runner(
                &trigger_slot,
                triggers_json.as_deref(),
                &session_id,
                cmd_tx.clone(),
                &app,
            );
            let stats_task =
                spawn_stats_emitter(app.clone(), session_id.clone(), Arc::clone(&rx_total));

            state.sessions.lock().unwrap().insert(
                session_id.clone(),
                ActiveSession {
                    cmd_tx,
                    cols,
                    rows,
                    ssh_handle: None,
                    remote_forwards: None,
                    platform_detector: None,
                    tcp_mss: core_transport::TcpMss::default(),
                    sftp: None,
                    rx_total,
                    stats_task: Some(stats_task),
                },
            );

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
    let hl = Arc::clone(&state.highlighter);
    let lg = Arc::clone(&state.loggers);
    let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let cmd_tx = transport.start_io_loop(
        cols,
        rows,
        make_on_data(
            session_id.clone(),
            lg,
            hl,
            empty_login_slot(),
            empty_trigger_slot(),
            Arc::clone(&rx_total),
            None, // no platform detector for non-SSH
            None,
            move |data| {
                let _ = app_handle.emit(
                    "terminal-data",
                    TerminalDataPayload {
                        session_id: sid.clone(),
                        data,
                    },
                );
            },
        ),
    );
    let stats_task = Some(spawn_stats_emitter(
        app.clone(),
        session_id.clone(),
        Arc::clone(&rx_total),
    ));

    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        ActiveSession {
            cmd_tx,
            cols,
            rows,
            ssh_handle: None,
            remote_forwards: None,
            platform_detector: None,
            tcp_mss: core_transport::TcpMss::default(),
            sftp: None,
            rx_total,
            stats_task,
        },
    );

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
    let hl = Arc::clone(&state.highlighter);
    let lg = Arc::clone(&state.loggers);
    let rx_total = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let cmd_tx = transport.start_io_loop(make_on_data(
        session_id.clone(),
        lg,
        hl,
        empty_login_slot(),
        empty_trigger_slot(),
        Arc::clone(&rx_total),
        None, // no platform detector for non-SSH
        None,
        move |data| {
            let _ = app_handle.emit(
                "terminal-data",
                TerminalDataPayload {
                    session_id: sid.clone(),
                    data,
                },
            );
        },
    ));
    let stats_task = Some(spawn_stats_emitter(
        app.clone(),
        session_id.clone(),
        Arc::clone(&rx_total),
    ));

    // Serial sessions get a default size; resize is ignored by the transport.
    state.sessions.lock().unwrap().insert(
        session_id.clone(),
        ActiveSession {
            cmd_tx,
            cols: 80,
            rows: 24,
            ssh_handle: None,
            remote_forwards: None,
            platform_detector: None,
            tcp_mss: core_transport::TcpMss::default(),
            sftp: None,
            rx_total,
            stats_task,
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

#[derive(serde::Serialize, Clone)]
struct DisconnectPayload {
    session_id: String,
}

/// Build the `on_close` callback for an SSH session: when the remote side drops
/// the link (server closed the channel, or keepalives went unanswered), emit
/// `session-disconnected` so the frontend can show a reconnect affordance and
/// optionally auto-reconnect. Not fired when the user closes the tab.
fn make_on_close(app: AppHandle, session_id: String) -> impl FnOnce() + Send + 'static {
    move || {
        let _ = app.emit("session-disconnected", DisconnectPayload { session_id });
    }
}
