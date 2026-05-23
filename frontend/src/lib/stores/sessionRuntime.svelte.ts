// Reactive map of live terminal panes → runtime session ids, plus the global
// MultiExec broadcast toggle.
//
// TerminalTab calls `registerSession(paneId, sessionId)` when it finishes
// opening the backend session and `unregisterSession(paneId)` on destroy.
// App.svelte mirrors its `focusedPaneId` here so the snippets UI and any
// other "act on the focused session" code can look up the live session id
// without prop-drilling.

import { SvelteMap } from 'svelte/reactivity'

// paneId → live session UUID
export const paneToSession = new SvelteMap<number, string>()

// Mirrored from App.svelte's focusedPaneId. -1 = "none focused".
export const sessionRuntime = $state({
  focusedPaneId: -1 as number,
})

// MultiExec broadcast: when enabled, input typed in one terminal is also
// dispatched to all OTHER registered sessions.
export const broadcast = $state({ enabled: false })

export function registerSession(paneId: number, sessionId: string): void {
  paneToSession.set(paneId, sessionId)
}

export function unregisterSession(paneId: number): void {
  paneToSession.delete(paneId)
}

/** Live session id for the currently focused pane, or null. */
export function getFocusedSessionId(): string | null {
  return paneToSession.get(sessionRuntime.focusedPaneId) ?? null
}

/** All registered session ids except the sender — broadcast targets. */
export function otherSessionIds(senderSessionId: string): string[] {
  const out: string[] = []
  for (const id of paneToSession.values()) {
    if (id !== senderSessionId) out.push(id)
  }
  return out
}
