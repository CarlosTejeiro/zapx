#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("malformed ciphertext")]
    BadCiphertext,
}
