<script lang="ts">
  import { onMount } from 'svelte'
  import {
    listSnippets,
    createSnippet,
    updateSnippet,
    deleteSnippet,
    sendInputText,
  } from '$lib/bridge/commands'
  import type { Snippet } from '$lib/bridge/types'
  import { getFocusedSessionId, broadcast, otherSessionIds } from '$lib/stores/sessionRuntime.svelte'

  interface Props {
    onClose: () => void
  }

  let { onClose }: Props = $props()

  let snippets = $state<Snippet[]>([])
  let loading = $state(true)
  let status = $state<{ kind: 'ok' | 'err'; text: string } | null>(null)

  // Add form
  let showAdd = $state(false)
  let newName = $state('')
  let newContent = $state('')

  // Edit form
  let editingId = $state<number | null>(null)
  let editName = $state('')
  let editContent = $state('')

  onMount(refresh)

  async function refresh() {
    loading = true
    try {
      snippets = await listSnippets()
    } catch (e) {
      status = { kind: 'err', text: fmt(e) }
    } finally {
      loading = false
    }
  }

  async function add() {
    if (!newName.trim()) {
      status = { kind: 'err', text: 'Name is required.' }
      return
    }
    try {
      await createSnippet(newName.trim(), newContent)
      newName = ''
      newContent = ''
      showAdd = false
      status = { kind: 'ok', text: 'Snippet added.' }
      await refresh()
    } catch (e) {
      status = { kind: 'err', text: fmt(e) }
    }
  }

  function beginEdit(s: Snippet) {
    editingId = s.id
    editName = s.name
    editContent = s.content
  }

  async function commitEdit() {
    if (editingId == null) return
    if (!editName.trim()) {
      status = { kind: 'err', text: 'Name is required.' }
      return
    }
    try {
      await updateSnippet(editingId, editName.trim(), editContent)
      editingId = null
      status = { kind: 'ok', text: 'Snippet updated.' }
      await refresh()
    } catch (e) {
      status = { kind: 'err', text: fmt(e) }
    }
  }

  function cancelEdit() {
    editingId = null
  }

  async function remove(s: Snippet) {
    if (!confirm(`Delete snippet "${s.name}"?`)) return
    try {
      await deleteSnippet(s.id)
      status = { kind: 'ok', text: 'Deleted.' }
      await refresh()
    } catch (e) {
      status = { kind: 'err', text: fmt(e) }
    }
  }

  /// Send the snippet to the focused session (and to every other session if
  /// MultiExec broadcast is on).
  async function send(s: Snippet) {
    const focused = getFocusedSessionId()
    if (!focused) {
      status = { kind: 'err', text: 'No session is focused. Click a terminal first.' }
      return
    }
    try {
      await sendInputText(focused, s.content)
      if (broadcast.enabled) {
        for (const other of otherSessionIds(focused)) {
          await sendInputText(other, s.content).catch(() => {})
        }
        status = { kind: 'ok', text: `Sent to ${1 + otherSessionIds(focused).length} sessions.` }
      } else {
        status = { kind: 'ok', text: 'Sent.' }
      }
    } catch (e) {
      status = { kind: 'err', text: fmt(e) }
    }
  }

  function preview(content: string): string {
    const first = content.split(/\r?\n/)[0] ?? ''
    return first.length > 80 ? first.slice(0, 80) + '…' : first
  }

  function fmt(e: unknown): string {
    return e instanceof Error ? e.message : String(e)
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (editingId != null) cancelEdit()
      else if (showAdd) showAdd = false
      else onClose()
    }
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" tabindex="-1" onkeydown={onKeydown}>
  <div class="dialog">
    <div class="header">
      <h2>Snippets</h2>
      <div class="header-right">
        <button class="btn" onclick={() => (showAdd = !showAdd)}>+ New</button>
        <button class="btn" onclick={onClose} title="Close">✕</button>
      </div>
    </div>

    {#if broadcast.enabled}
      <p class="hint broadcast">
        ⚠ MultiExec broadcast is ON — sending will dispatch to <strong>all</strong> open sessions.
      </p>
    {:else}
      <p class="hint">Click ▶ to send the snippet to the focused terminal.</p>
    {/if}

    {#if showAdd}
      <form class="form" onsubmit={(e) => { e.preventDefault(); add() }}>
        <label>
          <span>Name</span>
          <input bind:value={newName} placeholder="e.g. show interface description" autofocus />
        </label>
        <label>
          <span>Content</span>
          <textarea
            bind:value={newContent}
            rows="4"
            placeholder="show interface description"
            spellcheck="false"
          ></textarea>
        </label>
        <div class="form-actions">
          <button type="button" class="btn" onclick={() => (showAdd = false)}>Cancel</button>
          <button type="submit" class="btn primary" disabled={!newName.trim()}>Save</button>
        </div>
      </form>
    {/if}

    {#if status}
      <p class="msg" class:err={status.kind === 'err'}>{status.text}</p>
    {/if}

    <div class="listing">
      {#if loading}
        <p class="muted center">Loading…</p>
      {:else if snippets.length === 0}
        <p class="muted center">No snippets yet.</p>
      {:else}
        <ul>
          {#each snippets as s (s.id)}
            <li>
              {#if editingId === s.id}
                <form class="form inline" onsubmit={(e) => { e.preventDefault(); commitEdit() }}>
                  <input bind:value={editName} />
                  <textarea bind:value={editContent} rows="3" spellcheck="false"></textarea>
                  <div class="form-actions">
                    <button type="button" class="btn" onclick={cancelEdit}>Cancel</button>
                    <button type="submit" class="btn primary" disabled={!editName.trim()}>Save</button>
                  </div>
                </form>
              {:else}
                <button class="send" onclick={() => send(s)} title="Send to focused session">▶</button>
                <div class="info">
                  <span class="name">{s.name}</span>
                  <span class="preview">{preview(s.content)}</span>
                </div>
                <button class="btn icon" onclick={() => beginEdit(s)} title="Edit">✎</button>
                <button class="btn icon danger" onclick={() => remove(s)} title="Delete">✕</button>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
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
    z-index: 115;
  }

  .dialog {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1rem 1.2rem 1.2rem;
    width: 36rem;
    max-width: 95vw;
    height: 32rem;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    gap: 0.7rem;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .header-right {
    display: flex;
    gap: 0.35rem;
  }

  h2 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
    color: #e4e4e7;
  }

  .hint {
    margin: 0;
    font-size: 0.78rem;
    color: #71717a;
  }

  .hint.broadcast {
    color: #fbbf24;
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: #0a0a0b;
    border: 1px solid #27272a;
    border-radius: 0.3rem;
    padding: 0.7rem 0.9rem;
  }

  .form label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.78rem;
    color: #a1a1aa;
  }

  input,
  textarea {
    background: #09090b;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #e4e4e7;
    font-size: 0.85rem;
    padding: 0.35rem 0.55rem;
    outline: none;
    font-family: inherit;
  }

  textarea {
    font-family: monospace;
    resize: vertical;
  }

  input:focus,
  textarea:focus {
    border-color: #3b82f6;
  }

  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.35rem 0.7rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: 1px solid #3f3f46;
    background: #27272a;
    color: #e4e4e7;
    cursor: pointer;
    font-family: inherit;
  }

  .btn:hover:not(:disabled) {
    background: #3f3f46;
  }

  .btn.primary {
    background: #3b82f6;
    border-color: #3b82f6;
    color: #fff;
  }

  .btn.primary:hover:not(:disabled) {
    background: #2563eb;
  }

  .btn.primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn.icon {
    padding: 0.2rem 0.45rem;
  }

  .btn.danger {
    color: #f87171;
  }

  .btn.danger:hover {
    background: rgba(239, 68, 68, 0.15);
  }

  .msg {
    margin: 0;
    font-size: 0.8rem;
    color: #34d399;
  }

  .msg.err {
    color: #f87171;
  }

  .listing {
    flex: 1;
    overflow: auto;
    background: #09090b;
    border: 1px solid #27272a;
    border-radius: 0.3rem;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.55rem 0.7rem;
    border-bottom: 1px solid #18181b;
  }

  li:last-child {
    border-bottom: none;
  }

  .send {
    background: #16a34a;
    color: #fff;
    border: none;
    width: 1.75rem;
    height: 1.75rem;
    border-radius: 0.25rem;
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
  }

  .send:hover {
    background: #15803d;
  }

  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    overflow: hidden;
  }

  .name {
    font-size: 0.85rem;
    font-weight: 500;
    color: #e4e4e7;
  }

  .preview {
    font-size: 0.74rem;
    color: #71717a;
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .form.inline {
    flex: 1;
    padding: 0.5rem;
  }

  .muted {
    color: #71717a;
    font-size: 0.82rem;
  }

  .center {
    text-align: center;
    padding: 1.5rem 0;
  }
</style>
