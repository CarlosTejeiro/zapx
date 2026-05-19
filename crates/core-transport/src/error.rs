/// Errors produced by [`core_transport`](crate).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    // Protocol-specific variants are added in Bloque 1–4.
    // Keep this enum exhaustive; callers must handle new variants.
}
