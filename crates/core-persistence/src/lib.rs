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
pub struct Folder {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub name: String,
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
    pub last_used_at: Option<String>,
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
    pub fn create_session(
        &self,
        folder_id: Option<i64>,
        name: &str,
        protocol: &str,
        host: Option<&str>,
        port: Option<u16>,
        username: Option<&str>,
        credential_id: Option<i64>,
    ) -> Result<i64, Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (folder_id, name, protocol, host, port, username, credential_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                folder_id,
                name,
                protocol,
                host,
                port.map(|p| p as i64),
                username,
                credential_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_sessions(&self) -> Result<Vec<SavedSession>, Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, folder_id, name, protocol, host, port, username, credential_id, last_used_at
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
                last_used_at: row.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_session(&self, id: i64) -> Result<SavedSession, Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, folder_id, name, protocol, host, port, username, credential_id, last_used_at
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
                    last_used_at: row.get(8)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::NotFound,
            e => Error::Rusqlite(e),
        })
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
}
