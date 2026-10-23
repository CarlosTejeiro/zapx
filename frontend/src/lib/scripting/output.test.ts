import { describe, it, expect } from 'vitest'
import {
  cleanText,
  echoEnd,
  extractRunOutput,
  lastLine,
  looksLikePrompt,
  MORE_RE,
  stripAnsi,
} from './output'

const FGT = [
  'fw1 # get vpn ipsec tunnel summary\r\n',
  "'SASE_INET' 203.0.113.1:0  selectors(total,up): 1/0  rx(pkt,err): 0/0  tx(pkt,err): 0/0\r\n",
  "'SASE_INET2' 203.0.113.2:0  selectors(total,up): 1/1  rx(pkt,err): 10/0  tx(pkt,err): 9/0\r\n",
  '\r\n',
  'fw1 # ',
].join('')

describe('extractRunOutput', () => {
  it('returns the lines between the echo and the returning prompt', () => {
    expect(extractRunOutput(FGT, 'get vpn ipsec tunnel summary')).toBe(
      [
        "'SASE_INET' 203.0.113.1:0  selectors(total,up): 1/0  rx(pkt,err): 0/0  tx(pkt,err): 0/0",
        "'SASE_INET2' 203.0.113.2:0  selectors(total,up): 1/1  rx(pkt,err): 10/0  tx(pkt,err): 9/0",
      ].join('\n'),
    )
  })

  it('ignores a leftover prompt printed before the command', () => {
    const raw = 'fw1 # \r\nfw1 # get system status\r\nVersion: FortiGate-60F v7.4.4\r\nfw1 # '
    expect(extractRunOutput(raw, 'get system status')).toBe('Version: FortiGate-60F v7.4.4')
  })

  it('is null until the echo arrives, and empty for a command with no output', () => {
    expect(extractRunOutput('fw1 # ', 'end')).toBeNull()
    expect(extractRunOutput('fw1 (interface) # end\r\nfw1 # ', 'end')).toBe('')
  })

  it('drops pager prompts erased with backspaces', () => {
    const raw =
      'sw # show run\r\nline 1\r\n --More-- \x08\x08\x08\x08\x08\x08\x08\x08\x08\x08          \x08\x08\x08\x08\x08\x08\x08\x08\x08\x08line 2\r\nsw # '
    expect(extractRunOutput(raw, 'show run')).toBe('line 1\nline 2')
  })
})

describe('helpers', () => {
  it('strips ANSI color and title sequences', () => {
    expect(stripAnsi('\x1b[1;32mOK\x1b[0m\x1b]0;title\x07!')).toBe('OK!')
  })

  it('keeps the last write of a carriage-return-rewritten line', () => {
    expect(cleanText('progress 10%\rprogress 100%\n')).toBe('progress 100%\n')
  })

  it('finds the echo end and the last line', () => {
    expect(echoEnd('a\nfw # ls\nout\n', 'ls')).toBe(10)
    // A hostname containing the command is not its echo.
    expect(echoEnd('als-fw # \nals-fw # ls\nx\n', 'ls')).toBe(22)
    // Regex metacharacters in the command are literal.
    expect(echoEnd('fw # show | grep a.b\nx\n', 'show | grep a.b')).toBe(21)
    expect(echoEnd('fw # ls', 'ls')).toBe(-1)
    expect(lastLine('out\r\nfw1 (vdom) # ')).toBe('fw1 (vdom) # ')
  })

  it('recognises a pager waiting at the end of the output', () => {
    expect(MORE_RE.test('line\n--More-- ')).toBe(true)
    expect(MORE_RE.test(' --More--')).toBe(true)
    expect(MORE_RE.test('more text')).toBe(false)
  })
})

describe('looksLikePrompt', () => {
  it('recognises common CLI prompts', () => {
    for (const p of [
      'fw1 # ',
      'fw1 (SASE_INET) # ',
      'FGT-60F (vdom-a) # ',
      'switch>',
      'core-sw1#',
      'user@host:~$ ',
      '[admin@MikroTik] > ',
      'root@vm:~# ',
    ]) {
      expect(looksLikePrompt(p), p).toBe(true)
    }
  })

  it('rejects ordinary output and pager/disclaimer lines', () => {
    for (const p of [
      '',
      "'SASE_INET' 203.0.113.1:0  selectors(total,up): 1/0",
      'Version: FortiGate-60F v7.4.4',
      ' --More-- ',
      "(Press 'a' to accept):",
      'progress 100%',
      '#',
    ]) {
      expect(looksLikePrompt(p), p).toBe(false)
    }
  })
})
