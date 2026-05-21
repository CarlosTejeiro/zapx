<script lang="ts">
  import { onMount } from 'svelte'
  import { getTheme, setTheme } from '$lib/themes/store.svelte'
  import TitleBar from '$lib/pylon/TitleBar.svelte'
  import Sidebar from '$lib/pylon/Sidebar.svelte'
  import TabBar from '$lib/pylon/TabBar.svelte'
  import type { TabEntry } from '$lib/pylon/TabBar.svelte'
  import Pane from '$lib/pylon/Pane.svelte'
  import type { PaneData } from '$lib/pylon/Pane.svelte'
  import StatusBar from '$lib/pylon/StatusBar.svelte'
  import NewSessionDialog from '$lib/sessions/NewSessionDialog.svelte'
  import SettingsModal from '$lib/settings/SettingsModal.svelte'
  import { listSessions, listFolders } from '$lib/bridge/commands'
  import { loadSettings } from '$lib/stores/settings.svelte'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  // ── types ───────────────────────────────────────────────────────────────────

  type PaneStatus = 'connecting' | 'connected' | 'error' | 'closed'

  interface AppTab {
    id: number
    pane: PaneData
    status: PaneStatus
  }

  // ── theme ────────────────────────────────────────────────────────────────────

  const theme = $derived(getTheme())

  function toggleTheme() {
    setTheme(theme.name === 'graphite' ? 'neonNoir' : 'graphite')
  }

  // ── state ────────────────────────────────────────────────────────────────────

  let nextId = 1

  const SESSION_COLORS = [
    '#22d3ee','#f472b6','#a78bfa','#f59e0b',
    '#ef4444','#10b981','#f97316','#84cc16','#c89b6b','#5eb3b2',
  ]

  function pickColor(id: number): string {
    return SESSION_COLORS[id % SESSION_COLORS.length] as string
  }

  function mkPane(label: string, opts?: Partial<Omit<PaneData, 'id' | 'label' | 'color'>>): PaneData {
    const id = nextId++
    return { id, label, color: pickColor(id), ...opts }
  }

  function mkTab(pane: PaneData): AppTab {
    return { id: nextId++, pane, status: 'connecting' }
  }

  const firstPane = mkPane('shell')
  const firstTab = mkTab(firstPane)

  let tabs = $state<AppTab[]>([firstTab])
  let activeTabId = $state(firstTab.id)
  let focusedPaneId = $state(firstPane.id)
  let sessions = $state<SavedSession[]>([])
  let folders = $state<Folder[]>([])
  let showNewSession = $state(false)
  let showSettings = $state(false)
  let showAbout = $state(false)
  let splitOn = $state(false)
  let multiOn = $state(false)

  // ── derived ──────────────────────────────────────────────────────────────────

  const activeTab = $derived(tabs.find((t) => t.id === activeTabId))

  const tabEntries = $derived<TabEntry[]>(
    tabs.map((t) => ({
      id: t.id,
      label: t.pane.label,
      color: t.pane.color,
      status: t.status,
    }))
  )

  const activePaneData = $derived(activeTab?.pane)

  const statusHost = $derived(
    activePaneData?.ssh?.host ??
    activePaneData?.savedSession?.host ??
    ''
  )

  const statusPort = $derived(
    activePaneData?.ssh?.port ??
    activePaneData?.savedSession?.port ??
    undefined
  )

  const statusProtocol = $derived(
    activePaneData?.savedSession?.protocol?.toUpperCase() ??
    (activePaneData?.ssh ? 'SSH' : activePaneData?.telnet ? 'TELNET' : 'LOCAL')
  )

  const activeSessionName = $derived(
    activeTab?.pane.label ?? ''
  )

  // ── data loading ─────────────────────────────────────────────────────────────

  async function load() {
    try {
      ;[sessions, folders] = await Promise.all([listSessions(), listFolders()])
    } catch (_) {
      // non-fatal — user can retry via sidebar
    }
  }

  $effect(() => {
    load()
    loadSettings()
  })

  // ── tab management ────────────────────────────────────────────────────────────

  function openSavedSessionTab(s: SavedSession) {
    const pane = mkPane(s.name, { savedSession: s, color: pickColor(s.id) } as Partial<PaneData>)
    pane.color = pickColor(s.id)
    const tab = mkTab(pane)
    tabs = [...tabs, tab]
    activeTabId = tab.id
    focusedPaneId = pane.id
  }

  function addLocalTab() {
    const pane = mkPane('shell')
    const tab = mkTab(pane)
    tabs = [...tabs, tab]
    activeTabId = tab.id
    focusedPaneId = pane.id
  }

  function closeTab(id: number) {
    if (tabs.length <= 1) return
    const idx = tabs.findIndex((t) => t.id === id)
    tabs = tabs.filter((t) => t.id !== id)
    if (activeTabId === id) {
      activeTabId = (tabs[idx] ?? tabs[idx - 1])?.id ?? tabs[0]!.id
    }
  }

  function activateTab(id: number) {
    activeTabId = id
    const tab = tabs.find((t) => t.id === id)
    if (tab) focusedPaneId = tab.pane.id
  }

  function handleStatusChange(tabId: number, status: PaneStatus) {
    const tab = tabs.find((t) => t.id === tabId)
    if (!tab) return
    tab.status = status
    tabs = tabs // trigger reactivity
  }

  function handleSessionAdded() {
    showNewSession = false
    load()
  }

  // ── keyboard shortcuts ────────────────────────────────────────────────────────

  function handleGlobalShortcut(key: string, e: KeyboardEvent) {
    switch (key) {
      case 'n': showNewSession = true; break
      case 't': addLocalTab(); break
      case 'w': closeTab(activeTabId); break
      case 'Tab': {
        const dir = e.shiftKey ? -1 : 1
        const idx = tabs.findIndex((t) => t.id === activeTabId)
        const next = tabs[(idx + dir + tabs.length) % tabs.length]
        if (next) activateTab(next.id)
        break
      }
      case ',': showSettings = true; break
      case '\\': splitOn = !splitOn; break
    }
  }

  onMount(() => {
    function onKeydown(e: KeyboardEvent) {
      if (!e.ctrlKey) return
      switch (e.key) {
        case 'n':    e.preventDefault(); showNewSession = true;        break
        case 't':    e.preventDefault(); addLocalTab();                break
        case 'w':    e.preventDefault(); closeTab(activeTabId);        break
        case 'Tab':  e.preventDefault(); handleGlobalShortcut('Tab', e); break
        case ',':    e.preventDefault(); showSettings = true;          break
        case '\\':   e.preventDefault(); splitOn = !splitOn;           break
      }
    }
    document.addEventListener('keydown', onKeydown)
    return () => document.removeEventListener('keydown', onKeydown)
  })
</script>

<!-- ── PYLON shell ──────────────────────────────────────────────────────────── -->
<div class="pylon-shell" style:background={theme.appBg}>

  <TitleBar
    {theme}
    sessionName={activeSessionName}
    themeName={theme.name}
    onNewSession={() => showNewSession = true}
    onSettings={() => showSettings = true}
    onToggleTheme={toggleTheme}
    onAbout={() => showAbout = true}
  />

  <div class="pylon-body">

    <Sidebar
      {theme}
      {sessions}
      {folders}
      activeSessionId={activePaneData?.savedSession?.id}
      onSelect={openSavedSessionTab}
      onAddSession={() => showNewSession = true}
      onSettings={() => showSettings = true}
      onToggleTheme={toggleTheme}
    />

    <div class="pylon-workspace" style:background={theme.bodyBg}>

      <TabBar
        {theme}
        tabs={tabEntries}
        activeTabId={activeTabId}
        {splitOn}
        {multiOn}
        onActivate={activateTab}
        onAdd={addLocalTab}
        onClose={closeTab}
        onToggleSplit={() => splitOn = !splitOn}
        onToggleMulti={() => multiOn = !multiOn}
      />

      <div class="pylon-panes">
        {#each tabs as tab (tab.id)}
          <div class="pane-slot" class:hidden={tab.id !== activeTabId}>
            {#if tab.id === activeTabId}
              <Pane
                {theme}
                pane={tab.pane}
                focused={focusedPaneId === tab.pane.id}
                onFocus={() => focusedPaneId = tab.pane.id}
                onStatusChange={(s) => handleStatusChange(tab.id, s)}
                onGlobalShortcut={handleGlobalShortcut}
              />
            {/if}
          </div>
        {/each}
      </div>

      <StatusBar
        {theme}
        status={activeTab?.status ?? 'connecting'}
        host={statusHost || undefined}
        port={statusPort ?? undefined}
        protocol={statusProtocol}
      />

    </div>
  </div>

</div>

<!-- ── Dialogs ──────────────────────────────────────────────────────────────── -->
{#if showNewSession}
  <NewSessionDialog
    {folders}
    onCancel={() => showNewSession = false}
    onCreated={(_id) => handleSessionAdded()}
  />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => showSettings = false} />
{/if}

{#if showAbout}
  <!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
  <div class="about-backdrop" onclick={() => showAbout = false} tabindex="-1">
    <div
      class="about-modal"
      style:background={theme.sidebarBg}
      style:border="1px solid {theme.border}"
      style:font-family={theme.fontUi}
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="about-header" style:border-bottom="1px solid {theme.border}">
        <svg width="20" height="20" viewBox="0 0 16 16" fill="none">
          <rect x="2" y="2" width="3" height="12" rx="1" fill={theme.accent}/>
          <rect x="5" y="2" width="6" height="3" rx="1" fill={theme.accent}/>
          <rect x="5" y="7" width="5" height="3" rx="1" fill={theme.accent} opacity="0.7"/>
        </svg>
        <span style:color={theme.textPrimary} style:font-size="15px" style:font-weight="600">zapx</span>
        <button
          class="about-close"
          style:color={theme.textDim}
          onclick={() => showAbout = false}
        >✕</button>
      </div>
      <div class="about-body">
        <p style:color={theme.textMuted} style:font-size="12.5px" style:line-height="1.7">
          A modern multiprotocol terminal for network engineers.<br/>
          Built with Tauri · Svelte 5 · xterm.js · Rust.
        </p>
        <table class="about-table" style:color={theme.textMuted}>
          <tbody>
            <tr><td style:color={theme.textDim}>Version</td><td style:color={theme.textPrimary}>0.1.0</td></tr>
            <tr><td style:color={theme.textDim}>Runtime</td><td>Tauri 2 · WebView</td></tr>
            <tr><td style:color={theme.textDim}>Theme</td><td style:color={theme.accent}>{theme.name === 'neon-noir' ? 'Neon Noir' : 'Graphite'}</td></tr>
          </tbody>
        </table>
        <p class="about-hint" style:color={theme.textDim} style:font-family={theme.fontMono}>
          Credentials stored securely via system keyring
        </p>
      </div>
    </div>
  </div>
{/if}

<style>
  .pylon-shell {
    width: 100vw;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .pylon-body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .pylon-workspace {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  .pylon-panes {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
    position: relative;
  }

  .pane-slot {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }

  .pane-slot.hidden {
    display: none;
  }

  .about-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.55);
    backdrop-filter: blur(4px);
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .about-modal {
    width: 360px;
    border-radius: 8px;
    box-shadow: 0 24px 64px rgba(0,0,0,0.6);
    overflow: hidden;
  }

  .about-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
  }

  .about-close {
    margin-left: auto;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 13px;
    padding: 2px 6px;
    border-radius: 4px;
    color: inherit;
    transition: background 0.1s;
  }

  .about-close:hover { background: rgba(255,255,255,0.08); }

  .about-body {
    padding: 0 18px 18px;
  }

  .about-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
    margin: 10px 0;
  }

  .about-table td {
    padding: 4px 0;
  }

  .about-table td:first-child {
    width: 80px;
    font-size: 11px;
  }

  .about-hint {
    font-size: 10.5px;
    margin: 10px 0 0;
    opacity: 0.6;
  }
</style>
