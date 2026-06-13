# Changelog

All notable changes to ZAPX will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Multi-send on regular tabs**: with multi-exec on (Ctrl+Shift+M), the
  master input bar from grid mode now also appears on ordinary split tabs,
  broadcasting to every live pane of the active tab — including "live"
  keystroke streaming.
- **Send command list** (Tools menu / command palette): paste a runbook —
  one command per line, `#` comments skipped — pick any set of open
  sessions across tabs, set the pause between lines and dispatch it to all
  of them at once, with live progress and mid-run cancel. SecureCRT
  "command window" style.
- **Importers for PuTTY, MobaXterm and SecureCRT** (File menu / command
  palette), all feeding the same idempotent import pipeline:
  - **PuTTY**: a `.reg` export anywhere, or the live `HKCU` registry on
    Windows. Maps SSH/Telnet/Serial; `.ppk` keys are flagged to convert.
  - **MobaXterm**: `MobaXterm.ini` / `.mxtsessions` `[Bookmarks]` sections,
    rebuilding the `SubRep` folder tree. RDP/VNC bookmarks are skipped.
  - **SecureCRT**: walks the `Config/Sessions` directory of per-session
    `.ini` files, turning subfolders into ZAPX folders.

## [0.4.0] - 2026-06-12

### Added
- **Export / import of the ZAPX environment** (File menu and command
  palette): one versioned JSON file with sessions, folders, broadcast
  groups, snippets and highlight rules. Passwords never leave the OS
  keyring — imported password sessions prompt on first connect. Import is
  idempotent (existing items are skipped) and remaps all ids, including
  ProxyJump bastion references and group memberships, reporting a summary
  with warnings (missing key files, unresolved bastions).
- **Import from `~/.ssh/config`** (File → Import from SSH config…): parses
  Host blocks (HostName, User, Port, IdentityFile with `~` expansion,
  ProxyJump resolved between imported aliases, Include with `*` glob) into
  the same idempotent pipeline. Wildcard/`Match` blocks and unsupported
  directives are skipped with warnings.

## [0.3.0] - 2026-06-11

### Added
- **Portable Windows executable** (`ZAPX_x64_portable.exe`) published as a
  release asset — runs from any folder, no installation (WebView2 runtime
  required; preinstalled on Win11/updated Win10).
- **Portable mode**: a `portable` marker file next to the executable keeps
  all data (DB, settings, logs, catalogs) in a `data/` folder beside it.
  Credentials switch from the OS keyring to the AES-256-GCM database
  fallback with a stable seed so the folder works across machines.
- **User-selectable data folder**: Settings → Data shows where data lives
  and can migrate it to any folder (DB snapshot via `VACUUM INTO`, fallback
  secrets re-encrypted for the new location, logs + catalogs copied).
  Also overridable via `ZAPX_DATA_DIR` env var or `--data-dir` CLI flag.
- **Window state**: size, position and maximized state persist across
  launches (`tauri-plugin-window-state`); the first launch opens centered
  at ~75 % of the monitor work area instead of a fixed 1200×800.

## [0.2.0] - 2026-06-10

### Changed
- **Full UI redesign («Pulido»)** following the design handoff: 38 px title
  bar with the new brand mark, 248 px sidebar with 32 px session rows (active
  state by background + type weight, no border-left), 42 px tab bar with
  vertically-centred chip tabs and an inner accent underline, 28 px mono
  status bar with `│` separators and a glowing connection LED, and the
  terminal rendered as a **floating card** (11 px radius, soft shadow, 14 px
  frame, 10 px gap between splits).
- **New app icon «Tile»** — a single lightning bolt knocked out of a rounded
  indigo tile. Replaces the crossed-bolts mark in the title bar, the About
  dialog and every bundle asset (icns/ico/png), regenerated via `tauri icon`.
- **7 new themes** replace the previous four: Parchment (refined), Oxide,
  Fjord, Nocturne, Porcelain, Phosphor and Amber — each with a full ANSI
  terminal palette; the CRT pair (Phosphor/Amber) enables cursor glow.
  Error/warning/ok colours stay semantic in every theme so keyword
  highlighting keeps meaning.
- **Unified SVG icon set** (16×16, stroke 1.5) across the entire chrome and
  all dialogs — no more text/emoji glyphs (✎ ✕ 📁 ▸ ● ─ □) as controls.
- **Bundled fonts**: Geist and JetBrains Mono (variable) now ship with the
  app, so the UI no longer depends on system-installed fonts.
- **Terminal scrollback scrollbar** is now visible: slim 4 px track with a
  rounded thumb (was hidden entirely).

### Removed
- Dead legacy components (`SplitView`, `TerminalPane`, `SessionTree`) that
  bypassed the theme system with hardcoded colors.

### Known issues
- **macOS**: bundles are not yet signed/notarized, so Gatekeeper reports the
  downloaded app as "damaged". Workaround: right-click → Open, or run
  `xattr -cr /Applications/ZAPX.app` after installing. Proper signing +
  notarization will ship once an Apple Developer ID is available.

## [0.1.0] - 2026-06-09

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
