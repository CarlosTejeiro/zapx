#![forbid(unsafe_code)]

//! `core-vault` — credential storage backed by the OS keyring.
//!
//! Passwords, passphrases, and SSH key material never enter the database or log files.
//! This crate wraps the OS keyring (Windows Credential Manager, macOS Keychain,
//! Linux Secret Service) and ensures secrets are zeroed on drop.

pub mod error;
