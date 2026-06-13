<script lang="ts">
  import type { PylonTheme } from '$lib/themes/index'
  import type { HostCapture } from './commandRunner.svelte'
  import Icon from '$lib/icons/Icon.svelte'

  interface Props {
    theme: PylonTheme
    command: string
    hosts: HostCapture[]
    running: boolean
    onClose: () => void
  }

  const { theme, command, hosts, running, onClose }: Props = $props()

  // ── normalization ─────────────────────────────────────────────────────────
  // Strip the echoed command (first line) + the trailing prompt line, collapse
  // CRLF, trim per-line trailing whitespace, so "byte-identical" means
  // "the meaningful output matched", not "the prompt string matched".
  function normalize(buffer: string): string {
    let lines = buffer.replace(/\r\n/g, '\n').replace(/\r/g, '\n').split('\n')
    // Drop the first line if it's the echoed command (possibly with a prompt
    // prefix ending in the command text).
    if (lines.length && command && lines[0]!.includes(command)) lines = lines.slice(1)
    // Drop a trailing prompt-ish line (last non-empty line) + trailing blanks.
    while (lines.length && lines[lines.length - 1]!.trim() === '') lines.pop()
    if (lines.length) lines = lines.slice(0, -1) // the returned prompt line
    return lines.map((l) => l.replace(/\s+$/, '')).join('\n').trim()
  }

  interface OutputGroup {
    text: string
    hosts: HostCapture[]
  }

  const groups = $derived.by<OutputGroup[]>(() => {
    const map = new Map<string, OutputGroup>()
    for (const h of hosts) {
      if (h.state === 'timeout') continue
      const text = normalize(h.buffer)
      const g = map.get(text)
      if (g) g.hosts.push(h)
      else map.set(text, { text, hosts: [h] })
    }
    return [...map.values()].sort((a, b) => b.hosts.length - a.hosts.length)
  })

  const timedOut = $derived(hosts.filter((h) => h.state === 'timeout'))
  const allSame = $derived(!running && groups.length === 1 && timedOut.length === 0)

  // Expand/collapse a group's output.
  let open = $state<Set<number>>(new Set([0]))
  function toggle(i: number) {
    const next = new Set(open)
    next.has(i) ? next.delete(i) : next.add(i)
    open = next
  }

  // ── diff between two groups ────────────────────────────────────────────────
  let diffA = $state<number | null>(null)
  let diffB = $state<number | null>(null)

  interface DiffLine {
    kind: 'same' | 'a' | 'b'
    text: string
  }

  // Minimal LCS line diff — fine for the modest output sizes of a CLI command.
  function lineDiff(a: string, b: string): DiffLine[] {
    const A = a.split('\n')
    const B = b.split('\n')
    const n = A.length
    const m = B.length
    const lcs: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0))
    for (let i = n - 1; i >= 0; i--) {
      for (let j = m - 1; j >= 0; j--) {
        lcs[i]![j] = A[i] === B[j] ? lcs[i + 1]![j + 1]! + 1 : Math.max(lcs[i + 1]![j]!, lcs[i]![j + 1]!)
      }
    }
    const out: DiffLine[] = []
    let i = 0
    let j = 0
    while (i < n && j < m) {
      if (A[i] === B[j]) {
        out.push({ kind: 'same', text: A[i]! })
        i++
        j++
      } else if (lcs[i + 1]![j]! >= lcs[i]![j + 1]!) {
        out.push({ kind: 'a', text: A[i]! })
        i++
      } else {
        out.push({ kind: 'b', text: B[j]! })
        j++
      }
    }
    while (i < n) out.push({ kind: 'a', text: A[i++]! })
    while (j < m) out.push({ kind: 'b', text: B[j++]! })
    return out
  }

  const diff = $derived.by<DiffLine[] | null>(() => {
    if (diffA == null || diffB == null) return null
    const ga = groups[diffA]
    const gb = groups[diffB]
    if (!ga || !gb) return null
    return lineDiff(ga.text, gb.text)
  })

  function pickDiff(i: number) {
    if (diffA === i) { diffA = null; return }
    if (diffB === i) { diffB = null; return }
    if (diffA == null) diffA = i
    else if (diffB == null) diffB = i
    else { diffA = i; diffB = null }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose()
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
  <div class="panel" style:background="#18181b" style:font-family={theme.fontUi}>
    <div class="header">
      <div class="title">
        <span class="cmd" style:color={theme.accent}>$ {command || '—'}</span>
        {#if running}
          <span class="badge running">running…</span>
        {:else if allSame}
          <span class="badge ok">✓ {hosts.length} hosts identical</span>
        {:else}
          <span class="badge warn">{groups.length} variants{timedOut.length ? ` · ${timedOut.length} no reply` : ''}</span>
        {/if}
      </div>
      <button class="btn" onclick={onClose} title="Close"><Icon name="x" size={12} /></button>
    </div>

    <p class="hint">
      Outputs grouped by identical content. Select two groups to see the diff.
    </p>

    <div class="groups">
      {#each groups as g, i (i)}
        <div class="group" class:diff-a={diffA === i} class:diff-b={diffB === i}>
          <div class="group-head">
            <button class="caret" onclick={() => toggle(i)}><Icon name="chevron" size={11} open={open.has(i)} /></button>
            <span class="count" style:background={theme.accent}>{g.hosts.length}</span>
            <span class="hostnames">{g.hosts.map((h) => h.label).join(', ')}</span>
            <button
              class="btn small"
              class:active={diffA === i || diffB === i}
              onclick={() => pickDiff(i)}
              title="Mark for diff"
            >diff</button>
          </div>
          {#if open.has(i)}
            <pre class="output">{g.text || '(no output)'}</pre>
          {/if}
        </div>
      {/each}

      {#if timedOut.length}
        <div class="group timeout">
          <div class="group-head">
            <span class="count to">{timedOut.length}</span>
            <span class="hostnames">{timedOut.map((h) => h.label).join(', ')}</span>
            <span class="to-label">timed out</span>
          </div>
        </div>
      {/if}

      {#if groups.length === 0 && !running}
        <p class="empty">No host returned output.</p>
      {/if}
    </div>

    {#if diff}
      <div class="diff">
        <div class="diff-head">
          Diff: group {(diffA ?? 0) + 1} (red) ↔ group {(diffB ?? 0) + 1} (green)
        </div>
        <pre class="diff-body">{#each diff as l (l.text + l.kind)}<span class="dl {l.kind}">{l.kind === 'a' ? '- ' : l.kind === 'b' ? '+ ' : '  '}{l.text}
</span>{/each}</pre>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
  }

  .panel {
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1rem 1.2rem 1.2rem;
    width: 52rem;
    max-width: 96vw;
    height: 40rem;
    max-height: 92vh;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }

  .title {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
  }

  .cmd {
    font-family: monospace;
    font-size: 0.9rem;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    font-size: 0.7rem;
    font-weight: 600;
    padding: 0.1rem 0.45rem;
    border-radius: 0.6rem;
    flex-shrink: 0;
  }
  .badge.ok { background: rgba(34, 197, 94, 0.18); color: #4ade80; }
  .badge.warn { background: rgba(251, 191, 36, 0.18); color: #fbbf24; }
  .badge.running { background: rgba(96, 165, 250, 0.18); color: #60a5fa; }

  .hint {
    margin: 0;
    font-size: 0.75rem;
    color: #71717a;
  }

  .groups {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-height: 0;
  }

  .group {
    background: #0a0a0b;
    border: 1px solid #27272a;
    border-radius: 0.3rem;
    overflow: hidden;
  }
  .group.diff-a { border-color: #ef4444; }
  .group.diff-b { border-color: #22c55e; }
  .group.timeout { border-color: #52525b; }

  .group-head {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
  }

  .caret {
    background: none;
    border: none;
    color: #a1a1aa;
    cursor: pointer;
    font-size: 0.8rem;
    padding: 0;
  }

  .count {
    font-size: 0.72rem;
    font-weight: 700;
    color: #0a0a0b;
    border-radius: 0.5rem;
    padding: 0.05rem 0.4rem;
    flex-shrink: 0;
  }
  .count.to { background: #52525b; color: #e4e4e7; }

  .hostnames {
    flex: 1;
    font-size: 0.78rem;
    color: #d4d4d8;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .to-label {
    font-size: 0.72rem;
    color: #71717a;
    flex-shrink: 0;
  }

  .output {
    margin: 0;
    padding: 0.5rem 0.7rem;
    border-top: 1px solid #27272a;
    font-family: monospace;
    font-size: 0.75rem;
    color: #cbd5e1;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 14rem;
    overflow-y: auto;
  }

  .empty {
    font-size: 0.8rem;
    color: #52525b;
  }

  .diff {
    border: 1px solid #3f3f46;
    border-radius: 0.3rem;
    max-height: 12rem;
    overflow-y: auto;
    flex-shrink: 0;
  }

  .diff-head {
    font-size: 0.72rem;
    color: #a1a1aa;
    padding: 0.35rem 0.6rem;
    border-bottom: 1px solid #27272a;
    position: sticky;
    top: 0;
    background: #18181b;
  }

  .diff-body {
    margin: 0;
    padding: 0.4rem 0.6rem;
    font-family: monospace;
    font-size: 0.74rem;
  }

  .dl { display: block; white-space: pre-wrap; word-break: break-word; }
  .dl.same { color: #71717a; }
  .dl.a { color: #f87171; background: rgba(239, 68, 68, 0.08); }
  .dl.b { color: #4ade80; background: rgba(34, 197, 94, 0.08); }

  .btn {
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #d4d4d8;
    font-size: 0.78rem;
    padding: 0.25rem 0.55rem;
    cursor: pointer;
    font-family: inherit;
    flex-shrink: 0;
  }
  .btn:hover { background: #3f3f46; }
  .btn.small { font-size: 0.7rem; padding: 0.15rem 0.45rem; }
  .btn.active { background: #1d4ed8; border-color: #2563eb; color: #fff; }
</style>
