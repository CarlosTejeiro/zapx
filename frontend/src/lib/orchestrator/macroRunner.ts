// On-demand macro execution (the snippet-as-macro path).
//
// Runs a saved expect/send/wait step list against ONE live session, entirely in
// the frontend — the same approach the audit CommandRunner uses: subscribe to
// `terminal-data` for that session, buffer the (ANSI-stripped) output, and send
// keystrokes back via `send_input`. The connect-time login script is the
// backend twin of this (it must run inside the session I/O task); this one is
// user-triggered, so the frontend drives it.

import { invoke } from '@tauri-apps/api/core'
import { onTerminalData } from '$lib/bridge/events'
import type { LoginStep } from '$lib/bridge/types'

function stripAnsi(s: string): string {
  return s
    .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g, '')
    .replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, '')
    .replace(/\x1b[@-Z\\-_]/g, '')
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms))

function stepMatches(step: LoginStep, haystack: string): boolean {
  if (!step.expect) return false
  if (step.is_regex) {
    try {
      return new RegExp(step.expect).test(haystack)
    } catch {
      return haystack.includes(step.expect) // invalid regex → literal
    }
  }
  return haystack.includes(step.expect)
}

/** Encode `text` for the PTY, appending Enter unless the user ended the line. */
function withEnter(text: string): number[] {
  let t = text
  if (t.length > 0 && !t.endsWith('\n') && !t.endsWith('\r')) t += '\r'
  return Array.from(new TextEncoder().encode(t))
}

async function send(sessionId: string, text: string): Promise<void> {
  if (text.length === 0) return
  await invoke('send_input', { sessionId, data: withEnter(text) }).catch(() => {})
}

export interface MacroResult {
  ok: boolean
  /** Index of the step that timed out, when `ok` is false. */
  failedStep?: number
}

/**
 * Execute `steps` against `sessionId`. Resolves when the macro finishes or a
 * step times out. `expect` waits for its pattern (then sends), `send` fires
 * immediately, `wait` pauses. Best-effort: PTY send failures are swallowed.
 */
export async function runMacro(sessionId: string, steps: LoginStep[]): Promise<MacroResult> {
  if (steps.length === 0) return { ok: true }
  const decoder = new TextDecoder()
  let buffer = ''
  const unlisten = await onTerminalData((p) => {
    if (p.session_id === sessionId) {
      buffer += stripAnsi(decoder.decode(new Uint8Array(p.data), { stream: true }))
      if (buffer.length > 64 * 1024) buffer = buffer.slice(-64 * 1024)
    }
  })
  try {
    for (let i = 0; i < steps.length; i++) {
      const step = steps[i]!
      if (step.kind === 'send') {
        await send(sessionId, step.send)
      } else if (step.kind === 'wait') {
        await sleep(step.timeout_ms)
      } else {
        // expect: reset the window, then poll until the pattern shows or the
        // per-step timeout elapses; on match, send the step's payload.
        buffer = ''
        const deadline = Date.now() + step.timeout_ms
        let matched = false
        while (Date.now() < deadline) {
          if (stepMatches(step, buffer)) {
            matched = true
            break
          }
          await sleep(50)
        }
        if (!matched) return { ok: false, failedStep: i }
        await send(sessionId, step.send)
      }
    }
    return { ok: true }
  } finally {
    unlisten()
  }
}
