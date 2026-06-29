// Parse a MobaXterm macro (the line list from Moba's "Macro edition" dialog)
// into ZAPX expect/send/wait steps.
//
// MobaXterm macros are a flat keystroke script: each line is either typed text
// or a special token. We map them onto our model:
//
//   text            → buffered as a `send` payload (typed as-is)
//   RETURN / ENTER  → append `\r` to the buffer and flush it as one `send`
//   BACK / BACKSPACE→ append a DEL (`\x7f`) to the buffer (erases a char on the
//                     remote line editor, e.g. bash readline)
//   TAB             → append `\t`
//   ESC / ESCAPE    → append `\x1b`
//   SPACE           → append a space
//   SLEEP=<ms>      → flush any pending buffer, then a `wait` step of <ms>
//
// Buffering until RETURN matters: it means a command isn't sent before it's
// fully composed (Moba builds a line with text + BACK edits, then RETURN). The
// escape sequences we emit are decoded at run time by the macro runner's
// `decodeEscapes`, so they reach the PTY as the real control bytes.

import type { LoginStep } from '$lib/bridge/types'

function sendStep(send: string): LoginStep {
  return { kind: 'send', expect: '', is_regex: false, send, timeout_ms: 10000 }
}

function waitStep(ms: number): LoginStep {
  return { kind: 'wait', expect: '', is_regex: false, send: '', timeout_ms: ms }
}

/** Escape backslashes in literal typed text so the runner's escape decoder
 *  doesn't reinterpret e.g. a Windows path `C:\temp` as control bytes. */
function literal(text: string): string {
  return text.replace(/\\/g, '\\\\')
}

/**
 * Parse MobaXterm macro text into ZAPX macro steps. Unrecognised non-empty
 * lines are treated as text to type. Returns an empty array for empty input.
 */
export function parseMobaMacro(text: string): LoginStep[] {
  const steps: LoginStep[] = []
  let buf = ''

  const flush = () => {
    if (buf.length > 0) {
      steps.push(sendStep(buf))
      buf = ''
    }
  }

  for (const raw of text.split(/\r?\n/)) {
    const trimmed = raw.trim()
    if (trimmed === '') continue

    const sleep = /^SLEEP\s*=\s*(\d+)$/i.exec(trimmed)
    if (sleep) {
      flush()
      steps.push(waitStep(parseInt(sleep[1]!, 10)))
      continue
    }

    switch (trimmed.toUpperCase()) {
      case 'RETURN':
      case 'ENTER':
        buf += '\\r'
        flush()
        break
      case 'BACK':
      case 'BACKSPACE':
        buf += '\\x7f'
        break
      case 'TAB':
        buf += '\\t'
        break
      case 'ESC':
      case 'ESCAPE':
        buf += '\\e'
        break
      case 'SPACE':
        buf += ' '
        break
      default:
        // Literal text to type. Use the raw line (sans surrounding whitespace
        // from the editor) and escape backslashes so they survive decoding.
        buf += literal(trimmed)
    }
  }

  flush()
  return steps
}
