# Backlog

Out-of-scope ideas tracked per ADR-007. Features listed here are not implemented until the active
block that gates them is closed.

Add entries with format: `- **Feature name**: brief description. Gated by: Bloque N.`

## Planned for 0.5

- **Third-party session importers (PuTTY / MobaXterm / SecureCRT)**: parsers
  that produce an `ExportFile` and feed the existing idempotent pipeline
  (`crates/app/src/commands/transfer.rs::apply_import` — remapping, duplicate
  skipping and warnings come for free; the `~/.ssh/config` importer in
  `crates/app/src/importers/ssh_config.rs` is the template). Sources: PuTTY =
  Windows registry `HKCU\Software\SimonTatham\PuTTY\Sessions` (or exported
  .reg file), MobaXterm = `MobaXterm.ini` `[Bookmarks]` sections (positional
  `#`-separated fields), SecureCRT = its XML config export. Partial mappings
  expected: import what ZAPX supports, warn about the rest.
- **Multi-send in regular tabs**: today broadcast typing works in grid tabs
  (MasterInputBar) and via the Multi toggle fan-out; extend the master-input
  experience to ordinary split/tab layouts so any set of open sessions can be
  driven at once, MobaXterm-style.
- **Send command list to N sessions (SecureCRT "Command Window")**: paste or
  load a list of commands and dispatch it line-by-line to selected open
  sessions — sequential with per-line pacing (and reuse the prompt-return
  detection from the orchestrator/CommandRunner for "wait for prompt between
  lines"). Combine with broadcast groups for one-click fleet changes.
- **Custom snippet button bar (SecureCRT button bar)**: today the bar under
  the terminal (`SnippetButtonBar.svelte`) auto-populates from per-platform
  snippets + recents. Add a user-curated mode: the user creates/pins buttons
  (label, command, optional color, order), persisted per platform or global,
  drag-to-reorder, same `--zx-*` design system as the rest of the chrome.
  `Ctrl+Shift+1..9` keeps firing the first nine visible buttons.

## Ideas

- **Linux packaging — AUR / Flathub / Snap**: the AppImage/deb/rpm already
  work on Arch, but native channels help adoption. Cheapest first: publish a
  `zapx-bin` PKGBUILD to the AUR that repacks the released .deb (update via
  CI on each release). Then Flathub (best cross-distro discoverability for
  GUI apps; needs a flatpak manifest + review) and optionally Snapcraft.
- **Smarter hints — command-sequence learning**: beyond frecency, learn
  per-platform command bigrams ("after `conf t` you usually run `interface…`")
  from `command_history` (already persisted with timestamps per session) and
  boost those continuations in the hint popup right after the previous
  command runs. Pure local heuristic (counts + recency decay), no telemetry;
  fits in `core-hints` next to the existing frecency scorer.

<!-- Add out-of-scope ideas below this line -->
