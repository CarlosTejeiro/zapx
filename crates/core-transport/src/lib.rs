#![forbid(unsafe_code)]

//! `core-transport` — the [`Transport`] trait and per-protocol implementations.
//!
//! This crate owns the boundary between session management and the underlying
//! network/serial protocols. Protocol-specific code must not leak past this crate's
//! public API (see style guide §2, API design).

pub mod error;
pub mod forwards;
pub mod local_pty;
pub mod serial;
pub mod sftp;
pub mod ssh;
pub mod telnet;

pub use forwards::{open_dynamic_forward, open_local_forward, ForwardController, ForwardInfo};
pub use local_pty::LocalPty;
pub use serial::{list_ports as list_serial_ports, SerialTransport};
pub use sftp::{empty_slot as empty_sftp_slot, SftpClient, SftpEntry, SftpSlot};
pub use ssh::{
    connect_authenticated, connect_authenticated_via, preflight_host_key, trust_host_key,
    HostKeyStatus, KiPrompt, KiRequest, KiResponder, SshAuth, SshTransport,
};

/// Type alias for the authenticated SSH session handle that the app layer
/// passes around as a bastion link in ProxyJump chains.
pub type SshHandle = russh::client::Handle<ssh::SshClientHandler>;
pub use telnet::TelnetTransport;

/// Commands sent to a live session's I/O task.
#[derive(Debug)]
pub enum SessionCmd {
    Data(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

/// A bidirectional byte stream with session lifecycle management.
///
/// Each protocol (SSH, Telnet, Serial, Local PTY) provides its own implementation.
/// The trait is the only API surface that the rest of the application sees;
/// protocol-specific types stay private to this crate.
///
/// # Cancellation safety
///
/// Every `async fn` in this trait must be cancellation-safe: dropping the future
/// mid-await must leave the world in a consistent state.
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Establish the connection. Idempotent if already connected.
    async fn connect(&mut self) -> Result<(), error::Error>;

    /// Tear down the connection gracefully.
    async fn disconnect(&mut self) -> Result<(), error::Error>;

    /// Notify the remote end of a terminal resize.
    async fn resize(&mut self, cols: u16, rows: u16) -> Result<(), error::Error>;
}
