<script lang="ts">
  import { onMount } from 'svelte'
  import TerminalTab from '$lib/terminal/TerminalTab.svelte'
  import SessionTree from '$lib/sessions/SessionTree.svelte'
  import NewSessionDialog from '$lib/sessions/NewSessionDialog.svelte'
  import QuickConnectDialog from '$lib/sessions/QuickConnectDialog.svelte'
  import type { ConnectParams } from '$lib/sessions/QuickConnectDialog.svelte'
  import HighlightRulesPanel from '$lib/sessions/HighlightRulesPanel.svelte'
  import TerminalSettingsPanel from '$lib/settings/TerminalSettingsPanel.svelte'
  import SettingsModal from '$lib/settings/SettingsModal.svelte'
  import { listFolders } from '$lib/bridge/commands'
  import { loadSettings } from '$lib/stores/settings.svelte'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  interface Tab {
    id: number
    label: string
    savedSession?: SavedSession
    ssh?: { host: string; port: number; user: string; password: string }
    telnet?: { host: string; port: number }
  }

  let nextId = 1
  const firstTab: Tab = { id: nextId++, label: 'shell' }
  let tabs = $state<Tab[]>([firstTab])
  let activeId = $state(firstTab.id)
  let showNewSession = $state(false)
  let showQuickConnect = $state(false)
  let showSettings = $state(false)
  let showHighlights = $state(false)
  let showAppearance = $state(false)
  let folders = $state<Folder[]>([])
  let sessionTreeKey = $state(0)

  // Load settings (themes, font prefs) on app start
  $effect(() => {
    loadSettings()
  })

  onMount(() => {
    function handleKeydown(e: KeyboardEvent) {
      if (!e.ctrlKey) return
      switch (e.key) {
        case 'n':
          e.preventDefault()
          showQuickConnect = true
          break
        case 't':
          e.preventDefault()
          addLocalTab()
          break
        case 'w':
          e.preventDefault()
          if (tabs.length > 1) closeTab(activeId)
          break
        case 'Tab':
          e.preventDefault()
          cycleTab(e.shiftKey ? -1 : 1)
          break
        case ',':
          e.preventDefault()
          showSettings = true
          break
      }
    }
    document.addEventListener('keydown', handleKeydown)
    return () => document.removeEventListener('keydown', handleKeydown)
  })

  // Called by TerminalTab when the terminal captures a global shortcut
  function handleGlobalShortcut(key: string, e: KeyboardEvent) {
    switch (key) {
      case 'n':
        showQuickConnect = true
        break
      case 't':
        addLocalTab()
        break
      case 'w':
        if (tabs.length > 1) closeTab(activeId)
        break
      case 'Tab':
        cycleTab(e.shiftKey ? -1 : 1)
        break
      case ',':
        showSettings = true
        break
    }
  }

  function cycleTab(dir: 1 | -1) {
    const idx = tabs.findIndex((t) => t.id === activeId)
    const next = tabs[(idx + dir + tabs.length) % tabs.length]
    if (next) activeId = next.id
  }

  function addLocalTab() {
    const tab: Tab = { id: nextId++, label: 'shell' }
    tabs = [...tabs, tab]
    activeId = tab.id
  }

  function handleQuickConnect(params: ConnectParams) {
    showQuickConnect = false
    let tab: Tab
    if (params.type === 'local') {
      tab = { id: nextId++, label: 'shell' }
    } else if (params.type === 'ssh') {
      tab = {
        id: nextId++,
        label: params.host,
        ssh: { host: params.host, port: params.port, user: params.user, password: params.password },
      }
    } else {
      tab = {
        id: nextId++,
        label: `telnet:${params.host}`,
        telnet: { host: params.host, port: params.port },
      }
    }
    tabs = [...tabs, tab]
    activeId = tab.id
  }

  async function loadFolders() {
    try {
      folders = await listFolders()
    } catch {
      // non-fatal; folders list just stays empty
    }
  }

  function openSavedSession(s: SavedSession) {
    const tab: Tab = { id: nextId++, label: s.name, savedSession: s }
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
      <button class="new-tab-btn" onclick={addLocalTab} title="New local tab (Ctrl+T)">+</button>
    </div>
    <span class="flex-1"></span>
    <button
      class="header-btn"
      onclick={() => (showQuickConnect = true)}
      title="Quick Connect (Ctrl+N)"
    >
      ⚡ Connect
    </button>
    <button
      class="header-btn"
      onclick={() => (showSettings = true)}
      title="Settings (Ctrl+,)"
    >
      ⚙ Settings
    </button>
  </header>

  <div class="body">
    <aside class="sidebar">
      {#key sessionTreeKey}
        <SessionTree onOpen={openSavedSession} onAdd={openNewSessionDialog} />
      {/key}
      <div class="sidebar-section">
        <button
          class="section-toggle"
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
      <div class="sidebar-section">
        <button
          class="section-toggle"
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
          <TerminalTab
            savedSession={tab.savedSession}
            ssh={tab.ssh}
            telnet={tab.telnet}
            onGlobalShortcut={handleGlobalShortcut}
          />
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

{#if showQuickConnect}
  <QuickConnectDialog onConnect={handleQuickConnect} onCancel={() => (showQuickConnect = false)} />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
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
    gap: 0.5rem;
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
    align-items: center;
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

  .new-tab-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #52525b;
    font-size: 1.1rem;
    line-height: 1;
    padding: 0.1rem 0.35rem;
    border-radius: 0.2rem;
    font-family: inherit;
  }
  .new-tab-btn:hover {
    color: #e4e4e7;
    background: #27272a;
  }

  .flex-1 {
    flex: 1;
  }

  .header-btn {
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    cursor: pointer;
    color: #a1a1aa;
    font-size: 0.72rem;
    padding: 0.15rem 0.6rem;
    font-family: inherit;
    white-space: nowrap;
  }
  .header-btn:hover {
    color: #e4e4e7;
    border-color: #71717a;
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

  .sidebar-section {
    border-top: 1px solid #27272a;
    flex-shrink: 0;
    overflow-y: auto;
    max-height: 50%;
  }

  .section-toggle {
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

  .section-toggle:hover {
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
