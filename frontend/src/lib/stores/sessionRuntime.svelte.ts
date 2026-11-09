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

// Scoped broadcast target set, orthogonal to `broadcast.enabled`. When a grid
// tab is open it sets `activeSessionIds` to its own live session UUIDs so that
// fan-out (master bar, per-pane typing, snippets) targets ONLY that group
// instead of every open pane. `null` = legacy "all panes" behaviour.
export const broadcastGroup = $state({
  activeSessionIds: null as Set<string> | null,
})

// Which grid holds the scope above. Plain (non-reactive) on purpose: an effect
// teardown reads $state as it was BEFORE the change that re-runs it, so a grid
// checking `broadcastGroup.activeSessionIds` there would see its own old scope
// and clear the one another grid had just claimed.
let scopeOwner: object | null = null

/** Scope broadcast fan-out to `ids` on behalf of `owner` (the active grid). */
export function claimBroadcastScope(owner: object, ids: Set<string>): void {
  scopeOwner = owner
  broadcastGroup.activeSessionIds = ids
}

/** Drop the scope — only if `owner` still holds it. */
export function releaseBroadcastScope(owner: object): void {
  if (scopeOwner !== owner) return
  scopeOwner = null
  broadcastGroup.activeSessionIds = null
}

export function registerSession(paneId: number, sessionId: string): void {
  paneToSession.set(paneId, sessionId)
}

/** Forget the pane's session. With `sessionId`, only if the pane still maps
 *  to it — a terminal being torn down must not drop the session of the one
 *  that replaced it in the same pane. */
export function unregisterSession(paneId: number, sessionId?: string): void {
  if (sessionId !== undefined && paneToSession.get(paneId) !== sessionId) return
  paneToSession.delete(paneId)
}

// sessionId → focus the pane's terminal. Registered by TerminalTab so UI
// outside the terminal (snippet buttons, master input bar) can hand focus back
// after sending — otherwise a snippet without a trailing newline leaves the
// caret on the button and the next keystrokes go nowhere.
const focusCallbacks = new Map<string, () => void>()

export function registerFocus(sessionId: string, focus: () => void): void {
  focusCallbacks.set(sessionId, focus)
}

export function unregisterFocus(sessionId: string): void {
  focusCallbacks.delete(sessionId)
}

export function focusSession(sessionId: string): void {
  focusCallbacks.get(sessionId)?.()
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

/**
 * Broadcast targets for `senderSessionId`, honouring an active group scope.
 * When `broadcastGroup.activeSessionIds` is null this is identical to
 * `otherSessionIds` (legacy all-panes behaviour); when a group is active it
 * returns only the group members other than the sender. Existing call-sites
 * can switch to this freely without changing the no-group case.
 */
export function broadcastTargets(senderSessionId: string): string[] {
  const grp = broadcastGroup.activeSessionIds
  if (!grp) return otherSessionIds(senderSessionId)
  const out: string[] = []
  for (const id of paneToSession.values()) {
    if (id !== senderSessionId && grp.has(id)) out.push(id)
  }
  return out
}
