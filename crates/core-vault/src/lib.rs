#![forbid(unsafe_code)]

//! `core-vault` — credential storage backed by the OS keyring.
//!
//! Secrets (passwords, passphrases) are stored in Windows Credential Manager,
//! macOS Keychain, or Linux Secret Service. The database only holds an opaque
//! `keyring_key` reference — the secret itself never enters SQLite or logs.

pub mod error;

pub use error::Error;

const SERVICE: &str = "zapx";

/// Thin wrapper around the OS keyring.
pub struct Vault;

impl Vault {
    /// Store `secret` under `key` in the OS keyring.
    pub fn store(key: &str, secret: &str) -> Result<(), Error> {
        keyring::Entry::new(SERVICE, key)?.set_password(secret)?;
        Ok(())
    }

    /// Retrieve the secret stored under `key`.
    pub fn retrieve(key: &str) -> Result<String, Error> {
        Ok(keyring::Entry::new(SERVICE, key)?.get_password()?)
    }

    /// Delete the credential stored under `key`.
    pub fn delete(key: &str) -> Result<(), Error> {
        keyring::Entry::new(SERVICE, key)?.delete_credential()?;
        Ok(())
    }
}
