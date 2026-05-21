# Handoff — PYLON SSH Client (Parchment theme)

A cross-platform SSH client for Windows / macOS / Linux. Tabbed terminal sessions,
split panes, MultiExec broadcast, hybrid sidebar (favorites + folder tree),
command palette, status bar with live telemetry, command snippets.

Direct competitors: SecureCRT, MobaXterm, Termius. PYLON's differentiator is
**look and feel** — a more polished, modern aesthetic than incumbents.

This bundle ships **Parchment** as the only theme. Parchment is a **light-mode**
shell with a **dark terminal embedded** inside — same approach as VS Code Light
or IntelliJ Light. The intent is a calm, daytime-friendly app for sysadmins who
spend hours in router/switch CLIs and don't want to stare at pure black.

---

## About these files

The files in `design_source/` are **design references created in HTML/React**.
They are interactive prototypes showing the intended look, layout, and behavior —
**not production code to copy verbatim**. They run in a browser as a comparison
canvas; the real product is a desktop application.

Your task is to **recreate this design in a real desktop-app codebase** using the
right tooling for an SSH client:

- **Recommended stack:** Tauri (Rust + WebView) or Electron, with React +
  TypeScript for the renderer. Tauri is preferred for bundle size, memory,
  and the fact that this project is "rust terminal".
- **Terminal emulation:** [xterm.js](https://xtermjs.org/) for the actual VT320
  terminal in each pane (the mocks render *static transcripts* — xterm.js will
  render *real* SSH I/O).
- **SSH transport:** `russh` (Rust) or `ssh2` (Node-side).
- **Pane layout:** [react-resizable-panels](https://github.com/bvaughn/react-resizable-panels)
  or [allotment](https://github.com/johnwalley/allotment).
- **DnD for tab reordering:** `@dnd-kit/sortable`.
- **Icons:** Lucide React (or Tabler).

If the codebase already exists with conventions, **follow them** — these are
just reasonable defaults.

---

## Fidelity

**High-fidelity.** Colors, spacing, typography, and interaction details are
intended to be reproduced pixel-faithfully. The HTML mocks are themed by a
single `theme` object — your design tokens should mirror that shape.

---

## Design language — Parchment

**Mood**: a "stationery" / "field-notebook" aesthetic. Daytime-comfortable,
quiet, premium-feeling without being flashy. Think: a serious notebook on a
clean desk, not a hacker movie.

**Two surfaces:**

1. **Chrome surfaces** (titlebar, sidebar, tab bar, status bar, command
   palette, dialogs) are **warm paper tones**: `#efeae0`, `#f0ece2`,
   `#f5f2eb`, `#faf8f3`. Slightly off-white, never pure white. Borders
   are soft brown-tinted `rgba(85,70,40,0.10)`.

2. **Terminal surfaces** are **dark slate**: `#1d1e26`. The terminal is the
   "primary work surface" — it stays high-contrast for legibility regardless
   of the chrome being light. Inside the terminal, prompt + cursor + banner
   use a muted indigo `#8e85e0`; output is light gray `#d4d6dc`; accent
   (table heads, key labels) is warm amber `#d4a26c`.

**Accents:**

- Primary: `#564ca0` — muted indigo. Used on sidebar tag-pill active session
  border, command palette focus ring, focused pane inset, button focus rings.
- Secondary: `#b3793a` — warm amber. Used sparingly: connection-active dot
  glow when accent indigo would clash, MultiExec pill, command palette icon.

**No glows. No gradients on chrome.** The visual interest comes from the
warm/cool contrast between chrome and terminal, and from the saved-session
colors that live across both.

---

## Screens / Views

The prototype is a single main window. There is no auth / onboarding screen yet
(see "Next steps for design"). The window contains, top to bottom:

### 1. Title bar — `32px` tall

Custom-drawn window chrome (Tauri `decorations: false` + frameless Electron).

- **Background:** `#efeae0` (warm paper).
- **Left:** PYLON wordmark (700-weight, 1px letter-spacing) in dark
  `#2d2a24` + 18×18 logo glyph (a stylized `>` shape — see SVG path in
  `app.jsx` `TitleBar`). The logo glyph uses a linear-gradient from
  `#564ca0` to `#b3793a`. Then the active session name in dim text
  `#8a8174`: `— core-sw-01 — netops@10.5.0.1`.
- **Center / drag area:** transparent, draggable to move the window.
- **Right:** menu strip `File · Edit · View · Session · Tools · Help` (each
  is a button with hover background `itemHoverBg = rgba(85,70,40,0.04)`).
  Then Windows traffic lights: minimize (—), maximize (▢), close (✕). Close
  button hover is solid red `#e81123` with white glyph. Each button is
  44×32px.

### 2. Sidebar — `248px` wide, full-height left

Hybrid: pinned favorites on top, then folder tree.

- **Background:** `#f0ece2`. Border-right: `rgba(85,70,40,0.10)`.
- **Search input** at the top — full width, background `#fdfbf6`, 1px
  divider border, `⌕` glyph inside in `#8a8174`. Filters tree in real time
  (case-insensitive, matches name + tag). Focus state: 2px indigo ring at
  `#564ca0 + "22"`.
- **Section headers** — uppercase 11px 600-weight in dim `#8a8174`, with
  disclosure caret (`▶` rotating to `▼`). The `★ Favorites` section uses
  indigo `#564ca0` for its label and is open by default.
- **Session row** — 12px left padding from section content, 26px overall.
  Left to right: 8×8 colored status dot · session name (12.5px dark
  `#2d2a24`) · tag pill (10px uppercase, colored 1px border at
  `color + "44"`, transparent fill). Active session: left-border 2px in the
  session's color, background tint `rgba(86,76,160,0.10)`. Hover: `rgba(85,70,40,0.04)`.
  Status dot has a soft glow + slow pulse animation when "active".
- **Footer** — user chip (initial in a 22px gradient square from indigo to
  amber), name in `#2d2a24`, host count in `#8a8174`, settings cog. 10px
  border-top.

### 3. Tab bar — `36px` tall, sits under titlebar inside the main pane

Chrome-style draggable tabs. `@dnd-kit` for reorder.

- **Background:** `#efeae0` (same as titlebar — visually continuous).
- **Tab** — min 130px / max 220px wide. Idle: transparent bg, dim text
  `#7d7466`. Active: background `#faf8f3` (lifts off the bar like real
  paper), accent-colored top border (the tab's own color, 1.5px), border-
  radius `6px 6px 0 0`, raised 4px (margin-top `4px` vs `8px` idle).
- **Contents** — 7px colored dot · tab name (ellipsis) in `#2d2a24` · close
  `✕` (shows on hover or when active, dim).
- **`+` button** after tabs to create a new session.
- **Right side:** layout toggles. Split-vertical and MultiExec icons. Active
  state: 1px indigo border + indigo tint background `rgba(86,76,160,0.10)`.
  **No glow** on these — Parchment is flat.

### 4. Terminal area — fills remaining vertical space

One or two panes side-by-side, separated by a 5px draggable resizer in
`rgba(85,70,40,0.10)`.

- **Pane header** — 22px tall, background `rgba(0,0,0,.25)` overlaid on the
  dark terminal area (effectively darker still, ~`#15161c`). Colored dot
  with glow · session name in light `#d4d6dc` · dim host in `#7e8090`.
  Right side shows a `MULTI` indigo pill when MultiExec is on.
- **Terminal viewport** — fills the rest. Renders ANSI output via xterm.js in
  the real product. **Background:** `#1d1e26`. **Foreground:** `#d4d6dc`.
  See "Terminal styling" below for the full xterm.js mapping.
- **Focus indicator** — the focused pane has an inset 1.5px indigo border at
  `#564ca0 + "88"`. Click any pane to focus.

### 5. Status bar — `24px` tall, bottom

Monospace 11px in `#6a6151` on `#e7e1d3` (slightly darker paper than the
rest of chrome). Left-to-right segments separated by faint `│` dividers:

- `● CONNECTED` (animated pulse, `#3e8f60` — muted ok-green)
- `host <addr>`
- `ssh 22`
- ping in ms — color shifts ok/warn/err by latency (`#3e8f60` / `#b88528` / `#b13a3a`)
- `rx <bytes>` / `tx <bytes>` — live counters
- `up <hh:mm:ss>` — session age
- (spacer)
- `MULTIEXEC · N panes` pill (only when on, amber `#b3793a`)
- `utf-8`, `VT320`, layout mode (`split-v` / `single`)

### 6. Command palette — modal overlay, `Ctrl+K`

- **Backdrop:** `rgba(5,5,12,.55)` + `backdrop-filter: blur(8px)`. Yes — the
  backdrop stays dark even with a light theme; this gives modal focus.
- **Panel:** 520px wide, top-aligned 110px from top of window. Background
  `rgba(252,250,245,0.98)`. 1px indigo border at `#564ca0 + "66"`. Plain
  shadow `0 24px 80px rgba(85,70,40,0.25)`. **No glow** (unlike Neon Noir).
- **Input row** with `⌘` icon left in indigo, ESC pill right.
- **Result row:** icon · label (`#2d2a24`) · sub-label (`#8a8174`) · `kbd`
  pill on right. Selected row: `rgba(86,76,160,0.13)` background + 1px
  indigo border at low opacity. Arrow keys navigate, Enter selects.
- **Footer** with hint keys and result count in indigo.

---

## Interactions & Behavior

| Action                       | Trigger                          | Behavior                                          |
| ---------------------------- | -------------------------------- | ------------------------------------------------- |
| Open command palette         | `Ctrl+K` / `⌘K`               | Modal opens, input focused                        |
| Toggle vertical split        | `Ctrl+\`                       | Adds/removes second pane                          |
| Toggle MultiExec broadcast   | `Ctrl+Shift+M`                  | All panes share input; pill in status bar         |
| Find in terminal             | `Ctrl+F`                        | Inline find bar inside focused terminal           |
| Run snippet                  | `Ctrl+;`                        | Snippet picker                                    |
| Quick-connect from sidebar   | Click session                    | Opens new tab (or activates existing)             |
| Reorder tab                  | Drag tab                         | DnD reorders the tab list                         |
| Close tab                    | Click ✕ / middle-click           | Removes tab; activates neighbor                   |
| Focus pane                   | Click pane                       | Inset indigo ring on focused pane                 |
| Resize panes                 | Drag the 5px resizer             | Both panes resize live                            |
| New session                  | Click `+` in tab bar            | Opens "New Session" dialog (TODO — not in mock)   |

**Animations** (kept minimal — Parchment is a quiet theme):

- Tab activate: 0.1s ease background + color
- Status dot pulse: `pylon-glow-pulse` 2s ease infinite (opacity .6 ↔ 1)
- Cursor blink: `pylon-cursor-blink` 1.05s steps(2) infinite (terminal only)
- Hover backgrounds: 0.1s ease
- Section disclosure caret: 0.12s rotate
- Command palette: instant open (no animation), trust the backdrop blur

**Typewriter effect:** The mocks animate command typing on the most recent
prompt line. In production this just reflects real SSH stream.

---

## State management

State that needs to live in the renderer process:

```ts
type Session = {
  id: string;
  name: string;
  host: string;           // "user@host" form
  port: number;
  auth: 'password' | 'key' | 'agent';
  keyPath?: string;
  color: string;          // hex — used in sidebar dot + tab indicator
  tag?: string;           // short label, e.g. "FGT", "cisco"
  folderId: string;
  pinned?: boolean;
};

type Tab = {
  id: string;
  sessionId: string;
  txPid: string;          // active xterm session id
  status: 'connecting' | 'connected' | 'disconnected' | 'error';
  rxBytes: number;
  txBytes: number;
  latencyMs: number;
  connectedAt?: number;
};

type Layout =
  | { kind: 'single', tabId: string }
  | { kind: 'split-v', tabIds: [string, string] }
  | { kind: 'grid-2x2', tabIds: [string, string, string, string] };

type AppState = {
  sessions: Session[];
  folders: SessionFolder[];
  tabs: Tab[];
  activeTabId: string;
  layout: Layout;
  multiExec: boolean;
  focusedPane: number;
  paletteOpen: boolean;
  searchQuery: string;
  theme: 'parchment';
};
```

Persistence: sessions + folders + last layout + theme go to a settings file
(`~/.pylon/config.toml` or appdata equivalent). Tabs are ephemeral.

---

## Design tokens — Parchment

### Theme shape

```ts
type Theme = {
  // Fonts
  ui: string;
  mono: string;

  // Geometry
  radius: number;
  tabRadius: string;

  // Surfaces
  appBg: string;
  bodyBg: string;
  titlebarBg: string;
  sidebarBg: string;
  tabBarBg: string;
  tabActiveBg: string;
  tabActiveFg: string;
  tabIdleBg: string;
  tabIdleFg: string;
  statusBg: string;
  statusFg: string;
  inputBg: string;
  paletteBg: string;

  // Foreground / dividers
  divider: string;
  uiFg: string;
  uiDim: string;
  itemHoverBg: string;
  itemActiveBg: string;
  itemActiveFg: string;

  // Accents + semantic
  accent: string;
  accent2: string;
  ok: string;
  warn: string;
  err: string;

  // Terminal sub-theme — passed to xterm.js
  terminal: {
    bg: string;
    fg: string;
    dim: string;
    ok: string; warn: string; err: string;
    cmd: string;
    prompt: string;
    cursor: string;
    banner: string;
    bannerBg: string;
    accent: string;
    glow: boolean;
    fontSize: number;
    lineHeight: number;
    padding: string;
  };
};
```

### Parchment tokens — exact values

```ts
const parchment: Theme = {
  ui:   '"Geist", "SF Pro Text", system-ui, sans-serif',
  mono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',
  radius: 6,
  tabRadius: "6px 6px 0 0",

  // ── Surfaces ──────────────────────────────────────────────
  appBg:        "#f5f2eb",                         // window background
  bodyBg:       "#faf8f3",                         // content area
  titlebarBg:   "#efeae0",                         // title bar
  sidebarBg:    "#f0ece2",                         // sidebar
  tabBarBg:     "#efeae0",                         // tab bar (matches titlebar)
  tabActiveBg:  "#faf8f3",                         // active tab lifts to body color
  tabActiveFg:  "#2d2a24",
  tabIdleBg:    "transparent",
  tabIdleFg:    "#7d7466",
  statusBg:     "#e7e1d3",
  statusFg:     "#6a6151",
  inputBg:      "#fdfbf6",
  paletteBg:    "rgba(252,250,245,0.98)",

  // ── Foreground / dividers ─────────────────────────────────
  divider:        "rgba(85,70,40,0.10)",
  uiFg:           "#2d2a24",
  uiDim:          "#8a8174",
  itemHoverBg:    "rgba(85,70,40,0.04)",
  itemActiveBg:   "rgba(86,76,160,0.10)",
  itemActiveFg:   "#2d2a24",

  // ── Accents ───────────────────────────────────────────────
  accent:  "#564ca0",      // muted indigo — primary
  accent2: "#b3793a",      // warm amber — secondary
  ok:      "#3e8f60",      // muted forest green
  warn:    "#b88528",      // umber amber
  err:     "#b13a3a",      // muted brick

  // ── Terminal (stays dark!) ────────────────────────────────
  terminal: {
    bg:       "#1d1e26",
    fg:       "#d4d6dc",
    dim:      "#7e8090",
    ok:       "#7fb89a",
    warn:     "#e0b06c",
    err:      "#e08a8a",
    cmd:      "#eef0f4",   // user-typed commands
    prompt:   "#8e85e0",   // PS1 line (muted indigo, lifted for dark bg)
    cursor:   "#8e85e0",
    banner:   "#8e85e0",
    bannerBg: "rgba(142,133,224,0.10)",
    accent:   "#d4a26c",   // amber — for table heads, key sections
    fontSize:   13,
    lineHeight: 1.55,
    padding:    "14px 18px 60px 18px",
    glow:       false,
  },
};
```

### Spacing scale

`4, 6, 8, 10, 12, 16, 22, 26, 32, 36, 44, 60` — driven by the typeface
metrics (12.5px UI base, 13px terminal). Don't deviate.

### Typography

| Use                  | Font                | Size  | Weight | Tracking |
| -------------------- | ------------------- | ----- | ------ | -------- |
| Title bar brand      | Geist               | 12px  | 700    | 1px      |
| Title bar menus      | Geist               | 12px  | 400    | —        |
| Sidebar search       | Geist               | 12px  | 400    | —        |
| Sidebar section head | Geist               | 11px  | 600    | 0.6px UC |
| Sidebar session row  | Geist               | 12.5px| 400    | —        |
| Tag pill             | Geist               | 9.5px | 600    | 0.4px UC |
| Tab label            | Geist               | 12px  | 400/600| —        |
| Status bar           | JetBrains Mono      | 11px  | 400/600| —        |
| Terminal text        | JetBrains Mono      | 13px  | 400    | —        |
| Command palette item | Geist               | 13px  | 400    | —        |
| Command palette sub  | Geist               | 11px  | 400    | —        |

Use `tabular-nums` font-feature for numeric counters (rx/tx, ping, uptime).

### Border-radius

- `6px` — buttons, inputs, pills, palette items
- `6px 6px 0 0` — chrome-style tab corners
- `3px` — kbd pills, tag pills, small chips
- `4px` — close-button hover hit
- `10px` — command palette outer container

### Shadows

Parchment is **flat**. Only two shadow uses:

- Outer window frame: `0 24px 70px rgba(85,70,40,0.18)` — soft, warm-tinted
  so it doesn't read as a pure black drop on the paper background.
- Command palette: `0 24px 80px rgba(85,70,40,0.25)`.

**No glows anywhere.** Don't add `text-shadow` to accent colors. The only
"glow" effects are on connection status dots (`box-shadow: 0 0 8px <color>`)
and they should be subtle.

### Saved-session color palette

Every session has a user-chosen color. Used in: sidebar status dot, tab top
border, tab indicator dot, pane header dot. Curated palette (paper-friendly):

`#22d3ee #f472b6 #a78bfa #f59e0b #ef4444 #10b981 #f97316 #84cc16 #c89b6b #5eb3b2`

These pop against both the warm paper chrome and the dark terminal, so the
same color reads cleanly in either surface.

---

## Light/dark surface handling — important

**Do not let chrome styles bleed into the terminal pane.** xterm.js renders to
canvas, so it can't accidentally inherit, but the pane header sitting above it
must be styled to feel like part of the dark terminal:

- Pane header background: solid `#15161c` (slightly darker than terminal bg
  for separation) — not a light chrome color.
- Pane header dividers: `rgba(255,255,255,0.06)` — light-on-dark dividers,
  not the paper divider color.
- Pane header text: `#d4d6dc` (light) and `#7e8090` (dim), not the chrome
  dark text.

Conversely, **don't let dark terminal colors bleed into chrome**. The tab bar
behind the active tab, the resizer between panes — these are chrome and stay
warm/light.

---

## Tab behavior — closer specification

- Tab width: `clamp(130px, content, 220px)`. Below 130px text is ellipsized
  and `+` button hides.
- **Drag-reorder:** `@dnd-kit` with 5px activation threshold so click-to-
  activate still works.
- **Close behavior:** if closed tab was active, activate next-right (or next-
  left if last).
- **Tab overflow:** above N tabs, the bar becomes horizontally scrollable with
  hidden scrollbar; arrow buttons appear at edges.

---

## Sidebar behavior — closer specification

- Search filters tree live. Empty groups hidden while query is active.
- Section open/closed state persists per-user.
- Section header click toggles; caret rotates.
- Right-click on session: context menu (Connect / Connect in new window / Edit
  / Duplicate / Delete).
- Right-click on section: New session here / New folder / Rename / Delete.
- Drag-reorder of sessions within section: optional.

---

## Terminal styling — handoff to xterm.js

xterm.js takes its own theme object. Map from `theme.terminal`:

```ts
const xtermTheme = {
  background: t.terminal.bg,                       // #1d1e26
  foreground: t.terminal.fg,                       // #d4d6dc
  cursor: t.terminal.cursor,                       // #8e85e0
  cursorAccent: t.terminal.bg,
  selectionBackground: '#564ca040',                // indigo @ 25%

  // ANSI 16
  black:         '#1d2025',
  red:           t.terminal.err,                   // #e08a8a
  green:         t.terminal.ok,                    // #7fb89a
  yellow:        t.terminal.warn,                  // #e0b06c
  blue:          '#7c9ddb',
  magenta:       t.terminal.prompt,                // #8e85e0
  cyan:          '#82bfd4',
  white:         t.terminal.fg,                    // #d4d6dc

  brightBlack:   t.terminal.dim,                   // #7e8090
  brightRed:     '#ec9a9a',
  brightGreen:   '#9bcfaf',
  brightYellow:  '#ecc586',
  brightBlue:    '#9bbae0',
  brightMagenta: '#a39bdf',
  brightCyan:    '#9ccfde',
  brightWhite:   '#ffffff',
};

new Terminal({
  fontFamily: t.terminal.mono,
  fontSize:   t.terminal.fontSize,
  lineHeight: t.terminal.lineHeight,
  cursorBlink: true,
  cursorStyle: 'block',
  scrollback:  50000,
  allowProposedApi: true,
  theme: xtermTheme,
});
```

Recommended addons: `@xterm/addon-fit`, `@xterm/addon-search`,
`@xterm/addon-web-links`, `@xterm/addon-webgl`, `@xterm/addon-serialize`
(for session logging).

---

## Files in this bundle

`design_source/index.html` — entry, mounts the comparison canvas with all
        explored themes (Neon Noir, Graphite, Parchment).
`design_source/app.jsx` — PylonApp React component. Every UI region
        (titlebar, sidebar, tabs, panes, status bar, command palette) is a
        sub-component here. **Read this first.**
`design_source/themes.js` — theme objects. **Parchment lives at line 213.**
`design_source/terminal.jsx` — typed-line terminal renderer used by the mock.
        Not for production — replace with xterm.js.
`design_source/data.js` — fake session list, transcripts, snippets, palette
        items. Useful as test fixtures.
`design_source/design-canvas.jsx` — comparison-canvas component (Figma-like
        pan/zoom). **Not needed in production.**

To preview the mocks: open `design_source/index.html` in a browser. The
Parchment artboard is the third one (`C · Parchment`). Click its label to
focus, then interact with it (Ctrl+K opens the palette, click sessions in
the sidebar, etc.).

---

## Next steps not yet designed

These were on the roadmap but not built into the mock. Design them in the
Parchment language:

1. **"New Session" dialog** — full SSH options: host, port, user, auth method
   (password / key / agent), key path with file picker, jump host chain,
   folder, color picker (from curated palette above), tags, advanced
   (compression, keepalive, X11 fwd, identity file, terminal type).
2. **Onboarding / empty state** — first-run when there are no sessions:
   prominent "Add session" CTA, "Import from PuTTY / OpenSSH config /
   MobaXterm / SecureCRT" buttons.
3. **Snippets panel** — collapsible right-side panel listing reusable command
   snippets; drag-drop into terminal, or click to send to active pane.
4. **Session log viewer** — per-session searchable log with filters.
5. **Settings screen** — themes, fonts, keybindings, default terminal,
   privacy/telemetry, updates.
6. **Grid 2x2 panes** — beyond split-vertical.
7. **SFTP browser** — split between terminal and remote filesystem.

---

## Build / runtime notes

- **Bundle size target:** under 30MB on Windows. Tauri easily; Electron will
  need aggressive trimming.
- **Cold start:** under 800ms to interactive on a modest laptop.
- **Memory:** SSH session ≈ 30MB resident is acceptable; aim lower.
- **Window controls on macOS** — use platform-native traffic lights (left
  side, hide custom Windows controls). On Linux, follow GTK conventions.
- **Title bar height** is 32px on Windows; 28px (macOS) and 36px (GNOME)
  work too. Adjust per platform.
- **Light/dark system preference:** Parchment is a *light* theme. If the user
  sets system to dark mode, fall back to Graphite (or whichever dark theme
  ships alongside). Don't auto-darken Parchment — the whole point is the
  warm/cool contrast.

---

## Questions for the implementer

- Window resize edge behavior (custom hit zones in a frameless window).
- Behavior when same host dragged from sidebar to a specific pane vs. tab bar
  (replace pane vs. new tab).
- "Save layout as workspace" feature — mock has tabs+split but no workspace
  concept.
- Auto-reconnect semantics on network drop.
- Whether MultiExec broadcast survives across tab switches or is per-layout.
- Whether Parchment ships first, or alongside a dark theme. The mock has
  Parchment + Graphite + Neon Noir together — you can ship them all from
  one theme dropdown in Settings.
