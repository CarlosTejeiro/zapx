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
// own Keychain entry, or portable mode).
//
// The AES-256 key is derived from a high-entropy *seed*: 32 random bytes
// generated on first run and persisted to `vault.key` (mode 0600 on Unix) via
// [`load_or_create_seed`]. Earlier builds derived the key from a predictable
// value (the data-directory path, or a constant in portable mode), which made
// the key guessable — or, in portable mode, identical across every install.
// A random per-install keyfile means recovering the plaintext requires reading
// the keyfile itself, not merely knowing where the app stores its data.
// Ciphertexts are AES-256-GCM with a fresh random nonce prepended.
//
// This is still a *dev-mode convenience*, not a substitute for code-signing +
// OS keyring: an attacker who can read both `zapx.db` and `vault.key` can
// recover the plaintext. The keyfile's randomness and 0600 permissions are
// what separate this from plain obfuscation.
// ---------------------------------------------------------------------------

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::Path;
use zeroize::Zeroize;

/// Load the local-fallback seed from `dir/vault.key`, generating it on first
/// use. The seed is 32 random bytes, hex-encoded. On Unix the file is created
/// with mode 0600 so other users on the machine cannot read it.
///
/// Pass the returned seed to [`encrypt_with_seed`] / [`decrypt_with_seed`].
/// On failure (e.g. a read-only data dir) the caller is responsible for
/// choosing a degraded fallback.
pub fn load_or_create_seed(dir: &Path) -> Result<String, Error> {
    let key_path = dir.join("vault.key");
    if let Ok(existing) = std::fs::read_to_string(&key_path) {
        let trimmed = existing.trim();
        // 32 random bytes hex-encode to 64 chars; accept anything long enough
        // to carry adequate entropy so a truncated/garbled file is regenerated.
        if trimmed.len() >= 32 {
            return Ok(trimmed.to_string());
        }
    }
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    bytes.zeroize();
    write_private(&key_path, hex.as_bytes()).map_err(|e| Error::Io(e.to_string()))?;
    Ok(hex)
}

/// Write `contents` to `path`, restricting access to the owner (mode 0600) on
/// Unix. On other platforms the file inherits default permissions.
fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(contents)?;
        f.flush()
    }
    #[cfg(not(unix))]
    {
        let mut f = std::fs::File::create(path)?;
        f.write_all(contents)?;
        f.flush()
    }
}

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
    let nonce = Nonce::from_slice(nonce_bytes);

    let cipher = build_cipher_with_prefix(VAULT_PREFIX_CURRENT, seed);
    let Ok(mut pt) = cipher.decrypt(nonce, ct) else {
        return Err(Error::Crypto("AEAD verification failed".into()));
    };
    // Copy the secret out as UTF-8, then wipe the intermediate plaintext
    // buffer. The returned String still holds the secret — zeroizing that is
    // the caller's responsibility (see the password-cache lifecycle).
    let result = std::str::from_utf8(&pt)
        .map(str::to_owned)
        .map_err(|e| Error::Crypto(e.to_string()));
    pt.zeroize();
    result
}

const VAULT_PREFIX_CURRENT: &[u8] = b"zapx-vault-v1:";

fn build_cipher(seed: &str) -> Aes256Gcm {
    build_cipher_with_prefix(VAULT_PREFIX_CURRENT, seed)
}

fn build_cipher_with_prefix(prefix: &[u8], seed: &str) -> Aes256Gcm {
    // SHA-256(prefix || seed) → 32-byte AES-256 key.
    let mut hasher = Sha256::new();
    hasher.update(prefix);
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

    #[test]
    fn seed_is_random_and_stable() {
        // A fresh dir yields a 64-hex-char seed; a second call returns the
        // SAME seed (read from the keyfile), and a different dir differs.
        let base = std::env::temp_dir().join(format!("zapx-vault-{}", std::process::id()));
        let dir_a = base.join("a");
        let dir_b = base.join("b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        let s1 = load_or_create_seed(&dir_a).unwrap();
        assert_eq!(s1.len(), 64, "32 random bytes hex-encode to 64 chars");
        let s2 = load_or_create_seed(&dir_a).unwrap();
        assert_eq!(s1, s2, "seed must persist across calls");
        let s3 = load_or_create_seed(&dir_b).unwrap();
        assert_ne!(s1, s3, "independent dirs get independent seeds");

        // A blob written under the keyfile seed round-trips.
        let blob = encrypt_with_seed(&s1, "hunter2").unwrap();
        assert_eq!(decrypt_with_seed(&s1, &blob).unwrap(), "hunter2");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir_a.join("vault.key"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600, "keyfile must be owner-only");
        }

        let _ = std::fs::remove_dir_all(&base);
    }
}
