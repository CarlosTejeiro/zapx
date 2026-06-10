# Backlog

Out-of-scope ideas tracked per ADR-007. Features listed here are not implemented until the active
block that gates them is closed.

Add entries with format: `- **Feature name**: brief description. Gated by: Bloque N.`

## Planned for 0.3

- **Portable Windows executable (release asset)**: publish the raw `ZAPX.exe`
  (already produced by `cargo tauri build`) as a `ZAPX_x64_portable.exe` asset in
  the release workflow, Windows matrix only. Document the WebView2 runtime
  requirement (preinstalled on Win11 / updated Win10; the installers bootstrap
  it, the portable exe does not).
- **Portable mode (data travels with the exe)**: when a `portable` marker file
  (or `data/` folder) exists next to the executable, redirect the app data dir
  (DB, settings, logs, catalogs) there instead of `%APPDATA%`/`~/Library`.
  Credentials switch from the OS keyring to the existing `core-vault`
  AES-256-GCM fallback, with an explicit security notice (KeePass-style).
- **User-selectable data directory**: setting (+ `ZAPX_DATA_DIR` env var /
  `--data-dir` CLI flag) that relocates `zapx.db`, settings, `session_logs/`
  and `catalogs/` (snippet/hint files) to a user-chosen folder — e.g. a synced
  drive or a network share. Needs a single data-dir resolution point in
  `crates/app` (today the path is resolved per subsystem), plus a "move
  existing data" migration helper in Settings. Consider accepting YAML in
  addition to JSON for user catalogs while at it.
- **Window size/state awareness**: remember window size, position and
  maximized state across launches (`tauri-plugin-window-state`); on first run,
  open centered at ~75 % of the monitor work area (clamped to min 900×600)
  instead of the fixed 1200×800. This is what SecureCRT/MobaXterm do
  (SecureCRT additionally syncs terminal rows×cols — out of scope here since
  xterm.js + FitAddon already reflow on resize).

## Ideas

<!-- Add out-of-scope ideas below this line -->
