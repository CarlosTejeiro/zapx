# Backlog

Out-of-scope ideas tracked per ADR-007. Features listed here are not implemented until the active
block that gates them is closed.

Add entries with format: `- **Feature name**: brief description. Gated by: Bloque N.`

## Planned for 0.6

- **Saved port-forwards (auto-start tunnels per session)**: the tunnel engine
  already supports `-L` / `-D` (SOCKS5) / `-R` (`crates/core-transport/
  forwards.rs`), but forwards are transient today — configured on a live
  session via `TunnelsDialog` (keyed by runtime session id) and lost on close,
  with no DB persistence. Add a `session_forwards` table + migration, an
  editor in the New/Edit Session dialog to define forwards (local/dynamic/
  remote, bind addr+port, target), and auto-open them when the session
  connects — matching SecureCRT/MobaXterm "Port Forwarding" config. Include
  them in export/import (`transfer.rs`) and the third-party importers
  (PuTTY tunnels, MobaXterm/SecureCRT forward fields).
- **Smarter hints — command-sequence learning**: beyond frecency, learn
  per-platform command bigrams ("after `conf t` you usually run `interface…`")
  from `command_history` (already persisted with timestamps per session) and
  boost those continuations in the hint popup right after the previous
  command runs. Pure local heuristic (counts + recency decay), no telemetry;
  fits in `core-hints` next to the existing frecency scorer.

## Ideas

- **Linux packaging — AUR / Flathub / Snap**: the AppImage/deb/rpm already
  work on Arch, but native channels help adoption. Cheapest first: publish a
  `zapx-bin` PKGBUILD to the AUR that repacks the released .deb (update via
  CI on each release). Then Flathub (best cross-distro discoverability for
  GUI apps; needs a flatpak manifest + review) and optionally Snapcraft.
- **Drag-to-reorder button-bar buttons**: the bar buttons (snippets) have a
  `sort_order`; add drag-to-reorder in the bar so users can arrange the first
  nine (the `Ctrl+Shift+1..9` slots) deliberately.

<!-- Add out-of-scope ideas below this line -->
