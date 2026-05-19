use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem as _};

use crate::error::Error;

/// A local pseudo-terminal connected to the system shell.
///
/// Created via [`LocalPty::spawn`]. Dropping it tears down the PTY and the child process.
pub struct LocalPty {
    pair: portable_pty::PtyPair,
    child: Box<dyn portable_pty::Child + Send>,
    /// Shared writer so it can be held by the session while reading runs in a task.
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    cols: u16,
    rows: u16,
}

impl LocalPty {
    /// Spawn a PTY with the system shell at the given initial dimensions.
    pub fn spawn(cols: u16, rows: u16) -> Result<Self, Error> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::SpawnFailed(e.to_string()))?;

        let shell = detect_shell();
        let mut cmd = CommandBuilder::new(&shell);
        if let Some(home) = home_dir() {
            cmd.cwd(home);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| Error::SpawnFailed(e.to_string()))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| Error::SpawnFailed(e.to_string()))?;

        Ok(Self {
            pair,
            child,
            writer: Arc::new(Mutex::new(writer)),
            cols,
            rows,
        })
    }

    /// Clone the reader end of the PTY.
    ///
    /// The returned reader is blocking; run it inside `tokio::task::spawn_blocking`.
    pub fn take_reader(&self) -> Result<Box<dyn Read + Send>, Error> {
        self.pair
            .master
            .try_clone_reader()
            .map_err(|e| Error::SpawnFailed(e.to_string()))
    }

    /// A cloneable handle for writing bytes into the PTY (keyboard input).
    pub fn writer(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        Arc::clone(&self.writer)
    }

    /// Resize the PTY window and update stored dimensions.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        self.pair
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| Error::ResizeFailed(e.to_string()))?;
        self.cols = cols;
        self.rows = rows;
        Ok(())
    }

    /// Returns true if the child process has exited.
    pub fn is_done(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    /// Kill the child process. Idempotent.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

fn detect_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into())
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(Into::into)
}
