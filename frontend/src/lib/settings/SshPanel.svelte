<script lang="ts">
  /// SSH-specific settings. Today the only one is the Windows-only agent
  /// priority. Lives in its own tab so future SSH knobs (default port,
  /// keep-alive interval, prompt-detect opt-out) have a sensible home.

  import { onMount } from 'svelte'
  import { getSetting, setSetting } from '$lib/bridge/commands'

  type Priority = 'auto' | 'pageant-first' | 'openssh-only' | 'pageant-only'

  const OPTIONS: { value: Priority; label: string; help: string }[] = [
    {
      value: 'auto',
      label: 'Auto (OpenSSH → Pageant)',
      help: 'Default. Try Windows OpenSSH first; fall back to Pageant if it isn\'t reachable or holds no usable identity.',
    },
    {
      value: 'pageant-first',
      label: 'Pageant first (Pageant → OpenSSH)',
      help: 'For PuTTY-centric workflows: keys live in Pageant. Avoids the failed OpenSSH probe on every connection.',
    },
    {
      value: 'openssh-only',
      label: 'OpenSSH only (no fallback)',
      help: 'Use the Windows OpenSSH named pipe and nothing else. A missing pipe fails loudly instead of silently falling back.',
    },
    {
      value: 'pageant-only',
      label: 'Pageant only (no fallback)',
      help: 'Use Pageant and nothing else. A missing Pageant process fails loudly.',
    },
  ]

  let current = $state<Priority>('auto')
  let saving = $state(false)

  onMount(async () => {
    const raw = (await getSetting('ssh.agent_priority')) ?? 'auto'
    if (OPTIONS.some((o) => o.value === raw)) current = raw as Priority
  })

  async function onChange(value: Priority) {
    if (value === current) return
    saving = true
    try {
      await setSetting('ssh.agent_priority', value)
      current = value
    } finally {
      saving = false
    }
  }

  const selectedHelp = $derived(
    OPTIONS.find((o) => o.value === current)?.help ?? '',
  )
</script>

<div class="panel">
  <section>
    <h3>SSH agent priority (Windows)</h3>
    <p class="hint-help">
      Which agent backend ZAPX tries when a session uses
      <code>Agent</code> authentication. <strong>macOS and Linux ignore this</strong> —
      they always use <code>$SSH_AUTH_SOCK</code>.
    </p>

    <select
      bind:value={current}
      onchange={(e) => onChange((e.currentTarget as HTMLSelectElement).value as Priority)}
      disabled={saving}
    >
      {#each OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>

    {#if selectedHelp}
      <p class="hint-explain">{selectedHelp}</p>
    {/if}
  </section>
</div>

<style>
  .panel {
    padding: 0.75rem 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    color: var(--zx-text);
    font-family: var(--zx-font-ui);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  h3 {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--zx-text);
    margin: 0 0 0.15rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .hint-help {
    font-size: 0.78rem;
    color: var(--zx-text-muted);
    margin: 0 0 0.25rem 0;
    line-height: 1.5;
  }

  .hint-explain {
    font-size: 0.72rem;
    color: var(--zx-text-muted);
    margin: 0.1rem 0 0;
    line-height: 1.45;
    background: color-mix(in srgb, var(--zx-accent) 8%, transparent);
    border-left: 2px solid color-mix(in srgb, var(--zx-accent) 35%, transparent);
    padding: 0.4rem 0.6rem;
    border-radius: 0 0.2rem 0.2rem 0;
  }

  select {
    width: 22rem;
    max-width: 100%;
    background: var(--zx-surface-2);
    border: 1px solid var(--zx-border);
    border-radius: 0.25rem;
    padding: 0.35rem 0.45rem;
    color: var(--zx-text);
    font-size: 0.85rem;
    font-family: inherit;
  }

  select:focus-visible {
    border-color: var(--zx-accent);
    box-shadow: var(--zx-ring);
    outline: none;
  }

  select:disabled {
    opacity: 0.6;
  }

  code {
    background: var(--zx-surface-2);
    border-radius: 0.2rem;
    padding: 0 0.3rem;
    font-size: 0.72rem;
  }
</style>
