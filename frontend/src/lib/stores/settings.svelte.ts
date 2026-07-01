import { getSetting, setSetting, setSshKeepalive, listColorSchemes } from '$lib/bridge/commands'
import type { ColorScheme } from '$lib/bridge/types'

// ── reactive global terminal settings ──────────────────────────────────────

export const terminalSettings = $state({
  fontFamily: 'JetBrains Mono Variable, JetBrains Mono, monospace',
  fontSize: 14,
  lineHeight: 1.2,
  cursorStyle: 'block' as 'block' | 'underline' | 'bar',
  cursorBlink: true,
  activeColorScheme: 'One Dark',
})

// ── connection reliability settings ─────────────────────────────────────────

export const connectionSettings = $state({
  // SSH server-alive interval in seconds (0 = off). Backend default is 60.
  keepaliveSecs: 60,
  // Auto-reconnect a session when the remote drops the link.
  autoReconnect: true,
})

// ── UI layout settings ──────────────────────────────────────────────────────

/** Default sidebar width (px). Matches the historical hard-coded `.sidebar`. */
export const DEFAULT_SIDEBAR_WIDTH = 248
/** Min sidebar width so the search box + section headers still fit. */
export const MIN_SIDEBAR_WIDTH = 180

export const uiSettings = $state({
  sidebarWidth: DEFAULT_SIDEBAR_WIDTH,
})

export const colorSchemes = $state<ColorScheme[]>([])

// ── load from DB on app start ───────────────────────────────────────────────

export async function loadSettings(): Promise<void> {
  try {
    const schemes = await listColorSchemes()
    colorSchemes.length = 0
    colorSchemes.push(...schemes)
  } catch {
    // non-fatal; built-in themes defined by DEFAULT_PALETTE fallback
  }

  const keys = [
    'terminal.fontFamily',
    'terminal.fontSize',
    'terminal.lineHeight',
    'terminal.cursorStyle',
    'terminal.cursorBlink',
    'terminal.activeColorScheme',
    'ssh.keepalive_secs',
    'connection.autoReconnect',
    'ui.sidebar_width',
  ]
  try {
    const values = await Promise.all(keys.map((k) => getSetting(k)))
    if (values[0]) terminalSettings.fontFamily = values[0]
    if (values[1]) terminalSettings.fontSize = parseInt(values[1], 10)
    if (values[2]) terminalSettings.lineHeight = parseFloat(values[2])
    if (values[3]) terminalSettings.cursorStyle = values[3] as 'block' | 'underline' | 'bar'
    if (values[4] !== null && values[4] !== undefined)
      terminalSettings.cursorBlink = values[4] === 'true'
    if (values[5]) terminalSettings.activeColorScheme = values[5]
    if (values[6] !== null && values[6] !== undefined) {
      const n = parseInt(values[6], 10)
      if (!Number.isNaN(n)) connectionSettings.keepaliveSecs = n
    }
    if (values[7] !== null && values[7] !== undefined)
      connectionSettings.autoReconnect = values[7] === 'true'
    if (values[8] !== null && values[8] !== undefined) {
      const w = parseInt(values[8], 10)
      if (!Number.isNaN(w)) uiSettings.sidebarWidth = clampSidebarWidth(w)
    }
  } catch {
    // use defaults
  }
}

/** Clamp a sidebar width to [MIN, 50% of the window]. Re-run on window
 *  resize so a previously-saved wide sidebar can't exceed half the shrunken
 *  window. Guards against a zero/undefined `innerWidth` (e.g. under tests). */
export function clampSidebarWidth(width: number): number {
  const winW = typeof window !== 'undefined' ? window.innerWidth : 0
  const max = winW > 0 ? Math.floor(winW * 0.5) : Number.POSITIVE_INFINITY
  return Math.min(Math.max(width, MIN_SIDEBAR_WIDTH), max)
}

/** Persist the chosen sidebar width (call on drag-end, not on every move). */
export async function applySidebarWidth(width: number): Promise<void> {
  uiSettings.sidebarWidth = clampSidebarWidth(width)
  await setSetting('ui.sidebar_width', String(uiSettings.sidebarWidth))
}

export async function applyKeepalive(secs: number): Promise<void> {
  connectionSettings.keepaliveSecs = secs
  await setSshKeepalive(secs)
}

export async function applyAutoReconnect(enabled: boolean): Promise<void> {
  connectionSettings.autoReconnect = enabled
  await setSetting('connection.autoReconnect', String(enabled))
}

export async function applyColorScheme(name: string): Promise<void> {
  terminalSettings.activeColorScheme = name
  await setSetting('terminal.activeColorScheme', name)
}

export async function applyFontSize(size: number): Promise<void> {
  terminalSettings.fontSize = size
  await setSetting('terminal.fontSize', String(size))
}

export async function applyFontFamily(family: string): Promise<void> {
  terminalSettings.fontFamily = family
  await setSetting('terminal.fontFamily', family)
}

export async function applyCursorStyle(style: 'block' | 'underline' | 'bar'): Promise<void> {
  terminalSettings.cursorStyle = style
  await setSetting('terminal.cursorStyle', style)
}

export async function applyCursorBlink(blink: boolean): Promise<void> {
  terminalSettings.cursorBlink = blink
  await setSetting('terminal.cursorBlink', String(blink))
}
