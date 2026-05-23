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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Snippet {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub sort_order: i64,
    pub created_at: String,
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
                    options_json, last_used_at, auth_method, via_session_id, login_script_json
             FROM sessions ORDER BY folder_id NULLS LAST, name",
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
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_session(&self, id: i64) -> Result<SavedSession, Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, folder_id, name, protocol, host, port, username, credential_id,
                    options_json, last_used_at, auth_method, via_session_id, login_script_json
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

    // -----------------------------------------------------------------------
    // Snippets CRUD (migration 007)
    // -----------------------------------------------------------------------

    pub fn list_snippets(&self) -> Result<Vec<Snippet>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, content, sort_order, created_at
             FROM snippets ORDER BY sort_order, name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Snippet {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create_snippet(&self, name: &str, content: &str) -> Result<i64, Error> {
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
            "INSERT INTO snippets (name, content, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, content, next],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_snippet(&self, id: i64, name: &str, content: &str) -> Result<(), Error> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute(
            "UPDATE snippets SET name = ?1, content = ?2 WHERE id = ?3",
            rusqlite::params![name, content, id],
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
}
