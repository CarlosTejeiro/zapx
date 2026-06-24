use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use russh::client;
use russh::client::KeyboardInteractiveAuthResponse;
use russh::ChannelMsg;
use russh_keys::key;

use crate::error::Error;
use crate::SessionCmd;

// ---------------------------------------------------------------------------
// Authentication strategy
// ---------------------------------------------------------------------------

/// One server-driven keyboard-interactive prompt.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KiPrompt {
    pub prompt: String,
    pub echo: bool,
}

/// A keyboard-interactive `InfoRequest` from the server. The caller must
/// provide a `Vec<String>` of responses with one entry per `prompts` element.
#[derive(Debug, Clone, serde::Serialize)]
pub struct KiRequest {
    pub name: String,
    pub instructions: String,
    pub prompts: Vec<KiPrompt>,
}

/// Async callback that receives a [`KiRequest`] and returns the user's answers.
///
/// Kept as a trait object so `core-transport` stays free of Tauri/UI concerns:
/// the app layer supplies a responder that bridges to the frontend via a
/// Tauri event + oneshot channel.
pub type KiResponder =
    Arc<dyn Fn(KiRequest) -> Pin<Box<dyn Future<Output = Vec<String>> + Send>> + Send + Sync>;

/// How to authenticate an SSH session.
///
/// The key *path* (not the key bytes) is carried here: the file is not a secret
/// to copy around, and `russh_keys::load_secret_key` reads from a path. Only the
/// passphrase is a secret and lives in the OS vault.
#[derive(Clone)]
pub enum SshAuth {
    Password(String),
    PublicKey {
        key_path: String,
        passphrase: Option<String>,
    },
    /// 2FA / OTP. The `responder` is invoked once per server `InfoRequest`
    /// (there can be several) and must return one answer per prompt.
    KeyboardInteractive {
        responder: KiResponder,
    },
    /// SSH agent auth — implemented in a later phase.
    Agent,
}

// ---------------------------------------------------------------------------
// Host-key verification (known_hosts)
// ---------------------------------------------------------------------------

/// Result of checking a server's host key against `~/.ssh/known_hosts`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum HostKeyStatus {
    /// Recorded and matches — safe to connect silently.
    Known,
    /// Not in known_hosts — first connection (TOFU decision needed).
    Unknown { fingerprint: String },
    /// Recorded but DIFFERENT — possible MITM; must warn the user.
    Changed { fingerprint: String },
}

fn fingerprint(pubkey: &key::PublicKey) -> String {
    format!("SHA256:{}", pubkey.fingerprint())
}

/// Expand a leading `~/` to the user's home directory (`load_secret_key` opens
/// the path verbatim and does not expand `~`).
fn expand_tilde(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
            return std::path::Path::new(&home).join(rest);
        }
    }
    std::path::PathBuf::from(path)
}

/// Strict handler used for real connections: accepts only host keys that are
/// already recorded in `known_hosts` and match. Unknown or changed keys are
/// rejected (fail closed). Trust is established out-of-band via
/// [`trust_host_key`] after the user approves the preflight result.
pub struct SshClientHandler {
    host: String,
    port: u16,
}

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(
            russh_keys::check_known_hosts(&self.host, self.port, server_public_key)
                .unwrap_or(false),
        )
    }
}

/// Handler used only for preflight/trust: captures the server's host key (so we
/// can inspect it) and accepts the connection so the KEX completes. It never
/// authenticates.
struct KeyCaptureHandler {
    captured: Arc<Mutex<Option<key::PublicKey>>>,
}

#[async_trait]
impl client::Handler for KeyCaptureHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        *self.captured.lock().unwrap() = Some(server_public_key.clone());
        Ok(true)
    }
}

/// Open a throwaway connection, capture the host key, and return it.
async fn capture_host_key(host: &str, port: u16) -> Result<key::PublicKey, Error> {
    let captured = Arc::new(Mutex::new(None));
    let handler = KeyCaptureHandler {
        captured: Arc::clone(&captured),
    };
    let config = Arc::new(client::Config::default());
    // The session is dropped at the end of this fn, closing the connection.
    let _session = client::connect(config, (host.to_owned(), port), handler)
        .await
        .map_err(Error::Ssh)?;
    let key = captured
        .lock()
        .unwrap()
        .take()
        .ok_or(Error::ConnectionFailed)?;
    Ok(key)
}

/// Fetch the server host key and classify it against `known_hosts`.
///
/// Run this *before* connecting so the UI can prompt for TOFU approval or warn
/// about a changed key.
pub async fn preflight_host_key(host: String, port: u16) -> Result<HostKeyStatus, Error> {
    let key = capture_host_key(&host, port).await?;
    let fp = fingerprint(&key);
    let status = match russh_keys::check_known_hosts(&host, port, &key) {
        Ok(true) => HostKeyStatus::Known,
        Err(russh_keys::Error::KeyChanged { .. }) => HostKeyStatus::Changed { fingerprint: fp },
        // Ok(false) = not recorded; any other error = unreadable known_hosts,
        // both treated as "unknown" so the user gets a TOFU prompt.
        _ => HostKeyStatus::Unknown { fingerprint: fp },
    };
    Ok(status)
}

/// Persist trust for a host key (writes to `~/.ssh/known_hosts`).
///
/// `expected_fp` is the fingerprint the user actually approved during
/// [`preflight_host_key`]. We re-fetch the key here (a separate connection) and
/// refuse to trust it unless it still matches `expected_fp` — otherwise a MITM
/// could present a benign key at preflight and a different one at trust time,
/// getting their key written to `known_hosts`. Matching the full SHA-256
/// fingerprint closes that window.
pub async fn trust_host_key(host: String, port: u16, expected_fp: String) -> Result<(), Error> {
    let key = capture_host_key(&host, port).await?;
    let actual_fp = fingerprint(&key);
    if actual_fp != expected_fp {
        return Err(Error::HostKeyChanged {
            fingerprint: actual_fp,
        });
    }
    russh_keys::known_hosts::learn_known_hosts(&host, port, &key)
        .map_err(|e| Error::KeyLoad(e.to_string()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct SshTransport {
    /// Shared so the app layer can open port-forwards on this session without
    /// stealing the Handle from the I/O task. Dropping the last Arc closes the
    /// SSH connection.
    session: Arc<client::Handle<SshClientHandler>>,
    channel: russh::Channel<client::Msg>,
    /// Chain of bastion handles whose `direct-tcpip` channels carry this
    /// session. Kept alive for the duration of the transport — dropping any
    /// of them would tear down the tunnel beneath us.
    _bastions: Vec<client::Handle<SshClientHandler>>,
}

impl SshTransport {
    /// Connect, authenticate with `auth`, and open a PTY shell channel.
    ///
    /// The host key is verified strictly against `known_hosts`; callers must run
    /// [`preflight_host_key`] (and [`trust_host_key`] on approval) first for
    /// hosts that aren't already trusted.
    pub fn open_shell(
        host: String,
        port: u16,
        user: String,
        auth: SshAuth,
        cols: u16,
        rows: u16,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SshTransport, Error>> + Send + 'static>,
    > {
        Box::pin(open_shell_async(host, port, user, auth, cols, rows))
    }

    /// Like [`open_shell`], but reaches the target through a chain of bastion
    /// SSH sessions (ProxyJump). The last `Handle` in `bastions` is used to
    /// open a `direct-tcpip` channel to `target_host:target_port`; that
    /// channel's stream becomes the transport for a brand-new SSH session.
    ///
    /// `bastions` is kept alive on the returned [`SshTransport`] for the
    /// duration of the connection.
    #[allow(clippy::too_many_arguments)]
    pub fn open_shell_via(
        bastions: Vec<client::Handle<SshClientHandler>>,
        target_host: String,
        target_port: u16,
        target_user: String,
        target_auth: SshAuth,
        cols: u16,
        rows: u16,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SshTransport, Error>> + Send + 'static>,
    > {
        Box::pin(open_shell_via_async(
            bastions,
            target_host,
            target_port,
            target_user,
            target_auth,
            cols,
            rows,
        ))
    }

    /// Spawn the async I/O loop and return the command sender PLUS a shared
    /// reference to the SSH session handle (for opening port-forwards from
    /// the app layer). The shell stays alive until either the caller drops
    /// `cmd_tx` or the remote closes the channel.
    pub fn start_io_loop(
        self,
        on_data: impl Fn(Vec<u8>) + Send + Sync + 'static,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<SessionCmd>,
        Arc<client::Handle<SshClientHandler>>,
    ) {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SessionCmd>();
        let mut channel = self.channel;
        let session = self.session;
        let session_for_task = Arc::clone(&session);
        let bastions = self._bastions;

        tokio::spawn(async move {
            // Keep session AND any bastion handles alive for the duration of
            // the I/O loop — dropping bastions would tear the tunnel down.
            let _session = session_for_task;
            let _bastions = bastions;
            loop {
                tokio::select! {
                    cmd = rx.recv() => {
                        match cmd {
                            Some(SessionCmd::Data(data)) => {
                                if channel.data(data.as_ref()).await.is_err() {
                                    break;
                                }
                            }
                            Some(SessionCmd::Resize { cols, rows }) => {
                                channel
                                    .window_change(cols.into(), rows.into(), 0, 0)
                                    .await
                                    .ok();
                            }
                            None => break, // cmd_tx dropped → session closed
                        }
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { ref data }) => {
                                on_data(data.to_vec());
                            }
                            // stderr stream — treat as regular output
                            Some(ChannelMsg::ExtendedData { ref data, .. }) => {
                                on_data(data.to_vec());
                            }
                            // Channel closed or shell exited
                            Some(ChannelMsg::ExitStatus { .. })
                            | Some(ChannelMsg::Close)
                            | Some(ChannelMsg::Eof)
                            | None => break,
                            _ => {}
                        }
                    }
                }
            }
        });

        (tx, session)
    }
}

// ---------------------------------------------------------------------------
// Private async implementation — separated from Box::pin to avoid the
// rustc 1.95.0 ICE (mir_borrowck panic in the diagnostic renderer when
// complex async logic lives directly inside Box::pin(async move {...})).
// ---------------------------------------------------------------------------

async fn open_shell_async(
    host: String,
    port: u16,
    user: String,
    auth: SshAuth,
    cols: u16,
    rows: u16,
) -> Result<SshTransport, Error> {
    let mut session = connect_only(host.clone(), port).await?;
    let authenticated = perform_auth(&mut session, user, auth).await?;
    if !authenticated {
        return Err(Error::AuthFailed);
    }
    let channel = open_pty_shell(&mut session, cols, rows).await?;
    Ok(SshTransport {
        session: Arc::new(session),
        channel,
        _bastions: Vec::new(),
    })
}

/// Open a fresh SSH session to `host:port`, drive KEX (which enforces
/// known_hosts via [`SshClientHandler`]), and return the unauthenticated handle.
async fn connect_only(host: String, port: u16) -> Result<client::Handle<SshClientHandler>, Error> {
    let config = Arc::new(client::Config::default());
    let handler = SshClientHandler {
        host: host.clone(),
        port,
    };
    client::connect(config, (host, port), handler)
        .await
        .map_err(Error::Ssh)
}

/// Authenticate `user` against an already-connected session using `auth`.
/// Returns true on success, false if the server rejected every attempt.
async fn perform_auth(
    session: &mut client::Handle<SshClientHandler>,
    user: String,
    auth: SshAuth,
) -> Result<bool, Error> {
    match auth {
        SshAuth::Password(password) => session
            .authenticate_password(user, password)
            .await
            .map_err(Error::Ssh),
        SshAuth::PublicKey {
            key_path,
            passphrase,
        } => {
            let key = russh_keys::load_secret_key(expand_tilde(&key_path), passphrase.as_deref())
                .map_err(|e| match e {
                // Encrypted key with no/empty passphrase, or a wrong one.
                russh_keys::Error::KeyIsEncrypted => Error::KeyPassphrase,
                _ if passphrase.is_some() => Error::KeyPassphrase,
                other => Error::KeyLoad(other.to_string()),
            })?;
            session
                .authenticate_publickey(user, Arc::new(key))
                .await
                .map_err(Error::Ssh)
        }
        SshAuth::KeyboardInteractive { responder } => {
            let mut response = session
                .authenticate_keyboard_interactive_start(user, None)
                .await
                .map_err(Error::Ssh)?;
            loop {
                match response {
                    KeyboardInteractiveAuthResponse::Success => return Ok(true),
                    KeyboardInteractiveAuthResponse::Failure => return Ok(false),
                    KeyboardInteractiveAuthResponse::InfoRequest {
                        name,
                        instructions,
                        prompts,
                    } => {
                        let req = KiRequest {
                            name,
                            instructions,
                            prompts: prompts
                                .into_iter()
                                .map(|p| KiPrompt {
                                    prompt: p.prompt,
                                    echo: p.echo,
                                })
                                .collect(),
                        };
                        let answers = (responder)(req).await;
                        response = session
                            .authenticate_keyboard_interactive_respond(answers)
                            .await
                            .map_err(Error::Ssh)?;
                    }
                }
            }
        }
        SshAuth::Agent => agent_auth(session, user).await,
    }
}

async fn open_pty_shell(
    session: &mut client::Handle<SshClientHandler>,
    cols: u16,
    rows: u16,
) -> Result<russh::Channel<client::Msg>, Error> {
    let channel = session.channel_open_session().await.map_err(Error::Ssh)?;
    channel
        .request_pty(false, "xterm-256color", cols.into(), rows.into(), 0, 0, &[])
        .await
        .map_err(Error::Ssh)?;
    channel.request_shell(false).await.map_err(Error::Ssh)?;
    Ok(channel)
}

/// Connect to an SSH host and authenticate, but do NOT open a shell.
///
/// Used by the app layer to build bastion handles for ProxyJump chains —
/// the resulting [`client::Handle`] is fed to [`SshTransport::open_shell_via`]
/// (possibly after opening another `direct-tcpip` channel through it for the
/// next bastion in the chain).
pub async fn connect_authenticated(
    host: String,
    port: u16,
    user: String,
    auth: SshAuth,
) -> Result<client::Handle<SshClientHandler>, Error> {
    let mut session = connect_only(host, port).await?;
    let authenticated = perform_auth(&mut session, user, auth).await?;
    if !authenticated {
        return Err(Error::AuthFailed);
    }
    Ok(session)
}

/// Connect to an SSH host THROUGH an already-authenticated bastion, then
/// authenticate the new connection. Used when chaining several jump hosts.
pub async fn connect_authenticated_via(
    bastion: &client::Handle<SshClientHandler>,
    host: String,
    port: u16,
    user: String,
    auth: SshAuth,
) -> Result<client::Handle<SshClientHandler>, Error> {
    let channel = bastion
        .channel_open_direct_tcpip(host.clone(), u32::from(port), "127.0.0.1", 0)
        .await
        .map_err(Error::Ssh)?;
    let stream = channel.into_stream();
    let config = Arc::new(client::Config::default());
    let handler = SshClientHandler {
        host: host.clone(),
        port,
    };
    let mut session = client::connect_stream(config, stream, handler)
        .await
        .map_err(Error::Ssh)?;
    let authenticated = perform_auth(&mut session, user, auth).await?;
    if !authenticated {
        return Err(Error::AuthFailed);
    }
    Ok(session)
}

/// Open an SSH session to `target_host:target_port` over a `direct-tcpip`
/// channel through the LAST handle in `bastions`. The whole chain is kept
/// alive on the returned [`SshTransport`].
async fn open_shell_via_async(
    bastions: Vec<client::Handle<SshClientHandler>>,
    target_host: String,
    target_port: u16,
    target_user: String,
    target_auth: SshAuth,
    cols: u16,
    rows: u16,
) -> Result<SshTransport, Error> {
    let last = bastions
        .last()
        .ok_or_else(|| Error::Agent("open_shell_via called with empty bastion chain".into()))?;

    // Tunnel to (target_host, target_port) via the last bastion.
    let channel = last
        .channel_open_direct_tcpip(target_host.clone(), u32::from(target_port), "127.0.0.1", 0)
        .await
        .map_err(Error::Ssh)?;
    let stream = channel.into_stream();

    // Run a fresh SSH client over that tunneled stream. The strict
    // known_hosts handler is keyed to the TARGET, not the bastion.
    let config = Arc::new(client::Config::default());
    let handler = SshClientHandler {
        host: target_host.clone(),
        port: target_port,
    };
    let mut session = client::connect_stream(config, stream, handler)
        .await
        .map_err(Error::Ssh)?;

    let authenticated = perform_auth(&mut session, target_user, target_auth).await?;
    if !authenticated {
        return Err(Error::AuthFailed);
    }

    let channel = open_pty_shell(&mut session, cols, rows).await?;

    Ok(SshTransport {
        session: Arc::new(session),
        channel,
        _bastions: bastions,
    })
}

// ---------------------------------------------------------------------------
// SSH agent authentication (Phase 3)
//
// Connect to the OS ssh-agent, enumerate identities, and try each one against
// the server via `authenticate_future` (which threads the `Signer` back out so
// we can reuse it across attempts). On non-Unix platforms we currently return
// a clear "not supported" error — the russh-keys `connect_env` impl is
// gated on `cfg(unix)`; Windows pageant/named-pipe support is a follow-up.
// ---------------------------------------------------------------------------

#[cfg(unix)]
async fn agent_auth(
    session: &mut client::Handle<SshClientHandler>,
    user: String,
) -> Result<bool, Error> {
    use russh_keys::agent::client::AgentClient;

    let mut agent = AgentClient::connect_env()
        .await
        .map_err(|e| Error::Agent(format!("no SSH agent available: {e}")))?;

    let identities = agent
        .request_identities()
        .await
        .map_err(|e| Error::Agent(format!("failed to list agent identities: {e}")))?;

    if identities.is_empty() {
        return Err(Error::Agent("ssh-agent has no identities loaded".into()));
    }

    // `authenticate_future` consumes and returns the Signer so we can try the
    // next identity on failure without reconnecting to the agent.
    let mut signer = agent;
    for key in identities {
        let (next, result) = session.authenticate_future(user.clone(), key, signer).await;
        signer = next;
        if matches!(result, Ok(true)) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(not(unix))]
async fn agent_auth(
    _session: &mut client::Handle<SshClientHandler>,
    _user: String,
) -> Result<bool, Error> {
    Err(Error::Agent(
        "agent authentication is currently Unix-only (Windows pageant support coming later)".into(),
    ))
}
