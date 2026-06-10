export interface PylonTheme {
  name: string

  // Fonts
  fontUi: string
  fontMono: string

  // Chrome layout
  appBg: string
  bodyBg: string
  sidebarBg: string
  titlebarBg: string
  statusbarBg: string
  tabBarBg: string

  // Tabs
  tabActiveBg: string
  tabIdleBg: string
  tabBorder: string
  tabRadius: string

  // Interactive
  accent: string
  accent2: string
  /** Foreground for text/icons sitting on top of an `accent`-filled surface
   *  (e.g. primary buttons). Dark on light accents, light on dark accents. */
  onAccent: string
  itemHoverBg: string
  itemActiveBg: string
  itemActiveBorder: string

  // Text
  textPrimary: string
  textMuted: string
  textDim: string

  // Borders
  border: string
  radius: string

  // Status / connection
  ok: string
  warn: string
  err: string

  // Terminal
  terminal: {
    bg: string
    fg: string
    cursor: string
    dim: string
    ok: string
    warn: string
    err: string
    // ANSI
    black: string
    red: string
    green: string
    yellow: string
    blue: string
    magenta: string
    cyan: string
    white: string
    brightBlack: string
    brightRed: string
    brightGreen: string
    brightYellow: string
    brightBlue: string
    brightMagenta: string
    brightCyan: string
    brightWhite: string
  }

  // Effects
  glows: boolean
  windowShadow: string
}

/* ---------------------------------------------------------------------------
 * The 7 themes from the redesign handoff. Each maps the design tokens
 * (`paper`, `paper2`, `ink`, `inkMuted`, `inkDim`, `line`, `hover`) onto the
 * PylonTheme fields: paper → appBg/bodyBg/tabBarBg, paper2 → sidebarBg/
 * titlebarBg/statusbarBg/tabActiveBg, line → border/tabBorder, hover →
 * itemHoverBg. `itemActiveBg` is always accent at 10%; the active row is
 * marked by background + type weight (no border-left). Remaining ANSI slots:
 * black = terminal bg lightened one step, brightBlack = dim, bright* = the
 * base colour at +10–15% lightness with the hue kept. ok/warn/err stay
 * semantic (red errors stay red even in Phosphor/Amber).
 * ------------------------------------------------------------------------- */

// Bundled via @fontsource-variable (imported in main.ts); the plain names
// stay as fallbacks for environments where the webfonts fail to load.
const fontUi = '"Geist Variable", "Geist", "SF Pro Text", system-ui, sans-serif'
const fontMono = '"JetBrains Mono Variable", "JetBrains Mono", "Fira Code", ui-monospace, monospace'

/** Accent at 10% over transparent — shared recipe for the active-item fill. */
const activeMix = (accent: string) => `color-mix(in srgb, ${accent} 10%, transparent)`

// ── 1 · Parchment — warm light (refined original) ───────────────────────────
export const parchment: PylonTheme = {
  name: 'parchment',
  fontUi,
  fontMono,

  appBg: '#f7f4ed',
  bodyBg: '#f7f4ed',
  sidebarBg: '#efebdf',
  titlebarBg: '#efebdf',
  statusbarBg: '#efebdf',
  tabBarBg: '#f7f4ed',

  tabActiveBg: '#efebdf',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(82,68,40,0.16)',
  tabRadius: '7px',

  accent: '#564ca0',
  accent2: '#b3793a',
  onAccent: '#faf8f3',
  itemHoverBg: 'rgba(82,68,40,0.06)',
  itemActiveBg: activeMix('#564ca0'),
  itemActiveBorder: '#564ca0',

  textPrimary: '#2c2924',
  textMuted: '#6c6354',
  textDim: '#a09684',

  border: 'rgba(82,68,40,0.16)',
  radius: '7px',

  ok: '#3e8f60',
  warn: '#b88528',
  err: '#b13a3a',

  terminal: {
    bg: '#20212b',
    fg: '#d6d8e0',
    cursor: '#9a91e8',
    dim: '#5c5f74',
    ok: '#5fb784',
    warn: '#d8a85e',
    err: '#e07474',
    black: '#2a2b37',
    red: '#e07474',
    green: '#5fb784',
    yellow: '#d8a85e',
    blue: '#5b8fc9',
    magenta: '#9a91e8',
    cyan: '#6fbdbc',
    white: '#eceef4',
    brightBlack: '#5c5f74',
    brightRed: '#e98c8c',
    brightGreen: '#74c697',
    brightYellow: '#e2b878',
    brightBlue: '#74a3d8',
    brightMagenta: '#b1aaf0',
    brightCyan: '#8acfce',
    brightWhite: '#ffffff',
  },

  glows: false,
  windowShadow: '0 24px 70px rgba(0,0,0,.15)',
}

// ── 2 · Oxide — warm dark, copper accent ────────────────────────────────────
export const oxide: PylonTheme = {
  name: 'oxide',
  fontUi,
  fontMono,

  appBg: '#1d1a16',
  bodyBg: '#1d1a16',
  sidebarBg: '#262119',
  titlebarBg: '#262119',
  statusbarBg: '#262119',
  tabBarBg: '#1d1a16',

  tabActiveBg: '#262119',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(255,230,180,0.11)',
  tabRadius: '7px',

  accent: '#cf8a46',
  accent2: '#7aa893',
  onAccent: '#1d1a16',
  itemHoverBg: 'rgba(255,230,180,0.05)',
  itemActiveBg: activeMix('#cf8a46'),
  itemActiveBorder: '#cf8a46',

  textPrimary: '#eae3d4',
  textMuted: '#a2967f',
  textDim: '#6f6452',

  border: 'rgba(255,230,180,0.11)',
  radius: '7px',

  ok: '#6fbf8d',
  warn: '#d8b16a',
  err: '#e07b6d',

  terminal: {
    bg: '#15120e',
    fg: '#ddd5c4',
    cursor: '#cf8a46',
    dim: '#5d5546',
    ok: '#7dbb82',
    warn: '#d8a85e',
    err: '#e07b6d',
    black: '#1f1b15',
    red: '#e07b6d',
    green: '#7dbb82',
    yellow: '#d8a85e',
    blue: '#7a9ab8',
    magenta: '#c490a8',
    cyan: '#8fb8a4',
    white: '#f2ecdf',
    brightBlack: '#5d5546',
    brightRed: '#ea9285',
    brightGreen: '#93cc97',
    brightYellow: '#e2b878',
    brightBlue: '#92aec8',
    brightMagenta: '#d3a6bb',
    brightCyan: '#a5c8b6',
    brightWhite: '#fbf7ee',
  },

  glows: false,
  windowShadow: '0 24px 70px rgba(0,0,0,.5)',
}

// ── 3 · Fjord — cool dark, ice accent ───────────────────────────────────────
export const fjord: PylonTheme = {
  name: 'fjord',
  fontUi,
  fontMono,

  appBg: '#161a21',
  bodyBg: '#161a21',
  sidebarBg: '#1d232c',
  titlebarBg: '#1d232c',
  statusbarBg: '#1d232c',
  tabBarBg: '#161a21',

  tabActiveBg: '#1d232c',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(180,210,255,0.11)',
  tabRadius: '7px',

  accent: '#5ca3d6',
  accent2: '#d68a6e',
  onAccent: '#161a21',
  itemHoverBg: 'rgba(180,210,255,0.05)',
  itemActiveBg: activeMix('#5ca3d6'),
  itemActiveBorder: '#5ca3d6',

  textPrimary: '#dee5ee',
  textMuted: '#91a0b2',
  textDim: '#5a6878',

  border: 'rgba(180,210,255,0.11)',
  radius: '7px',

  ok: '#5fb784',
  warn: '#d8b16a',
  err: '#e07474',

  terminal: {
    bg: '#11141a',
    fg: '#d3dce6',
    cursor: '#8ab8e8',
    dim: '#4d5b6c',
    ok: '#66bb8a',
    warn: '#d8b16a',
    err: '#e07474',
    black: '#1a1f27',
    red: '#e07474',
    green: '#66bb8a',
    yellow: '#d8b16a',
    blue: '#82a8d8',
    magenta: '#b094cf',
    cyan: '#6fc3d8',
    white: '#eef3f8',
    brightBlack: '#4d5b6c',
    brightRed: '#e98c8c',
    brightGreen: '#7ecca0',
    brightYellow: '#e2c184',
    brightBlue: '#9abce4',
    brightMagenta: '#c2a8dd',
    brightCyan: '#88d2e4',
    brightWhite: '#ffffff',
  },

  glows: false,
  windowShadow: '0 24px 70px rgba(0,0,0,.5)',
}

// ── 4 · Nocturne — violet dark, periwinkle accent ───────────────────────────
export const nocturne: PylonTheme = {
  name: 'nocturne',
  fontUi,
  fontMono,

  appBg: '#181520',
  bodyBg: '#181520',
  sidebarBg: '#201c2c',
  titlebarBg: '#201c2c',
  statusbarBg: '#201c2c',
  tabBarBg: '#181520',

  tabActiveBg: '#201c2c',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(215,200,255,0.11)',
  tabRadius: '7px',

  accent: '#9d92ea',
  accent2: '#5eb3b2',
  onAccent: '#181520',
  itemHoverBg: 'rgba(215,200,255,0.05)',
  itemActiveBg: activeMix('#9d92ea'),
  itemActiveBorder: '#9d92ea',

  textPrimary: '#e6e2f0',
  textMuted: '#9b93b4',
  textDim: '#655d80',

  border: 'rgba(215,200,255,0.11)',
  radius: '7px',

  ok: '#6fbf8d',
  warn: '#d4a85a',
  err: '#e07b8a',

  terminal: {
    bg: '#121019',
    fg: '#dcd8e8',
    cursor: '#b3a7f7',
    dim: '#4f4868',
    ok: '#6fbf8d',
    warn: '#d8b16a',
    err: '#e07b8a',
    black: '#1b1825',
    red: '#e07b8a',
    green: '#6fbf8d',
    yellow: '#d8b16a',
    blue: '#8a9de8',
    magenta: '#c9a3e8',
    cyan: '#7fd0ce',
    white: '#f0edf8',
    brightBlack: '#4f4868',
    brightRed: '#ea93a0',
    brightGreen: '#86cda1',
    brightYellow: '#e2c184',
    brightBlue: '#a2b2ef',
    brightMagenta: '#d7b8f0',
    brightCyan: '#96dcda',
    brightWhite: '#ffffff',
  },

  glows: false,
  windowShadow: '0 24px 70px rgba(0,0,0,.5)',
}

// ── 5 · Porcelain — cool light, steel-indigo accent ─────────────────────────
export const porcelain: PylonTheme = {
  name: 'porcelain',
  fontUi,
  fontMono,

  appBg: '#f3f5f7',
  bodyBg: '#f3f5f7',
  sidebarBg: '#e8ecf0',
  titlebarBg: '#e8ecf0',
  statusbarBg: '#e8ecf0',
  tabBarBg: '#f3f5f7',

  tabActiveBg: '#e8ecf0',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(40,60,90,0.15)',
  tabRadius: '7px',

  accent: '#4a66a0',
  accent2: '#b06a4f',
  onAccent: '#f7f9fb',
  itemHoverBg: 'rgba(40,60,90,0.05)',
  itemActiveBg: activeMix('#4a66a0'),
  itemActiveBorder: '#4a66a0',

  textPrimary: '#252a31',
  textMuted: '#5e6873',
  textDim: '#98a2ad',

  border: 'rgba(40,60,90,0.15)',
  radius: '7px',

  ok: '#3e8f60',
  warn: '#b88528',
  err: '#b13a3a',

  terminal: {
    bg: '#1f242e',
    fg: '#d6dce4',
    cursor: '#8aa6e8',
    dim: '#566074',
    ok: '#5fb784',
    warn: '#d8a85e',
    err: '#e07474',
    black: '#29303c',
    red: '#e07474',
    green: '#5fb784',
    yellow: '#d8a85e',
    blue: '#82a8d8',
    magenta: '#b094cf',
    cyan: '#6fbdbc',
    white: '#eef1f6',
    brightBlack: '#566074',
    brightRed: '#e98c8c',
    brightGreen: '#74c697',
    brightYellow: '#e2b878',
    brightBlue: '#9abce4',
    brightMagenta: '#c2a8dd',
    brightCyan: '#8acfce',
    brightWhite: '#ffffff',
  },

  glows: false,
  windowShadow: '0 24px 70px rgba(0,0,0,.15)',
}

// ── 6 · Phosphor — green on black (classic CRT) ─────────────────────────────
export const phosphor: PylonTheme = {
  name: 'phosphor',
  fontUi,
  fontMono,

  appBg: '#0c100c',
  bodyBg: '#0c100c',
  sidebarBg: '#121812',
  titlebarBg: '#121812',
  statusbarBg: '#121812',
  tabBarBg: '#0c100c',

  tabActiveBg: '#121812',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(80,255,120,0.13)',
  tabRadius: '7px',

  accent: '#36d97c',
  accent2: '#9dd87a',
  onAccent: '#0c100c',
  itemHoverBg: 'rgba(80,255,120,0.05)',
  itemActiveBg: activeMix('#36d97c'),
  itemActiveBorder: '#36d97c',

  textPrimary: '#cfe8cf',
  textMuted: '#7da87d',
  textDim: '#4a6b4a',

  border: 'rgba(80,255,120,0.13)',
  radius: '7px',

  ok: '#3ddc84',
  warn: '#d8b16a',
  err: '#e07474',

  terminal: {
    bg: '#050905',
    fg: '#8ae89a',
    cursor: '#57e389',
    dim: '#2f5a3a',
    ok: '#57e389',
    warn: '#d8c16a',
    err: '#e07474',
    black: '#0c130c',
    red: '#e07474',
    green: '#57e389',
    yellow: '#d8c16a',
    blue: '#6fb8a8',
    magenta: '#8fd8a8',
    cyan: '#6fd8b0',
    white: '#c9f5d0',
    brightBlack: '#2f5a3a',
    brightRed: '#e98c8c',
    brightGreen: '#74ec9e',
    brightYellow: '#e2cf84',
    brightBlue: '#87c9bb',
    brightMagenta: '#a6e3ba',
    brightCyan: '#88e3c2',
    brightWhite: '#e2fbe6',
  },

  glows: true,
  windowShadow: '0 24px 70px rgba(0,0,0,.6)',
}

// ── 7 · Amber — amber on black (classic CRT) ────────────────────────────────
export const amber: PylonTheme = {
  name: 'amber',
  fontUi,
  fontMono,

  appBg: '#120e07',
  bodyBg: '#120e07',
  sidebarBg: '#1a140b',
  titlebarBg: '#1a140b',
  statusbarBg: '#1a140b',
  tabBarBg: '#120e07',

  tabActiveBg: '#1a140b',
  tabIdleBg: 'transparent',
  tabBorder: 'rgba(255,200,100,0.13)',
  tabRadius: '7px',

  accent: '#e8a33c',
  accent2: '#c97c4a',
  onAccent: '#120e07',
  itemHoverBg: 'rgba(255,200,100,0.05)',
  itemActiveBg: activeMix('#e8a33c'),
  itemActiveBorder: '#e8a33c',

  textPrimary: '#f0dcb8',
  textMuted: '#ad9468',
  textDim: '#715f42',

  border: 'rgba(255,200,100,0.13)',
  radius: '7px',

  ok: '#8fb96a',
  warn: '#f0c060',
  err: '#e07b5e',

  terminal: {
    bg: '#0a0703',
    fg: '#ecb45c',
    cursor: '#ffb84d',
    dim: '#5c4a2c',
    ok: '#8fb96a',
    warn: '#f0c060',
    err: '#e07b5e',
    black: '#150f08',
    red: '#e07b5e',
    green: '#8fb96a',
    yellow: '#f0c060',
    blue: '#c9a85e',
    magenta: '#d89a6a',
    cyan: '#f0cd8a',
    white: '#f8e2b8',
    brightBlack: '#5c4a2c',
    brightRed: '#ea9377',
    brightGreen: '#a3c97e',
    brightYellow: '#f5cf7e',
    brightBlue: '#d6ba78',
    brightMagenta: '#e3ad80',
    brightCyan: '#f5dba2',
    brightWhite: '#fcedce',
  },

  glows: true,
  windowShadow: '0 24px 70px rgba(0,0,0,.6)',
}

export const themes: Record<string, PylonTheme> = {
  parchment,
  oxide,
  fjord,
  nocturne,
  porcelain,
  phosphor,
  amber,
}

/** Display labels for menus / palette / about, keyed like `themes`. */
export const themeLabels: Record<string, string> = {
  parchment: 'Parchment',
  oxide: 'Oxide',
  fjord: 'Fjord',
  nocturne: 'Nocturne',
  porcelain: 'Porcelain',
  phosphor: 'Phosphor',
  amber: 'Amber',
}
