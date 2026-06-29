import { describe, it, expect } from 'vitest'
import { parseMobaMacro } from './mobaMacro'

describe('parseMobaMacro', () => {
  it('parses the real MobaXterm macro from the editor', () => {
    // The macro from MobaXterm's "Macro edition" dialog (screenshot): build a
    // command with a BACK edit, RETURN, then timed prompts.
    const macro = [
      'ssh t769',
      'BACK',
      '0784@213.0.143.226',
      'RETURN',
      'SLEEP=1200',
      'yes',
      'RETURN',
      'SLEEP=1200',
      'RETURN',
      'SLEEP=1200',
      'a',
    ].join('\n')

    expect(parseMobaMacro(macro)).toEqual([
      // text + BACK(\x7f) + text, flushed on RETURN(\r) as a single send
      {
        kind: 'send',
        expect: '',
        is_regex: false,
        send: 'ssh t769\\x7f0784@213.0.143.226\\r',
        timeout_ms: 10000,
      },
      { kind: 'wait', expect: '', is_regex: false, send: '', timeout_ms: 1200 },
      { kind: 'send', expect: '', is_regex: false, send: 'yes\\r', timeout_ms: 10000 },
      { kind: 'wait', expect: '', is_regex: false, send: '', timeout_ms: 1200 },
      { kind: 'send', expect: '', is_regex: false, send: '\\r', timeout_ms: 10000 },
      { kind: 'wait', expect: '', is_regex: false, send: '', timeout_ms: 1200 },
      // trailing text with no RETURN — flushed at EOF (runner appends Enter)
      { kind: 'send', expect: '', is_regex: false, send: 'a', timeout_ms: 10000 },
    ])
  })

  it('maps special tokens and is case-insensitive', () => {
    const steps = parseMobaMacro('echo hi\nTab\nenter\nsleep = 500')
    expect(steps).toEqual([
      { kind: 'send', expect: '', is_regex: false, send: 'echo hi\\t\\r', timeout_ms: 10000 },
      { kind: 'wait', expect: '', is_regex: false, send: '', timeout_ms: 500 },
    ])
  })

  it('escapes literal backslashes in typed text', () => {
    const steps = parseMobaMacro('cd C:\\temp\nRETURN')
    expect(steps).toEqual([
      { kind: 'send', expect: '', is_regex: false, send: 'cd C:\\\\temp\\r', timeout_ms: 10000 },
    ])
  })

  it('returns nothing for empty input', () => {
    expect(parseMobaMacro('')).toEqual([])
    expect(parseMobaMacro('\n  \n')).toEqual([])
  })
})
