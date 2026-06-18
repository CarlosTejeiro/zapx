# ZAPX

> Modern terminal for network engineers — SSH · Telnet · Serial in one app.

**Status: alpha — functional but not yet hardened for production.**

ZAPX is a multi-protocol terminal client built for network engineers who need a lightweight, fast,
and modern replacement for SecureCRT, MobaXterm, or PuTTY. Proprietary software — see
[License](#license).

---

## Features

| Feature | Details |
|---|---|
| **Protocols** | SSH2 (password / public key / SSH agent / 2FA keyboard-interactive), Telnet, Serial (COM / TTY), local shell |
| **Jump hosts & tunnels** | ProxyJump through saved sessions, port forwarding `-L` / `-D` (SOCKS5) / `-R` |
| **Reliability** | SSH keepalive (configurable), auto-reconnect on drop with a one-click banner, reconnect/clear/save-output toolbar |
| **Triggers** | Per-session regex/literal on output → notify, auto-send text, or ring the bell |
| **SFTP** | File browser with streaming up/downloads, progress and cancellation |
| **Session manager** | Folder tree, drag-and-drop, search, broadcast groups (open N hosts as a grid with a master input bar); tabs rename / colour / duplicate |
| **Splits & multi-exec** | Recursive split panes, grid view, broadcast typing to many sessions |
| **Login automation** | Expect-like login scripts (expect/send steps) per saved session |
| **Hints & snippets** | Command autocomplete from history + vendor catalogs (12 platforms), per-platform snippets on `Ctrl+Shift+1..9`, automatic platform detection from the prompt |
| **Keyword highlighting** | Per-rule regex, true-color ANSI, user-overridable vendor catalogs (Cisco, Juniper, Fortinet, Palo Alto, F5…) |
| **Session logging** | Plain or raw capture, 50 MB rotation, per-session history panel |
| **Themes** | 7 full themes — Parchment, Oxide, Fjord, Nocturne, Porcelain, Phosphor, Amber — UI chrome + terminal ANSI palettes, bundled Geist / JetBrains Mono fonts |
| **Command palette** | `Ctrl+K` — fuzzy-launch sessions, groups, actions and themes |
| **Portable & data control** | Windows portable exe, portable mode (data travels with the binary), user-selectable data folder |
| **Export / import** | One JSON file with sessions, folders, groups, snippets and highlight rules (never passwords) — idempotent import for backup, migration and team sharing. Imports from `~/.ssh/config`, PuTTY, MobaXterm and SecureCRT too |
| **Window memory** | Remembers size/position/maximized; first launch sizes to your monitor |

## Keyboard shortcuts

All customizable in **Settings → Shortcuts**. Defaults:

| Shortcut | Action |
|---|---|
| `Ctrl+K` | Command palette |
| `Ctrl+N` | New saved session |
| `Ctrl+Shift+N` | Quick Connect (ad-hoc, nothing saved) |
| `Ctrl+T` | New local shell tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Ctrl+\` / `Ctrl+Shift+\` | Split pane horizontally / vertically |
| `Ctrl+Shift+M` | Toggle multi-exec broadcast |
| `Ctrl+Shift+S` | Snippets |
| `Ctrl+Shift+1..9` | Fire snippet 1..9 |
| `Ctrl+F` | In-terminal search — match count, highlight-all, case/word/regex (`Enter` / `Shift+Enter` to step) |
| `Ctrl+,` | Settings |
| `Escape` | Close search / close dialog |

## Installation

Download the installer for your platform from the
[releases page](https://github.com/CarlosTejeiro/zapx/releases).

### macOS: "ZAPX is damaged and can't be opened"

The macOS bundles are **not yet signed/notarized** (an Apple Developer ID is
on the roadmap), so Gatekeeper quarantines the downloaded app and reports it
as damaged. The app is fine — clear the quarantine flag after dragging it to
Applications:

```sh
xattr -cr /Applications/ZAPX.app
```

or right-click the app → **Open** → **Open** on first launch.

### Windows portable

`ZAPX_x64_portable.exe` runs from any folder without installation (it needs
the WebView2 runtime, preinstalled on Windows 11 and updated Windows 10).
To make it fully self-contained — sessions, settings, snippet catalogs and
logs travelling with the exe — create an empty file named `portable` next to
it; everything is then kept in a `data/` folder beside the executable. In
portable mode credentials are stored AES-256-GCM-encrypted in the database
instead of the OS keyring: anyone with access to the folder can use them, so
treat the folder like a physical key.

### Custom data folder

Where ZAPX keeps its data (`zapx.db`, `session_logs/`, `catalogs/`) can be
changed in **Settings → Data** (migrates your data and restarts), or forced
with the `ZAPX_DATA_DIR` environment variable / `--data-dir <path>` CLI flag.

## Building from source

Prerequisites: Rust stable ≥ 1.80, Node.js LTS, pnpm.

```sh
# Install frontend dependencies (first time only)
cd frontend && pnpm install

# Start the Tauri dev server (compiles Rust + starts Vite)
cargo tauri dev
```

### Produce a release binary / installer

```sh
cargo tauri build
```

The MSI installer (Windows) is written to `target/release/bundle/msi/`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development setup.

## Architecture

```
zapx/
├── crates/
│   ├── app/               # Tauri shell — commands, state, IPC bridge, data-dir resolver
│   ├── core-transport/    # SSH, Telnet, Serial, local PTY, SFTP, port forwarding
│   ├── core-persistence/  # SQLite via rusqlite — sessions, rules, logs, settings
│   ├── core-vault/        # OS keyring + AES-256-GCM portable fallback
│   ├── core-highlight/    # Regex keyword highlighting, ANSI injection
│   ├── core-hints/        # Command suggestions: history, vendor catalogs, snippets
│   ├── core-terminal/     # VT100/xterm escape-sequence parser
│   ├── core-logging/      # Session log writer with 50 MB rotation
│   ├── core-session/      # Session lifecycle types
│   └── core-config/       # Config helpers
├── frontend/              # Svelte 5 + xterm.js + Tailwind CSS 4
└── xtask/                 # Developer automation (cargo xtask dev, etc.)
```

## Security

- Passwords are stored in the **OS keyring** (Windows Credential Manager, macOS Keychain,
  libsecret on Linux) — never in SQLite and never in logs. In portable mode they are
  stored AES-256-GCM-encrypted in the database instead (see *Windows portable* above).
- Host key verification with SHA-256 fingerprints and known_hosts.
- Session logs capture raw terminal bytes only; credentials are not echoed by the PTY.
- `#![forbid(unsafe_code)]` in all core crates.

## Releases

Per-version notes live in [docs/releases/](docs/releases/) and the full history in
[CHANGELOG.md](CHANGELOG.md).

## Roadmap

- [ ] Code signing: Apple Developer ID (macOS notarization, fixes the "damaged" prompt) + Windows certificate
- [ ] App updates: private update server (Windows/Linux self-update; macOS gated on signing)
- [ ] Importers: SecureCRT port forwards (PuTTY + MobaXterm already supported)
- [x] Linux packaging: from-source Arch `PKGBUILD` (`packaging/arch/`) — AUR/Flathub/Snap pending public distribution
- [x] Regex matching in login-script expects (0.8)
- [x] Edit remote files with a local editor over SFTP (0.8) — OS drag & drop upload deferred

## License

**Proprietary — © Carlos Tejeiro. All rights reserved.** See [LICENSE](LICENSE).
This repository and its source code are private; nothing here is licensed for
reuse, redistribution or modification.

Third-party open-source components are under their own permissive licenses; see
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

> Releases up to and including 0.6.0 were published under Apache 2.0 and remain
> available under that license for those specific versions; this proprietary
> license applies from 0.7.0 onward.
