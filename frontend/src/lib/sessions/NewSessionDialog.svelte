<script lang="ts">
  import { createSavedSession } from '$lib/bridge/commands'
  import type { Folder } from '$lib/bridge/types'

  interface Props {
    folders: Folder[]
    onCreated: (id: number) => void
    onCancel: () => void
  }

  const { folders, onCreated, onCancel }: Props = $props()

  let name = $state('')
  let host = $state('')
  let port = $state(22)
  let username = $state('')
  let password = $state('')
  let folderId = $state<number | null>(null)
  let error = $state('')
  let saving = $state(false)

  async function submit() {
    if (!name.trim() || !host.trim() || !username.trim()) {
      error = 'Name, host, and username are required.'
      return
    }
    saving = true
    error = ''
    try {
      const id = await createSavedSession(
        name.trim(),
        folderId,
        host.trim(),
        port,
        username.trim(),
        password,
      )
      onCreated(id)
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      saving = false
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel()
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label="New session"
  tabindex="-1"
  onkeydown={onkeydown}
>
  <form class="dialog" onsubmit={(e) => { e.preventDefault(); submit() }}>
    <h2>New SSH Session</h2>

    <label>
      Name
      <input bind:value={name} placeholder="My server" required />
    </label>

    <label>
      Host
      <input bind:value={host} placeholder="192.168.1.1" required />
    </label>

    <div class="row">
      <label class="grow">
        Username
        <input bind:value={username} placeholder="admin" required />
      </label>
      <label class="port">
        Port
        <input type="number" bind:value={port} min={1} max={65535} />
      </label>
    </div>

    <label>
      Password
      <input type="password" bind:value={password} placeholder="(optional if using key auth)" />
    </label>

    {#if folders.length > 0}
      <label>
        Folder
        <select bind:value={folderId}>
          <option value={null}>— None —</option>
          {#each folders as f (f.id)}
            <option value={f.id}>{f.name}</option>
          {/each}
        </select>
      </label>
    {/if}

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <div class="actions">
      <button type="button" class="cancel" onclick={onCancel}>Cancel</button>
      <button type="submit" class="save" disabled={saving}>
        {saving ? 'Saving…' : 'Save'}
      </button>
    </div>
  </form>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #27272a;
    border-radius: 0.5rem;
    padding: 1.5rem;
    width: 26rem;
    max-width: 95vw;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  h2 {
    margin: 0 0 0.25rem;
    font-size: 1rem;
    font-weight: 600;
    color: #e4e4e7;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
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
    border-color: #6366f1;
  }

  .row {
    display: flex;
    gap: 0.5rem;
    align-items: flex-end;
  }

  .grow { flex: 1; }

  .port { width: 5rem; flex-shrink: 0; }

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

  button {
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

  .cancel:hover { background: #3f3f46; }

  .save {
    background: #6366f1;
    color: #fff;
  }

  .save:hover:not(:disabled) { background: #4f46e5; }
  .save:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
