# Backlog

Out-of-scope ideas tracked per ADR-007. Features listed here are not implemented until the active
block that gates them is closed.

Add entries with format: `- **Feature name**: brief description. Gated by: Bloque N.`

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
