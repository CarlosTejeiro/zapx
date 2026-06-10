# ZAPX

> Modern terminal for network engineers — SSH · Telnet · Serial in one app.

**Status: alpha — functional but not yet hardened for production.**

ZAPX is a multi-protocol terminal client built for network engineers who need a lightweight, fast,
and modern replacement for SecureCRT, MobaXterm, or PuTTY. Free and open source under Apache 2.0.

---

## Features (v0.1.0)

| Feature | Details |
|---|---|
| **Protocols** | SSH2, Telnet, Serial (COM / TTY) |
| **Session manager** | Folder tree, drag-and-drop, search |
| **Quick Connect** | Open SSH/Telnet/Local sessions instantly without saving |
| **Keyword highlighting** | Per-rule regex, true-color ANSI, 10 default Cisco/IOS patterns |
| **Session logging** | Raw byte capture, 50 MB rotation, per-session history panel |
| **Themes** | 10 built-in colour schemes (One Dark, Dracula, Tokyo Night, Nord, and more) |
| **Appearance** | Live font family, font size, cursor style, cursor blink |
| **In-terminal search** | xterm.js SearchAddon, incremental, Ctrl+F |
| **Keyboard shortcuts** | Ctrl+N Quick Connect, Ctrl+T New Tab, Ctrl+W Close, Ctrl+Tab cycle, Ctrl+, Settings |

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| `Ctrl+N` | Quick Connect dialog |
| `Ctrl+T` | New local shell tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+F` | Open in-terminal search |
| `Ctrl+,` | Open Settings |
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
rust_terminal/
├── crates/
│   ├── app/               # Tauri shell — commands, state, IPC bridge
│   ├── core-transport/    # SSH, Telnet, Serial, local PTY transports
│   ├── core-persistence/  # SQLite via rusqlite — sessions, rules, logs, themes
│   ├── core-vault/        # OS keyring (passwords never in SQLite)
│   ├── core-highlight/    # Regex keyword highlighting, ANSI injection
│   ├── core-logging/      # Session log writer with 50 MB rotation
│   ├── core-session/      # Session lifecycle types
│   └── core-config/       # Config helpers
├── frontend/              # Svelte 5 + xterm.js + Tailwind CSS 4
└── xtask/                 # Developer automation (cargo xtask dev, etc.)
```

## Security

- Passwords are stored in the **OS keyring** (Windows Credential Manager, macOS Keychain,
  libsecret on Linux) — never in SQLite and never in logs.
- Session logs capture raw terminal bytes only; credentials are not echoed by the PTY.
- `#![forbid(unsafe_code)]` in all core crates.

## Roadmap

- [ ] macOS and Linux builds
- [ ] Signed Windows installer (code-signing certificate)
- [ ] Auto-update via Tauri Updater
- [ ] Jump hosts / SSH tunnels
- [ ] SFTP file browser
- [ ] Scripted sessions (Expect-like automation)

## License

Apache 2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE).
