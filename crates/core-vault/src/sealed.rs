//! Passphrase-sealed envelopes for backup bundles.
//!
//! A sealed blob is meant to live *outside* the app's trust boundary — in a
//! cloud-synced folder, on a USB stick, in an e-mail — so unlike the local
//! fallback cipher in the crate root it cannot lean on a per-install keyfile.
//! The only key material is a user-chosen passphrase, stretched with
//! **Argon2id** (memory-hard, so offline guessing is expensive) into an
//! AES-256-GCM key.
//!
//! Wire format (all integers little-endian):
//!
//! ```text
//! magic     6 bytes   "ZAPXB" + version byte (0x01)
//! m_cost    u32       Argon2 memory cost, KiB
//! t_cost    u32       Argon2 iterations
//! p_cost    u32       Argon2 parallelism
//! salt     16 bytes   random, per blob
//! nonce    12 bytes   random, per blob
//! ct        …         AES-256-GCM ciphertext + 16-byte tag
//! ```
//!
//! Everything before `ct` is bound as AEAD associated data, so a tampered
//! header (say, KDF parameters lowered to speed up cracking) fails
//! authentication instead of silently producing garbage.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use zeroize::Zeroize;

use crate::Error;

const MAGIC: &[u8; 6] = b"ZAPXB\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const HEADER_LEN: usize = MAGIC.len() + 3 * 4 + SALT_LEN + NONCE_LEN;

/// Default Argon2id cost: 64 MiB, 3 passes, 1 lane. Roughly a quarter of a
/// second on a laptop — imperceptible for a manual backup, painful for an
/// attacker guessing millions of passphrases.
const M_COST_KIB: u32 = 64 * 1024;
const T_COST: u32 = 3;
const P_COST: u32 = 1;

/// Upper bound accepted when *opening* a blob, so a malicious header can't
/// make us allocate gigabytes before the AEAD check even runs.
const MAX_M_COST_KIB: u32 = 1024 * 1024;

/// Seal `plaintext` under `passphrase`. The result is self-describing and can
/// be opened by [`open`] on any machine with the same passphrase.
pub fn seal(passphrase: &str, plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&M_COST_KIB.to_le_bytes());
    header.extend_from_slice(&T_COST.to_le_bytes());
    header.extend_from_slice(&P_COST.to_le_bytes());
    header.extend_from_slice(&salt);
    header.extend_from_slice(&nonce);

    let mut key = derive_key(passphrase, &salt, M_COST_KIB, T_COST, P_COST)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let ct = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad: &header,
            },
        )
        .map_err(|e| Error::Crypto(e.to_string()));
    key.zeroize();
    let mut ct = ct?;

    let mut out = header;
    out.append(&mut ct);
    Ok(out)
}

/// Inverse of [`seal`]. A wrong passphrase, a truncated file and a tampered
/// byte all surface as [`Error::Crypto`]; a blob that isn't ours at all is
/// [`Error::BadCiphertext`].
pub fn open(passphrase: &str, blob: &[u8]) -> Result<Vec<u8>, Error> {
    if blob.len() < HEADER_LEN + 16 || &blob[..MAGIC.len()] != MAGIC {
        return Err(Error::BadCiphertext);
    }
    let (header, ct) = blob.split_at(HEADER_LEN);
    let mut at = MAGIC.len();
    let u32_at = |at: &mut usize| {
        let v = u32::from_le_bytes(header[*at..*at + 4].try_into().expect("4 bytes"));
        *at += 4;
        v
    };
    let m_cost = u32_at(&mut at);
    let t_cost = u32_at(&mut at);
    let p_cost = u32_at(&mut at);
    if m_cost == 0 || m_cost > MAX_M_COST_KIB || t_cost == 0 || p_cost == 0 {
        return Err(Error::BadCiphertext);
    }
    let salt = &header[at..at + SALT_LEN];
    let nonce = &header[at + SALT_LEN..at + SALT_LEN + NONCE_LEN];

    let mut key = derive_key(passphrase, salt, m_cost, t_cost, p_cost)?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let pt = cipher.decrypt(
        Nonce::from_slice(nonce),
        Payload {
            msg: ct,
            aad: header,
        },
    );
    key.zeroize();
    pt.map_err(|_| Error::Crypto("wrong passphrase or corrupted bundle".into()))
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<[u8; 32], Error> {
    let params = Params::new(m_cost, t_cost, p_cost, Some(32))
        .map_err(|e| Error::Crypto(format!("argon2 params: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| Error::Crypto(format!("argon2: {e}")))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let blob = seal("correct horse", b"{\"hello\":\"world\"}").unwrap();
        assert_eq!(&blob[..6], b"ZAPXB\x01");
        assert_eq!(
            open("correct horse", &blob).unwrap(),
            b"{\"hello\":\"world\"}"
        );
    }

    #[test]
    fn wrong_passphrase_fails() {
        let blob = seal("correct horse", b"secret").unwrap();
        assert!(matches!(
            open("battery staple", &blob),
            Err(Error::Crypto(_))
        ));
    }

    #[test]
    fn tampered_body_and_header_fail() {
        let blob = seal("pw", b"secret payload").unwrap();
        // Flip a ciphertext byte.
        let mut body = blob.clone();
        let last = body.len() - 1;
        body[last] ^= 0x01;
        assert!(open("pw", &body).is_err());
        // Lower the memory cost in the header: bound as AAD, so it must fail
        // authentication rather than decrypt under a cheaper key.
        let mut header = blob.clone();
        header[6..10].copy_from_slice(&1024u32.to_le_bytes());
        assert!(open("pw", &header).is_err());
    }

    #[test]
    fn garbage_is_rejected_early() {
        assert!(matches!(
            open("pw", b"not a bundle"),
            Err(Error::BadCiphertext)
        ));
        assert!(matches!(open("pw", b""), Err(Error::BadCiphertext)));
        // Right magic, absurd memory cost: rejected before any allocation.
        let mut blob = seal("pw", b"x").unwrap();
        blob[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(open("pw", &blob), Err(Error::BadCiphertext)));
    }

    #[test]
    fn salts_and_nonces_are_fresh() {
        let a = seal("pw", b"same").unwrap();
        let b = seal("pw", b"same").unwrap();
        assert_ne!(a, b);
    }
}
