//! Automated expect/send login scripts.
//!
//! A [`LoginRunner`] watches the raw output of a session and, when it spots
//! the current step's `expect` pattern, sends the corresponding bytes via the
//! session's [`SessionCmd::Data`] command channel. It then arms the next step
//! with its own timeout. The matcher strips ANSI escape sequences before
//! searching so coloured prompts don't break literal patterns.
//!
//! The runner is fed synchronously from inside `make_on_data` (see
//! `commands/sessions.rs`), which already lives in the per-session I/O task.
//! A small mutex guards the buffer + step index; matching is a memmem-style
//! substring scan, so the data callback stays fast.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::UnboundedSender;

use core_transport::SessionCmd;

/// One step of a login script.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoginStep {
    /// Pattern to wait for in the (ANSI-stripped) output stream.
    pub expect: String,
    /// If true, `expect` should be treated as a regex (currently not yet wired
    /// — falls back to literal matching). Kept on the wire so the UI / DB
    /// schema is forward-compatible.
    #[serde(default)]
    pub is_regex: bool,
    /// Bytes to send when `expect` matches (interpreted as UTF-8).
    pub send: String,
    /// How long to wait for this step before giving up (milliseconds).
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    10_000
}

/// Status broadcast to the frontend so the terminal toolbar can show progress.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LoginProgress {
    pub session_id: String,
    /// 1-based index of the step just executed (`0` means "still running first"); equal to `total` when complete.
    pub current: usize,
    pub total: usize,
    /// `"running"`, `"complete"`, or `"timeout"`.
    pub status: &'static str,
}

const MAX_BUF: usize = 64 * 1024;

pub struct LoginRunner {
    steps: Vec<LoginStep>,
    cmd_tx: UnboundedSender<SessionCmd>,
    inner: Mutex<RunnerInner>,
    on_progress: Box<dyn Fn(LoginProgress) + Send + Sync>,
    session_id: String,
}

struct RunnerInner {
    index: usize,
    buffer: Vec<u8>,
    deadline: Instant,
}

impl LoginRunner {
    pub fn new(
        session_id: String,
        steps: Vec<LoginStep>,
        cmd_tx: UnboundedSender<SessionCmd>,
        on_progress: Box<dyn Fn(LoginProgress) + Send + Sync>,
    ) -> Self {
        let first_timeout = steps.first().map(|s| s.timeout_ms).unwrap_or(10_000);
        let total = steps.len();
        let runner = Self {
            steps,
            cmd_tx,
            session_id: session_id.clone(),
            on_progress,
            inner: Mutex::new(RunnerInner {
                index: 0,
                buffer: Vec::with_capacity(8192),
                deadline: Instant::now() + Duration::from_millis(first_timeout),
            }),
        };
        (runner.on_progress)(LoginProgress {
            session_id,
            current: 0,
            total,
            status: "running",
        });
        runner
    }

    /// Feed a chunk of session output to the runner. Cheap when the script
    /// is already complete (early return).
    pub fn feed(&self, data: &[u8]) {
        let mut inner = self.inner.lock().unwrap();
        if inner.index >= self.steps.len() {
            return;
        }

        // Deadline check: a silent server still expires the current step.
        if Instant::now() > inner.deadline {
            tracing::warn!(
                session_id = %self.session_id,
                step = inner.index,
                "login script: step timed out"
            );
            let current = inner.index;
            inner.index = self.steps.len();
            (self.on_progress)(LoginProgress {
                session_id: self.session_id.clone(),
                current,
                total: self.steps.len(),
                status: "timeout",
            });
            return;
        }

        // Append + cap the buffer so a server flood can't make the matcher
        // walk megabytes per chunk.
        inner.buffer.extend_from_slice(data);
        if inner.buffer.len() > MAX_BUF {
            let drop = inner.buffer.len() - MAX_BUF;
            inner.buffer.drain(0..drop);
        }

        let stripped = strip_ansi(&inner.buffer);
        let step = &self.steps[inner.index];
        // is_regex flag honoured at the schema level; literal-only for v1.
        if !step.expect.is_empty() && stripped.contains(step.expect.as_str()) {
            let _ = self
                .cmd_tx
                .send(SessionCmd::Data(step.send.as_bytes().to_vec()));
            inner.index += 1;
            inner.buffer.clear();
            if inner.index < self.steps.len() {
                inner.deadline =
                    Instant::now() + Duration::from_millis(self.steps[inner.index].timeout_ms);
                (self.on_progress)(LoginProgress {
                    session_id: self.session_id.clone(),
                    current: inner.index,
                    total: self.steps.len(),
                    status: "running",
                });
            } else {
                (self.on_progress)(LoginProgress {
                    session_id: self.session_id.clone(),
                    current: inner.index,
                    total: self.steps.len(),
                    status: "complete",
                });
            }
        }
    }
}

/// Strip ANSI CSI / OSC escape sequences so literal patterns match coloured
/// prompts. Returns a lossy UTF-8 string suitable for `String::contains`.
fn strip_ansi(bytes: &[u8]) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == 0x1b && i + 1 < bytes.len() {
            let next = bytes[i + 1];
            if next == b'[' {
                // CSI: ESC '[' <params> <final 0x40-0x7e>
                i += 2;
                while i < bytes.len() && !(bytes[i] >= 0x40 && bytes[i] <= 0x7e) {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
                continue;
            }
            if next == b']' {
                // OSC: ESC ']' ... (BEL | ESC '\')
                i += 2;
                while i < bytes.len() {
                    if bytes[i] == 0x07 {
                        i += 1;
                        break;
                    }
                    if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            // ESC + single byte (unsupported / 2-byte sequence) — drop both.
            i += 2;
            continue;
        }
        out.push(b);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_ansi_basics() {
        let s = strip_ansi(b"\x1b[32mhello\x1b[0m world");
        assert_eq!(s, "hello world");
    }

    #[test]
    fn strip_ansi_osc() {
        let s = strip_ansi(b"\x1b]0;title\x07Plain text");
        assert_eq!(s, "Plain text");
    }
}
