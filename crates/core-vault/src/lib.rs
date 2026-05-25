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

// ---------------------------------------------------------------------------
// Local-fallback encryption — used when the OS keyring is unusable (e.g.
// macOS dev builds where the unsigned binary is denied read access to its
// own Keychain entry).
//
// We derive a process-stable AES-256 key from a caller-supplied seed (the
// app's data directory path is a sensible choice — it's per-user, stable
// across reboots, and not exposed via the DB content). Ciphertexts are
// AES-256-GCM with a fresh random nonce prepended.
//
// This is a *dev-mode convenience*, not a substitute for code-signing +
// OS keyring. An attacker who has read access to both the SQLite file and
// the seed can recover the plaintext.
// ---------------------------------------------------------------------------

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Encrypt `plaintext` with a key derived from `seed`. Returns
/// `nonce || ciphertext` ready to be stored as a single BLOB.
pub fn encrypt_with_seed(seed: &str, plaintext: &str) -> Result<Vec<u8>, Error> {
    let cipher = build_cipher(seed);
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| Error::Crypto(e.to_string()))?;
    // Prepend the nonce so decrypt is self-contained.
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.append(&mut ct);
    Ok(out)
}

/// Inverse of [`encrypt_with_seed`].
pub fn decrypt_with_seed(seed: &str, blob: &[u8]) -> Result<String, Error> {
    if blob.len() < 12 {
        return Err(Error::BadCiphertext);
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let cipher = build_cipher(seed);
    let nonce = Nonce::from_slice(nonce_bytes);
    let pt = cipher
        .decrypt(nonce, ct)
        .map_err(|e| Error::Crypto(e.to_string()))?;
    String::from_utf8(pt).map_err(|e| Error::Crypto(e.to_string()))
}

fn build_cipher(seed: &str) -> Aes256Gcm {
    // SHA-256(seed) → 32-byte AES-256 key.
    let mut hasher = Sha256::new();
    hasher.update(b"zapx-vault-v1:");
    hasher.update(seed.as_bytes());
    let key_bytes = hasher.finalize();
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    Aes256Gcm::new(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let blob = encrypt_with_seed("/path/to/data", "hunter2").unwrap();
        let pt = decrypt_with_seed("/path/to/data", &blob).unwrap();
        assert_eq!(pt, "hunter2");
    }

    #[test]
    fn different_seed_fails() {
        let blob = encrypt_with_seed("seed-a", "hunter2").unwrap();
        assert!(decrypt_with_seed("seed-b", &blob).is_err());
    }

    #[test]
    fn nonce_makes_ciphertexts_unique() {
        let a = encrypt_with_seed("s", "same").unwrap();
        let b = encrypt_with_seed("s", "same").unwrap();
        assert_ne!(a, b);
    }
}
