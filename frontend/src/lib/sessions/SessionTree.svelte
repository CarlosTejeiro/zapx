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
  const sessionsByFolder = $derived((id: number) => sessions.filter((s) => s.folder_id === id))

  function protocolColor(protocol: string): string {
    if (protocol === 'ssh') return '#22c55e'
    if (protocol === 'telnet') return '#f59e0b'
    if (protocol === 'serial') return '#a78bfa'
    return '#3b82f6'
  }

  $effect(() => { load() })
</script>

<div class="tree-root">
  {#if loading}
    <p class="hint">Loading…</p>
  {:else if error}
    <p class="hint err">{error}</p>
  {:else}
    <ul class="tree">
      {#each rootSessions as s (s.id)}
        <li>
          <button class="session-row" onclick={() => onOpen(s)}>
            <span class="proto-dot" style:background={protocolColor(s.protocol)}></span>
            <span class="session-name">{s.name}</span>
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

      {#each folders as folder (folder.id)}
        <li>
          <button class="folder-row" onclick={() => toggleFolder(folder.id)}>
            <span class="folder-chevron">{expandedFolders.has(folder.id) ? '▾' : '▸'}</span>
            <span class="folder-icon">📁</span>
            <span class="session-name">{folder.name}</span>
          </button>

          {#if expandedFolders.has(folder.id)}
            <ul class="subtree">
              {#each sessionsByFolder(folder.id) as s (s.id)}
                <li>
                  <button class="session-row session-indented" onclick={() => onOpen(s)}>
                    <span class="proto-dot" style:background={protocolColor(s.protocol)}></span>
                    <span class="session-name">{s.name}</span>
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
        <li>
          <p class="hint">No sessions.<br />
            <button class="hint-add" onclick={onAdd}>+ Add session</button>
          </p>
        </li>
      {/if}
    </ul>
  {/if}
</div>

<style>
  .tree-root {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .tree,
  .subtree {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .tree {
    padding: 0.2rem 0;
  }

  .subtree {
    padding-left: 0.5rem;
  }

  .session-row,
  .folder-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.22rem 0.5rem;
    font-size: 0.75rem;
    font-family: inherit;
    background: transparent;
    border: none;
    color: #a1a1aa;
    cursor: pointer;
    text-align: left;
    border-left: 2px solid transparent;
  }

  .session-row:hover,
  .folder-row:hover {
    background: #1e1e1e;
    color: #e4e4e7;
    border-left-color: #3b82f6;
  }

  .session-indented {
    padding-left: 0.75rem;
  }

  .proto-dot {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .folder-chevron {
    font-size: 0.6rem;
    color: #52525b;
    flex-shrink: 0;
  }

  .folder-icon {
    font-size: 0.7rem;
    flex-shrink: 0;
  }

  .session-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .del-btn {
    color: #52525b;
    font-size: 0.85rem;
    cursor: pointer;
    padding: 0 0.1rem;
    opacity: 0;
    flex-shrink: 0;
    background: none;
    border: none;
    line-height: 1;
    font-family: inherit;
  }

  .session-row:hover .del-btn {
    opacity: 1;
  }

  .del-btn:hover {
    color: #ef4444;
  }

  .hint {
    font-size: 0.72rem;
    color: #52525b;
    padding: 0.6rem 0.6rem;
    line-height: 1.5;
    margin: 0;
  }

  .hint-add {
    background: none;
    border: none;
    color: #4a9eff;
    cursor: pointer;
    font-size: 0.72rem;
    padding: 0;
    font-family: inherit;
  }

  .hint-add:hover {
    text-decoration: underline;
  }

  .err {
    color: #ef4444;
  }

  .empty {
    font-size: 0.7rem;
    color: #3f3f46;
    padding: 0.15rem 0.75rem;
    display: block;
    font-style: italic;
  }
</style>
