#![forbid(unsafe_code)]

//! `core-persistence` — SQLite storage and schema migrations.
//!
//! Credentials are never stored here; only opaque keyring references
//! live in the database (see `core-vault`).

pub mod error;

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

pub use error::Error;

// ---------------------------------------------------------------------------
// Public model types (serialisable for the Tauri bridge)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColorScheme {
    pub id: i64,
    pub name: String,
    pub palette_json: String,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionLog {
    pub id: i64,
    pub session_id: Option<i64>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub file_path: String,
    pub bytes: i64,
    pub format: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Folder {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
    pub sort_order: i32,
}

/// A named, ordered set of saved sessions opened together for multi-host
/// orchestration. `session_ids` is the ordered membership (saved-session ids).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BroadcastGroup {
    pub id: i64,
    pub name: String,
    pub sort_order: i32,
    pub session_ids: Vec<i64>,
}

/// A port-forward saved on a session, auto-started when it connects.
/// `target_*` are `None` for dynamic (SOCKS5) forwards.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedForward {
    /// "local" (-L), "dynamic" (-D, SOCKS5) or "remote" (-R).
    pub kind: String,
    pub bind_addr: String,
    pub bind_port: u16,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub sort_order: i64,
    pub created_at: String,
    /// `None` = global (visible in every session). `Some(p)` = scoped to a
    /// single platform (matching the `core_hints::Platform::as_str()` form).
    pub platform: Option<String>,
    /// Optional accent color (hex) for the button-bar tile. `None` = default
    /// theme tint.
    pub color: Option<String>,
}

/// One row from `command_history`, used by the hint engine for
/// frecency-scored suggestions.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandHistoryEntry {
    pub command: String,
    pub freq: i64,
    pub last_used: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HighlightRule {
    pub id: i64,
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub fg_color: Option<String>,
    pub bg_color: Option<String>,
    pub bold: bool,
    pub underline: bool,
    pub enabled: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedSession {
    pub id: i64,
    pub folder_id: Option<i64>,
    pub name: String,
    pub protocol: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub credential_id: Option<i64>,
    pub options_json: String,
    pub last_used_at: Option<String>,
    /// SSH auth method: NULL/"password" | "key" | "agent".
    pub auth_method: Option<String>,
    /// Optional ProxyJump: another saved session whose live SSH connection is
    /// used as the transport for this session (via a `direct-tcpip` channel).
    pub via_session_id: Option<i64>,
    /// Optional JSON array of `{expect, send, is_regex, timeout_ms}` login
    /// steps run automatically when the session is opened. NULL = no automation.
    pub login_script_json: Option<String>,
    /// Sort order inside the row's parent (folder or root). Migration 011
    /// seeded existing rows with their `id` so the previous "newest first"
    /// order is roughly preserved until the user re-arranges.
    pub sort_order: i64,
}

// ---------------------------------------------------------------------------
// Database handle
// ---------------------------------------------------------------------------

// rusqlite::Connection is Send; Mutex<Connection> is therefore Send + Sync.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the SQLite database at `path` and apply migrations.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        // Enable WAL for better concurrent reads.
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.run_migrations()?;
        Ok(db)
    }

    /// Write a clean, single-file snapshot of the live database to `dest`
    /// via `VACUUM INTO` — safe while the connection is open (no WAL/SHM
    /// sidecar files to copy). `dest` must not already exist.
    pub fn vacuum_into(&self, dest: &Path) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "VACUUM INTO ?1",
            rusqlite::params![dest.to_string_lossy().into_owned()],
        )?;
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version),0) FROM schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if version < 1 {
            conn.execute_batch(include_str!("migrations/001_initial.sql"))?;
        }
        if version < 2 {
            conn.execute_batch(include_str!("migrations/002_highlight_rules.sql"))?;
        }
        if version < 3 {
            conn.execute_batch(include_str!("migrations/003_session_logs.sql"))?;
        }
        if version < 4 {
            conn.execute_batch(include_str!("migrations/004_appearance.sql"))?;
        }
        if version < 5 {
            conn.execute_batch(include_str!("migrations/005_ssh_key_auth.sql"))?;
        }
        if version < 6 {
            conn.execute_batch(include_str!("migrations/006_jump_host.sql"))?;
        }
        if version < 7 {
            conn.execute_batch(include_str!("migrations/007_snippets.sql"))?;
        }
        if version < 8 {
            conn.execute_batch(include_str!("migrations/008_login_script.sql"))?;
        }
        if version < 9 {
            conn.execute_batch(include_str!("migrations/009_hints.sql"))?;
        }
        if version < 10 {
            conn.execute_batch(include_str!("migrations/010_session_secrets.sql"))?;
        }
        if version < 11 {
            conn.execute_batch(include_str!("migrations/011_session_order.sql"))?;
        }
        if version < 12 {
            conn.execute_batch(include_str!("migrations/012_snippet_platform.sql"))?;
        }
        if version < 13 {
            conn.execute_batch(include_str!("migrations/013_broadcast_groups.sql"))?;
        }
        if version < 14 {
            conn.execute_batch(include_str!("migrations/014_snippet_color.sql"))?;
        }
        if version < 15 {
            conn.execute_batch(include_str!("migrations/015_session_forwards.sql"))?;
        }
        if version < 16 {
            conn.execute_batch(include_str!("migrations/016_command_transitions.sql"))?;
        }
        if version < 17 {
            conn.execute_batch(include_str!("migrations/017_session_triggers.sql"))?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Folder CRUD
    // -----------------------------------------------------------------------

    pub fn create_folder(&self, name: &str, parent_id: Option<i64>) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO folders (name, parent_id) VALUES (?1, ?2)",
            rusqlite::params![name, parent_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_folders(&self) -> Result<Vec<Folder>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, parent_id, name, sort_order FROM folders ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Folder {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn rename_folder(&self, id: i64, name: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE folders SET name=?1, updated_at=datetime('now') WHERE id=?2",
            rusqlite::params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_folder(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM folders WHERE id=?1", rusqlite::params![id])?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Broadcast group CRUD (multi-host orchestration)
    // -----------------------------------------------------------------------

    pub fn create_broadcast_group(&self, name: &str) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO broadcast_groups (name) VALUES (?1)",
            rusqlite::params![name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// All groups with their ordered membership. Members are returned in their
    /// per-group `sort_order` so the grid lays them out predictably.
    pub fn list_broadcast_groups(&self) -> Result<Vec<BroadcastGroup>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, sort_order FROM broadcast_groups ORDER BY sort_order, name",
        )?;
        let groups = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut member_stmt = conn.prepare(
            "SELECT session_id FROM broadcast_group_members WHERE group_id=?1 ORDER BY sort_order",
        )?;
        let mut out = Vec::with_capacity(groups.len());
        for (id, name, sort_order) in groups {
            let session_ids = member_stmt
                .query_map(rusqlite::params![id], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            out.push(BroadcastGroup {
                id,
                name,
                sort_order,
                session_ids,
            });
        }
        Ok(out)
    }

    pub fn rename_broadcast_group(&self, id: i64, name: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE broadcast_groups SET name=?1, updated_at=datetime('now') WHERE id=?2",
            rusqlite::params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_broadcast_group(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        // Members cascade via the FK ON DELETE CASCADE.
        conn.execute(
            "DELETE FROM broadcast_groups WHERE id=?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    /// Replace a group's membership wholesale with `session_ids`, preserving
    /// the given order via `sort_order`. Idempotent — safe to call on every
    /// edit of the group in the UI.
    pub fn set_broadcast_group_members(
        &self,
        group_id: i64,
        session_ids: &[i64],
    ) -> Result<(), Error> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM broadcast_group_members WHERE group_id=?1",
            rusqlite::params![group_id],
        )?;
        for (idx, sid) in session_ids.iter().enumerate() {
            tx.execute(
                "INSERT INTO broadcast_group_members (group_id, session_id, sort_order)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![group_id, sid, idx as i64],
            )?;
        }
        tx.execute(
            "UPDATE broadcast_groups SET updated_at=datetime('now') WHERE id=?1",
            rusqlite::params![group_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Session CRUD
    // -----------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub fn create_session(
        &self,
        folder_id: Option<i64>,
        name: &str,
        protocol: &str,
        host: Option<&str>,
        port: Option<u16>,
        username: Option<&str>,
        credential_id: Option<i64>,
        options_json: &str,
    ) -> Result<i64, Error> {
        self.create_session_full(
            folder_id,
            name,
            protocol,
            host,
            port,
            username,
            credential_id,
            options_json,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_session_full(
        &self,
        folder_id: Option<i64>,
        name: &str,
        protocol: &str,
        host: Option<&str>,
        port: Option<u16>,
        username: Option<&str>,
        credential_id: Option<i64>,
        options_json: &str,
        auth_method: Option<&str>,
        via_session_id: Option<i64>,
    ) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions
             (folder_id, name, protocol, host, port, username, credential_id, options_json, auth_method, via_session_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                folder_id,
                name,
                protocol,
                host,
                port.map(|p| p as i64),
                username,
                credential_id,
                options_json,
                auth_method,
                via_session_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_sessions(&self) -> Result<Vec<SavedSession>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, name, protocol, host, port, username, credential_id,
                    options_json, last_used_at, auth_method, via_session_id, login_script_json,
                    sort_order
             FROM sessions
             ORDER BY folder_id IS NULL DESC, folder_id, sort_order, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SavedSession {
                id: row.get(0)?,
                folder_id: row.get(1)?,
                name: row.get(2)?,
                protocol: row.get(3)?,
                host: row.get(4)?,
                port: row.get::<_, Option<i64>>(5)?.map(|p| p as u16),
                username: row.get(6)?,
                credential_id: row.get(7)?,
                options_json: row.get(8)?,
                last_used_at: row.get(9)?,
                auth_method: row.get(10)?,
                via_session_id: row.get(11)?,
                login_script_json: row.get(12)?,
                sort_order: row.get(13)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_session(&self, id: i64) -> Result<SavedSession, Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, folder_id, name, protocol, host, port, username, credential_id,
                    options_json, last_used_at, auth_method, via_session_id, login_script_json,
                    sort_order
             FROM sessions WHERE id=?1",
            rusqlite::params![id],
            |row| {
                Ok(SavedSession {
                    id: row.get(0)?,
                    folder_id: row.get(1)?,
                    name: row.get(2)?,
                    protocol: row.get(3)?,
                    host: row.get(4)?,
                    port: row.get::<_, Option<i64>>(5)?.map(|p| p as u16),
                    username: row.get(6)?,
                    credential_id: row.get(7)?,
                    options_json: row.get(8)?,
                    last_used_at: row.get(9)?,
                    auth_method: row.get(10)?,
                    via_session_id: row.get(11)?,
                    login_script_json: row.get(12)?,
                    sort_order: row.get(13)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
            e => Error::Rusqlite(e),
        })
    }

    /// Reparent a session to a folder (or `None` for the root). Used by the
    /// session-tree drag-and-drop.
    pub fn set_session_folder(&self, id: i64, folder_id: Option<i64>) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE sessions SET folder_id = ?1 WHERE id = ?2",
            rusqlite::params![folder_id, id],
        )?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    /// Move `id` to position `target_index` inside `folder_id` (None = root).
    /// All rows currently in that container are re-numbered 0..N-1 so the
    /// next drag has stable, predictable inputs.
    ///
    /// Also updates `folder_id` if it differs from the row's current value —
    /// that way a single call handles both "drop into folder" and "reorder
    /// within folder" without needing the caller to issue two updates.
    pub fn reorder_session(
        &self,
        id: i64,
        folder_id: Option<i64>,
        target_index: usize,
    ) -> Result<(), Error> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        // Verify the row exists and pull its current container so we know
        // whether this is also a folder change.
        let current_folder: Option<i64> = tx
            .query_row(
                "SELECT folder_id FROM sessions WHERE id = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
                e => Error::Rusqlite(e),
            })?;

        // Collect every other row currently in the destination, ordered by
        // existing sort_order. We rebuild the order around the inserted row.
        let mut peers: Vec<i64> = Vec::new();
        match folder_id {
            Some(fid) => {
                let mut stmt = tx.prepare(
                    "SELECT id FROM sessions WHERE folder_id = ?1 AND id != ?2
                     ORDER BY sort_order, name",
                )?;
                let rows = stmt.query_map(rusqlite::params![fid, id], |r| r.get::<_, i64>(0))?;
                for row in rows {
                    peers.push(row?);
                }
            }
            None => {
                let mut stmt = tx.prepare(
                    "SELECT id FROM sessions WHERE folder_id IS NULL AND id != ?1
                     ORDER BY sort_order, name",
                )?;
                let rows = stmt.query_map(rusqlite::params![id], |r| r.get::<_, i64>(0))?;
                for row in rows {
                    peers.push(row?);
                }
            }
        }
        let clamped = target_index.min(peers.len());
        peers.insert(clamped, id);

        // Apply folder change (if any) before renumbering — keeps the
        // sort_order writes deterministic.
        if current_folder != folder_id {
            tx.execute(
                "UPDATE sessions SET folder_id = ?1 WHERE id = ?2",
                rusqlite::params![folder_id, id],
            )?;
        }

        for (i, rid) in peers.iter().enumerate() {
            tx.execute(
                "UPDATE sessions SET sort_order = ?1 WHERE id = ?2",
                rusqlite::params![i as i64, rid],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Update the user-editable fields of a saved session. Auth method and
    /// credential reference are intentionally NOT updated here — to change
    /// credentials the user re-creates the session in v1.
    #[allow(clippy::too_many_arguments)]
    pub fn update_session_fields(
        &self,
        id: i64,
        name: &str,
        folder_id: Option<i64>,
        host: Option<&str>,
        port: Option<u16>,
        username: Option<&str>,
        via_session_id: Option<i64>,
        options_json: Option<&str>,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE sessions
             SET name = ?1,
                 folder_id = ?2,
                 host = ?3,
                 port = ?4,
                 username = ?5,
                 via_session_id = ?6,
                 options_json = COALESCE(?7, options_json)
             WHERE id = ?8",
            rusqlite::params![
                name,
                folder_id,
                host,
                port.map(|p| p as i64),
                username,
                via_session_id,
                options_json,
                id,
            ],
        )?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    /// List the port-forwards saved on a session, in display order.
    pub fn list_session_forwards(&self, session_id: i64) -> Result<Vec<SavedForward>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT kind, bind_addr, bind_port, target_host, target_port
             FROM session_forwards WHERE session_id = ?1 ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok(SavedForward {
                kind: row.get(0)?,
                bind_addr: row.get(1)?,
                bind_port: row.get::<_, i64>(2)? as u16,
                target_host: row.get(3)?,
                target_port: row.get::<_, Option<i64>>(4)?.map(|p| p as u16),
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Replace the saved port-forwards of a session (delete-then-insert).
    pub fn set_session_forwards(
        &self,
        session_id: i64,
        forwards: &[SavedForward],
    ) -> Result<(), Error> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "DELETE FROM session_forwards WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        for (idx, f) in forwards.iter().enumerate() {
            tx.execute(
                "INSERT INTO session_forwards
                 (session_id, kind, bind_addr, bind_port, target_host, target_port, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    session_id,
                    f.kind,
                    f.bind_addr,
                    f.bind_port as i64,
                    f.target_host,
                    f.target_port.map(|p| p as i64),
                    idx as i64,
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Set (or clear) the ProxyJump bastion of a session. Used by the import
    /// pipeline's second pass, after every session id has been remapped.
    pub fn set_session_via(&self, id: i64, via_session_id: Option<i64>) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE sessions SET via_session_id = ?1 WHERE id = ?2",
            rusqlite::params![via_session_id, id],
        )?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    pub fn delete_session(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sessions WHERE id=?1", rusqlite::params![id])?;
        Ok(())
    }

    pub fn touch_session(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET last_used_at=datetime('now') WHERE id=?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Session secrets (encrypted local-fallback password cache)
    // -----------------------------------------------------------------------

    pub fn set_session_secret(&self, session_id: i64, ciphertext: &[u8]) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_secrets (session_id, ciphertext) VALUES (?1, ?2)
             ON CONFLICT(session_id) DO UPDATE SET
                 ciphertext = excluded.ciphertext,
                 updated_at = datetime('now')",
            rusqlite::params![session_id, ciphertext],
        )?;
        Ok(())
    }

    pub fn get_session_secret(&self, session_id: i64) -> Result<Option<Vec<u8>>, Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT ciphertext FROM session_secrets WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| row.get::<_, Vec<u8>>(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn delete_session_secret(&self, session_id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM session_secrets WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Credential CRUD (keyring references only — no secrets in SQLite)
    // -----------------------------------------------------------------------

    pub fn create_credential(
        &self,
        name: &str,
        kind: &str,
        username: Option<&str>,
        keyring_key: &str,
    ) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO credentials (name, kind, username, keyring_key)
             VALUES (?1,?2,?3,?4)",
            rusqlite::params![name, kind, username, keyring_key],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_credential_keyring_key(&self, id: i64) -> Result<String, Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT keyring_key FROM credentials WHERE id=?1",
            rusqlite::params![id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
            e => Error::Rusqlite(e),
        })
    }

    pub fn delete_credential(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM credentials WHERE id=?1", rusqlite::params![id])?;
        Ok(())
    }

    /// All `keyring_key` values currently referenced by `credentials`.
    pub fn list_credential_keyring_keys(&self) -> Result<Vec<String>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT keyring_key FROM credentials")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // -----------------------------------------------------------------------
    // Highlight rule CRUD
    // -----------------------------------------------------------------------

    pub fn list_highlight_rules(&self) -> Result<Vec<HighlightRule>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, pattern, is_regex, fg_color, bg_color,
                    bold, underline, enabled, sort_order
             FROM highlight_rules ORDER BY sort_order, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(HighlightRule {
                id: row.get(0)?,
                name: row.get(1)?,
                pattern: row.get(2)?,
                is_regex: row.get::<_, i64>(3)? != 0,
                fg_color: row.get(4)?,
                bg_color: row.get(5)?,
                bold: row.get::<_, i64>(6)? != 0,
                underline: row.get::<_, i64>(7)? != 0,
                enabled: row.get::<_, i64>(8)? != 0,
                sort_order: row.get(9)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_highlight_rule(
        &self,
        name: &str,
        pattern: &str,
        is_regex: bool,
        fg_color: Option<&str>,
        bg_color: Option<&str>,
        bold: bool,
        underline: bool,
    ) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        let sort_order: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order),0)+1 FROM highlight_rules",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO highlight_rules
             (name, pattern, is_regex, fg_color, bg_color, bold, underline, sort_order)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            rusqlite::params![
                name,
                pattern,
                is_regex as i64,
                fg_color,
                bg_color,
                bold as i64,
                underline as i64,
                sort_order
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn toggle_highlight_rule(&self, id: i64, enabled: bool) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE highlight_rules SET enabled=?1 WHERE id=?2",
            rusqlite::params![enabled as i64, id],
        )?;
        Ok(())
    }

    pub fn delete_highlight_rule(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM highlight_rules WHERE id=?1",
            rusqlite::params![id],
        )?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Session log CRUD
    // -----------------------------------------------------------------------

    pub fn create_session_log(
        &self,
        session_id: Option<i64>,
        file_path: &str,
        started_at: &str,
    ) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_logs (session_id, file_path, started_at) VALUES (?1,?2,?3)",
            rusqlite::params![session_id, file_path, started_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn end_session_log(&self, id: i64, ended_at: &str, bytes: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE session_logs SET ended_at=?1, bytes=?2 WHERE id=?3",
            rusqlite::params![ended_at, bytes, id],
        )?;
        Ok(())
    }

    pub fn list_session_logs(&self, session_id: i64) -> Result<Vec<SessionLog>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, started_at, ended_at, file_path, bytes, format
             FROM session_logs WHERE session_id=?1 ORDER BY started_at DESC",
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok(SessionLog {
                id: row.get(0)?,
                session_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                file_path: row.get(4)?,
                bytes: row.get(5)?,
                format: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn list_all_session_logs(&self) -> Result<Vec<SessionLog>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, started_at, ended_at, file_path, bytes, format
             FROM session_logs ORDER BY started_at DESC LIMIT 200",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SessionLog {
                id: row.get(0)?,
                session_id: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                file_path: row.get(4)?,
                bytes: row.get(5)?,
                format: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // -----------------------------------------------------------------------
    // Color scheme CRUD
    // -----------------------------------------------------------------------

    pub fn list_color_schemes(&self) -> Result<Vec<ColorScheme>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, palette_json, is_builtin FROM color_schemes ORDER BY is_builtin DESC, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ColorScheme {
                id: row.get(0)?,
                name: row.get(1)?,
                palette_json: row.get(2)?,
                is_builtin: row.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // -----------------------------------------------------------------------
    // Settings CRUD
    // -----------------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key=?1",
            rusqlite::params![key],
            |row| row.get(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut map = std::collections::HashMap::new();
        for r in rows {
            let (k, v) = r?;
            map.insert(k, v);
        }
        Ok(map)
    }

    // -----------------------------------------------------------------------
    // Login script (migration 008) — JSON blob on `sessions`.
    // -----------------------------------------------------------------------

    pub fn set_login_script(
        &self,
        session_id: i64,
        script_json: Option<&str>,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE sessions SET login_script_json = ?1 WHERE id = ?2",
            rusqlite::params![script_json, session_id],
        )?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    pub fn get_login_script(&self, session_id: i64) -> Result<Option<String>, Error> {
        Ok(self.get_session(session_id)?.login_script_json)
    }

    /// Output triggers, stored as a JSON array on the session. Kept as dedicated
    /// queries (not part of the SavedSession SELECT) so the column is additive.
    pub fn set_session_triggers(
        &self,
        session_id: i64,
        triggers_json: Option<&str>,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE sessions SET triggers_json = ?1 WHERE id = ?2",
            rusqlite::params![triggers_json, session_id],
        )?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    pub fn get_session_triggers(&self, session_id: i64) -> Result<Option<String>, Error> {
        let conn = self.conn.lock().unwrap();
        let json: Option<String> = conn
            .query_row(
                "SELECT triggers_json FROM sessions WHERE id = ?1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
                other => Error::from(other),
            })?;
        Ok(json)
    }

    // -----------------------------------------------------------------------
    // Snippets CRUD (migration 007)
    // -----------------------------------------------------------------------

    pub fn list_snippets(&self) -> Result<Vec<Snippet>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, content, sort_order, created_at, platform, color
             FROM snippets ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Snippet {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                platform: row.get(5)?,
                color: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Same as [`list_snippets`] but filtered to snippets visible from a
    /// session of `platform` — globals (NULL) plus matches. Globals first,
    /// then platform-specific.
    pub fn list_snippets_for_platform(&self, platform: &str) -> Result<Vec<Snippet>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, content, sort_order, created_at, platform, color
             FROM snippets
             WHERE platform IS NULL OR platform = ?1
             ORDER BY (platform IS NULL) DESC, sort_order, name",
        )?;
        let rows = stmt.query_map(rusqlite::params![platform], |row| {
            Ok(Snippet {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                platform: row.get(5)?,
                color: row.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// True iff there is at least one snippet tagged with `platform`. Used
    /// by the app layer to gate first-time-seen seeding of platform defaults.
    pub fn snippets_exist_for_platform(&self, platform: &str) -> Result<bool, Error> {
        let conn = self.conn.lock().unwrap();
        let n: i64 = conn.query_row(
            "SELECT COUNT(*) FROM snippets WHERE platform = ?1",
            rusqlite::params![platform],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    pub fn create_snippet(
        &self,
        name: &str,
        content: &str,
        platform: Option<&str>,
        color: Option<&str>,
    ) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        // Next sort_order = current max + 1; keeps user-created entries after seeds.
        let next: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM snippets",
                [],
                |r| r.get(0),
            )
            .unwrap_or(1);
        conn.execute(
            "INSERT INTO snippets (name, content, sort_order, platform, color)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, content, next, platform, color],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_snippet(
        &self,
        id: i64,
        name: &str,
        content: &str,
        platform: Option<&str>,
        color: Option<&str>,
    ) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE snippets SET name = ?1, content = ?2, platform = ?3, color = ?4 WHERE id = ?5",
            rusqlite::params![name, content, platform, color, id],
        )?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        Ok(())
    }

    pub fn delete_snippet(&self, id: i64) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM snippets WHERE id = ?1", rusqlite::params![id])?;
        Ok(())
    }

    /// Persist a new display order for the given snippet ids: `sort_order`
    /// becomes each id's position in the list. Ids not listed keep theirs.
    pub fn set_snippets_order(&self, ordered_ids: &[i64]) -> Result<(), Error> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for (idx, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE snippets SET sort_order = ?1 WHERE id = ?2",
                rusqlite::params![idx as i64, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Command history (powers the ghost-text / popup hints)
    // -----------------------------------------------------------------------

    /// Insert-or-bump: if (session_id, command) already exists, increment
    /// freq and refresh last_used. Otherwise insert a new row.
    pub fn record_command(&self, session_id: Option<i64>, command: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE command_history
                SET freq = freq + 1, last_used = datetime('now')
              WHERE command = ?1
                AND ((?2 IS NULL AND session_id IS NULL)
                  OR session_id = ?2)",
            rusqlite::params![command, session_id],
        )?;
        if updated == 0 {
            conn.execute(
                "INSERT INTO command_history (session_id, command) VALUES (?1, ?2)",
                rusqlite::params![session_id, command],
            )?;
        }
        Ok(())
    }

    /// Return history entries matching `prefix` (case-insensitive). Pulls
    /// both session-scoped and global rows so generic commands stay useful
    /// in fresh sessions.
    pub fn history_by_prefix(
        &self,
        session_id: Option<i64>,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<CommandHistoryEntry>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT command, freq, last_used FROM command_history
              WHERE (session_id = ?1 OR session_id IS NULL)
                AND command LIKE ?2 || '%' ESCAPE '\\'
              ORDER BY last_used DESC
              LIMIT ?3",
        )?;
        let escaped = escape_like(prefix);
        let rows = stmt.query_map(
            rusqlite::params![session_id, escaped, limit as i64],
            |row| {
                Ok(CommandHistoryEntry {
                    command: row.get(0)?,
                    freq: row.get(1)?,
                    last_used: row.get(2)?,
                })
            },
        )?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Top `limit` commands for the dynamic "Recents" snippet bar. Ranked
    /// by frecency: a freshly-used command outranks an old frequent one,
    /// but raw frequency still matters within the same age band.
    ///
    /// We hide short / trivial commands (≤ 2 chars) — they're noisy and
    /// don't earn their slot in the bar.
    pub fn recent_commands(
        &self,
        session_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<CommandHistoryEntry>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT command, freq, last_used FROM command_history
              WHERE (session_id = ?1 OR session_id IS NULL)
                AND length(command) > 2
              ORDER BY last_used DESC, freq DESC
              LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![session_id, limit as i64], |row| {
            Ok(CommandHistoryEntry {
                command: row.get(0)?,
                freq: row.get(1)?,
                last_used: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn clear_history(&self, session_id: Option<i64>) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        match session_id {
            Some(id) => conn.execute(
                "DELETE FROM command_history WHERE session_id = ?1",
                rusqlite::params![id],
            )?,
            None => conn.execute("DELETE FROM command_history", [])?,
        };
        // Transitions are platform-keyed (not session-scoped); clear them
        // wholesale only on a global wipe.
        if session_id.is_none() {
            conn.execute("DELETE FROM command_transitions", [])?;
        }
        Ok(())
    }

    /// Record that `next` followed `prev` for `platform` (insert-or-bump).
    /// Used by the command-sequence learner.
    pub fn record_transition(&self, platform: &str, prev: &str, next: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let updated = conn.execute(
            "UPDATE command_transitions
                SET freq = freq + 1, last_used = datetime('now')
              WHERE platform = ?1 AND prev = ?2 AND next = ?3",
            rusqlite::params![platform, prev, next],
        )?;
        if updated == 0 {
            conn.execute(
                "INSERT INTO command_transitions (platform, prev, next) VALUES (?1, ?2, ?3)",
                rusqlite::params![platform, prev, next],
            )?;
        }
        Ok(())
    }

    /// Commands that frequently follow `prev` on `platform`, as
    /// `(next_command, freq)` ordered by frequency. Powers the next-command
    /// boost in the hint engine.
    pub fn next_commands_after(
        &self,
        platform: &str,
        prev: &str,
        limit: usize,
    ) -> Result<Vec<(String, i64)>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT next, freq FROM command_transitions
              WHERE platform = ?1 AND prev = ?2
              ORDER BY freq DESC, last_used DESC
              LIMIT ?3",
        )?;
        let rows = stmt.query_map(rusqlite::params![platform, prev, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Read global snippets (managed by the dedicated snippets dialog).
    /// The hint engine surfaces these in the popup alongside history.
    pub fn list_snippet_contents_by_prefix(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<(String, String)>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, content FROM snippets
              WHERE content LIKE ?1 || '%' ESCAPE '\\'
              ORDER BY sort_order, name
              LIMIT ?2",
        )?;
        let escaped = escape_like(prefix);
        let rows = stmt.query_map(rusqlite::params![escaped, limit as i64], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    // -----------------------------------------------------------------------
    // Per-session platform override (for hint catalogs)
    // -----------------------------------------------------------------------

    pub fn set_session_platform(&self, session_id: i64, platform: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO session_platforms (session_id, platform) VALUES (?1, ?2)
             ON CONFLICT(session_id) DO UPDATE SET
                 platform = excluded.platform,
                 updated_at = datetime('now')",
            rusqlite::params![session_id, platform],
        )?;
        Ok(())
    }

    pub fn get_session_platform(&self, session_id: i64) -> Result<Option<String>, Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT platform FROM session_platforms WHERE session_id = ?1",
            rusqlite::params![session_id],
            |row| row.get(0),
        );
        match result {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

/// Escape LIKE wildcards (%, _, \) in user input so a prefix containing a
/// literal % does not behave as a wildcard.
fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch == '\\' || ch == '%' || ch == '_' {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Open a throwaway DB in a unique temp path so migrations run fresh.
    fn temp_db() -> (Database, std::path::PathBuf) {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("zapx_groups_test_{nanos}.db"));
        let db = Database::open(&path).expect("open db");
        (db, path)
    }

    #[test]
    fn broadcast_group_crud_roundtrip() {
        let (db, path) = temp_db();

        // Seed two saved sessions to reference as members.
        let s1 = db
            .create_session(
                None,
                "r1",
                "ssh",
                Some("10.0.0.1"),
                Some(22),
                Some("admin"),
                None,
                "{}",
            )
            .unwrap();
        let s2 = db
            .create_session(
                None,
                "r2",
                "ssh",
                Some("10.0.0.2"),
                Some(22),
                Some("admin"),
                None,
                "{}",
            )
            .unwrap();

        // Create + set members (note reversed order is preserved).
        let gid = db.create_broadcast_group("core").unwrap();
        db.set_broadcast_group_members(gid, &[s2, s1]).unwrap();

        let groups = db.list_broadcast_groups().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "core");
        assert_eq!(groups[0].session_ids, vec![s2, s1]);

        // Rename.
        db.rename_broadcast_group(gid, "core-switches").unwrap();
        assert_eq!(db.list_broadcast_groups().unwrap()[0].name, "core-switches");

        // Deleting a referenced session cascades it out of the membership.
        db.delete_session(s2).unwrap();
        assert_eq!(db.list_broadcast_groups().unwrap()[0].session_ids, vec![s1]);

        // Deleting the group removes it (and members cascade).
        db.delete_broadcast_group(gid).unwrap();
        assert!(db.list_broadcast_groups().unwrap().is_empty());

        let _ = std::fs::remove_file(&path);
    }
}
