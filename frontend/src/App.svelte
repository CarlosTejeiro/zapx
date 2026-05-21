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
</style>
