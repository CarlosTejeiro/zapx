//! SFTP wrapper over an established SSH session.
//!
//! Opens an SFTP subsystem channel through an [`Arc<SshHandle>`] and exposes a
//! small, app-friendly API (entries, stat, mkdir, rm, rename, upload/download)
//! that returns serializable types. The russh-sftp `SftpSession` is hidden
//! behind [`SftpClient`] so the app crate doesn't depend on russh-sftp.

use std::path::Path;
use std::sync::Arc;

use tokio::io::AsyncWriteExt;

use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileType;

use crate::error::Error;

/// One entry in a remote directory listing (mirrored 1:1 to TS).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SftpEntry {
    pub name: String,
    /// `"dir" | "file" | "symlink" | "other"`.
    pub kind: String,
    /// File size in bytes (None for some kinds).
    pub size: Option<u64>,
    /// POSIX permission bits (mode & 0o7777), if reported.
    pub permissions: Option<u32>,
    /// Last-modified time as Unix seconds (None if not reported).
    pub mtime: Option<u64>,
}

fn kind_str(t: FileType) -> &'static str {
    if t.is_dir() {
        "dir"
    } else if t.is_file() {
        "file"
    } else if t.is_symlink() {
        "symlink"
    } else {
        "other"
    }
}

fn map_err<E: std::fmt::Display>(e: E) -> Error {
    Error::Sftp(e.to_string())
}

/// App-facing SFTP client. Cheap to clone via `Arc<SftpClient>`.
///
/// All methods take `&self` so multiple Tauri commands can issue concurrent
/// requests through the same SFTP subsystem.
pub struct SftpClient {
    session: SftpSession,
}

impl SftpClient {
    /// Open an SFTP subsystem channel through the given SSH session handle and
    /// wrap it as an SFTP client.
    pub async fn open(
        handle: &russh::client::Handle<crate::ssh::SshClientHandler>,
    ) -> Result<Self, Error> {
        let channel = handle.channel_open_session().await.map_err(Error::Ssh)?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(Error::Ssh)?;
        let stream = channel.into_stream();
        let session = SftpSession::new(stream).await.map_err(map_err)?;
        Ok(Self { session })
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<SftpEntry>, Error> {
        let read_dir = self.session.read_dir(path).await.map_err(map_err)?;
        let mut entries = Vec::new();
        for entry in read_dir {
            let md = entry.metadata();
            entries.push(SftpEntry {
                name: entry.file_name(),
                kind: kind_str(entry.file_type()).to_string(),
                size: md.size,
                permissions: md.permissions.map(|p| p & 0o7777),
                mtime: md.mtime.map(u64::from),
            });
        }
        // Stable ordering: directories first, then files; case-insensitive name.
        entries.sort_by(|a, b| {
            let ad = a.kind == "dir";
            let bd = b.kind == "dir";
            bd.cmp(&ad)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(entries)
    }

    pub async fn stat(&self, path: &str) -> Result<SftpEntry, Error> {
        let md = self.session.metadata(path).await.map_err(map_err)?;
        let name = std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string());
        Ok(SftpEntry {
            name,
            kind: kind_str(md.file_type()).to_string(),
            size: md.size,
            permissions: md.permissions.map(|p| p & 0o7777),
            mtime: md.mtime.map(u64::from),
        })
    }

    /// Resolve `path` (which may be `"."` or relative) to an absolute path on the
    /// remote side. Useful to discover the user's `$HOME` at panel open time.
    pub async fn canonicalize(&self, path: &str) -> Result<String, Error> {
        self.session.canonicalize(path).await.map_err(map_err)
    }

    pub async fn create_dir(&self, path: &str) -> Result<(), Error> {
        self.session.create_dir(path).await.map_err(map_err)
    }

    pub async fn remove_dir(&self, path: &str) -> Result<(), Error> {
        self.session.remove_dir(path).await.map_err(map_err)
    }

    pub async fn remove_file(&self, path: &str) -> Result<(), Error> {
        self.session.remove_file(path).await.map_err(map_err)
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<(), Error> {
        self.session.rename(from, to).await.map_err(map_err)
    }

    /// Download a remote file to `local_path`. Returns the byte count.
    /// Small/medium files only — buffers in memory. Streaming + progress is a
    /// follow-up.
    pub async fn download(&self, remote_path: &str, local_path: &Path) -> Result<u64, Error> {
        let bytes = self.session.read(remote_path).await.map_err(map_err)?;
        let mut f = tokio::fs::File::create(local_path)
            .await
            .map_err(Error::Io)?;
        f.write_all(&bytes).await.map_err(Error::Io)?;
        f.flush().await.map_err(Error::Io)?;
        Ok(bytes.len() as u64)
    }

    /// Upload `local_path` to the remote `remote_path`. Returns the byte count.
    pub async fn upload(&self, local_path: &Path, remote_path: &str) -> Result<u64, Error> {
        let bytes = tokio::fs::read(local_path).await.map_err(Error::Io)?;
        let len = bytes.len() as u64;
        self.session
            .write(remote_path, &bytes)
            .await
            .map_err(map_err)?;
        Ok(len)
    }
}

/// Convenient handle stored on `ActiveSession` for lazy SFTP init.
pub type SftpSlot = Arc<tokio::sync::Mutex<Option<Arc<SftpClient>>>>;

/// Build an empty lazy slot — call this when constructing an SSH `ActiveSession`.
pub fn empty_slot() -> SftpSlot {
    Arc::new(tokio::sync::Mutex::new(None))
}
