#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("persistence error: {0}")]
    Persistence(#[from] core_persistence::Error),

    #[error("invalid platform: {0}")]
    InvalidPlatform(String),

    #[error("catalog parse error: {0}")]
    CatalogParse(#[from] serde_json::Error),
}
