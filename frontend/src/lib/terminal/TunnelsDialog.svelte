<script lang="ts">
  import { onMount } from 'svelte'
  import {
    addLocalForward,
    addDynamicForward,
    listForwards,
    removeForward,
  } from '$lib/bridge/commands'
  import type { ForwardInfo } from '$lib/bridge/types'

  interface Props {
    sessionId: string
    onClose: () => void
  }

  let { sessionId, onClose }: Props = $props()

  let forwards = $state<ForwardInfo[]>([])
  let error = $state('')
  let busy = $state(false)

  let kind = $state<'local' | 'dynamic'>('local')
  let bindAddr = $state('127.0.0.1')
  let bindPort = $state<number | null>(null)
  let targetHost = $state('')
  let targetPort = $state<number | null>(null)

  onMount(refresh)

  async function refresh() {
    try {
      forwards = await listForwards(sessionId)
    } catch {
      forwards = []
    }
  }

  async function add() {
    error = ''
    if (!bindPort || bindPort < 1 || bindPort > 65535) {
      error = 'Bind port must be 1–65535.'
      return
    }
    if (kind === 'local' && (!targetHost.trim() || !targetPort)) {
      error = 'Target host and port are required for local forward.'
      return
    }
    busy = true
    try {
      if (kind === 'local') {
        await addLocalForward(sessionId, bindAddr.trim(), bindPort, targetHost.trim(), targetPort!)
      } else {
        await addDynamicForward(sessionId, bindAddr.trim(), bindPort)
      }
      bindPort = null
      targetHost = ''
      targetPort = null
      await refresh()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      busy = false
    }
  }

  async function remove(id: string) {
    try {
      await removeForward(sessionId, id)
      await refresh()
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose()
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
  <div class="dialog">
    <h2>Port forwards</h2>

    {#if forwards.length === 0}
      <p class="muted">No active forwards.</p>
    {:else}
      <ul class="list">
        {#each forwards as f (f.id)}
          <li>
            <span class="kind" class:dyn={f.kind === 'dynamic'}>
              {f.kind === 'local' ? 'L' : 'D'}
            </span>
            <span class="bind">{f.bind_addr}:{f.bind_port}</span>
            <span class="arrow">→</span>
            {#if f.target_host}
              <span class="target">{f.target_host}:{f.target_port}</span>
            {:else}
              <span class="target socks">SOCKS5</span>
            {/if}
            <button class="rm" onclick={() => remove(f.id)} title="Remove forward">✕</button>
          </li>
        {/each}
      </ul>
    {/if}

    <form class="add" onsubmit={(e) => { e.preventDefault(); add() }}>
      <h3>Add forward</h3>

      <label>
        Type
        <select bind:value={kind}>
          <option value="local">Local (-L)</option>
          <option value="dynamic">Dynamic SOCKS5 (-D)</option>
        </select>
      </label>

      <div class="row">
        <label class="grow">
          Bind address
          <input bind:value={bindAddr} placeholder="127.0.0.1" />
        </label>
        <label class="port">
          Bind port
          <input type="number" bind:value={bindPort} min={1} max={65535} placeholder="8080" />
        </label>
      </div>

      {#if kind === 'local'}
        <div class="row">
          <label class="grow">
            Target host
            <input bind:value={targetHost} placeholder="db.internal" />
          </label>
          <label class="port">
            Target port
            <input type="number" bind:value={targetPort} min={1} max={65535} placeholder="5432" />
          </label>
        </div>
      {/if}

      {#if error}
        <p class="error">{error}</p>
      {/if}

      <div class="actions">
        <button type="button" class="cancel" onclick={onClose}>Close</button>
        <button type="submit" class="add-btn" disabled={busy}>
          {busy ? 'Adding…' : 'Add forward'}
        </button>
      </div>
    </form>
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
    z-index: 110;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1.5rem;
    width: 30rem;
    max-width: 95vw;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: #e4e4e7;
  }

  h3 {
    margin: 0 0 0.5rem;
    font-size: 0.8rem;
    font-weight: 600;
    color: #a1a1aa;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .muted {
    color: #71717a;
    font-size: 0.82rem;
    margin: 0;
  }

  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .list li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: #09090b;
    border: 1px solid #27272a;
    border-radius: 0.25rem;
    padding: 0.4rem 0.6rem;
    font-size: 0.82rem;
    color: #e4e4e7;
    font-family: monospace;
  }

  .kind {
    background: #3b82f6;
    color: #fff;
    font-weight: 700;
    border-radius: 0.2rem;
    padding: 0.05rem 0.35rem;
    font-size: 0.7rem;
  }

  .kind.dyn {
    background: #a855f7;
  }

  .bind, .target {
    color: #e4e4e7;
  }

  .target.socks {
    color: #a855f7;
  }

  .arrow {
    color: #52525b;
  }

  .rm {
    margin-left: auto;
    background: transparent;
    color: #52525b;
    border: none;
    cursor: pointer;
    font-size: 0.9rem;
    line-height: 1;
    padding: 0.15rem 0.4rem;
    border-radius: 0.2rem;
  }

  .rm:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }

  .add {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
    border-top: 1px solid #27272a;
    padding-top: 0.85rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.78rem;
    color: #a1a1aa;
  }

  input,
  select {
    background: #09090b;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #e4e4e7;
    font-size: 0.85rem;
    padding: 0.35rem 0.5rem;
    outline: none;
    font-family: inherit;
  }

  input:focus,
  select:focus {
    border-color: #3b82f6;
  }

  .row {
    display: flex;
    gap: 0.5rem;
  }

  .grow {
    flex: 1;
  }

  .port {
    width: 7rem;
    flex-shrink: 0;
  }

  .error {
    font-size: 0.8rem;
    color: #ef4444;
    margin: 0;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }

  button.cancel,
  button.add-btn {
    font-size: 0.82rem;
    font-family: inherit;
    padding: 0.35rem 0.9rem;
    border-radius: 0.25rem;
    border: none;
    cursor: pointer;
  }

  .cancel {
    background: #27272a;
    color: #a1a1aa;
  }

  .cancel:hover {
    background: #3f3f46;
  }

  .add-btn {
    background: #3b82f6;
    color: #fff;
  }

  .add-btn:hover:not(:disabled) {
    background: #2563eb;
  }

  .add-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
