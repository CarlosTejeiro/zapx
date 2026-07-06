// Pure keyboard-shortcut logic (no Svelte runes), so it can be unit-tested
// under Vitest's plain-node environment. `keybindings.svelte.ts` holds the
// reactive `bindings` state and wires these helpers to it.

export const SHORTCUT_ACTIONS = [
  { id: 'new-session', label: 'New session…', default: 'Ctrl+N' },
  { id: 'new-tab', label: 'New local tab', default: 'Ctrl+T' },
  { id: 'close-tab', label: 'Close tab', default: 'Ctrl+W' },
  { id: 'next-tab', label: 'Next tab', default: 'Ctrl+Tab' },
  { id: 'prev-tab', label: 'Previous tab', default: 'Ctrl+Shift+Tab' },
  { id: 'settings', label: 'Open settings', default: 'Ctrl+,' },
  { id: 'split-h', label: 'Split horizontally', default: 'Ctrl+Shift+H' },
  { id: 'split-v', label: 'Split vertically', default: 'Ctrl+Shift+K' },
  { id: 'multi-exec', label: 'Toggle MultiExec broadcast', default: 'Ctrl+Shift+M' },
  { id: 'snippets', label: 'Open snippets', default: 'Ctrl+Shift+S' },
  { id: 'quick-connect', label: 'Quick connect (ad-hoc)', default: 'Ctrl+Shift+N' },
] as const

export type ShortcutAction = (typeof SHORTCUT_ACTIONS)[number]['id']

// Physical punctuation keys whose emitted character changes with Shift or the
// active keyboard layout — e.g. Shift+`\` yields `|` on a US layout, and `\`
// needs AltGr on a Spanish one. Binding these by the character the event
// reports makes the default combos unreachable (a `Ctrl+Shift+\` binding never
// matches because the event carries `|`, and `Ctrl+\` can't be typed at all on
// layouts where `\` is an AltGr combo). We canonicalise them by physical
// position (`e.code`) instead, so the documented combo works on every layout,
// whatever character the key produces. Letters/digits still use the produced
// character so they follow the user's layout as printed.
const CODE_TO_SYMBOL: Record<string, string> = {
  Backquote: '`',
  Minus: '-',
  Equal: '=',
  BracketLeft: '[',
  BracketRight: ']',
  Backslash: '\\',
  Semicolon: ';',
  Quote: "'",
  Comma: ',',
  Period: '.',
  Slash: '/',
}

/// Build a combo string from a KeyboardEvent, or `null` if only modifier keys
/// are currently pressed (so we don't capture half-typed shortcuts).
export function eventToCombo(e: KeyboardEvent): string | null {
  const raw = e.key
  if (raw === 'Control' || raw === 'Meta' || raw === 'Alt' || raw === 'Shift') {
    return null
  }
  // Prefer the physical key for punctuation so Shift / keyboard layout can't
  // scramble the combo; otherwise use the produced character (upper-cased for
  // single letters) or the named key verbatim (Tab, Enter, arrows…).
  let key = CODE_TO_SYMBOL[e.code]
  if (!key) key = raw.length === 1 ? raw.toUpperCase() : raw
  // Stable modifier order so combos serialise deterministically.
  const parts: string[] = []
  if (e.ctrlKey) parts.push('Ctrl')
  if (e.metaKey) parts.push('Meta')
  if (e.altKey) parts.push('Alt')
  if (e.shiftKey) parts.push('Shift')
  parts.push(key)
  return parts.join('+')
}

/// Find the action whose bound combo equals `combo`, or null. `bindings` maps
/// action id → combo string.
export function findAction(
  combo: string | null,
  bindings: Record<string, string>,
): ShortcutAction | null {
  if (!combo) return null
  for (const a of SHORTCUT_ACTIONS) {
    if (bindings[a.id] === combo) return a.id
  }
  return null
}
