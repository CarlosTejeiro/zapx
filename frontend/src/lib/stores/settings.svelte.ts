import { getSetting, setSetting, listColorSchemes } from '$lib/bridge/commands'
import type { ColorScheme } from '$lib/bridge/types'

// ── reactive global terminal settings ──────────────────────────────────────

export const terminalSettings = $state({
  fontFamily: 'Cascadia Code, JetBrains Mono, monospace',
  fontSize: 14,
  lineHeight: 1.2,
  cursorStyle: 'block' as 'block' | 'underline' | 'bar',
  cursorBlink: true,
  activeColorScheme: 'One Dark',
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
