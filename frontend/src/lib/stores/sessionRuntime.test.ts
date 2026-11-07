import { describe, expect, it } from 'vitest'

// Vitest runs without the Svelte compiler (see vitest.config.ts), so give the
// module the one rune it uses at load time: plain state is enough here.
;(globalThis as unknown as { $state: <T>(v: T) => T }).$state = (v) => v
const { paneToSession, registerSession, unregisterSession } =
  await import('./sessionRuntime.svelte')

describe('pane ↔ session registry', () => {
  it('a torn-down terminal does not drop the session that replaced it', () => {
    registerSession(7, 'old')
    registerSession(7, 'new') // the replacement opened first
    unregisterSession(7, 'old') // then the old one is destroyed
    expect(paneToSession.get(7)).toBe('new')
    unregisterSession(7, 'new')
    expect(paneToSession.has(7)).toBe(false)
  })

  it('without a session id it forgets the pane unconditionally', () => {
    registerSession(8, 'x')
    unregisterSession(8)
    expect(paneToSession.has(8)).toBe(false)
  })
})
