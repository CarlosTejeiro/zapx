<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte'
  import {
    listHighlightRules,
    createHighlightRule,
    toggleHighlightRule,
    deleteHighlightRule,
  } from '$lib/bridge/commands'
  import type { HighlightRule } from '$lib/bridge/types'

  // ── state ──────────────────────────────────────────────────────────────────
  let rules = $state<HighlightRule[]>([])
  let loading = $state(false)
  let error = $state<string | null>(null)

  // new-rule form
  let newName = $state('')
  let newPattern = $state('')
  let newIsRegex = $state(false)
  let newFgColor = $state('#00ff00')
  let newBgColor = $state('')
  let newBold = $state(false)
  let newUnderline = $state(false)
  let creating = $state(false)
  let formError = $state<string | null>(null)

  // live regex tester
  let testInput = $state('')
  let testError = $state<string | null>(null)

  // ── derived: highlight spans for the live tester ───────────────────────────
  interface Span {
    text: string
    fg: string | null
    bg: string | null
    bold: boolean
    underline: boolean
  }

  let testSpans = $derived.by<Span[]>(() => {
    testError = null
    if (!testInput || !newPattern) return [{ text: testInput, fg: null, bg: null, bold: false, underline: false }]

    let regex: RegExp
    try {
      regex = new RegExp(newIsRegex ? newPattern : escapeRegex(newPattern), 'g')
    } catch (e) {
      testError = String(e)
      return [{ text: testInput, fg: null, bg: null, bold: false, underline: false }]
    }

    const spans: Span[] = []
    let last = 0
    for (const m of testInput.matchAll(regex)) {
      const start = m.index ?? 0
      if (start > last) spans.push({ text: testInput.slice(last, start), fg: null, bg: null, bold: false, underline: false })
      spans.push({
        text: m[0],
        fg: newFgColor || null,
        bg: newBgColor || null,
        bold: newBold,
        underline: newUnderline,
      })
      last = start + m[0].length
    }
    if (last < testInput.length) spans.push({ text: testInput.slice(last), fg: null, bg: null, bold: false, underline: false })
    return spans.length ? spans : [{ text: testInput, fg: null, bg: null, bold: false, underline: false }]
  })

  // ── helpers ────────────────────────────────────────────────────────────────
  function escapeRegex(s: string) {
    return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  }

  async function load() {
    loading = true
    error = null
    try {
      rules = await listHighlightRules()
    } catch (e) {
      error = String(e)
    } finally {
      loading = false
    }
  }

  async function handleCreate() {
    formError = null
    if (!newName.trim()) { formError = 'Name is required'; return }
    if (!newPattern.trim()) { formError = 'Pattern is required'; return }
    if (newIsRegex) {
      try { new RegExp(newPattern) } catch (e) { formError = `Invalid regex: ${e}`; return }
    }
    creating = true
    try {
      await createHighlightRule(
        newName.trim(),
        newPattern.trim(),
        newIsRegex,
        newFgColor || null,
        newBgColor || null,
        newBold,
        newUnderline,
      )
      newName = ''
      newPattern = ''
      newIsRegex = false
      newFgColor = '#00ff00'
      newBgColor = ''
      newBold = false
      newUnderline = false
      await load()
    } catch (e) {
      formError = String(e)
    } finally {
      creating = false
    }
  }

  async function handleToggle(rule: HighlightRule) {
    try {
      await toggleHighlightRule(rule.id, !rule.enabled)
      await load()
    } catch (e) {
      error = String(e)
    }
  }

  async function handleDelete(id: number) {
    try {
      await deleteHighlightRule(id)
      await load()
    } catch (e) {
      error = String(e)
    }
  }

  // load on mount
  $effect(() => { load() })
</script>

<div class="flex flex-col gap-4 p-4 text-sm">
  <h2 class="text-base font-semibold">Highlight Rules</h2>

  {#if error}
    <p class="text-red-400">{error}</p>
  {/if}

  <!-- ── existing rules ───────────────────────────────────────────────────── -->
  {#if loading}
    <p class="text-zinc-400">Loading…</p>
  {:else if rules.length === 0}
    <p class="text-zinc-500">No rules yet.</p>
  {:else}
    <ul class="flex flex-col gap-1">
      {#each rules as rule (rule.id)}
        <li class="rule-row">
          <input
            type="checkbox"
            checked={rule.enabled}
            onchange={() => handleToggle(rule)}
            class="accent-green-500"
            aria-label="Enable {rule.name}"
          />
          <span class="rule-name">{rule.name}</span>
          <span
            class="rule-chip"
            style:color={rule.fg_color ?? 'var(--zx-text)'}
            style:background-color={rule.bg_color ?? 'color-mix(in srgb, var(--zx-text) 6%, transparent)'}
            style:font-weight={rule.bold ? 'bold' : undefined}
            style:text-decoration={rule.underline ? 'underline' : undefined}
            title="Preview"
          >Aa</span>
          <code class="rule-pattern" title={rule.pattern}>{rule.pattern}</code>
          <span class="rule-kind">{rule.is_regex ? 'rx' : 'str'}</span>
          <button
            onclick={() => handleDelete(rule.id)}
            class="rule-del"
            aria-label="Delete {rule.name}"
          ><Icon name="x" size={12} /></button>
        </li>
      {/each}
    </ul>
  {/if}

  <!-- ── new rule form ─────────────────────────────────────────────────────── -->
  <details class="rounded bg-zinc-800">
    <summary class="cursor-pointer px-3 py-2 font-medium select-none">Add rule…</summary>
    <form
      onsubmit={(e) => { e.preventDefault(); handleCreate() }}
      class="flex flex-col gap-3 p-3"
    >
      {#if formError}
        <p class="text-red-400 text-xs">{formError}</p>
      {/if}

      <label class="flex flex-col gap-1">
        <span class="text-zinc-400">Name</span>
        <input bind:value={newName} class="bg-zinc-700 rounded px-2 py-1" placeholder="Cisco Error" />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-zinc-400">Pattern</span>
        <input bind:value={newPattern} class="bg-zinc-700 rounded px-2 py-1 font-mono" placeholder="%ERROR" />
      </label>

      <label class="flex items-center gap-2">
        <input type="checkbox" bind:checked={newIsRegex} class="accent-green-500" />
        <span class="text-zinc-400">Regular expression</span>
      </label>

      <div class="flex gap-4">
        <label class="flex flex-col gap-1">
          <span class="text-zinc-400">Foreground</span>
          <input type="color" bind:value={newFgColor} class="w-10 h-8 rounded cursor-pointer bg-transparent" />
        </label>
        <label class="flex flex-col gap-1">
          <span class="text-zinc-400">Background</span>
          <input type="color" bind:value={newBgColor} class="w-10 h-8 rounded cursor-pointer bg-transparent" />
        </label>
        <label class="flex items-end gap-2 pb-1">
          <input type="checkbox" bind:checked={newBold} class="accent-green-500" />
          <span class="text-zinc-400">Bold</span>
        </label>
        <label class="flex items-end gap-2 pb-1">
          <input type="checkbox" bind:checked={newUnderline} class="accent-green-500" />
          <span class="text-zinc-400">Underline</span>
        </label>
      </div>

      <!-- live tester -->
      <label class="flex flex-col gap-1">
        <span class="text-zinc-400">Live test</span>
        <input
          bind:value={testInput}
          class="bg-zinc-700 rounded px-2 py-1 font-mono"
          placeholder="Paste sample output here…"
        />
      </label>
      {#if testError}
        <p class="text-red-400 text-xs">{testError}</p>
      {:else if testInput}
        <p class="font-mono bg-zinc-900 rounded px-2 py-1 whitespace-pre-wrap break-all">
          {#each testSpans as span}
            <span
              style:color={span.fg ?? undefined}
              style:background-color={span.bg ?? undefined}
              style:font-weight={span.bold ? 'bold' : undefined}
              style:text-decoration={span.underline ? 'underline' : undefined}
            >{span.text}</span>
          {/each}
        </p>
      {/if}

      <button
        type="submit"
        disabled={creating}
        class="self-start bg-green-700 hover:bg-green-600 disabled:opacity-50 text-white rounded px-3 py-1"
      >{creating ? 'Saving…' : 'Add rule'}</button>
    </form>
  </details>
</div>

<style>
  .rule-row {
    display: grid;
    grid-template-columns: auto 1fr auto auto auto auto;
    gap: 0.6rem;
    align-items: center;
    padding: 0.35rem 0.6rem;
    background: color-mix(in srgb, var(--zx-text) 6%, transparent);
    border: 1px solid var(--zx-border);
    border-radius: 4px;
  }

  .rule-name {
    color: var(--zx-text);
    font-size: 0.82rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rule-chip {
    display: inline-block;
    padding: 0.05rem 0.5rem;
    border-radius: 3px;
    font-family: var(--zx-font-mono);
    font-size: 0.78rem;
    line-height: 1.3;
    min-width: 2rem;
    text-align: center;
    border: 1px solid var(--zx-border);
  }

  .rule-pattern {
    color: var(--zx-text-muted);
    font-family: var(--zx-font-mono);
    font-size: 0.7rem;
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background: var(--zx-surface-2);
    padding: 0.05rem 0.4rem;
    border-radius: 3px;
  }

  .rule-kind {
    font-size: 0.62rem;
    color: var(--zx-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.05rem 0.35rem;
    background: color-mix(in srgb, var(--zx-text) 6%, transparent);
    border-radius: 3px;
  }

  .rule-del {
    background: transparent;
    border: 0;
    color: var(--zx-text-muted);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0 0.25rem;
    border-radius: 3px;
  }
  .rule-del:hover {
    color: var(--zx-err);
    background: color-mix(in srgb, var(--zx-err) 10%, transparent);
  }
</style>
