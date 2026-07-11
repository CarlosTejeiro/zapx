# Changelog

All notable changes to ZAPX will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.20.9] - 2026-07-08

### Fixed
- **Windows portable: sessions with a typed username + password failed to
  authenticate** — in portable mode ZAPX deliberately doesn't depend on the OS
  keyring, but a session's own password was still stored in the keyring only,
  with no encrypted-database fallback (vault credentials already had one since
  0.20.1). So on the portable build the password couldn't be recovered and
  every connect failed authentication, while vault-based sessions worked.
  Portable installs now also write the same AES-256-GCM `session_secrets`
  fallback for a session's password/passphrase, and a keyring failure no longer
  aborts creating the session. (Sessions created on the portable build before
  this update must be re-created once to write the fallback.)
- **Typing did nothing after the multi-line paste dialog** — confirming or
  cancelling the "paste N lines?" dialog (or clicking outside it) left keyboard
  focus on the dismissed dialog instead of the terminal, so the pane looked
  frozen until you clicked it. Focus now returns to the terminal automatically.
- **Ctrl+C now copies the selection** — with text selected in the terminal,
  Ctrl+C copies it to the clipboard (and clears the selection) instead of only
  sending SIGINT; with nothing selected it still sends ^C to interrupt, as
  before. Ctrl+Shift+C always copies. (Copy-on-select and right-click copy are
  unchanged.)

## [0.20.8] - 2026-07-08

### Added
- **Connection progress for jump hosts (ProxyJump)** — opening a session that
  connects through one or more jump hosts now prints live status in the pane as
  it goes: `→ [jump 1/2] connecting to «bastion» (10.0.0.5:22)…`, `✓ jump host
  «bastion» ready`, `→ opening tunnel to «target» (…) via the jump host…`,
  `✓ connected`. Previously the pane stayed blank until it either connected or
  failed, so you couldn't tell whether it reached the jump host or the hop to
  the target was the problem.
- **Clearer jump-host labelling in the session dialog** — the "Connect through"
  field is now **Jump host (ProxyJump)** with an inline explanation: ZAPX
  connects to the jump host and then tunnels an *end-to-end* SSH session to the
  target through it (like `ssh -J`), so the jump host only forwards encrypted
  traffic and never sees the target session's credentials or host key.

### Fixed
- **Jump-host connection errors now say which hop failed** — a failure is
  reported as e.g. `jump host «bastion» (10.0.0.5:22) unreachable: …` or
  `target «device» (…) not reachable through the jump host: …`, instead of a
  bare `I/O error: … (os error 10060)` with no host. Unreachable hosts also
  fail after ~15s instead of hanging on the OS default (~21s on Windows); this
  timeout covers only the pre-auth TCP connect, so it never interrupts password
  or 2FA prompts.

## [0.20.6] - 2026-07-06

### Fixed
- **Split-pane shortcuts now work on every keyboard layout** — the split
  defaults were `Ctrl+\` / `Ctrl+Shift+\`, which were unreachable: holding
  Shift makes the `\` key emit a different character (`|` on a US layout) so
  the combo never matched, and `\` needs AltGr on layouts like Spanish. The
  split defaults are now **`Ctrl+Shift+H`** (horizontal) and **`Ctrl+Shift+K`**
  (vertical), which read and fire the same on any layout. In addition, any
  shortcut bound to a punctuation key is now matched by physical key position
  rather than the character the key emits, so such combos are robust to Shift
  and layout too (letter/number shortcuts still follow your layout as printed).
  Everything remains rebindable in **Settings → Shortcuts**.

## [0.20.5] - 2026-07-05

### Fixed
- **Duplicated / "crazy" typing in the terminal on Linux Wayland (the real
  fix)** — the remaining cause of duplicated keystrokes on Wayland was
  WebKitGTK's native-Wayland input handling, not the IME. It reproduced on
  Fedora + KDE Plasma + Wayland **regardless of the NVIDIA driver** (with the
  proprietary driver installed and without it) and persisted even with the IME
  modules emptied — but launching under XWayland (`GDK_BACKEND=x11`) fixes it
  completely. ZAPX now defaults to the X11/XWayland backend on Linux Wayland
  sessions. XWayland is present in every Wayland session, so this is safe; a
  user who wants native Wayland opts out with `ZAPX_ENABLE_WAYLAND=1` (and a
  user-set `GDK_BACKEND` is always respected). The 0.20.3 IME fix and the
  0.20.4 DMABUF change both stay.
- **`ZAPX_NO_LAUNCH_ENV=1` escape hatch** — set it to skip *all* of ZAPX's
  Linux launch-environment fixups (no re-exec, no `GDK_BACKEND`/IME overrides),
  to isolate whether one of them is interacting badly with a specific machine.

## [0.20.4] - 2026-07-03

### Fixed
- **Garbled rendering/typing on Linux laptops without the NVIDIA driver** — ZAPX
  no longer force-disables the WebKitGTK DMABUF renderer by default. That was a
  workaround for a high-CPU issue on the NVIDIA *proprietary* driver, but forcing
  it off broke rendering on other setups (ARM, and x86_64 hybrid laptops running
  on Intel/nouveau under Wayland — an NVIDIA GPU present but its driver not
  loaded), which is what re-broke the terminal in 0.20.3 even after the IBus fix.
  It's now opt-in: if you hit the NVIDIA high-CPU issue, launch with
  `WEBKIT_DISABLE_DMABUF_RENDERER=1`. The IBus/IME fix from 0.20.3 stays.

## [0.20.3] - 2026-07-03

### Fixed
- **Garbled / duplicated typing in the terminal on Linux (Wayland)** — on some
  setups (reproduced on Fedora + KDE Plasma + Wayland) WebKitGTK mishandles the
  IBus/Fcitx input method and produced duplicated or "ghost" characters, making
  the terminal unusable. ZAPX now launches with the input-method modules
  disabled by default on Linux (direct keyboard input). If you need composed /
  CJK / emoji input, set `ZAPX_ENABLE_IME=1` to keep your session's IME.
- **Terminal never opens at a 1×1 size** — hardened the terminal measurement so
  a failed font/layout measurement can't create a 1-column PTY (which made the
  remote shell's line editor misbehave). The size now waits for fonts/layout,
  falls back to 80×24 if measurement fails, and self-corrects to the pane's real
  size — with teardown-safe deferred fits.

## [0.20.2] - 2026-07-01

### Docs
- **New User Guide** (`docs/user-guide/`) — a full English walkthrough covering
  the workspace, sessions, the credential vault, macros (expect/send/wait, regex
  expects, `{{vault:…}}` references, folders, run-on-connect, import/export),
  snippets, SFTP, splits/multi-exec, logging, settings and the security model,
  with image slots to drop screenshots into.
- **README refreshed** to document the credential vault, the macros library and
  vault references, the "passwords typed at a prompt aren't recorded" behaviour,
  the resizable sidebar, the encrypted-DB keyring fallback, and a link to the
  new guide.

## [0.20.1] - 2026-07-01

### Fixed
- **Vault credentials work on unsigned builds** — a `{{vault:Name.Password}}`
  reference could fail at run time with *"keyring error: No matching entry found
  in secure storage"* because vault secrets lived only in the OS keyring, which
  on unsigned builds often denies the app access to its own entries. Vault
  secrets now also have the same AES-256-GCM encrypted-database fallback that
  saved-session passwords already use: the keyring stays primary, and the
  encrypted local copy is used when the keyring can't return the secret.
  (Vault entries created before this update need to be re-saved once — Edit the
  entry and re-enter the password — to write the fallback; existing secrets the
  keyring won't return can't be recovered automatically.)

## [0.20.0] - 2026-07-01

### Security
- **Passwords typed at a prompt no longer land in command history / recents** —
  when you type a value at a password/passphrase prompt (which the remote echoes
  as nothing), that line is no longer recorded to history, the recents bar, or
  hint suggestions, and it can never resurface as the "previous command" of a
  later entry. This is the primary guard for bare secrets: the existing filter
  only caught lines that *contain* a keyword like `password:`, not a bare secret
  value. Detection covers common prompts (including several non-English labels
  and wrapped prompt lines). Vault references and macro sends already bypassed
  this path.

### Added
- **Macro editor warns on a malformed vault reference** — a `send` step whose
  `{{vault:…}}` reference has an unrecognised field (e.g. a typo like
  `{{vault:admin.Pass word}}`) now shows a ⚠️ badge instead of the 🔒, so the
  mistake is visible before you run the macro. Pick the entry from the 🔒 menu
  to insert a correct `Name.Password` / `Name.Username` reference.

## [0.19.2] - 2026-07-01

### Fixed
- **Macros now report why they fail instead of failing silently** — a failed
  send, a vault reference that can't be resolved (e.g. no vault credential with
  that name), or an `expect` that never matches now surface a clear error toast
  naming the step and the reason (the `expect` message includes the pattern it
  was waiting for). Previously these errors were swallowed, so a macro that
  didn't work gave no feedback. Also covers on-connect macros. Added an
  integration test suite that drives `runMacro` against a simulated terminal
  stream to verify expect (literal + regex) matching and vault sends end-to-end.

## [0.19.1] - 2026-07-01

### Changed
- **Terminal fills more of the window** — the floating-card frame around the
  terminal shrank from 14px to 6px, so the pane sits closer to the sidebar,
  tabs and window edges while staying a rounded card. (The earlier 0.18.0 change
  only reduced the card's inner padding; this trims the outer frame that was
  the actual gap.)

## [0.19.0] - 2026-07-01

### Fixed
- **Macro `expect` steps no longer miss on fast hosts** — the runner used to
  clear its capture buffer at the start of each `expect`, which discarded output
  (including the very prompt being awaited) that arrived while the previous
  `send` step was in flight. On quick SSH/local sessions this made an `expect`
  time out without ever matching. It now keeps the stream and consumes only up
  to each match, so expects match reliably and a later step can't re-match
  already-seen text.

### Added
- **Mark a macro `expect` as a regular expression** — the step editor now has a
  regex (`.*`) toggle on `expect` rows, so you can type a pattern like
  `password:\s*$` and have it matched as a regex (no JSON escaping needed).

### Changed
- **Human-readable macro export** — exported macro files now store each macro's
  steps as a real JSON array (`"steps": [ … ]`) instead of an escaped
  single-string blob, so the file is readable and a regex reads as
  `"password:\\s*$"` rather than a quadruple-escaped mess. The format is v2;
  older v1 export files still import unchanged.

## [0.18.0] - 2026-07-01

### Added
- **Resizable sidebar** — drag the divider between the sidebar and the terminal
  to widen the sessions/macros panel (handy for long session names). The width
  is clamped between 180px and 50% of the window, re-clamps if the window
  shrinks, persists across restarts, and double-clicking the divider resets it
  to the default.

### Changed
- **Tighter, more homogeneous chrome** — smaller session-name and tab text, tab
  font aligned with the rest of the UI, and the terminal card now uses less
  inner padding so its content fills more of the rounded pane.

## [0.17.0] - 2026-07-01

### Changed
- **Macro `expect` steps are now pure "wait for text"** — an expect step waits
  for its pattern and nothing more; whatever you want to type goes in a separate
  `send` step. Because sending now always happens in a `send` step, you can
  reference a vault credential (username/password) for any input, including
  right after an expect — the vault picker lives on send steps. Existing macros
  that combined an expect with a payload are split automatically into
  expect + send when opened, with identical run-time behaviour (nothing is lost,
  no migration). Login-automation scripts on saved sessions are unaffected.

## [0.16.0] - 2026-06-30

### Changed
- **Macros sidebar matches the sessions design** — macro rows are now visually
  homogeneous with session rows: a leading glyph + the full-width name + a
  hover-revealed action cluster (clone, export, edit), so names are no longer
  crushed to an ellipsis. The inline "move to folder" dropdown and the ▲▼
  reorder arrows are gone.
- **Organise macros by drag-and-drop** — drag a macro to reorder it within its
  group, or drop it on a folder header (or the ungrouped area) to move it
  between folders, mirroring how saved sessions already work. Folder is also
  still editable in the macro dialog.
- **Macros section icons** — import/export now follow the usual convention
  (import = arrow down, export = arrow up), drawn from the same icon set as the
  rest of the sidebar; the clone action keeps the duplicate icon.

## [0.15.0] - 2026-06-30

### Removed
- **"Recent" sessions section in the sidebar** — it duplicated entries already
  shown in the main sessions list and only took up vertical space, so it has
  been removed. (The bottom bar's recent-*commands* zone is unrelated and
  stays.) Sessions are still ordered by last use in the main list.

## [0.14.2] - 2026-06-30

### Added
- **Linux ARM (aarch64) release builds** — the Release workflow now also builds
  on a native arm64 Linux runner and publishes an `aarch64-unknown-linux-gnu`
  AppImage/deb alongside the x86_64 ones, so ARM Linux users get an official
  binary (including the 0.14.1 DMABUF/ARM terminal fix) instead of having to
  build from source.

## [0.14.1] - 2026-06-30

### Fixed
- **Linux ARM terminal — typing and rendering** — on aarch64 (ARM) the app no
  longer force-disables the WebKitGTK DMABUF renderer. Forcing it off on ARM
  pushed rendering onto a fallback path that, on several ARM GPUs/VMs, painted
  garbage and swallowed keyboard input (the terminal "went crazy" and you
  couldn't type). The high-CPU workaround this came from is an x86/NVIDIA/
  Wayland issue, so it now applies only on x86_64; ARM keeps WebKit's default.
  An ARM user who still wants it off can export
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` before launch.

## [0.14.0] - 2026-06-30

### Added
- **Reference vault credentials by name** — macros and the New/Edit Session
  dialog can now use `{{vault:<Name>.Username}}` / `{{vault:<Name>.Password}}`
  (e.g. `admin.Password`) instead of a numeric id, so you only ever pass vault
  *variables* — never the secret itself. The value is resolved in the backend at
  run time and written straight to the session; it never enters the macro JSON,
  exports, logs, or the frontend. Vault names are unique, and names ending in
  `.username`/`.password` are rejected to keep references unambiguous. The
  legacy `{{vault:<id>}}` form still works.
- **Edit a macro as raw JSON** — the macro dialog has a Steps / JSON toggle so
  you can paste or hand-edit the step list directly, with live validation.
- **Run a macro automatically on connect** — attach an already-defined macro to
  a saved session ("automatismo"); it runs once each time the session reaches
  connected.
- **Clone a macro** — duplicate any macro (steps, color and folder) from the
  Macros sidebar zone.
- **Organise the Macros zone** — move a macro between folders, and reorder
  macros within a folder with ▲/▼ controls (order persists).

### Changed
- **Vault picker fills the Username** — choosing a saved credential in the
  New/Edit Session dialog now sets the Username field (left empty if the entry
  has none); the password stays in the vault and is never shown.
- **Macros no longer appear in the bottom snippet bar** — they live in the
  Macros sidebar zone instead, keeping the bottom bar to plain snippets.
- **Clearer Macros-section icons** — import and export now use distinct,
  action-specific icons (no more reused "copy" glyph) with tooltips.

## [0.13.0] - 2026-06-30

### Added
- **Credential vault** — create reusable named credentials (name + username +
  password, stored in the OS keyring). Attach one to a saved session, or
  reference it from a macro with a `{{vault:<id>}}` step. The secret is resolved
  in the backend at run time and written straight to the session — it never
  enters the macro JSON, exports, logs, or the frontend. Managed from a new
  Vault dialog.
- **Export / import macros as JSON** — export a single macro or all of them to a
  versioned `.json` file, and import them back (idempotent by name). Macros with
  a vault reference export the placeholder only, never the password.

### Changed
- **MobaXterm importer handles the real `[Macros]` export** — paste the
  `name=step|step|…` lines from an exported `MobaXterm.ini` (not just the macro
  editor's line list). `WAITFOR=`→expect, `SLEEP=`→wait, `RETURN`→Enter; encoded
  specials (`__DBLDOT__`→`:`, …) are decoded; multiple macros import at once; the
  format is auto-detected. On import, a password typed after a prompt can be
  stashed in the vault (you confirm which step) instead of landing in plaintext.

## [0.12.0] - 2026-06-29

### Added
- **Per-session auto-reconnect toggle** — the New/Edit Session dialog has an
  "Auto-reconnect if the connection drops" checkbox, so you can disable
  automatic reconnection on specific saved hosts. Manual reconnect always
  works. Quick-connect (and local) sessions never auto-reconnect.

## [0.11.5] - 2026-06-29

### Added
- **Anti-idle / keepalive per session** — the New/Edit Session dialog can send a
  string on a timer while connected (e.g. `\x00` for an invisible keepalive, or
  `!`), to beat server idle timeouts that fire despite SSH keepalives.

### Changed
- **Tab strip shows all sessions in a split** — a split tab now labels itself
  with its sessions ("A · B", or "A +2" for more) instead of keeping only the
  first session's name.
- **Releases publish directly** — the Release workflow no longer creates a
  draft, so re-pushing a tag stops producing duplicate empty draft releases.

## [0.11.4] - 2026-06-29

### Added
- **Open a saved session into a split pane** — splitting a pane now creates an
  empty placeholder instead of a local shell; clicking a session in the sidebar
  opens it into that focused empty pane. This lets you put two hosts side by
  side in one tab (and then Compare them), instead of every session opening in
  its own tab.

## [0.11.3] - 2026-06-29

Mis-tagged release — points at the 0.11.2 tree, with no changes over 0.11.2.

## [0.11.2] - 2026-06-29

### Fixed
- **Multi-host Compare no longer drops output as a timeout** — the runner only
  considered a host "done" when it recognised the device prompt (and not at all
  for unknown platforms), so when detection missed, the host waited out the full
  timeout and the comparison panel discarded its captured output ("No host
  returned output"). The runner now also settles a host once its output has
  echoed the command and gone quiet, regardless of prompt recognition, and the
  panel shows captured output even if a host ends as timed-out — only a host
  that returned nothing at all is reported as "no reply".

## [0.11.1] - 2026-06-29

### Added
- **Macros library in the sidebar** — a "Macros" section below the sessions
  lists your saved macros; click one to run it on the focused session, edit it
  in a dialog, or organise them into folders.
- **Import a MobaXterm macro** — paste the lines from MobaXterm's macro editor
  (text, `RETURN`, `SLEEP=ms`, `BACK`…) and ZAPX turns them into runnable
  expect/send/wait steps.
- **Reorder macro steps** — the step editor gained ▲/▼ controls to move a step
  up or down.
- **Onyx theme** — a true-black background with white text and vivid ANSI
  colours.

### Changed
- **Removed the user chip** from the sidebar footer (single-user app); the
  theme and settings buttons remain.

### Fixed
- **Suggestions popup no longer gets clipped** — the command-hint popup was
  anchored below the cursor inside the terminal's `overflow:hidden` stage, so it
  was cut off when the cursor sat near the bottom. It now flips above the line
  when there's no room below, clamps to the terminal's width, and caps its
  height to the available space (scrolling if needed).
- **Macro / login-script `send` now decodes escapes** — a step written as
  `cmd\r` presses Enter instead of typing the two literal characters `\` and
  `r`. `\r` `\n` `\t` `\b` `\e` `\0` `\\` and `\xHH` are all decoded (and an
  explicit trailing `\r` is no longer doubled); unknown escapes are left
  verbatim. Applies to both on-demand macros and connect-time login scripts.
- **MobaXterm import no longer defaults SSH to agent auth** — imported SSH
  sessions default to password (ZAPX prompts on connect) instead of failing on
  Windows hosts that have no OpenSSH agent / Pageant running. Connecting a
  credential-less password session now shows the inline re-auth prompt instead
  of erroring with "missing credential".

## [0.11.0] - 2026-06-29

### Added
- **Macros (expect / send / wait)** — login scripts gained explicit step kinds:
  *expect* a pattern then send, *send* immediately, or *wait* a fixed delay. The
  same engine powers on-demand macros: any button-bar snippet can be a macro
  (marked with a bolt) that runs the sequence against the focused session.
- **Clone a saved session** — duplicate a host from the sidebar, including its
  credential (copied into a fresh keyring entry), login script, triggers and
  forwards.
- **Per-host notes and tags** — annotate sessions; tags show as chips and both
  notes and tags are searchable from the sidebar filter.
- **"Recent" quick-access** — the sidebar surfaces your most recently used
  sessions in a dedicated section.
- **Multi-host compare on split tabs** — run a command across panes and diff the
  per-host output outside of grids too, with a one-click "copy as text report".
- **Session identity avatars** — initials tinted by the session colour, a
  four-state connection dot, and a protocol glyph (T/S/L) for non-SSH hosts.

### Changed
- **Visual redesign** — themed avatars and sidebar, tab-bar polish, SVG icons in
  the command palette, latency health colour in the status bar, title-bar
  shortcut chips, a unified frosted dialog backdrop, accented native form
  controls, and a refreshed settings shell.
- **Theme-aware polish** — terminal panes, the master input bar, the snippet bar
  and toasts now read correctly on all seven themes (no more hardcoded colours).
- **CI quality gates** — Prettier formatting, ESLint, type-check and tests on the
  frontend; `cargo fmt`/`clippy -D warnings`/tests on the Rust side.

### Fixed
- **SFTP streaming upload** closes the remote handle (awaiting the SFTP CLOSE)
  before reporting success, instead of relying on drop.
- **Terminal listener leak** and **session-log finalisation** on close.

### Security
- **Host keys fail-closed on direct connects** — unknown keys are rejected;
  Trust-On-First-Use is limited to bastions and jump targets, with a TOCTOU
  guard so the trusted key matches the one preflighted.
- **Vault seed is a random per-install keyfile** (no longer derived from a
  predictable path/constant), with a migration that re-encrypts existing
  secrets.
- **Tightened Content-Security-Policy**; the output highlighter preserves server
  ANSI and bounds regex pattern/compiled size.

## [0.10.4] - 2026-06-23

### Fixed
- **Jump hosts connect now** — the strict host-key check rejected any key not
  already in `known_hosts`, but a host reachable only through a bastion can't be
  preflighted by the UI (no direct route), so its key (and the bastion's) was
  never trusted and every jump-host connection was refused. Host keys are now
  Trust-On-First-Use: an unknown key is recorded and accepted, while a *changed*
  key is still rejected (MITM protection). Direct connections are unchanged.
- **Login-script steps run to completion** — each step's *send* now presses
  Enter automatically (unless you already ended the line), so a step like
  `enable` actually executes; previously the text was typed but not submitted,
  so the next step's *expect* never appeared and the script stalled.

### Changed
- **Wider session dialog** — the New/Edit Session form is now a two-column
  layout (wider, less tall) instead of a long vertical stack; it collapses to
  one column on narrow windows.

## [0.10.3] - 2026-06-19

### Fixed
- **Focus returns to the terminal after firing a snippet/button** — running a
  command button or snippet that carries no trailing newline (e.g.
  `ps -aux | grep `, meant to be completed by hand) left the caret on the button
  or variable dialog, so the next keystrokes went nowhere. ZAPX now hands focus
  back to the terminal right after sending.

## [0.10.2] - 2026-06-19

### Fixed
- **Maximize button did nothing** — the custom titlebar's maximize toggle was
  missing the `toggle-maximize` / `unmaximize` window permissions, so the call
  was silently denied. Added them (plus `is-maximized`).
- **Can't resize from the window corners** — the corner resize handles were only
  8×8 px, too small to grab reliably (the edges worked). Enlarged them to 16 px;
  the window control buttons now stack above the handles so they stay clickable.

## [0.10.1] - 2026-06-18

### Fixed
- **In-terminal search counter & navigation** (`Ctrl+F`): the bar could show
  "No results" even when matches existed, and the highlight could end up on the
  wrong match after stepping next then previous. Both came from the search
  addon's debounced match decorations. ZAPX now counts matches itself (always
  accurate) and lets xterm's selection mark the active match, so the counter is
  right and navigation lands cleanly. (Trade-off: matches are no longer all
  highlighted at once — only the active one is selected.)

## [0.10.0] - 2026-06-18

### Added
- **Connection reliability** — SSH **keepalive** (configurable server-alive
  interval, default 60s; Settings → SSH) keeps idle sessions alive and lets
  ZAPX notice dead links fast. When a link drops, a **"Connection lost" banner**
  offers a one-click reconnect, and **auto-reconnect** retries with backoff
  (toggle in Settings → SSH). Reconnect reuses the same terminal/scrollback.
- **Output triggers** — per session, when a **pattern** (literal or regex)
  matches a line of output, fire an action: **notify** (toast), **send** (type
  text + Enter back), or **bell**. Edited in the session dialog; a 300 ms
  per-trigger cooldown guards against send-loops.
- **Tab & session UX** — double-click a tab to **rename** it; right-click for a
  menu to **duplicate** the session or set a **colour** (shown as a left accent
  stripe). New terminal toolbar buttons: **reconnect**, **clear scrollback**,
  and **save output to file**.

## [0.9.4] - 2026-06-17

### Added
- **Proper in-terminal search** (`Ctrl+F`): the search bar now highlights every
  match (and marks them on the scrollbar), shows a live match counter
  (`3 / 12`), and adds **match-case**, **whole-word** and **regex** toggles.
  `Enter` jumps to the next match, `Shift+Enter` to the previous; the input
  turns red when there are no results.

## [0.9.3] - 2026-06-15

### Fixed
- **Copying from the terminal kept newlines on Linux**: selecting multi-line
  output and pasting it dropped the line breaks (everything landed on one line)
  on Linux/WebKitGTK, because the browser clipboard API strips them there.
  Copy-on-select now writes through Tauri's native clipboard, which preserves
  newlines; same path on macOS/Windows, with the browser API as a fallback.

## [0.9.2] - 2026-06-15

### Fixed
- **Linux high CPU — the DMABUF fix now actually applies**: 0.9.1 set
  `WEBKIT_DISABLE_DMABUF_RENDERER=1` from inside the process, which runs *after*
  WebKit has already started its web/GPU process, so it had no effect (CPU
  stayed high unless you launched with the variable set in the shell). ZAPX now
  re-execs itself once at startup with the variable already in the environment,
  so WebKit sees it from the first moment — the same result as
  `WEBKIT_DISABLE_DMABUF_RENDERER=1 zapx`, automatically. Still Linux-only and
  skipped when you've set the variable yourself.

## [0.9.1] - 2026-06-14

### Fixed
- **High CPU on Linux (WebKitWebProcess pinned near 100%)**: on many NVIDIA
  setups — especially under Wayland — WebKitGTK's DMABUF renderer drives the
  web process into a busy repaint loop, so the webview pegs a CPU core even
  when idle. ZAPX now sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` at startup on
  Linux (only when you haven't set it yourself), falling back to a renderer
  that idles correctly. Affects the AppImage and any Linux build.

### Changed
- **Linux: opaque window** — window transparency is now off on Linux (it stays
  on for macOS vibrancy / Windows acrylic). The window already fills the screen
  with an opaque background and square corners, so there's no visual change —
  it just drops the per-frame alpha compositing WebKitGTK was doing for nothing.
- **Cheaper cursor glow** — the glow themes (Phosphor, Amber) no longer animate
  the cursor's `drop-shadow` filter on an infinite loop (a constant repaint).
  The glow is now static, which looks the same at rest but stops the continuous
  CPU draw.

## [0.9.0] - 2026-06-14

### Added
- **Import MobaXterm port forwards**: the `[PortForwarding]` tunnels in a
  MobaXterm `.mobaconf`/`.ini` now import too. Each tunnel (Local/Dynamic/
  Remote) is attached to its SSH gateway — matched against an imported
  bookmark when possible, otherwise a session is created for the gateway —
  so the forwards auto-start on connect. (PuTTY already supported; SecureCRT
  forward storage still pending a sample.)
- **Arch Linux packaging**: a from-source `PKGBUILD` under `packaging/arch/`
  builds and installs ZAPX on Arch (`makepkg -si`). It clones the private repo
  over SSH at the release tag, so it's for your own/collaborators' machines —
  not the public AUR (private repo, no anonymous download).

### Changed
- **Release notes now include the CHANGELOG**: the release workflow appends
  this version's `CHANGELOG.md` section to the GitHub release body (after the
  curated `docs/releases/<tag>.md` notes, when present).

## [0.8.0] - 2026-06-14

### Added
- **Regex in login-script expects**: a step's `expect` can now be a regular
  expression (per-step `.*` toggle in the New/Edit Session dialog), matched
  against the ANSI-stripped output. An invalid regex falls back to literal
  matching with a warning. Great for variable prompts like `[\w.-]+[#>]\s*$`.
- **Edit remote files (SFTP)**: an "Edit" action in the SFTP browser downloads
  the file to a temp copy, opens it in your OS default editor, and re-uploads
  it on every save until the session closes — edit remote configs with your
  own editor, no manual download/upload round-trip.

## [0.7.0] - 2026-06-14

### Added
- **Customizable button bar — drag to reorder**: drag a button in the bottom
  bar to a new slot; the order persists. Lets you deliberately arrange the
  first nine (the `Ctrl+Shift+1..9` slots).
- **Snippet variables**: snippets/buttons can contain `{{name}}` placeholders
  (optional default with `{{name=Default}}`). Running one prompts for the
  values, then substitutes and sends — reusable command templates.
- **Import PuTTY port-forwards**: PuTTY's saved tunnels (`PortForwardings`)
  now import into the session's saved forwards, so they auto-start on connect.
  (MobaXterm/SecureCRT forward storage isn't reliably documented; their
  imports don't carry forwards yet.)
- **Global tunnels manager** (Tools → *Active tunnels…*): one panel listing
  every active port-forward across all sessions — kind, route and session —
  each with a stop button.

### Changed
- **License: proprietary.** ZAPX's own code is now proprietary (© Carlos
  Tejeiro, all rights reserved) from 0.7.0 onward. Releases up to 0.6.0 remain
  available under Apache 2.0 for those versions. Third-party dependencies stay
  under their own permissive licenses (see THIRD-PARTY-NOTICES.md).
- **"Check for updates" opens the releases page** instead of a non-functional
  in-app updater (the repo/releases are private; no public update manifest).

## [0.6.0] - 2026-06-13

### Added
- **Smarter hints — command-sequence learning**: ZAPX now learns which command
  you usually run right after another (per platform) and boosts that
  continuation in the hint popup the moment you start typing it — e.g. after
  `configure terminal`, `interface …` floats to the top. Learned suggestions
  show a "usual next" badge. Pure local heuristic over your own history (counts
  + recency), no telemetry; generic/unrecognised platforms don't learn, and
  clearing the command history clears the learned sequences too.
- **Saved port-forwards (auto-start tunnels per session)**: SSH sessions can
  now carry `-L` (local), `-D` (dynamic SOCKS5) and `-R` (remote) forwards,
  edited in the New/Edit Session dialog and opened automatically when the
  session connects — like SecureCRT/MobaXterm "Port Forwarding". Per-forward
  failures (e.g. a bind port already in use) are logged and skipped, never
  fatal to the connection. Forwards travel with export/import (remapped to the
  new session, applied only to newly-created sessions).
- **SFTP and Tunnels buttons back in the pane header**: the 0.2 redesign had
  hidden the toolbar that hosted them, so both features were unreachable in
  the UI. They now appear in each connected SSH pane's header.

### Fixed
- **Session dialog overflow**: the New/Edit Session form now caps to the
  window and scrolls, with a sticky Cancel/Save row — it could previously
  overflow off-screen on shorter windows.
- **macOS launch crash**: cosmetic window vibrancy (private macOS APIs) and
  first-run sizing are now contained, so a failure on brand-new macOS builds
  no longer aborts startup.

## [0.5.0] - 2026-06-13

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
- **Customizable button bar**: the bottom snippet bar is now a SecureCRT-style
  button bar — create buttons inline with the `+`, edit/delete them on hover,
  and give each an accent color. Buttons are platform-scoped or global; the
  first nine still fire with `Ctrl+Shift+1..9`. Snippets gain a `color` field
  (also editable in Tools → Snippets).

### Changed
- **UI unified to English** — the remaining Spanish strings (toasts,
  confirmations, command palette, dialogs and panels) now match the English
  menus, Settings and Snippets UI.

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
