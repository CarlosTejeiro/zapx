#![forbid(unsafe_code)]

//! `core-persistence` — SQLite storage, at-rest encryption, and schema migrations.
//!
//! Uses `rusqlite` with the `bundled` feature so SQLite is compiled into the binary
//! with no OS-level dependency. Credentials are never stored here; only opaque
//! keyring references live in the database (see `core-vault`).

pub mod error;
