<script lang="ts">
  import { listSessions, listFolders, deleteSavedSession } from '$lib/bridge/commands'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  interface Props {
    onOpen: (session: SavedSession) => void
    onAdd: () => void
  }

  const { onOpen, onAdd }: Props = $props()

  let sessions = $state<SavedSession[]>([])
  let folders = $state<Folder[]>([])
  let loading = $state(true)
  let error = $state('')
  let expandedFolders = $state<Set<number>>(new Set())

  async function load() {
    loading = true
    error = ''
    try {
      ;[sessions, folders] = await Promise.all([listSessions(), listFolders()])
      expandedFolders = new Set(folders.map((f) => f.id))
    } catch (e) {
      error = e instanceof Error ? e.message : String(e)
    } finally {
      loading = false
    }
  }

  async function remove(e: MouseEvent, session: SavedSession) {
    e.stopPropagation()
    if (!confirm(`Delete "${session.name}"?`)) return
    try {
      await deleteSavedSession(session.id)
      sessions = sessions.filter((s) => s.id !== session.id)
    } catch (err) {
      alert(err instanceof Error ? err.message : String(err))
    }
  }

  function toggleFolder(id: number) {
    const next = new Set(expandedFolders)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    expandedFolders = next
  }

  const rootSessions = $derived(sessions.filter((s) => s.folder_id === null))
  const sessionsByFolder = $derived(
    (id: number) => sessions.filter((s) => s.folder_id === id),
  )

  $effect(() => { load() })
</script>

<aside class="sidebar">
  <header class="header">
    <span class="title">Sessions</span>
    <button class="add-btn" onclick={onAdd} title="New session">+</button>
  </header>

  {#if loading}
    <p class="hint">Loading…</p>
  {:else if error}
    <p class="hint error">{error}</p>
  {:else}
    <ul class="tree">
      <!-- Root sessions (no folder) -->
      {#each rootSessions as s (s.id)}
        <li>
          <button class="session-row" ondblclick={() => onOpen(s)}>
            <span class="icon">⌨</span>
            <span class="label">{s.name}</span>
            <!-- svelte-ignore a11y_interactive_supports_focus -->
            <span
              class="del-btn"
              role="button"
              title="Delete"
              onclick={(e) => remove(e, s)}
              onkeydown={(e) => e.key === 'Enter' && remove(e as unknown as MouseEvent, s)}
            >×</span>
          </button>
        </li>
      {/each}

      <!-- Folders -->
      {#each folders as folder (folder.id)}
        <li class="folder-item">
          <button
            class="folder-row"
            onclick={() => toggleFolder(folder.id)}
          >
            <span class="chevron">{expandedFolders.has(folder.id) ? '▾' : '▸'}</span>
            <span class="label">{folder.name}</span>
          </button>

          {#if expandedFolders.has(folder.id)}
            <ul class="subtree">
              {#each sessionsByFolder(folder.id) as s (s.id)}
                <li>
                  <button class="session-row" ondblclick={() => onOpen(s)}>
                    <span class="icon">⌨</span>
                    <span class="label">{s.name}</span>
                    <!-- svelte-ignore a11y_interactive_supports_focus -->
                    <span
                      class="del-btn"
                      role="button"
                      title="Delete"
                      onclick={(e) => remove(e, s)}
                      onkeydown={(e) => e.key === 'Enter' && remove(e as unknown as MouseEvent, s)}
                    >×</span>
                  </button>
                </li>
              {/each}
              {#if sessionsByFolder(folder.id).length === 0}
                <li><span class="empty">empty</span></li>
              {/if}
            </ul>
          {/if}
        </li>
      {/each}

      {#if sessions.length === 0}
        <li><p class="hint">No sessions yet.<br />Click + to add one.</p></li>
      {/if}
    </ul>
  {/if}
</aside>

<style>
  .sidebar {
    width: 13rem;
    min-width: 10rem;
    max-width: 18rem;
    background: #18181b;
    border-right: 1px solid #27272a;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex-shrink: 0;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.65rem;
    border-bottom: 1px solid #27272a;
    flex-shrink: 0;
  }

  .title {
    font-size: 0.72rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #52525b;
  }

  .add-btn {
    background: transparent;
    border: none;
    color: #71717a;
    font-size: 1.1rem;
    cursor: pointer;
    line-height: 1;
    padding: 0 0.2rem;
  }

  .add-btn:hover { color: #e4e4e7; }

  .tree,
  .subtree {
    list-style: none;
    margin: 0;
    padding: 0;
    overflow-y: auto;
  }

  .tree { flex: 1; padding: 0.25rem 0; }

  .subtree { padding-left: 0.75rem; }

  .session-row,
  .folder-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.8rem;
    font-family: inherit;
    background: transparent;
    border: none;
    color: #a1a1aa;
    cursor: pointer;
    text-align: left;
  }

  .session-row:hover,
  .folder-row:hover { background: #27272a; color: #e4e4e7; }

  .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .icon { font-size: 0.75rem; flex-shrink: 0; }

  .chevron { font-size: 0.65rem; flex-shrink: 0; color: #52525b; }

  .del-btn {
    background: transparent;
    border: none;
    color: #52525b;
    font-size: 0.9rem;
    cursor: pointer;
    line-height: 1;
    padding: 0 0.15rem;
    opacity: 0;
  }

  .session-row:hover .del-btn { opacity: 1; }
  .del-btn:hover { color: #ef4444; }

  .hint {
    font-size: 0.78rem;
    color: #52525b;
    padding: 0.75rem 0.75rem;
    line-height: 1.5;
    margin: 0;
  }

  .error { color: #ef4444; }

  .empty {
    font-size: 0.75rem;
    color: #3f3f46;
    padding: 0.15rem 0.5rem;
    display: block;
    font-style: italic;
  }
</style>
