# Backlog

Out-of-scope ideas tracked per ADR-007. Features listed here are not implemented until the active
block that gates them is closed.

Add entries with format: `- **Feature name**: brief description. Gated by: Bloque N.`

## Planned for 0.7

- **App updates**. Today "Check for updates" fails: the updater plugin is
  `active: false`, the endpoint is a placeholder (`example.com`), and the
  release builds no updater artifacts (`createUpdaterArtifacts: false`), so
  there's no `latest.json` to fetch. Two complementary paths:
  - **A — check & open (recommended first, works everywhere now)**: replace
    `handleCheckUpdates` in `App.svelte` to query the GitHub Releases API
    (`/repos/CarlosTejeiro/zapx/releases/latest`), compare `tag_name` with
    `getVersion()`, and on a newer version show a notice + button that opens
    the releases page (shell plugin). No infra, no signing, cross-platform.
  - **B — native Tauri auto-update (Windows/Linux now, macOS after signing)**:
    set `bundle.createUpdaterArtifacts: true`, `plugins.updater.active: true`,
    and point `endpoints` at `https://github.com/CarlosTejeiro/zapx/releases/
    latest/download/latest.json` (tauri-action generates+uploads it with the
    `TAURI_SIGNING_PRIVATE_KEY` secret — confirm that secret exists and matches
    the configured `pubkey`). `check()`/`downloadAndInstall()` are already
    wired. **macOS blocker**: an unsigned/un-notarized updated `.app` is
    Gatekeeper-quarantined like the first download, so in-app update on macOS
    waits for the Apple Developer ID work; Windows + Linux self-update fine.

## Ideas

- **Third-party forward import**: PuTTY/MobaXterm/SecureCRT also store port
  forwards; map them into the new `session_forwards` table during import
  (the native export/import already carries them).
- **Linux packaging — AUR / Flathub / Snap**: the AppImage/deb/rpm already
  work on Arch, but native channels help adoption. Cheapest first: publish a
  `zapx-bin` PKGBUILD to the AUR that repacks the released .deb (update via
  CI on each release). Then Flathub (best cross-distro discoverability for
  GUI apps; needs a flatpak manifest + review) and optionally Snapcraft.
- **Drag-to-reorder button-bar buttons**: the bar buttons (snippets) have a
  `sort_order`; add drag-to-reorder in the bar so users can arrange the first
  nine (the `Ctrl+Shift+1..9` slots) deliberately.

<!-- Add out-of-scope ideas below this line -->
