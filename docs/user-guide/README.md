# ZAPX — User Guide

A modern multi-protocol terminal for network engineers — SSH, Telnet, Serial
and local shell in one app, with a session manager, credential vault, macros,
SFTP, port forwarding, split panes and multi-host broadcast.

> Screenshots below are marked with `<!-- CAPTURE: … -->` notes describing what
> each image should show. Drop PNGs into `docs/user-guide/images/` using the
> referenced file names to fill them in.

## Contents

1. [The workspace](#1-the-workspace)
2. [Sessions](#2-sessions)
3. [Connecting & authentication](#3-connecting--authentication)
4. [Credential Vault](#4-credential-vault)
5. [Macros](#5-macros)
6. [Snippets & the bottom bar](#6-snippets--the-bottom-bar)
7. [Command hints & platform detection](#7-command-hints--platform-detection)
8. [Keyword highlighting](#8-keyword-highlighting)
9. [SFTP & port forwarding](#9-sftp--port-forwarding)
10. [Splits & multi-exec](#10-splits--multi-exec)
11. [Session logging](#11-session-logging)
12. [Settings & appearance](#12-settings--appearance)
13. [Import & export](#13-import--export)
14. [Security model](#14-security-model)
15. [Keyboard shortcuts](#15-keyboard-shortcuts)

---

## 1. The workspace

<!-- CAPTURE: images/workspace.png — full window: sidebar on the left (SESSIONS +
MACROS zones), the tab strip on top, a connected terminal card, and the bottom
snippet bar. Use the v0.20 UI. -->
![ZAPX workspace](images/workspace.png)

- **Left sidebar** — your saved **Sessions** (in folders) and your **Macros**
  library. **Drag the divider** between the sidebar and the terminal to widen it
  (up to half the window) so long session names fit; double-click the divider to
  reset. The width is remembered across restarts.
- **Tab strip** — one tab per open session (or per split). Rename, recolour,
  duplicate or close from the tab; `Split` and `Multi` buttons on the right.
- **Terminal card** — the active terminal, a rounded card. Its header exposes
  per-session actions (record log, SFTP, port forwards, split).
- **Bottom bar** — one-click **snippets** for the focused session's platform,
  plus a **Recents** zone of recently-used commands.
- **Status bar** — connection state, host, protocol, encoding and terminal size.

---

## 2. Sessions

<!-- CAPTURE: images/new-session.png — the New/Edit Session dialog showing
host/port/username, the auth method selector, and the "Use a saved credential
(vault)" picker. -->
![New session dialog](images/new-session.png)

- **New saved session** (`Ctrl+N`): pick a protocol (SSH / Telnet / Serial /
  local), fill host, port, username and authentication. Optionally set a colour,
  notes and comma-separated tags.
- **Quick Connect** (`Ctrl+Shift+N`): connect ad-hoc without saving anything.
- **Organise**: create folders, **drag-and-drop** sessions between them and
  reorder; **search** (`⌘K` / `Ctrl+K` focuses the box) matches name, tags and
  notes.
- **Row actions** (hover): edit, duplicate (a fully independent copy), delete.
- Sessions are ordered by last use, so the ones you open most surface at the top
  of their folder.

---

## 3. Connecting & authentication

- **SSH auth**: password, public key, SSH agent, or keyboard-interactive (2FA).
- **Use a saved credential (vault)**: in the session dialog, pick a vault entry
  instead of typing a password. Choosing one fills the **Username** field (left
  empty if the entry has none); the **password stays in the vault** and is never
  shown. See [Credential Vault](#4-credential-vault).
- **Host-key verification**: on first connect ZAPX shows the server's SHA-256
  fingerprint for you to approve; it's pinned in `known_hosts` and a later change
  is flagged.
- **Jump hosts / ProxyJump**: route a session through another saved session.
- **Auto-reconnect**: on by default; a dropped link shows a one-click reconnect
  banner. Disable per session with the "Auto-reconnect if the connection drops"
  checkbox. Quick-connect and local sessions never auto-reconnect.
- **Anti-idle / keepalive**: send a string on a timer while connected (e.g. an
  invisible `\x00`) to beat server idle timeouts.

---

## 4. Credential Vault

Reusable named credentials, stored securely and referenced by name — so a
password lives in exactly one place and never appears in a macro, an export, or
a log.

<!-- CAPTURE: images/vault-dialog.png — the Vault manager dialog listing entries,
with the add/edit form (Name, Username, Password). -->
![Credential vault](images/vault-dialog.png)

- **An entry has a Name (label), a Username, and a Password.** The **Name** is
  what you reference elsewhere — it is *not* the username.
- **Create/edit** entries from the Vault dialog. Names are unique.
- **Use it two ways**:
  - Attach it to a saved session (the session's auth "Use a saved credential").
  - Reference it from a macro: `{{vault:<Name>.Password}}` or
    `{{vault:<Name>.Username}}` (see [Macros](#5-macros)).
- **Where the secret lives**: the OS keyring (Windows Credential Manager, macOS
  Keychain, libsecret on Linux). If the keyring can't return it (e.g. on an
  unsigned build), ZAPX falls back to an AES-256-GCM copy encrypted in its
  database. The secret is resolved in the backend at run time and written
  straight to the session — it never reaches the UI, JSON, exports or logs.

> If a vault reference stops working after an app update with a
> "keyring … No matching entry" error, **re-save the entry** once (Edit →
> re-enter the password) to write the encrypted fallback.

---

## 5. Macros

A macro is a saved **expect / send / wait** sequence you run against the focused
session with one click — for logins, jump-host hops, and repetitive command
flows. Macros live in the **MACROS** zone of the sidebar.

<!-- CAPTURE: images/macros-zone.png — the sidebar MACROS section with a couple
of folders and macros, and the header icons (import ↓ / export ↑ / MobaXterm /
new +). -->
![Macros sidebar zone](images/macros-zone.png)

### Running & organising

- **Click a macro** to run it on the focused session.
- Rows behave like sessions: hover for **clone / export / edit**; **drag** to
  reorder within a folder or drop onto a folder to move it. Organise into
  folders.
- Header icons: **export all** (arrow up), **import JSON** (arrow down), **import
  from MobaXterm**, **new** (+).

### Editing steps

<!-- CAPTURE: images/macro-editor.png — the Edit macro dialog showing the Steps
list: an `expect` step with the regex (.*) checkbox, a `send` step with the 🔒
vault picker, and a `wait` step. -->
![Macro editor](images/macro-editor.png)

Each step is one of:

- **expect** — wait for a string to appear. Tick **`.*`** to match it as a
  **regular expression** (e.g. `password:\s*$`); leave it unticked for a plain
  substring. An expect step *only waits* — it doesn't send anything.
- **send** — send text. Use `\r` for Enter (e.g. `show version\r`). To send a
  vault secret, use the **🔒 vault** picker on the step: it inserts
  `{{vault:<Name>.Password}}` (or `.Username`). A ⚠️ badge warns if a vault
  reference is malformed (e.g. a typo in the field).
- **wait** — pause a number of milliseconds.

A typical login macro:

```
send    ssh user@10.0.0.1\r
expect  password:\s*$            (.* ticked)
send    {{vault:core-router.Password}}\r
```

> **Why expect and send are separate:** expect just waits; the typing goes in the
> next `send` step, which is where the vault picker lives — so you can reference
> a vault password for any prompt. Older macros that combined the two are split
> automatically when opened.

### Vault references

- `{{vault:<Name>.Password}}` / `{{vault:<Name>.Username}}` — `<Name>` is the
  vault entry's **Name** (label), and the field must be exactly `Password` or
  `Username`. The secret is resolved server-side; only the placeholder is ever
  stored in the macro or its export.
- Pick from the **🔒 menu** rather than typing, to avoid typos.

### Edit as JSON

Toggle **Steps / JSON** in the macro dialog to hand-edit the step list as JSON
(with live validation). Remember JSON escaping: a regex `password:\s*$` is
written `"password:\\s*$"`.

### Run on connect

Attach a macro to a saved session so it runs automatically each time the session
connects (set in the New/Edit Session dialog).

### Import / export

- **Export** one macro or all of them to a human-readable JSON file (steps are a
  real array; vault references export as the placeholder only, never the
  secret).
- **Import** those JSON files back (older single-string exports still import).
- **Import from MobaXterm**: paste a MobaXterm macro/`[Macros]` export; ZAPX
  converts `WAITFOR=`→expect, `SLEEP=`→wait, `RETURN`→Enter, and can stash a
  password into the vault instead of importing it in plaintext.

---

## 6. Snippets & the bottom bar

- **Snippets** are one-line commands shown in the bottom bar, scoped to the
  focused session's platform (globals + that vendor's set). Fire snippet 1–9
  with `Ctrl+Shift+1..9`.
- **Recents** shows recently-used commands for quick re-fire.
- Snippets and **macros are separate**: snippets live in the bottom bar; macros
  live in the sidebar MACROS zone.
- **Security**: a value you type at a **password prompt is never recorded** to
  history, Recents or hints (and can't resurface as a "previous command").

---

## 7. Command hints & platform detection

- As you type, ZAPX suggests commands from your history and per-vendor catalogs
  (12 platforms). Accept a ghost suggestion with `→` / `End`; open the
  suggestion popup with `Ctrl+Space`.
- The session's **platform is auto-detected** from the prompt, so suggestions and
  snippets match the device.

---

## 8. Keyword highlighting

Per-rule regex highlighting with true-colour ANSI, over user-overridable vendor
catalogs (Cisco, Juniper, Fortinet, Palo Alto, F5…). Configure rules in Settings.

---

## 9. SFTP & port forwarding

<!-- CAPTURE: images/sftp.png — the SFTP file browser dialog with a transfer in
progress. -->
![SFTP browser](images/sftp.png)

- **SFTP** (SSH sessions): browse, upload and download with progress and cancel;
  edit a remote file with your local editor.
- **Port forwarding**: local `-L`, dynamic `-D` (SOCKS5) and remote `-R`
  forwards, managed per session from the pane header.

---

## 10. Splits & multi-exec

- **Split panes** (`Ctrl+\` / `Ctrl+Shift+\`): recursive horizontal/vertical
  splits; open a saved session into an empty split from the sidebar.
- **Multi-exec broadcast** (`Ctrl+Shift+M`): type once, send to many sessions;
  open N hosts as a grid with a master input bar and **Compare** their output.

---

## 11. Session logging

Record a session to a rotating file (plain text or raw), started from the pane
header's record button. A per-session history panel lists past logs.

---

## 12. Settings & appearance

<!-- CAPTURE: images/settings-terminal.png — Settings → Terminal panel: Theme
selector, Font family + size, cursor style. -->
![Settings — terminal](images/settings-terminal.png)

- **Themes**: full UI + terminal ANSI palettes (Settings → Terminal).
- **Font**: family and size for the terminal; cursor style and blink.
- **Shortcuts**: every keybinding is customizable (Settings → Shortcuts).
- **Data folder**: where ZAPX stores its database, logs and catalogs (Settings →
  Data), or force it with `ZAPX_DATA_DIR` / `--data-dir`.

---

## 13. Import & export

- **Sessions bundle**: one JSON with sessions, folders, groups, snippets and
  highlight rules (**never passwords**) — idempotent import for backup,
  migration and team sharing.
- **Importers**: `~/.ssh/config`, PuTTY, MobaXterm and SecureCRT.
- **Macros**: exported/imported separately as their own JSON (see
  [Macros](#5-macros)).
- The **vault and its secrets are never included** in any export.

---

## 14. Security model

- Passwords are stored in the **OS keyring**; when the keyring can't return a
  secret, an **AES-256-GCM** copy encrypted in the database is used as a
  fallback (also the mechanism in portable mode). Secrets never enter SQLite in
  cleartext, exports, or logs.
- **Vault secrets** are resolved in the backend at run time and written straight
  to the session — the macro/export only ever holds the `{{vault:…}}`
  placeholder.
- **Passwords typed at a prompt** are not recorded to command history / recents.
- **Host-key verification** with SHA-256 fingerprints and `known_hosts`.
- Session logs capture terminal bytes only; the PTY doesn't echo passwords.

---

## 15. Keyboard shortcuts

All customizable in **Settings → Shortcuts**. Defaults:

| Shortcut | Action |
|---|---|
| `Ctrl+K` | Command palette |
| `Ctrl+N` | New saved session |
| `Ctrl+Shift+N` | Quick Connect |
| `Ctrl+T` | New local shell tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous tab |
| `Ctrl+\` / `Ctrl+Shift+\` | Split horizontally / vertically |
| `Ctrl+Shift+M` | Toggle multi-exec broadcast |
| `Ctrl+Shift+S` | Snippets |
| `Ctrl+Shift+1..9` | Fire snippet 1..9 |
| `Ctrl+Space` | Command suggestion popup |
| `Ctrl+F` | In-terminal search |
| `Ctrl+,` | Settings |
| `Escape` | Close search / dialog |
