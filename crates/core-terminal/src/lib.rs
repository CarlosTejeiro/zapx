#![forbid(unsafe_code)]

//! `core-terminal` — terminal emulator state and VT sequence processing.
//!
//! Wraps `wezterm-term` to provide a stable API for maintaining the terminal grid,
//! parsing VT/ANSI escape sequences, and exposing the rendered cell grid to the
//! highlight engine and the application layer.

pub mod error;
