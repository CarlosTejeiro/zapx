#![forbid(unsafe_code)]

//! `core-highlight` — keyword highlighting engine with regex support.
//!
//! Highlighting is applied to the terminal cell grid *after* VT sequence processing
//! (see ADR-006). This crate owns rule evaluation, scope resolution (global vs.
//! per-session), and priority ordering.

pub mod error;
