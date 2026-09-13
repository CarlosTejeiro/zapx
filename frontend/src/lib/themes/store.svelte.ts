import { setSetting } from '$lib/bridge/commands'
import { parchment, themes } from './index'
import type { PylonTheme } from './index'

let active = $state<PylonTheme>(parchment)

export function getTheme(): PylonTheme {
  return active
}

/** In-memory switch only — also what `loadSettings` uses to restore the
 *  persisted choice at boot. An unknown name (e.g. a theme removed in a later
 *  version) is ignored, so the app falls back to the default instead of
 *  breaking. */
export function setTheme(name: string) {
  const t = themes[name]
  if (t) active = t
}

/** User-driven switch: apply and persist under `ui.theme` so the choice
 *  survives a restart. Only a valid name is ever written, and persistence is
 *  best-effort — a failed write must never undo or block the visible switch. */
export async function applyTheme(name: string): Promise<void> {
  if (!themes[name]) return
  setTheme(name)
  try {
    await setSetting('ui.theme', name)
  } catch {
    // Non-fatal: the theme is applied for this run; it just won't be remembered.
  }
}
