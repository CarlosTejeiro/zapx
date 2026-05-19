#![forbid(unsafe_code)]

//! `core-logging` — per-session loggers with rotation and search.
//!
//! Session logs are plain-text or timestamped-text files written to the OS app-data
//! directory. This crate handles file creation, rotation (by size and date), and
//! simple search over the log buffer.

pub mod error;
