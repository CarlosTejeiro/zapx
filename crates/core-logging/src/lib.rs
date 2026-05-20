#![forbid(unsafe_code)]

//! `core-logging` — per-session loggers with size-based rotation.
//!
//! Each `SessionLogger` writes raw terminal bytes to a file in the app log
//! directory. When the file reaches 50 MB a new part file is opened
//! transparently (e.g., `session_20240101_120000.log` →
//! `session_20240101_120000.1.log`).

pub mod error;
pub use error::Error;

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};

const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB per part
const BUF_CAP: usize = 64 * 1024; // 64 KB write buffer

pub struct SessionLogger {
    writer: BufWriter<File>,
    path: PathBuf,
    log_dir: PathBuf,
    base_name: String,
    part: u32,
    bytes_written: u64,
    started_at: DateTime<Utc>,
}

impl SessionLogger {
    /// Open a new log file inside `log_dir`.
    ///
    /// `base_name` becomes the filename prefix (invalid filesystem chars are
    /// replaced with `_`). The actual filename is
    /// `<base_name>_YYYYMMDD_HHMMSS.log`.
    pub fn open(log_dir: &Path, base_name: &str) -> Result<Self, Error> {
        std::fs::create_dir_all(log_dir)?;
        let safe_name = sanitize(base_name);
        let (file, path) = open_part(log_dir, &safe_name, 0)?;
        Ok(Self {
            writer: BufWriter::with_capacity(BUF_CAP, file),
            path,
            log_dir: log_dir.to_owned(),
            base_name: safe_name,
            part: 0,
            bytes_written: 0,
            started_at: Utc::now(),
        })
    }

    /// Write raw terminal bytes. Rotates to a new part file when ≥50 MB.
    pub fn write(&mut self, data: &[u8]) -> Result<(), Error> {
        if data.is_empty() {
            return Ok(());
        }
        self.writer.write_all(data)?;
        self.bytes_written += data.len() as u64;

        if self.bytes_written >= MAX_FILE_BYTES {
            self.writer.flush()?;
            let next = self.part + 1;
            let (new_file, new_path) = open_part(&self.log_dir, &self.base_name, next)?;
            self.writer = BufWriter::with_capacity(BUF_CAP, new_file);
            self.path = new_path;
            self.part = next;
            self.bytes_written = 0;
        }
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Flush and consume the logger. Returns `(path, total_bytes, started_at)`.
    pub fn close(mut self) -> Result<(PathBuf, u64, DateTime<Utc>), Error> {
        self.writer.flush()?;
        Ok((self.path, self.bytes_written, self.started_at))
    }
}

fn open_part(dir: &Path, base: &str, part: u32) -> Result<(File, PathBuf), Error> {
    let ts = Local::now().format("%Y%m%d_%H%M%S");
    let name = if part == 0 {
        format!("{base}_{ts}.log")
    } else {
        format!("{base}_{ts}.{part}.log")
    };
    let path = dir.join(name);
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((file, path))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn write_and_close() {
        let dir = std::env::temp_dir().join("zapx_log_test");
        let _ = fs::remove_dir_all(&dir);
        let mut logger = SessionLogger::open(&dir, "test-session").unwrap();
        logger.write(b"hello world\n").unwrap();
        assert_eq!(logger.bytes_written(), 12);
        let (path, bytes, _) = logger.close().unwrap();
        assert!(path.exists());
        assert_eq!(bytes, 12);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanitize_name() {
        assert_eq!(sanitize("my session/test:1"), "my_session_test_1");
    }
}
