use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem as _};

use crate::error::Error;

/// A local pseudo-terminal connected to the system shell.
///
/// Created via [`LocalPty::spawn`]. Dropping it tears down the PTY and the child process.
pub struct LocalPty {
    master: Box<dyn portable_pty::MasterPty + Send>,
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
        if let Some(home) = home::home_dir() {
            cmd.cwd(home);
        }
        // Force TERM so shells launched from the macOS .app bundle (which
        // inherits a sparse env from LaunchServices) still load the right
        // terminfo. Without this, zsh-syntax-highlighting + auto-suggestions
        // repaint the input line without proper cursor positioning and the
        // user sees doubled characters (typed `ls` shows as `lls`).
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        // Self-identification so user prompts / scripts can detect the host.
        cmd.env("TERM_PROGRAM", "zapx");
        cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| Error::SpawnFailed(e.to_string()))?;
        // Close our copy of the slave end: only the child may hold it, so that
        // reads on the master end (EOF / EIO) when the shell exits. Keeping it
        // would leave the reader blocked forever after `exit`.
        let portable_pty::PtyPair { master, slave } = pair;
        drop(slave);

        let writer = master
            .take_writer()
            .map_err(|e| Error::SpawnFailed(e.to_string()))?;

        Ok(Self {
            master,
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
        self.master
            .try_clone_reader()
            .map_err(|e| Error::SpawnFailed(e.to_string()))
    }

    /// A cloneable handle for writing bytes into the PTY (keyboard input).
    pub fn writer(&self) -> Arc<Mutex<Box<dyn Write + Send>>> {
        Arc::clone(&self.writer)
    }

    /// Resize the PTY window and update stored dimensions.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), Error> {
        self.master
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

    /// A handle that can kill the child from another thread (the reader loop
    /// owns the `LocalPty` itself while it blocks on reads).
    pub fn killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        self.child.clone_killer()
    }

    /// Kill the child process. Idempotent.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
    }
}

fn detect_shell() -> String {
    #[cfg(target_os = "windows")]
    {
        // PowerShell 7 (pwsh) when installed, else Windows PowerShell, else
        // the configured command interpreter (cmd.exe).
        let path = std::env::var_os("PATH").unwrap_or_default();
        first_on_path(&path, &["pwsh.exe", "powershell.exe"])
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into())
    }
}

/// The first of `names` found in a directory of `path` (a PATH-style list),
/// trying each name in order across every directory.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn first_on_path(path: &std::ffi::OsStr, names: &[&str]) -> Option<std::path::PathBuf> {
    let dirs: Vec<std::path::PathBuf> = std::env::split_paths(path).collect();
    names.iter().find_map(|name| {
        dirs.iter()
            .map(|d| d.join(name))
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::first_on_path;

    #[test]
    fn prefers_the_first_name_over_directory_order() {
        let base = std::env::temp_dir().join(format!("zapx_path_{}", std::process::id()));
        let (a, b) = (base.join("a"), base.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("powershell.exe"), b"").unwrap();
        std::fs::write(b.join("pwsh.exe"), b"").unwrap();
        let path = std::env::join_paths([&a, &b]).unwrap();
        // pwsh wins although its directory comes later in PATH.
        assert_eq!(
            first_on_path(&path, &["pwsh.exe", "powershell.exe"]),
            Some(b.join("pwsh.exe"))
        );
        assert_eq!(first_on_path(&path, &["nothing.exe"]), None);
        let _ = std::fs::remove_dir_all(&base);
    }
}
