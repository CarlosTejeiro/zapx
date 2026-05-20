<script lang="ts">
  import TerminalTab from '$lib/terminal/TerminalTab.svelte'
  import SessionTree from '$lib/sessions/SessionTree.svelte'
  import NewSessionDialog from '$lib/sessions/NewSessionDialog.svelte'
  import HighlightRulesPanel from '$lib/sessions/HighlightRulesPanel.svelte'
  import TerminalSettingsPanel from '$lib/settings/TerminalSettingsPanel.svelte'
  import { listFolders } from '$lib/bridge/commands'
  import { loadSettings } from '$lib/stores/settings.svelte'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  interface Tab {
    id: number
    label: string
    /** UUID of the live terminal session (set after connection is established) */
    liveSessionId?: string
    /** Saved session to open on mount */
    savedSession?: SavedSession
    /** Inline SSH params for ad-hoc connections */
    ssh?: { host: string; port: number; user: string; password: string }
  }

  let nextId = 1
  const firstTab: Tab = { id: nextId++, label: 'shell' }
  let tabs = $state<Tab[]>([firstTab])
  let activeId = $state(firstTab.id)
  let showNewSession = $state(false)
  let showHighlights = $state(false)
  let showAppearance = $state(false)
  let folders = $state<Folder[]>([])
  let sessionTreeKey = $state(0)

  // Load settings (themes, font prefs) on app start
  $effect(() => { loadSettings() })

  async function loadFolders() {
    try {
      folders = await listFolders()
    } catch {
      // non-fatal; folders list just stays empty
    }
  }

  function openSavedSession(s: SavedSession) {
    const tab: Tab = {
      id: nextId++,
      label: s.name,
      savedSession: s,
    }
    tabs = [...tabs, tab]
    activeId = tab.id
  }

  function handleSessionCreated() {
    showNewSession = false
    sessionTreeKey += 1
  }

  function openNewSessionDialog() {
    loadFolders()
    showNewSession = true
  }

  function closeTab(id: number) {
    tabs = tabs.filter((t) => t.id !== id)
    if (activeId === id) activeId = tabs.at(-1)?.id ?? 0
  }
</script>

<div class="layout">
  <header class="tab-bar">
    <span class="app-name">zapx</span>
    <div class="tabs">
      {#each tabs as tab (tab.id)}
        <button class="tab" class:active={activeId === tab.id} onclick={() => (activeId = tab.id)}>
          {tab.label}
          {#if tabs.length > 1}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <span
              class="close-btn"
              role="button"
              tabindex="-1"
              onclick={(e) => {
                e.stopPropagation()
                closeTab(tab.id)
              }}
            >×</span>
          {/if}
        </button>
      {/each}
    </div>
  </header>

  <div class="body">
    <aside class="sidebar">
      {#key sessionTreeKey}
        <SessionTree onOpen={openSavedSession} onAdd={openNewSessionDialog} />
      {/key}
      <div class="highlight-section">
        <button
          class="highlight-toggle"
          onclick={() => (showHighlights = !showHighlights)}
          aria-expanded={showHighlights}
        >
          <span>Highlight Rules</span>
          <span class="chevron">{showHighlights ? '▲' : '▼'}</span>
        </button>
        {#if showHighlights}
          <HighlightRulesPanel />
        {/if}
      </div>
      <div class="highlight-section">
        <button
          class="highlight-toggle"
          onclick={() => (showAppearance = !showAppearance)}
          aria-expanded={showAppearance}
        >
          <span>Appearance</span>
          <span class="chevron">{showAppearance ? '▲' : '▼'}</span>
        </button>
        {#if showAppearance}
          <TerminalSettingsPanel />
        {/if}
      </div>
    </aside>

    <main class="terminal-area">
      {#each tabs as tab (tab.id)}
        <div class="tab-content" class:hidden={activeId !== tab.id}>
          <TerminalTab savedSession={tab.savedSession} ssh={tab.ssh} />
        </div>
      {/each}
    </main>
  </div>
</div>

{#if showNewSession}
  <NewSessionDialog
    {folders}
    onCreated={() => handleSessionCreated()}
    onCancel={() => (showNewSession = false)}
  />
{/if}

<style>
  .layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #09090b;
    color: #e4e4e7;
  }

  .tab-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: 2.25rem;
    padding: 0 0.75rem;
    background: #18181b;
    border-bottom: 1px solid #27272a;
    flex-shrink: 0;
    user-select: none;
  }

  .app-name {
    font-size: 0.75rem;
    font-weight: 600;
    color: #71717a;
    letter-spacing: 0.05em;
  }

  .tabs {
    display: flex;
    gap: 0.25rem;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.75rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: none;
    cursor: pointer;
    background: transparent;
    color: #71717a;
    transition: background 0.1s;
    font-family: inherit;
  }

  .tab:hover {
    background: #27272a;
    color: #e4e4e7;
  }

  .tab.active {
    background: #27272a;
    color: #e4e4e7;
  }

  .close-btn {
    font-size: 0.9rem;
    line-height: 1;
    color: #52525b;
    padding: 0 0.1rem;
  }

  .close-btn:hover {
    color: #ef4444;
  }

  .body {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-right: 1px solid #27272a;
    min-width: 0;
  }

  .highlight-section {
    border-top: 1px solid #27272a;
    flex-shrink: 0;
    overflow-y: auto;
    max-height: 50%;
  }

  .highlight-toggle {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem 0.75rem;
    background: #18181b;
    border: none;
    cursor: pointer;
    color: #a1a1aa;
    font-size: 0.75rem;
    font-family: inherit;
  }

  .highlight-toggle:hover {
    color: #e4e4e7;
    background: #27272a;
  }

  .chevron {
    font-size: 0.6rem;
  }

  .terminal-area {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .tab-content.hidden {
    display: none;
  }
</style>
