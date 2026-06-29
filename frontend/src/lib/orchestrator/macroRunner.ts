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
import { getFocusedSessionId, focusSession } from '$lib/stores/sessionRuntime.svelte'
import { showToast, type ToastKind } from '$lib/ui/toast-store.svelte'
import type { LoginStep, Snippet } from '$lib/bridge/types'

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

/**
 * Decode C-style backslash escapes (`\r` `\n` `\t` `\b` `\e` `\0` `\\` `\xHH`)
 * so a step typed as `cmd\r` presses Enter instead of sending the two literal
 * characters. Unknown/malformed escapes are kept verbatim (backslash included).
 */
function decodeEscapes(s: string): string {
  let out = ''
  for (let i = 0; i < s.length; i++) {
    if (s[i] !== '\\') {
      out += s[i]
      continue
    }
    const n = s[i + 1]
    switch (n) {
      case 'r':
        out += '\r'
        i++
        break
      case 'n':
        out += '\n'
        i++
        break
      case 't':
        out += '\t'
        i++
        break
      case 'b':
        out += '\b'
        i++
        break
      case 'e':
        out += '\x1b'
        i++
        break
      case '0':
        out += '\0'
        i++
        break
      case '\\':
        out += '\\'
        i++
        break
      case 'x': {
        const hex = s.slice(i + 2, i + 4)
        if (/^[0-9a-fA-F]{2}$/.test(hex)) {
          out += String.fromCharCode(parseInt(hex, 16))
          i += 3
        } else {
          out += '\\' // not a valid \xHH — keep the backslash literally
        }
        break
      }
      default:
        out += '\\' // unknown escape — keep backslash; next iteration emits the char
    }
  }
  return out
}

/** Encode `text` for the PTY, appending Enter unless the user ended the line. */
function withEnter(text: string): number[] {
  let t = text
  if (t.length > 0 && !t.endsWith('\n') && !t.endsWith('\r')) t += '\r'
  return Array.from(new TextEncoder().encode(t))
}

async function send(sessionId: string, text: string): Promise<void> {
  const decoded = decodeEscapes(text)
  if (decoded.length === 0) return
  await invoke('send_input', { sessionId, data: withEnter(decoded) }).catch(() => {})
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

/** Status sink for {@link runMacroOnFocused}. Defaults to toasts; the button
 *  bar passes its own so feedback shows inline next to the bar instead. */
export type MacroNotify = (level: ToastKind, message: string) => void

const toastNotify: MacroNotify = (level, message) => showToast({ kind: level, title: message })

/**
 * Run a macro snippet against the currently-focused session. Shared by the
 * snippet button bar and the sidebar Macros section: resolves the focused
 * session, parses the snippet's steps, runs them, hands focus back to the
 * terminal, and reports progress through `notify`.
 */
export async function runMacroOnFocused(
  snippet: Snippet,
  notify: MacroNotify = toastNotify,
): Promise<void> {
  const focused = getFocusedSessionId()
  if (!focused) {
    notify('warning', 'No session focused — click a terminal first.')
    return
  }
  let steps: LoginStep[]
  try {
    steps = JSON.parse(snippet.steps_json ?? '[]')
  } catch {
    notify('error', `Macro "${snippet.name}" is corrupt`)
    return
  }
  notify('info', `Running macro "${snippet.name}"…`)
  const res = await runMacro(focused, steps)
  focusSession(focused)
  if (res.ok) {
    notify('success', `Macro "${snippet.name}" done`)
  } else {
    notify('error', `Macro "${snippet.name}" timed out at step ${(res.failedStep ?? 0) + 1}`)
  }
}
