# Backlog

Out-of-scope ideas tracked per ADR-007. Features listed here are not implemented until the active
block that gates them is closed.

Add entries with format: `- **Feature name**: brief description.`

## Distribution & updates (gated on code signing)

- **Code signing**: Apple Developer ID for macOS (notarization — fixes the
  "ZAPX is damaged" Gatekeeper prompt) and a Windows certificate. This is the
  blocker for both clean distribution and macOS auto-update. Until then, macOS
  users clear quarantine with `xattr -cr /Applications/ZAPX.app`.
- **App updates**: the repo/releases are private, so GitHub-hosted auto-update
  isn't viable (no anonymous manifest, can't ship a token). Options when
  wanted: a **private update server** (`latest.json` + signed bundles you host;
  Windows/Linux self-update now, macOS after signing), or keep distribution
  manual. "Check for updates" currently just opens the releases page.

## Features

- **Importers: MobaXterm / SecureCRT port forwards**: PuTTY tunnel import is
  done (`importers/putty.rs` → `session_forwards`). MobaXterm/SecureCRT store
  forwards in less-documented structures; needs real sample files to map them
  reliably into `session_forwards` during import.
- **Regex in login-script expects**: the schema already carries `is_regex`
  (`crates/app/src/login_script.rs`); wire it so expect patterns can be regex,
  not just literal.
- **SFTP drag & drop upload (blocked)**: dropping OS files to upload needs
  Tauri's native drag-drop (`dragDropEnabled: true`) to get file paths, but
  that disables the HTML5 drag-drop the sidebar (session reorder) and button
  bar (reorder) rely on — and breaks them on Windows. Not worth trading two
  working features for it; the Upload button covers uploads. ("Edit remote
  file" shipped in 0.8.)

## Linux packaging

- **AUR / Flathub / Snap**: the AppImage/deb/rpm already run on Arch. Cheapest
  first: a `zapx-bin` PKGBUILD on the AUR that repacks the released .deb
  (CI-updated per release). Then Flathub (manifest + review) and optionally
  Snapcraft. Note: with a private releases repo, the package source URLs need a
  reachable download location.

<!-- Add out-of-scope ideas below this line -->
