//! Resolution of the ZAPX data directory — where `zapx.db`, `session_logs/`
//! and `catalogs/` live.
//!
//! Priority order (first hit wins):
//!
//! 1. `--data-dir <path>` CLI flag (also `--data-dir=<path>`)
//! 2. `ZAPX_DATA_DIR` environment variable
//! 3. Portable marker: a file named `portable` (or `portable.txt`) next to
//!    the executable → data lives in `<exe_dir>/data/` and travels with it
//! 4. Pointer file `data-dir.txt` in the OS config dir, written by the
//!    "Data folder" setting in the UI
//! 5. The platform default (`app_data_dir()`)
//!
//! The vault seed normally derives from the data dir's absolute path. In
//! portable mode that path changes between machines (drive letters, mount
//! points), so a stable constant seed is used instead — equivalent security
//! (the seed only obfuscates the SQLite fallback secrets; the OS keyring
//! remains the primary store outside portable mode).

use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;

/// Name of the pointer file (lives in the OS config dir, NOT the data dir,
/// so it can be found before the data dir is known).
pub const POINTER_FILE: &str = "data-dir.txt";

/// Stable vault seed for portable installs (see module docs).
pub const PORTABLE_VAULT_SEED: &str = "zapx-portable-v1";

/// How the data dir was chosen — surfaced in Settings so the user can see
/// why the app is reading from a given location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    CliFlag,
    EnvVar,
    Portable,
    Pointer,
    Default,
}

#[derive(Debug, Clone, Serialize)]
pub struct DataDir {
    pub dir: PathBuf,
    pub source: Source,
}

impl DataDir {
    pub fn is_portable(&self) -> bool {
        self.source == Source::Portable
    }

    /// Seed for the AES-256-GCM `session_secrets` fallback.
    pub fn vault_seed(&self) -> String {
        if self.is_portable() {
            PORTABLE_VAULT_SEED.to_owned()
        } else {
            self.dir.to_string_lossy().into_owned()
        }
    }
}

/// Resolve the data dir per the priority order in the module docs.
pub fn resolve(app: &tauri::AppHandle) -> tauri::Result<DataDir> {
    if let Some(dir) = from_cli_flag() {
        return Ok(DataDir {
            dir,
            source: Source::CliFlag,
        });
    }
    if let Some(dir) = from_env() {
        return Ok(DataDir {
            dir,
            source: Source::EnvVar,
        });
    }
    if let Some(dir) = from_portable_marker() {
        return Ok(DataDir {
            dir,
            source: Source::Portable,
        });
    }
    if let Some(dir) = from_pointer_file(app) {
        return Ok(DataDir {
            dir,
            source: Source::Pointer,
        });
    }
    Ok(DataDir {
        dir: app.path().app_data_dir()?,
        source: Source::Default,
    })
}

fn from_cli_flag() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(v) = arg.strip_prefix("--data-dir=") {
            if !v.is_empty() {
                return Some(PathBuf::from(v));
            }
        } else if arg == "--data-dir" {
            return args.next().filter(|v| !v.is_empty()).map(PathBuf::from);
        }
    }
    None
}

fn from_env() -> Option<PathBuf> {
    std::env::var("ZAPX_DATA_DIR")
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn from_portable_marker() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    let marked = exe_dir.join("portable").is_file() || exe_dir.join("portable.txt").is_file();
    marked.then(|| exe_dir.join("data"))
}

fn from_pointer_file(app: &tauri::AppHandle) -> Option<PathBuf> {
    let pointer = pointer_path(app)?;
    let contents = std::fs::read_to_string(pointer).ok()?;
    let path = contents.trim();
    if path.is_empty() {
        return None;
    }
    let dir = PathBuf::from(path);
    // A stale pointer (folder deleted, drive unplugged) must not brick the
    // app — fall back to the default dir and let the user re-point it.
    if dir.is_dir() {
        Some(dir)
    } else {
        tracing::warn!("data-dir pointer targets missing dir {path:?}; using default");
        None
    }
}

/// Absolute path of the pointer file in the OS config dir.
pub fn pointer_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|d| d.join(POINTER_FILE))
}
