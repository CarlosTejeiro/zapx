# Changelog

All notable changes to ZAPX will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **Renamed to `ZAPX`** (the brief `RustTerm` identity was never released).
  Bundle identifier `app.zapx.client`; SQLite filename `zapx.db`; OS keyring
  service `zapx`; vault encryption prefix `zapx-vault-v1:`; `TERM_PROGRAM=zapx`.
  Display name **ZAPX** in the title bar, window title and About dialog. The
  app retains its lightning-bolt mark, which fits the name. Since nothing
  shipped under the interim name, there is **no data migration** — this is a
  clean start (the previous one-shot migration module was removed).
- **Theme-aware dialogs**: all modals/dialogs (sessions, settings, SFTP,
  tunnels, prompts, command palette, toasts) now read their colors from global
  `--zx-*` CSS custom properties published from the active theme, instead of a
  hardcoded dark palette. They render correctly in every theme, including the
  light **Parchment** theme where they were previously illegible. Added a
  consistent `:focus-visible` ring, an 8 px spacing scale, a `prefers-reduced-
  motion` fallback, and an `onAccent` theme token for text on accent buttons.

### Known issues
- The auto-updater endpoint in `tauri.conf.json` is still a placeholder
  (`https://example.com/zapx/...`) — updates won't work until a real release
  host is configured.

### Added
- **TCP MSS in StatusBar** with live polling (5 s) — Linux reports both
  directions via `TCP_INFO`, macOS/Windows the send-side only.
- **Snippets per platform** with auto-seeded curated defaults for each
  of the 12 supported vendors, plus a dynamic "Recents" zone fed from
  `command_history`. Bar capped at 9 user snippets so `Ctrl+Shift+1..9`
  always matches what's on screen.
- **Auto-detection of session platform** from PTY output (vendor banner
  + prompt regex). Latched per session; writes to the saved-session
  row only when none is set, never overrides.
- **User-overridable hint catalogs** at `<app_data>/catalogs/*.json`
  with file-watcher auto-reload (notify crate).
- **Reverse port forward (`-R`)** end-to-end, with bidirectional
  bridge via russh's `forwarded-tcpip` callback.
- **SFTP streaming + progress + cancel** for large transfers.
- **SSH agent on Windows** — OpenSSH named pipe first, Pageant
  fallback, configurable priority in Settings.
- **Drag-reorder of sessions inside a folder** (intra-folder DnD).
- **5 new vendor catalogs**: Palo Alto, F5 BIG-IP, Check Point Gaia,
  HPE Comware, Brocade FOS.

### Fixed
- `TERM_PROGRAM=zapx` env var so zsh-syntax-highlighting +
  autosuggestions render correctly in local PTY tabs (previously
  doubled characters because of missing terminfo).
- Pane-header buttons (●  ⊟  ⊞  ✕) bumped from 11 px / 0.55 opacity to
  14 px / 0.9 opacity on a faint chip background — now readable
  against any terminal color scheme.
- xterm auto-focus on tab mount + after connect, so the "Conectado"
  toast no longer appears to swallow keyboard input.
- All 11 svelte a11y warnings during build resolved.
