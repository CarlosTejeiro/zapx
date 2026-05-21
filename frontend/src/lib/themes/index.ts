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

export const graphite: PylonTheme = {
  name: 'graphite',

  fontUi: '"Geist", "SF Pro Text", system-ui, sans-serif',
  fontMono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',

  appBg: '#1a1d23',
  bodyBg: '#1e2127',
  sidebarBg: '#181b20',
  titlebarBg: '#13151a',
  statusbarBg: '#16191e',
  tabBarBg: '#13151a',

  tabActiveBg: '#1e2127',
  tabIdleBg: 'transparent',
  tabBorder: '#2a2d35',
  tabRadius: '6px 6px 0 0',

  accent: '#5eb3b2',
  accent2: '#c89b6b',
  itemHoverBg: 'rgba(255,255,255,0.04)',
  itemActiveBg: 'rgba(94,179,178,0.10)',
  itemActiveBorder: '#5eb3b2',

  textPrimary: '#c9cdd3',
  textMuted: '#6b7280',
  textDim: '#4b5563',

  border: '#2a2d35',
  radius: '6px',

  ok: '#7cb992',
  warn: '#d4a85a',
  err: '#d97070',

  terminal: {
    bg: '#15181d',
    fg: '#c9cdd3',
    cursor: '#5eb3b2',
    dim: '#4b5563',
    ok: '#7cb992',
    warn: '#d4a85a',
    err: '#d97070',
    black: '#1d2025',
    red: '#d97070',
    green: '#7cb992',
    yellow: '#d4a85a',
    blue: '#5b9dd9',
    magenta: '#c977c4',
    cyan: '#5eb3b2',
    white: '#c9cdd3',
    brightBlack: '#4b5563',
    brightRed: '#d97070',
    brightGreen: '#7cb992',
    brightYellow: '#d4a85a',
    brightBlue: '#82bfff',
    brightMagenta: '#e89edf',
    brightCyan: '#5eb3b2',
    brightWhite: '#ffffff',
  },

  glows: false,
  windowShadow: '0 24px 70px rgba(0,0,0,.35)',
}

export const neonNoir: PylonTheme = {
  name: 'neon-noir',

  fontUi: '"Geist", "SF Pro Display", system-ui, sans-serif',
  fontMono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',

  appBg: '#080b11',
  bodyBg: '#0d1220',
  sidebarBg: '#05080e',   // noticeably darker than bodyBg for contrast
  titlebarBg: '#04070c',
  statusbarBg: '#04070c',
  tabBarBg: '#04070c',

  tabActiveBg: '#0d1220',
  tabIdleBg: 'transparent',
  tabBorder: '#1e2840',
  tabRadius: '8px 8px 0 0',

  accent: '#22d3ee',
  accent2: '#f472b6',
  itemHoverBg: 'rgba(34,211,238,0.07)',
  itemActiveBg: 'rgba(34,211,238,0.13)',
  itemActiveBorder: '#22d3ee',

  textPrimary: '#d4dff0',
  textMuted: '#5a6e90',
  textDim: '#334060',

  border: '#1e2840',   // more visible than before
  radius: '5px',

  ok: '#4ade80',
  warn: '#fbbf24',
  err: '#ef4444',

  terminal: {
    bg: '#05080d',
    fg: '#cbd5e1',
    cursor: '#f472b6',
    dim: '#2d3a50',
    ok: '#4ade80',
    warn: '#fbbf24',
    err: '#ef4444',
    black: '#0d1119',
    red: '#ef4444',
    green: '#4ade80',
    yellow: '#fbbf24',
    blue: '#5b9dd9',
    magenta: '#f472b6',
    cyan: '#22d3ee',
    white: '#cbd5e1',
    brightBlack: '#2d3a50',
    brightRed: '#ef4444',
    brightGreen: '#4ade80',
    brightYellow: '#fbbf24',
    brightBlue: '#82bfff',
    brightMagenta: '#f9a8d4',
    brightCyan: '#67e8f9',
    brightWhite: '#ffffff',
  },

  glows: true,
  windowShadow: '0 30px 80px rgba(0,0,0,0.4), 0 0 80px rgba(34,211,238,0.08)',
}

export const themes: Record<string, PylonTheme> = { graphite, neonNoir }
