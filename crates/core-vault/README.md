# core-vault

OS-keyring-backed credential storage for zapx.

## Purpose

Wraps the OS keyring (Windows Credential Manager / macOS Keychain / Linux Secret Service) via the
`keyring` crate. Ensures that passwords, passphrases, and SSH key material are zeroed on drop and
never written to the database, logs, or error messages.

## Public API

_(Populated in Bloque 3)_

## Non-goals

- Does not store credentials itself — it is a thin adapter over the OS keyring.
- Does not encrypt the database — that is `core-persistence`.
