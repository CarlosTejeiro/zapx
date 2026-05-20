import { getSetting, setSetting, listColorSchemes } from '$lib/bridge/commands'
import type { ColorScheme, ColorPalette } from '$lib/bridge/types'

// ── reactive global terminal settings ──────────────────────────────────────

export const terminalSettings = $state({
  fontFamily: 'Cascadia Code, JetBrains Mono, monospace',
  fontSize: 14,
  lineHeight: 1.2,
  cursorStyle: 'block' as 'block' | 'underline' | 'bar',
  cursorBlink: true,
  activeColorScheme: 'One Dark',
})

export let colorSchemes = $state<ColorScheme[]>([])

export const activePalette = $derived<ColorPalette>(
  parsePalette(
    colorSchemes.find((s) => s.name === terminalSettings.activeColorScheme)?.palette_json ?? null,
  ),
)

function parsePalette(json: string | null): ColorPalette {
  if (!json) return DEFAULT_PALETTE
  try {
    return JSON.parse(json) as ColorPalette
  } catch {
    return DEFAULT_PALETTE
  }
}

// ── load from DB on app start ───────────────────────────────────────────────

export async function loadSettings(): Promise<void> {
  try {
    colorSchemes = await listColorSchemes()
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
  } catch {
    // use defaults
  }
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

// ── fallback palette (One Dark) used before DB loads ───────────────────────

const DEFAULT_PALETTE: ColorPalette = {
  background: '#282c34',
  foreground: '#abb2bf',
  cursor: '#528bff',
  black: '#282c34',
  red: '#e06c75',
  green: '#98c379',
  yellow: '#e5c07b',
  blue: '#61afef',
  magenta: '#c678dd',
  cyan: '#56b6c2',
  white: '#abb2bf',
  brightBlack: '#5c6370',
  brightRed: '#e06c75',
  brightGreen: '#98c379',
  brightYellow: '#e5c07b',
  brightBlue: '#61afef',
  brightMagenta: '#c678dd',
  brightCyan: '#56b6c2',
  brightWhite: '#ffffff',
}
