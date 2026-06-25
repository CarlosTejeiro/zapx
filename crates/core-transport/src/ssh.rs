use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use russh::client;
use russh::client::KeyboardInteractiveAuthResponse;
use russh::ChannelMsg;
use russh_keys::key;

use crate::error::Error;
use crate::SessionCmd;

/// Global SSH keepalive interval in seconds (0 = disabled). It's a single
/// user-wide preference, so a global avoids threading the value through every
/// connect signature (including the whole bastion chain). Read when each russh
/// client config is built, so it takes effect on connections opened afterwards.
static SSH_KEEPALIVE_SECS: AtomicU64 = AtomicU64::new(0);

/// Set the SSH server-alive interval (seconds; 0 disables). Affects sessions
/// opened after this call.
pub fn set_keepalive_secs(secs: u64) {
    SSH_KEEPALIVE_SECS.store(secs, Ordering::Relaxed);
}

/// russh client config carrying the configured keepalive. `keepalive_max` is
/// how many unanswered keepalives russh tolerates before dropping the link —
/// which then surfaces as a normal disconnect (and drives auto-reconnect).
fn build_client_config() -> Arc<client::Config> {
    let mut config = client::Config::default();
    let secs = SSH_KEEPALIVE_SECS.load(Ordering::Relaxed);
    if secs > 0 {
        config.keepalive_interval = Some(std::time::Duration::from_secs(secs));
        config.keepalive_max = 3;
    }
    Arc::new(config)
}

// ---------------------------------------------------------------------------
// Remote forward (-R) registry
// ---------------------------------------------------------------------------

/// Target of an active remote forward (where to bridge incoming
/// `forwarded-tcpip` channels). Owned by the registry described below.
#[derive(Debug, Clone)]
pub struct RemoteTarget {
    pub host: String,
    pub port: u16,
}

/// Shared map of `(bind_address, bind_port)` on the SSH server → local
/// target to bridge to. Both the russh client handler (which receives
/// `forwarded-tcpip` channels) and the [`crate::forwards::open_remote_forward`]
/// code (which inserts new entries) hold an `Arc` to the same map.
///
/// We don't use russh's per-message routing because it can't bridge to an
/// arbitrary local address — the bridge logic and the target lookup must
/// live in our handler.
pub type RemoteForwardRegistry = Arc<Mutex<HashMap<(String, u16), RemoteTarget>>>;

pub fn new_remote_forward_registry() -> RemoteForwardRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

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

/// On Windows, which agent backend(s) `SshAuth::Agent` tries — and in which
/// order. Unix builds ignore this (only one agent, via `SSH_AUTH_SOCK`).
///
/// `Auto` is the default and matches the historical behaviour: OpenSSH-for-
/// Windows first, fall back to Pageant. `PageantFirst` is for PuTTY-centric
/// users whose keys live exclusively in Pageant. The `*Only` variants skip
/// the fallback so a misconfiguration produces a clear error instead of a
/// silent attempt against the wrong agent.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AgentPriority {
    #[default]
    Auto,
    PageantFirst,
    OpenSshOnly,
    PageantOnly,
}

impl AgentPriority {
    /// Parse the user-facing string form persisted in `settings`. Unknown
    /// strings fall back to `Auto` so a stale or hand-edited DB never blocks
    /// the user from connecting.
    pub fn from_setting(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Self::Auto,
            "pageant-first" | "pageant_first" => Self::PageantFirst,
            "openssh-only" | "openssh_only" => Self::OpenSshOnly,
            "pageant-only" | "pageant_only" => Self::PageantOnly,
            _ => Self::Auto,
        }
    }

    pub fn as_setting(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::PageantFirst => "pageant-first",
            Self::OpenSshOnly => "openssh-only",
            Self::PageantOnly => "pageant-only",
        }
    }
}

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
    /// SSH agent auth. `priority` is only consulted on Windows where two
    /// agent implementations (OpenSSH named pipe, Pageant) might both be
    /// reachable; on Unix it's ignored.
    Agent {
        priority: AgentPriority,
    },
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

/// Host-key handler for real connections.
///
/// * **Changed** keys are always rejected (possible MITM — fail closed).
/// * **Unknown** keys depend on [`Self::allow_tofu`]:
///   - `false` (direct connections): rejected. Trust must be established
///     out-of-band first — the UI runs [`preflight_host_key`], shows the
///     fingerprint, and on approval calls [`trust_host_key`] to record it,
///     so by connect time the key is already `Known`. An unknown key here
///     means preflight was bypassed or the key changed in the gap, so we
///     fail closed rather than silently trusting.
///   - `true` (bastion hops and jump-host targets): Trust On First Use —
///     record and accept, logging the learned fingerprint for an audit
///     trail. This is unavoidable for hosts reachable *only* through a jump
///     host, since the UI preflight can't open a direct socket to prompt.
pub struct SshClientHandler {
    host: String,
    port: u16,
    /// Whether to Trust On First Use an unknown key (see the type docs).
    /// Only `true` for sessions that ride inside an SSH tunnel (bastion hops
    /// and jump-host targets), where a direct preflight is impossible.
    allow_tofu: bool,
    /// Shared with [`crate::forwards::open_remote_forward`]; the
    /// `server_channel_open_forwarded_tcpip` callback below uses it to find
    /// where to bridge each incoming forwarded channel.
    forwards: RemoteForwardRegistry,
}

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match russh_keys::check_known_hosts(&self.host, self.port, server_public_key) {
            // Recorded and matches → accept.
            Ok(true) => Ok(true),
            // Recorded but DIFFERENT → possible MITM; reject (fail closed).
            Err(russh_keys::Error::KeyChanged { .. }) => Ok(false),
            // Unknown (not recorded) or unreadable known_hosts.
            _ => {
                if !self.allow_tofu {
                    // Direct connection: the UI preflight should already have
                    // recorded this key. Reaching here means it didn't —
                    // fail closed instead of trusting blindly.
                    tracing::warn!(
                        host = %self.host,
                        port = self.port,
                        fingerprint = %fingerprint(server_public_key),
                        "rejecting unknown host key on a direct connection (preflight bypassed?)"
                    );
                    return Ok(false);
                }
                // Tunneled host (bastion / jump target): TOFU is the only
                // option, but leave an audit trail of what we trusted.
                tracing::warn!(
                    host = %self.host,
                    port = self.port,
                    fingerprint = %fingerprint(server_public_key),
                    "trusting unknown host key on first use (reachable only via jump host)"
                );
                let _ = russh_keys::known_hosts::learn_known_hosts(
                    &self.host,
                    self.port,
                    server_public_key,
                );
                Ok(true)
            }
        }
    }

    /// Handle an incoming `forwarded-tcpip` channel — fired when something
    /// on the remote side connects to one of our active `-R` bind ports.
    /// Look up the local target in [`Self::forwards`] and bridge bytes.
    /// Unknown entries are dropped on the floor (the server shouldn't be
    /// forwarding ports we didn't register, but we tolerate it).
    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let key = (connected_address.to_string(), connected_port as u16);
        let target = match self.forwards.lock() {
            Ok(map) => map.get(&key).cloned(),
            Err(_) => None,
        };
        let Some(target) = target else {
            tracing::debug!(
                "forwarded-tcpip for unknown bind {}:{}; dropping",
                connected_address,
                connected_port
            );
            return Ok(());
        };
        tokio::spawn(async move {
            let mut remote = channel.into_stream();
            match tokio::net::TcpStream::connect((target.host.as_str(), target.port)).await {
                Ok(mut local) => {
                    if let Err(e) = tokio::io::copy_bidirectional(&mut remote, &mut local).await {
                        tracing::debug!("remote forward bridge ended: {e}");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "remote forward → {}:{} connect failed: {e}",
                        target.host,
                        target.port
                    );
                }
            }
        });
        Ok(())
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
    let config = build_client_config();
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
/// `expected_fp` is the fingerprint the user approved during
/// [`preflight_host_key`]. We re-fetch the key here (a fresh connection) and
/// refuse to learn it unless it still matches `expected_fp` — otherwise a MITM
/// could present a benign key at preflight and swap in a different one at trust
/// time, getting their key written to `known_hosts`. Comparing the full
/// SHA-256 fingerprint closes that TOCTOU window.
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
    /// stealing the handle from the I/O task. Wrapped in a `tokio::sync::Mutex`
    /// because russh's `tcpip_forward` (`-R`) takes `&mut self`; the lock is
    /// only held for the duration of one short channel/request roundtrip.
    /// Dropping the last `Arc` closes the SSH connection.
    session: Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>,
    channel: russh::Channel<client::Msg>,
    /// Chain of bastion handles whose `direct-tcpip` channels carry this
    /// session. Kept alive for the duration of the transport — dropping any
    /// of them would tear down the tunnel beneath us.
    _bastions: Vec<client::Handle<SshClientHandler>>,
    /// Same `Arc` the russh handler holds. Surfaced via [`start_io_loop`] so
    /// the app layer can register `-R` targets without poking russh internals.
    forwards: RemoteForwardRegistry,
    /// TCP MSS snapshot taken at connect time. `default()` for bastion / SSH-
    /// over-SSH sessions where no TCP socket is owned directly.
    tcp_mss: crate::TcpMss,
    /// Owns a duplicated socket handle so MSS can be re-queried after russh
    /// has consumed the original `TcpStream`. `None` for bastion sessions
    /// and for platforms where the dup itself failed.
    mss_watcher: Option<crate::tcp_mss::MssWatcher>,
}

impl SshTransport {
    /// TCP MSS snapshot captured at connect time. Cheap (just a struct copy).
    pub fn tcp_mss(&self) -> crate::TcpMss {
        self.tcp_mss
    }

    /// Take ownership of the live MSS watcher so the app layer can poll it
    /// from a background task. Returns `None` for bastion sessions or when
    /// the watcher couldn't be created. Once taken, subsequent calls return
    /// `None`.
    pub fn take_mss_watcher(&mut self) -> Option<crate::tcp_mss::MssWatcher> {
        self.mss_watcher.take()
    }
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
    /// the app layer) AND the per-session remote-forward registry. The shell
    /// stays alive until either the caller drops `cmd_tx` or the remote
    /// closes the channel.
    pub fn start_io_loop(
        self,
        on_data: impl Fn(Vec<u8>) + Send + Sync + 'static,
        on_close: impl FnOnce() + Send + 'static,
    ) -> (
        tokio::sync::mpsc::UnboundedSender<SessionCmd>,
        Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>,
        RemoteForwardRegistry,
    ) {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SessionCmd>();
        let mut channel = self.channel;
        let session = self.session;
        let session_for_task = Arc::clone(&session);
        let bastions = self._bastions;
        let forwards = self.forwards;

        tokio::spawn(async move {
            // Keep session AND any bastion handles alive for the duration of
            // the I/O loop — dropping bastions would tear the tunnel down.
            let _session = session_for_task;
            let _bastions = bastions;
            // Did the loop end because the REMOTE side closed (vs. the user
            // closing the tab, which drops cmd_tx)? Only the former should fire
            // on_close → the app emits a disconnect event and may reconnect.
            let mut remote_closed = false;
            loop {
                tokio::select! {
                    cmd = rx.recv() => {
                        match cmd {
                            Some(SessionCmd::Data(data)) => {
                                if channel.data(data.as_ref()).await.is_err() {
                                    // Write failed → the link is gone.
                                    remote_closed = true;
                                    break;
                                }
                            }
                            Some(SessionCmd::Resize { cols, rows }) => {
                                channel
                                    .window_change(cols.into(), rows.into(), 0, 0)
                                    .await
                                    .ok();
                            }
                            None => break, // cmd_tx dropped → user closed the tab
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
                            // Channel closed or shell exited (incl. dropped link
                            // after keepalives went unanswered).
                            Some(ChannelMsg::ExitStatus { .. })
                            | Some(ChannelMsg::Close)
                            | Some(ChannelMsg::Eof)
                            | None => {
                                remote_closed = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            if remote_closed {
                on_close();
            }
        });

        (tx, session, forwards)
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
    // Direct connection: the UI preflight handles trust, so fail closed on
    // an unknown key here rather than silently trusting it.
    let (mut session, forwards, tcp_mss, mss_watcher) =
        connect_only(host.clone(), port, false).await?;
    let authenticated = perform_auth(&mut session, user, auth).await?;
    if !authenticated {
        return Err(Error::AuthFailed);
    }
    let channel = open_pty_shell(&mut session, cols, rows).await?;
    Ok(SshTransport {
        session: Arc::new(tokio::sync::Mutex::new(session)),
        channel,
        _bastions: Vec::new(),
        forwards,
        tcp_mss,
        mss_watcher,
    })
}

/// Open a fresh SSH session to `host:port`, drive KEX (which enforces
/// known_hosts via [`SshClientHandler`]), and return the unauthenticated
/// handle together with the per-session remote-forward registry the handler
/// holds, plus the TCP-level MSS snapshot taken right after the socket
/// connected (zeroed if the OS doesn't expose it).
///
/// We connect the TCP socket manually (instead of letting russh dial it for
/// us) so we own a `&TcpStream` we can `getsockopt` against — there's no
/// other way to recover the FD after russh swallows it.
async fn connect_only(
    host: String,
    port: u16,
    allow_tofu: bool,
) -> Result<
    (
        client::Handle<SshClientHandler>,
        RemoteForwardRegistry,
        crate::TcpMss,
        Option<crate::tcp_mss::MssWatcher>,
    ),
    Error,
> {
    let stream = tokio::net::TcpStream::connect((host.as_str(), port))
        .await
        .map_err(Error::Io)?;
    let mss = crate::tcp_mss::query(&stream);
    let watcher = crate::tcp_mss::MssWatcher::new(&stream);

    let config = build_client_config();
    let forwards = new_remote_forward_registry();
    let handler = SshClientHandler {
        host: host.clone(),
        port,
        allow_tofu,
        forwards: Arc::clone(&forwards),
    };
    let handle = client::connect_stream(config, stream, handler)
        .await
        .map_err(Error::Ssh)?;
    Ok((handle, forwards, mss, watcher))
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
        SshAuth::Agent { priority } => agent_auth(session, user, priority).await,
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
    // Bastion-only path: -R against a bastion is not supported, so we drop
    // the registry returned by `connect_only`. MSS is dropped too — we only
    // care about the FINAL session's MSS, not each bastion hop. Bastions are
    // not preflighted by the UI, so TOFU their host key on first use.
    let (mut session, _bastion_forwards, _bastion_mss, _bastion_watcher) =
        connect_only(host, port, true).await?;
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
    let config = build_client_config();
    // Bastion: discard the registry — the resulting handle is only used as
    // a hop for direct-tcpip channels, never the final SSH session. Reached
    // through a tunnel, so it can't be preflighted: TOFU on first use.
    let handler = SshClientHandler {
        host: host.clone(),
        port,
        allow_tofu: true,
        forwards: new_remote_forward_registry(),
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

    // Run a fresh SSH client over that tunneled stream. The known_hosts
    // handler is keyed to the TARGET, not the bastion. The target is only
    // reachable through the tunnel, so the UI can't preflight it: TOFU on
    // first use (the learned fingerprint is logged).
    let config = build_client_config();
    let forwards = new_remote_forward_registry();
    let handler = SshClientHandler {
        host: target_host.clone(),
        port: target_port,
        allow_tofu: true,
        forwards: Arc::clone(&forwards),
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
        session: Arc::new(tokio::sync::Mutex::new(session)),
        channel,
        _bastions: bastions,
        forwards,
        // Bastion sessions live inside an SSH tunnel — there's no direct TCP
        // socket whose MSS would be meaningful. Surface empty.
        tcp_mss: crate::TcpMss::default(),
        mss_watcher: None,
    })
}

// ---------------------------------------------------------------------------
// SSH agent authentication (Phase 3)
//
// Connect to the OS ssh-agent, enumerate identities, and try each one against
// the server via `authenticate_future` (which threads the `Signer` back out so
// we can reuse it across attempts).
//
// * Unix: `AgentClient::connect_env()` (uses `$SSH_AUTH_SOCK`).
// * Windows: try OpenSSH-for-Windows first (named pipe at the path given by
//   `$SSH_AUTH_SOCK` or `\\.\pipe\openssh-ssh-agent` by default), then fall
//   back to Pageant (PuTTY) via shared memory + WM_COPYDATA. Both shapes
//   share the same identity-enumeration + signing loop below.
// ---------------------------------------------------------------------------

#[cfg(unix)]
async fn agent_auth(
    session: &mut client::Handle<SshClientHandler>,
    user: String,
    // Only used on Windows; accepted here so the call site can stay
    // platform-agnostic. `_` to keep clippy quiet on Unix builds.
    _priority: AgentPriority,
) -> Result<bool, Error> {
    use russh_keys::agent::client::AgentClient;

    let agent = AgentClient::connect_env()
        .await
        .map_err(|e| Error::Agent(format!("no SSH agent available: {e}")))?;
    run_agent_auth(session, user, agent).await
}

#[cfg(windows)]
async fn agent_auth(
    session: &mut client::Handle<SshClientHandler>,
    user: String,
    priority: AgentPriority,
) -> Result<bool, Error> {
    // Build the ordered list of agent backends to try based on the user's
    // priority. `*Only` variants skip the fallback entirely so a config error
    // ("I told you to use Pageant only") fails loudly rather than silently
    // succeeding against the wrong agent.
    let backends: &[AgentBackend] = match priority {
        AgentPriority::Auto => &[AgentBackend::OpenSsh, AgentBackend::Pageant],
        AgentPriority::PageantFirst => &[AgentBackend::Pageant, AgentBackend::OpenSsh],
        AgentPriority::OpenSshOnly => &[AgentBackend::OpenSsh],
        AgentPriority::PageantOnly => &[AgentBackend::Pageant],
    };

    let mut errors: Vec<String> = Vec::new();
    for backend in backends {
        match try_windows_backend(*backend, session, user.clone()).await {
            Ok(true) => return Ok(true),
            // Identities found but server rejected all of them — that's a
            // final verdict for this backend; try the next one in priority
            // because the OTHER agent may hold a key the server accepts.
            Ok(false) => {
                errors.push(format!(
                    "{}: no identity accepted by server",
                    backend.label()
                ));
            }
            Err(e) => errors.push(format!("{}: {e}", backend.label())),
        }
    }
    Err(Error::Agent(format!(
        "no usable SSH agent on Windows ({})",
        errors.join("; ")
    )))
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy)]
enum AgentBackend {
    OpenSsh,
    Pageant,
}

#[cfg(windows)]
impl AgentBackend {
    fn label(self) -> &'static str {
        match self {
            Self::OpenSsh => "OpenSSH",
            Self::Pageant => "Pageant",
        }
    }
}

#[cfg(windows)]
async fn try_windows_backend(
    backend: AgentBackend,
    session: &mut client::Handle<SshClientHandler>,
    user: String,
) -> Result<bool, Error> {
    use russh_keys::agent::client::AgentClient;
    match backend {
        AgentBackend::OpenSsh => {
            // `$SSH_AUTH_SOCK` is the way Microsoft's port advertises its
            // path; we fall back to the well-known default for older setups.
            let pipe_path = std::env::var_os("SSH_AUTH_SOCK")
                .unwrap_or_else(|| std::ffi::OsString::from(r"\\.\pipe\openssh-ssh-agent"));
            let agent = AgentClient::connect_named_pipe(&pipe_path)
                .await
                .map_err(|e| {
                    Error::Agent(format!("named pipe '{}': {e}", pipe_path.to_string_lossy()))
                })?;
            run_agent_auth(session, user, agent).await
        }
        AgentBackend::Pageant => {
            // `connect_pageant` itself doesn't fail; the missing process
            // surfaces on the first `request_identities()` call inside the
            // shared helper.
            let agent = AgentClient::connect_pageant().await;
            run_agent_auth(session, user, agent).await
        }
    }
}

/// Identity-enumeration + signing loop shared by Unix and Windows agent
/// front-ends. Generic over the `AgentClient<R>` concrete stream so we
/// monomorphise once per platform without duplicating the body.
async fn run_agent_auth<S>(
    session: &mut client::Handle<SshClientHandler>,
    user: String,
    mut agent: S,
) -> Result<bool, Error>
where
    S: russh::Signer + Send,
    S: AgentIdentities,
{
    let identities = agent
        .request_identities_dyn()
        .await
        .map_err(|e| Error::Agent(format!("failed to list agent identities: {e}")))?;

    if identities.is_empty() {
        return Err(Error::Agent("ssh-agent has no identities loaded".into()));
    }

    // `authenticate_future` consumes and returns the Signer so we can try
    // the next identity on failure without reconnecting to the agent.
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

/// Thin async-trait shim that wraps `AgentClient::request_identities()` so the
/// generic `run_agent_auth` can list keys without referring to the concrete
/// stream type. The error type is erased to `String` so all platforms surface
/// agent failures the same way.
trait AgentIdentities {
    fn request_identities_dyn(
        &mut self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<russh_keys::key::PublicKey>, String>>
                + Send
                + '_,
        >,
    >;
}

impl<R> AgentIdentities for russh_keys::agent::client::AgentClient<R>
where
    R: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    fn request_identities_dyn(
        &mut self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<russh_keys::key::PublicKey>, String>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move { self.request_identities().await.map_err(|e| e.to_string()) })
    }
}

#[cfg(test)]
mod tests {
    use super::AgentPriority;

    #[test]
    fn priority_round_trip() {
        for v in [
            AgentPriority::Auto,
            AgentPriority::PageantFirst,
            AgentPriority::OpenSshOnly,
            AgentPriority::PageantOnly,
        ] {
            assert_eq!(AgentPriority::from_setting(v.as_setting()), v);
        }
    }

    #[test]
    fn priority_accepts_common_variants() {
        // Underscore form (some users hand-edit the DB), trailing whitespace,
        // mixed case — all should resolve cleanly.
        assert_eq!(AgentPriority::from_setting(""), AgentPriority::Auto);
        assert_eq!(AgentPriority::from_setting("AUTO"), AgentPriority::Auto);
        assert_eq!(
            AgentPriority::from_setting("pageant_first"),
            AgentPriority::PageantFirst
        );
        assert_eq!(
            AgentPriority::from_setting("  Pageant-First  "),
            AgentPriority::PageantFirst
        );
        assert_eq!(
            AgentPriority::from_setting("openssh_only"),
            AgentPriority::OpenSshOnly
        );
    }

    #[test]
    fn priority_unknown_falls_back_to_auto() {
        // A stale or corrupted setting must never block the user.
        assert_eq!(AgentPriority::from_setting("bogus"), AgentPriority::Auto);
        assert_eq!(
            AgentPriority::from_setting("openssh,pageant"),
            AgentPriority::Auto
        );
    }
}
