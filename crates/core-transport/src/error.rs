/// Errors produced by [`core_transport`](crate).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("failed to spawn shell process: {0}")]
    SpawnFailed(String),

    #[error("I/O closed unexpectedly")]
    IoClosed,

    #[error("failed to resize terminal: {0}")]
    ResizeFailed(String),

    #[error("not connected")]
    NotConnected,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),

    #[error("authentication failed")]
    AuthFailed,
}
