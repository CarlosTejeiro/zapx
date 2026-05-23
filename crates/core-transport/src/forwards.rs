//! Port forwarding over an established SSH session.
//!
//! Implements:
//! - **Local forward (`-L`)**: bind a local TCP port; for each incoming TCP
//!   connection open a `direct-tcpip` channel to a fixed `(target_host, target_port)`
//!   on the SSH side and bridge bytes both ways.
//! - **Dynamic forward (`-D`)**: bind a local TCP port as a SOCKS5 server;
//!   each accepted client's CONNECT request determines the target host/port
//!   for the `direct-tcpip` channel.
//!
//! Remote forward (`-R`) is not implemented in this module yet because it
//! requires routing `forwarded-tcpip` channels back through the
//! [`russh::client::Handler`] callback.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

use crate::error::Error;
use crate::ssh::SshClientHandler;

/// Live metadata for a forward, shipped to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForwardInfo {
    pub id: String,
    /// `"local"` or `"dynamic"`.
    pub kind: String,
    pub bind_addr: String,
    pub bind_port: u16,
    /// `Some((host, port))` for `local`; `None` for `dynamic` (SOCKS picks per connection).
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
}

/// Owns the spawned listener task; dropping aborts it and stops accepting new
/// connections (in-flight tunnels finish naturally when their channel closes).
pub struct ForwardController {
    pub info: ForwardInfo,
    task: tokio::task::AbortHandle,
}

impl Drop for ForwardController {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Open a local TCP listener on `bind_addr:bind_port`; tunnel every connection
/// to `(target_host, target_port)` via `direct-tcpip` over `handle`.
pub async fn open_local_forward(
    handle: Arc<russh::client::Handle<SshClientHandler>>,
    bind_addr: String,
    bind_port: u16,
    target_host: String,
    target_port: u16,
) -> Result<ForwardController, Error> {
    let listener = TcpListener::bind((bind_addr.as_str(), bind_port))
        .await
        .map_err(Error::Io)?;
    // Re-read the bound port in case the caller asked for 0 (OS-assigned).
    let local_addr = listener.local_addr().map_err(Error::Io)?;
    let resolved_bind_port = local_addr.port();

    let target_host_loop = target_host.clone();
    let join = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    let handle = Arc::clone(&handle);
                    let target_host = target_host_loop.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            bridge_local(stream, peer, handle, target_host, target_port).await
                        {
                            tracing::debug!("local forward bridge ended: {e}");
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("local forward accept error: {e}");
                    break;
                }
            }
        }
    });

    Ok(ForwardController {
        info: ForwardInfo {
            id: Uuid::new_v4().to_string(),
            kind: "local".to_string(),
            bind_addr,
            bind_port: resolved_bind_port,
            target_host: Some(target_host),
            target_port: Some(target_port),
        },
        task: join.abort_handle(),
    })
}

/// Open a local TCP listener serving SOCKS5; each accepted client's CONNECT
/// request determines the target for the `direct-tcpip` channel.
pub async fn open_dynamic_forward(
    handle: Arc<russh::client::Handle<SshClientHandler>>,
    bind_addr: String,
    bind_port: u16,
) -> Result<ForwardController, Error> {
    let listener = TcpListener::bind((bind_addr.as_str(), bind_port))
        .await
        .map_err(Error::Io)?;
    let local_addr = listener.local_addr().map_err(Error::Io)?;
    let resolved_bind_port = local_addr.port();

    let join = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    let handle = Arc::clone(&handle);
                    tokio::spawn(async move {
                        if let Err(e) = bridge_socks5(stream, peer, handle).await {
                            tracing::debug!("dynamic forward bridge ended: {e}");
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("dynamic forward accept error: {e}");
                    break;
                }
            }
        }
    });

    Ok(ForwardController {
        info: ForwardInfo {
            id: Uuid::new_v4().to_string(),
            kind: "dynamic".to_string(),
            bind_addr,
            bind_port: resolved_bind_port,
            target_host: None,
            target_port: None,
        },
        task: join.abort_handle(),
    })
}

// ---------------------------------------------------------------------------
// Bridges
// ---------------------------------------------------------------------------

async fn bridge_local(
    mut local: TcpStream,
    peer: SocketAddr,
    handle: Arc<russh::client::Handle<SshClientHandler>>,
    target_host: String,
    target_port: u16,
) -> Result<(), Error> {
    let channel = handle
        .channel_open_direct_tcpip(
            target_host,
            u32::from(target_port),
            peer.ip().to_string(),
            u32::from(peer.port()),
        )
        .await
        .map_err(Error::Ssh)?;
    let mut remote = channel.into_stream();
    tokio::io::copy_bidirectional(&mut local, &mut remote)
        .await
        .map_err(Error::Io)?;
    Ok(())
}

async fn bridge_socks5(
    mut local: TcpStream,
    peer: SocketAddr,
    handle: Arc<russh::client::Handle<SshClientHandler>>,
) -> Result<(), Error> {
    let (target_host, target_port) = socks5_handshake(&mut local).await?;
    let channel_result = handle
        .channel_open_direct_tcpip(
            target_host,
            u32::from(target_port),
            peer.ip().to_string(),
            u32::from(peer.port()),
        )
        .await;

    match channel_result {
        Ok(channel) => {
            // SOCKS5 success reply (REP=0x00, BND.ADDR=0.0.0.0:0).
            local
                .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await
                .map_err(Error::Io)?;
            let mut remote = channel.into_stream();
            tokio::io::copy_bidirectional(&mut local, &mut remote)
                .await
                .map_err(Error::Io)?;
            Ok(())
        }
        Err(e) => {
            // SOCKS5 general failure.
            let _ = local
                .write_all(&[0x05, 0x01, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await;
            Err(Error::Ssh(e))
        }
    }
}

// ---------------------------------------------------------------------------
// SOCKS5 (RFC 1928) — CONNECT only, no auth.
// ---------------------------------------------------------------------------

async fn socks5_handshake(s: &mut TcpStream) -> Result<(String, u16), Error> {
    // Greeting: [VER=5, NMETHODS, METHODS...]
    let mut head = [0u8; 2];
    s.read_exact(&mut head).await.map_err(Error::Io)?;
    if head[0] != 0x05 {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "not SOCKS5",
        )));
    }
    let mut methods = vec![0u8; head[1] as usize];
    s.read_exact(&mut methods).await.map_err(Error::Io)?;
    // Reply: pick "no auth" (0x00). If the client didn't offer it, reject.
    if !methods.contains(&0x00) {
        s.write_all(&[0x05, 0xFF]).await.map_err(Error::Io)?;
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "SOCKS5: no acceptable auth method",
        )));
    }
    s.write_all(&[0x05, 0x00]).await.map_err(Error::Io)?;

    // Request: [VER=5, CMD, RSV=0, ATYP, DST.ADDR, DST.PORT]
    let mut req = [0u8; 4];
    s.read_exact(&mut req).await.map_err(Error::Io)?;
    if req[0] != 0x05 {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "SOCKS5 request version",
        )));
    }
    if req[1] != 0x01 {
        // Only CONNECT supported; reply with command-not-supported.
        s.write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await
            .map_err(Error::Io)?;
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "SOCKS5: only CONNECT supported",
        )));
    }
    let host = match req[3] {
        0x01 => {
            let mut a = [0u8; 4];
            s.read_exact(&mut a).await.map_err(Error::Io)?;
            format!("{}.{}.{}.{}", a[0], a[1], a[2], a[3])
        }
        0x03 => {
            let mut len = [0u8; 1];
            s.read_exact(&mut len).await.map_err(Error::Io)?;
            let mut name = vec![0u8; len[0] as usize];
            s.read_exact(&mut name).await.map_err(Error::Io)?;
            String::from_utf8(name).map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "SOCKS5: non-UTF8 hostname",
                ))
            })?
        }
        0x04 => {
            let mut a = [0u8; 16];
            s.read_exact(&mut a).await.map_err(Error::Io)?;
            std::net::Ipv6Addr::from(a).to_string()
        }
        _ => {
            s.write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await
                .map_err(Error::Io)?;
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "SOCKS5: unsupported ATYP",
            )));
        }
    };
    let mut port_bytes = [0u8; 2];
    s.read_exact(&mut port_bytes).await.map_err(Error::Io)?;
    let port = u16::from_be_bytes(port_bytes);
    Ok((host, port))
}
