use std::sync::Arc;

use async_trait::async_trait;
use russh::client;
use russh::ChannelMsg;

use crate::error::Error;
use crate::SessionCmd;

// ---------------------------------------------------------------------------
// TOFU handler — accepts any server key on first connect.
// Proper known-hosts verification is added in Bloque 3 (persistence layer).
// ---------------------------------------------------------------------------

struct SshClientHandler;

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub struct SshTransport {
    // Keep the Handle alive — dropping it would close the SSH connection.
    _session: client::Handle<SshClientHandler>,
    channel: russh::Channel<client::Msg>,
}

impl SshTransport {
    /// Connect, authenticate with password, and open a PTY shell channel.
    pub fn open_shell(
        host: String,
        port: u16,
        user: String,
        password: String,
        cols: u16,
        rows: u16,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SshTransport, Error>> + Send + 'static>,
    > {
        Box::pin(open_shell_async(host, port, user, password, cols, rows))
    }

    /// Spawn the async I/O loop and return the command sender.
    ///
    /// `on_data` is called for every chunk of PTY output from the server.
    /// The loop exits when the remote shell closes the channel or `cmd_tx` is
    /// dropped (i.e. the session is removed from `AppState`).
    pub fn start_io_loop(
        self,
        on_data: impl Fn(Vec<u8>) + Send + Sync + 'static,
    ) -> tokio::sync::mpsc::UnboundedSender<SessionCmd> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SessionCmd>();
        let mut channel = self.channel;
        let session = self._session;

        tokio::spawn(async move {
            // Keep session alive for the duration of the I/O loop.
            let _session = session;
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

        tx
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
    password: String,
    cols: u16,
    rows: u16,
) -> Result<SshTransport, Error> {
    let config = Arc::new(client::Config::default());
    let mut session = client::connect(config, (host, port), SshClientHandler)
        .await
        .map_err(Error::Ssh)?;

    let authenticated = session
        .authenticate_password(user, password)
        .await
        .map_err(Error::Ssh)?;
    if !authenticated {
        return Err(Error::AuthFailed);
    }

    let channel = session.channel_open_session().await.map_err(Error::Ssh)?;

    channel
        .request_pty(false, "xterm-256color", cols.into(), rows.into(), 0, 0, &[])
        .await
        .map_err(Error::Ssh)?;

    channel.request_shell(false).await.map_err(Error::Ssh)?;

    Ok(SshTransport {
        _session: session,
        channel,
    })
}
