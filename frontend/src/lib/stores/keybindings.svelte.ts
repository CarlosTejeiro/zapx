// Editable keyboard shortcuts.
//
// Each action has a stable id, a human label, and a default combo. The user
// can override the combo from `SettingsModal` and it is persisted to the
// `settings` SQLite table under the key `shortcut.<action-id>`.
//
// `matchAction(e)` is the dispatch helper — `App.svelte` calls it from its
// global keydown listener and then runs the corresponding action handler.

import { getSetting, setSetting } from '$lib/bridge/commands'

export const SHORTCUT_ACTIONS = [
  { id: 'new-session', label: 'New session…', default: 'Ctrl+N' },
  { id: 'new-tab', label: 'New local tab', default: 'Ctrl+T' },
  { id: 'close-tab', label: 'Close tab', default: 'Ctrl+W' },
  { id: 'next-tab', label: 'Next tab', default: 'Ctrl+Tab' },
  { id: 'prev-tab', label: 'Previous tab', default: 'Ctrl+Shift+Tab' },
  { id: 'settings', label: 'Open settings', default: 'Ctrl+,' },
  { id: 'split-h', label: 'Split horizontally', default: 'Ctrl+\\' },
  { id: 'split-v', label: 'Split vertically', default: 'Ctrl+Shift+\\' },
  { id: 'multi-exec', label: 'Toggle MultiExec broadcast', default: 'Ctrl+Shift+M' },
  { id: 'snippets', label: 'Open snippets', default: 'Ctrl+Shift+S' },
  { id: 'quick-connect', label: 'Quick connect (ad-hoc)', default: 'Ctrl+Shift+N' },
] as const

export type ShortcutAction = (typeof SHORTCUT_ACTIONS)[number]['id']

/// Reactive bindings: action id → combo string (e.g. `"Ctrl+Shift+M"`).
export const bindings = $state<Record<string, string>>(
  Object.fromEntries(SHORTCUT_ACTIONS.map((a) => [a.id, a.default])),
)

/// Pull each action's override from the settings table at startup.
export async function loadBindings(): Promise<void> {
  for (const action of SHORTCUT_ACTIONS) {
    try {
      const stored = await getSetting(`shortcut.${action.id}`)
      if (stored && stored.trim()) {
        bindings[action.id] = stored
      }
    } catch {
      /* settings key absent → keep default */
    }
  }
}

/// Persist a single binding (also updates the reactive store).
export async function saveBinding(actionId: ShortcutAction, combo: string): Promise<void> {
  bindings[actionId] = combo
  await setSetting(`shortcut.${actionId}`, combo)
}

/// Reset an action to its compiled-in default and clear the override in DB.
export async function resetBinding(actionId: ShortcutAction): Promise<void> {
  const def = SHORTCUT_ACTIONS.find((a) => a.id === actionId)?.default ?? ''
  bindings[actionId] = def
  await setSetting(`shortcut.${actionId}`, '')
}

/// Build a combo string from a KeyboardEvent, or `null` if only modifier
/// keys are currently pressed (so we don't capture half-typed shortcuts).
export function eventToCombo(e: KeyboardEvent): string | null {
  let key = e.key
  if (key === 'Control' || key === 'Meta' || key === 'Alt' || key === 'Shift') {
    return null
  }
  if (key.length === 1) key = key.toUpperCase()
  // Stable modifier order so combos serialise deterministically.
  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.metaKey) parts.push('Meta')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  parts.push(key)
  return parts.join('+')
}

/// Find the action whose combo matches this event, or null.
export function matchAction(e: KeyboardEvent): ShortcutAction | null {
  const combo = eventToCombo(e)
  if (!combo) return null
  for (const a of SHORTCUT_ACTIONS) {
    if (bindings[a.id] === combo) return a.id
  }
  return null
}
