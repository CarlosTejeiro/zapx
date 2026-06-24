<script lang="ts">
  import { onMount } from 'svelte'
  import { getTheme, setTheme } from '$lib/themes/store.svelte'
  import TitleBar from '$lib/pylon/TitleBar.svelte'
  import Sidebar from '$lib/pylon/Sidebar.svelte'
  import TabBar from '$lib/pylon/TabBar.svelte'
  import type { TabEntry } from '$lib/pylon/TabBar.svelte'
  import Pane from '$lib/pylon/Pane.svelte'
  import type { PaneData } from '$lib/pylon/Pane.svelte'
  import SplitTree from '$lib/pylon/SplitTree.svelte'
  import {
    type PaneTreeNode,
    type SplitDirection,
    eachLeaf,
    firstLeafId,
    leaf as leafNode,
    leafCount,
    removeLeaf,
    setRatio,
    splitLeaf,
  } from '$lib/pylon/paneTree'
  import { SvelteMap } from 'svelte/reactivity'
  import StatusBar from '$lib/pylon/StatusBar.svelte'
  import NewSessionDialog from '$lib/sessions/NewSessionDialog.svelte'
  import QuickConnectDialog from '$lib/sessions/QuickConnectDialog.svelte'
  import type { ConnectParams } from '$lib/sessions/QuickConnectDialog.svelte'
  import ResizeHandles from '$lib/pylon/ResizeHandles.svelte'
  import SettingsModal from '$lib/settings/SettingsModal.svelte'
  import KeyboardInteractiveDialog from '$lib/terminal/KeyboardInteractiveDialog.svelte'
  import SnippetsDialog from '$lib/sessions/SnippetsDialog.svelte'
  import CommandPalette from '$lib/palette/CommandPalette.svelte'
  import type { PaletteItem } from '$lib/palette/CommandPalette.svelte'
  import Toasts from '$lib/ui/Toasts.svelte'
  import { showToast } from '$lib/ui/toast-store.svelte'
  import PromptDialog from '$lib/ui/PromptDialog.svelte'
  import { ask } from '@tauri-apps/plugin-dialog'
  import { loadHintsSettings } from '$lib/hints/store.svelte'
  import { broadcast, sessionRuntime, paneToSession } from '$lib/stores/sessionRuntime.svelte'
  import {
    loadBindings,
    matchAction,
    type ShortcutAction,
  } from '$lib/stores/keybindings.svelte'
  import {
    listSessions,
    listFolders,
    moveSavedSession,
    deleteSavedSession,
    createFolder,
    renameFolder,
    deleteFolder,
    listSnippets as listSnippetsApi,
    sendInputText,
    openLogsDir,
  } from '$lib/bridge/commands'
  import type { Snippet } from '$lib/bridge/types'
  import { getFocusedSessionId } from '$lib/stores/sessionRuntime.svelte'
  import { check as checkUpdate } from '@tauri-apps/plugin-updater'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { loadSettings } from '$lib/stores/settings.svelte'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  // ── types ───────────────────────────────────────────────────────────────────

  type PaneStatus = 'connecting' | 'connected' | 'error' | 'closed'

  interface AppTab {
    id: number
    /** Recursive split-pane tree. A new tab starts as a single leaf. */
    root: PaneTreeNode
    /** Pane status keyed by paneId. Reactive map for fine-grained updates. */
    statuses: SvelteMap<number, PaneStatus>
    /** Stable label/colour for the tab (mirrors the first leaf at creation). */
    label: string
    color: string
  }

  // ── theme ────────────────────────────────────────────────────────────────────

  const theme = $derived(getTheme())

  const themeKeyMap: Record<string, string> = {
    'parchment': 'graphite',
    'graphite':  'neonNoir',
    'neon-noir': 'aurora',
    'aurora':    'parchment',
  }

  function toggleTheme() {
    setTheme(themeKeyMap[theme.name] ?? 'parchment')
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
    return {
      id: nextId++,
      root: leafNode(pane),
      statuses: new SvelteMap<number, PaneStatus>([[pane.id, 'connecting']]),
      label: pane.label,
      color: pane.color,
    }
  }

  const firstPane = mkPane('shell')
  const firstTab = mkTab(firstPane)

  let tabs = $state<AppTab[]>([firstTab])
  let activeTabId = $state(firstTab.id)
  let focusedPaneId = $state(firstPane.id)
  let sessions = $state<SavedSession[]>([])
  let folders = $state<Folder[]>([])
  let snippets = $state<Snippet[]>([])
  let showNewSession = $state(false)
  let showQuickConnect = $state(false)
  let showSettings = $state(false)
  let showSnippets = $state(false)
  let showAbout = $state(false)
  let showPalette = $state(false)
  // Lightweight text-prompt dialog (replaces window.prompt which Tauri blocks).
  let prompt = $state<{
    title: string
    initial?: string
    placeholder?: string
    submitLabel?: string
    onSubmit: (v: string) => void
  } | null>(null)
  function openPrompt(p: NonNullable<typeof prompt>) { prompt = p }

  // Live throughput stats keyed by runtime session UUID, refreshed every
  // second from the backend's session-stats event.
  let bytesPerSecBySession = $state<Record<string, number>>({})
  /// When set, opens NewSessionDialog in edit mode with the given session pre-filled.
  let editingSession = $state<SavedSession | null>(null)
  let multiOn = $state(false)

  // Mirror focusedPaneId + multi-exec toggle into the shared runtime store so
  // snippets / broadcast can act on the currently focused session.
  $effect(() => {
    sessionRuntime.focusedPaneId = focusedPaneId
  })
  $effect(() => {
    broadcast.enabled = multiOn
  })

  // ── derived ──────────────────────────────────────────────────────────────────

  const activeTab = $derived(tabs.find((t) => t.id === activeTabId))

  /// Returns the first leaf's status as a coarse summary for the tab strip.
  function summaryStatus(tab: AppTab): PaneStatus {
    const id = firstLeafId(tab.root)
    return tab.statuses.get(id) ?? 'connecting'
  }

  const tabEntries = $derived<TabEntry[]>(
    tabs.map((t) => ({
      id: t.id,
      label: t.label,
      color: t.color,
      status: summaryStatus(t),
    }))
  )

  /// Find a leaf by id in the active tab's tree.
  function findActiveLeaf(paneId: number): PaneData | null {
    if (!activeTab) return null
    let found: PaneData | null = null
    eachLeaf(activeTab.root, (p) => {
      if (p.id === paneId) found = p
    })
    return found
  }

  const focusedPaneData = $derived<PaneData | null>(findActiveLeaf(focusedPaneId))

  const focusedStatus = $derived<PaneStatus>(
    activeTab?.statuses.get(focusedPaneId) ?? 'connecting',
  )

  const splitOn = $derived(!!activeTab && leafCount(activeTab.root) > 1)
  const canClosePanes = $derived(splitOn)

  const statusHost = $derived(
    focusedPaneData?.ssh?.host ??
    focusedPaneData?.savedSession?.host ??
    ''
  )

  const statusPort = $derived(
    focusedPaneData?.ssh?.port ??
    focusedPaneData?.savedSession?.port ??
    undefined
  )

  const statusProtocol = $derived(
    focusedPaneData?.savedSession?.protocol?.toUpperCase() ??
    (focusedPaneData?.ssh ? 'SSH' : focusedPaneData?.telnet ? 'TELNET' : 'LOCAL')
  )

  const activeSessionName = $derived(activeTab?.label ?? '')

  // ── data loading ─────────────────────────────────────────────────────────────

  async function load() {
    // Each call independent so a single failure doesn't blank the whole tree.
    listSessions().then((v) => (sessions = v)).catch((e) => console.error('listSessions', e))
    listFolders().then((v) => (folders = v)).catch((e) => console.error('listFolders', e))
    listSnippetsApi().then((v) => (snippets = v)).catch((e) => console.error('listSnippets', e))
  }

  $effect(() => {
    load()
    loadSettings()
    loadBindings()
    loadHintsSettings()
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
    if (tab) focusedPaneId = firstLeafId(tab.root)
  }

  function handleStatusChange(tabId: number, paneId: number, status: PaneStatus) {
    const tab = tabs.find((t) => t.id === tabId)
    if (!tab) return
    tab.statuses.set(paneId, status)
  }

  /// Split the focused leaf in the active tab in `direction`.
  function handleSplit(paneId: number, direction: SplitDirection) {
    const tab = tabs.find((t) => t.id === activeTabId)
    if (!tab) return
    const newPane = mkPane('shell')
    tab.root = splitLeaf(tab.root, paneId, direction, newPane)
    tab.statuses.set(newPane.id, 'connecting')
    focusedPaneId = newPane.id
  }

  /// Close a single leaf. If it was the last leaf, close the whole tab.
  function handleClosePane(paneId: number) {
    const tab = tabs.find((t) => t.id === activeTabId)
    if (!tab) return
    const next = removeLeaf(tab.root, paneId)
    tab.statuses.delete(paneId)
    if (next === null) {
      closeTab(tab.id)
      return
    }
    tab.root = next
    if (focusedPaneId === paneId) {
      focusedPaneId = firstLeafId(tab.root)
    }
  }

  /// Drag-resize a divider — `path` walks the tree from the root.
  function handleResize(path: number[], ratio: number) {
    const tab = tabs.find((t) => t.id === activeTabId)
    if (!tab) return
    tab.root = setRatio(tab.root, path, ratio)
  }

  /// TabBar's "Split" button: split the currently focused pane horizontally.
  function handleSplitFocused() {
    handleSplit(focusedPaneId, 'h')
  }

  function handleSessionAdded() {
    showNewSession = false
    load()
  }

  // ── keyboard shortcuts ────────────────────────────────────────────────────────

  /// Execute the handler tied to a [`ShortcutAction`]. Called both from the
  /// global document listener and from xterm's custom key handler (via
  /// `onGlobalShortcut`), so xterm-focused panes route shortcuts identically.
  function dispatchAction(action: ShortcutAction) {
    switch (action) {
      case 'new-session': showNewSession = true; break
      case 'new-tab':     addLocalTab(); break
      case 'close-tab':   closeTab(activeTabId); break
      case 'next-tab':
      case 'prev-tab': {
        const dir = action === 'prev-tab' ? -1 : 1
        const idx = tabs.findIndex((t) => t.id === activeTabId)
        const next = tabs[(idx + dir + tabs.length) % tabs.length]
        if (next) activateTab(next.id)
        break
      }
      case 'settings':    showSettings = true; break
      case 'snippets':    showSnippets = true; break
      case 'split-h':     handleSplit(focusedPaneId, 'h'); break
      case 'split-v':     handleSplit(focusedPaneId, 'v'); break
      case 'multi-exec':  multiOn = !multiOn; break
      case 'quick-connect': showQuickConnect = true; break
    }
  }

  /// Check the configured update endpoint, ask the user, and install on confirm.
  /// Requires the updater plugin to be `active` in tauri.conf.json with a valid
  /// signing pubkey + endpoint — until then this surfaces a clear error.
  async function handleCheckUpdates() {
    try {
      const update = await checkUpdate()
      if (!update) {
        window.alert('zapx is up to date.')
        return
      }
      const proceed = window.confirm(
        `Update ${update.version} available (current: ${update.currentVersion}).\n\n` +
          `${update.body ?? ''}\n\nDownload and install now?`,
      )
      if (!proceed) return
      await update.downloadAndInstall()
      window.alert('Update installed. Please restart zapx to use it.')
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e)
      window.alert(`Could not check for updates: ${msg}`)
    }
  }

  /// Open a new tab from a [`QuickConnectDialog`] submission. Connections are
  /// in-memory only (no SQLite row), so closing the tab cleans up everything.
  function handleQuickConnect(params: ConnectParams) {
    showQuickConnect = false
    if (params.type === 'local') {
      addLocalTab()
      return
    }
    if (params.type === 'ssh') {
      const pane = mkPane(`${params.user}@${params.host}`, {
        ssh: {
          host: params.host,
          port: params.port,
          user: params.user,
          auth: { type: 'password', password: params.password },
        },
      } as Partial<PaneData>)
      const tab = mkTab(pane)
      tabs = [...tabs, tab]
      activeTabId = tab.id
      focusedPaneId = pane.id
      return
    }
    // telnet
    const pane = mkPane(`telnet://${params.host}`, {
      telnet: { host: params.host, port: params.port },
    } as Partial<PaneData>)
    const tab = mkTab(pane)
    tabs = [...tabs, tab]
    activeTabId = tab.id
    focusedPaneId = pane.id
  }

  /// xterm forwards `(action, e)` here when its custom key handler matches a
  /// binding. `action` is the [`ShortcutAction`] id (typed as string for the
  /// existing callback signature).
  function handleGlobalShortcut(action: string, _e: KeyboardEvent) {
    dispatchAction(action as ShortcutAction)
  }

  // Build the command-palette item list. Re-derives whenever sessions or
  // theme change. Mirrors the existing dispatchAction surface plus theme
  // switching + the "open saved session" shortcut.
  const paletteItems = $derived<PaletteItem[]>([
    ...sessions.map((s) => ({
      id: `session-${s.id}`,
      label: s.name,
      subtitle: `${s.protocol.toUpperCase()} · ${s.host ?? ''}${s.port ? ':' + s.port : ''}${s.username ? ' · ' + s.username : ''}`,
      icon: s.protocol === 'ssh' ? '🔐' : s.protocol === 'telnet' ? '📡' : s.protocol === 'serial' ? '🔌' : '▶',
      section: 'Sessions' as const,
      run: () => openSavedSessionTab(s),
    })),
    { id: 'act-new-tab',       label: 'Nuevo tab local',           icon: '＋', section: 'Actions' as const, run: addLocalTab },
    { id: 'act-new-session',   label: 'Nueva sesión guardada',     icon: '🆕', section: 'Actions' as const, run: () => (showNewSession = true) },
    { id: 'act-quick',         label: 'Quick Connect',             icon: '⚡', section: 'Actions' as const, run: () => (showQuickConnect = true) },
    { id: 'act-snippets',      label: 'Abrir Snippets',            icon: '✂', section: 'Actions' as const, run: () => (showSnippets = true) },
    { id: 'act-settings',      label: 'Abrir Settings',            icon: '⚙', section: 'Actions' as const, run: () => (showSettings = true) },
    { id: 'act-split-h',       label: 'Split horizontal',          icon: '◫', section: 'Actions' as const, run: () => handleSplit(focusedPaneId, 'h') },
    { id: 'act-split-v',       label: 'Split vertical',            icon: '⬓', section: 'Actions' as const, run: () => handleSplit(focusedPaneId, 'v') },
    { id: 'act-multi',         label: 'Toggle multi-exec',         icon: '⇶', section: 'Actions' as const, run: () => (multiOn = !multiOn) },
    { id: 'act-close-tab',     label: 'Cerrar tab actual',         icon: '✕', section: 'Actions' as const, run: () => closeTab(activeTabId) },
    { id: 'act-open-logs',     label: 'Abrir carpeta de logs',     icon: '📂', section: 'Actions' as const, run: async () => {
      try {
        const path = await openLogsDir()
        showToast({ kind: 'info', title: 'Logs', detail: path })
      } catch (e) {
        showToast({ kind: 'error', title: 'Logs', detail: e instanceof Error ? e.message : String(e) })
      }
    } },
    { id: 'act-about',         label: 'Acerca de zapx',            icon: 'ℹ', section: 'Actions' as const, run: () => (showAbout = true) },
    { id: 'theme-neon-noir',   label: 'Tema · Neon Noir', icon: '◐', section: 'Themes' as const, run: () => setTheme('neonNoir') },
    { id: 'theme-graphite',    label: 'Tema · Graphite',  icon: '◐', section: 'Themes' as const, run: () => setTheme('graphite') },
    { id: 'theme-parchment',   label: 'Tema · Parchment', icon: '◑', section: 'Themes' as const, run: () => setTheme('parchment') },
    { id: 'theme-aurora',      label: 'Tema · Aurora',    icon: '✨', section: 'Themes' as const, run: () => setTheme('aurora') },
  ])

  // Throughput in the StatusBar reflects the runtime session of the focused
  // pane. paneToSession is the runtime-store map updated by TerminalTab.
  const focusedRuntimeSessionId = $derived(
    paneToSession.get(focusedPaneId) ?? null,
  )

  onMount(() => {
    function onKeydown(e: KeyboardEvent) {
      // Command palette: Ctrl+K or Cmd+K — handled BEFORE the keybindings
      // matcher so user overrides don't conflict.
      if ((e.ctrlKey || e.metaKey) && e.key === 'k' && !e.altKey && !e.shiftKey) {
        e.preventDefault()
        showPalette = true
        return
      }
      const action = matchAction(e)
      if (!action) return
      e.preventDefault()
      dispatchAction(action)
    }
    document.addEventListener('keydown', onKeydown)

    let unlistenStats: UnlistenFn | null = null
    listen<{ session_id: string; bytes_per_sec: number }>('session-stats', (e) => {
      bytesPerSecBySession = {
        ...bytesPerSecBySession,
        [e.payload.session_id]: e.payload.bytes_per_sec,
      }
    }).then((fn) => { unlistenStats = fn })

    return () => {
      document.removeEventListener('keydown', onKeydown)
      if (unlistenStats) unlistenStats()
    }
  })
</script>

<!-- ── Resize handles (frameless window) ───────────────────────────────────── -->
<ResizeHandles />

<!-- ── PYLON shell ──────────────────────────────────────────────────────────── -->
<div
  class="pylon-shell"
  style:background={theme.appBg}
  style:--cursor-glow={theme.terminal.cursor}
  data-glow={theme.glows ? 'true' : 'false'}
  data-theme={theme.name}
>

  <TitleBar
    {theme}
    sessionName={activeSessionName}
    themeName={theme.name}
    onNewSession={() => showNewSession = true}
    onQuickConnect={() => (showQuickConnect = true)}
    onSettings={() => showSettings = true}
    onSnippets={() => showSnippets = true}
    onCheckUpdates={handleCheckUpdates}
    onToggleTheme={toggleTheme}
    onSetTheme={(k) => setTheme(k)}
    onAbout={() => showAbout = true}
  />

  <div class="pylon-body">

    <Sidebar
      {theme}
      {sessions}
      {folders}
      activeSessionId={focusedPaneData?.savedSession?.id}
      onSelect={openSavedSessionTab}
      onEdit={(s) => (editingSession = s)}
      onDelete={async (s) => {
        const ok = await ask(`Borrar "${s.name}"? Esta acción no se puede deshacer.`, {
          title: 'Borrar sesión',
          kind: 'warning',
        })
        if (!ok) return
        try {
          await deleteSavedSession(s.id)
          await load()
        } catch (e) {
          console.error('delete session failed:', e)
        }
      }}
      onCreateFolder={() => {
        openPrompt({
          title: 'Nueva carpeta',
          placeholder: 'Mi carpeta',
          submitLabel: 'Crear',
          onSubmit: async (name) => {
            prompt = null
            try {
              await createFolder(name, null)
              await load()
            } catch (e) {
              console.error('create folder failed:', e)
            }
          },
        })
      }}
      onRenameFolder={(folder) => {
        openPrompt({
          title: `Renombrar "${folder.name}"`,
          initial: folder.name,
          submitLabel: 'Renombrar',
          onSubmit: async (name) => {
            prompt = null
            if (name === folder.name) return
            try {
              await renameFolder(folder.id, name)
              await load()
            } catch (e) {
              console.error('rename folder failed:', e)
            }
          },
        })
      }}
      onDeleteFolder={async (folder) => {
        const ok = await ask(
          `Borrar la carpeta "${folder.name}"? Las sesiones de dentro pasan a la raíz.`,
          { title: 'Borrar carpeta', kind: 'warning' },
        )
        if (!ok) return
        try {
          await deleteFolder(folder.id)
          await load()
        } catch (e) {
          console.error('delete folder failed:', e)
        }
      }}
      onMove={async (sessionId, folderId) => {
        try {
          await moveSavedSession(sessionId, folderId)
          await load()
        } catch (e) {
          console.error('move session failed:', e)
        }
      }}
      onAddSession={() => showNewSession = true}
      onSettings={() => showSettings = true}
      onToggleTheme={toggleTheme}
      {snippets}
      onSendSnippet={async (s) => {
        const sid = getFocusedSessionId()
        if (!sid) return
        try { await sendInputText(sid, s.content) } catch (e) { console.error(e) }
      }}
      onOpenSnippets={() => (showSnippets = true)}
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
        onToggleSplit={handleSplitFocused}
        onToggleMulti={() => multiOn = !multiOn}
      />

      <div class="pylon-panes">
        {#each tabs as tab (tab.id)}
          <div class="pane-slot" class:hidden={tab.id !== activeTabId}>
            {#if tab.id === activeTabId}
              <SplitTree
                node={tab.root}
                {theme}
                {focusedPaneId}
                {canClosePanes}
                onFocus={(id) => (focusedPaneId = id)}
                onSplit={(id, dir) => handleSplit(id, dir)}
                onClosePane={(id) => handleClosePane(id)}
                onStatusChange={(id, s) => handleStatusChange(tab.id, id, s)}
                onGlobalShortcut={handleGlobalShortcut}
                onResize={(path, ratio) => handleResize(path, ratio)}
              />
            {/if}
          </div>
        {/each}
      </div>

      <StatusBar
        {theme}
        status={focusedStatus}
        host={statusHost || undefined}
        port={statusPort ?? undefined}
        protocol={statusProtocol}
        bytesPerSec={focusedRuntimeSessionId ? bytesPerSecBySession[focusedRuntimeSessionId] : undefined}
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

{#if editingSession}
  <NewSessionDialog
    {folders}
    existing={editingSession}
    onCancel={() => (editingSession = null)}
    onCreated={() => { editingSession = null; load() }}
  />
{/if}

{#if showQuickConnect}
  <QuickConnectDialog
    onCancel={() => (showQuickConnect = false)}
    onConnect={handleQuickConnect}
  />
{/if}

<KeyboardInteractiveDialog />

{#if showSnippets}
  <SnippetsDialog onClose={() => (showSnippets = false)} />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => showSettings = false} />
{/if}

{#if showPalette}
  <CommandPalette
    items={paletteItems}
    accent={theme.accent}
    onClose={() => (showPalette = false)}
  />
{/if}

{#if prompt}
  <PromptDialog
    title={prompt.title}
    placeholder={prompt.placeholder}
    initial={prompt.initial}
    submitLabel={prompt.submitLabel}
    onSubmit={prompt.onSubmit}
    onCancel={() => (prompt = null)}
  />
{/if}

<Toasts />

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
            <tr><td style:color={theme.textDim}>Theme</td><td style:color={theme.accent}>{theme.name === 'neon-noir' ? 'Neon Noir' : theme.name === 'parchment' ? 'Parchment' : 'Graphite'}</td></tr>
          </tbody>
        </table>
        <p class="about-hint" style:color={theme.textDim} style:font-family={theme.fontMono}>
          Credentials stored in the OS keyring, with an encrypted local fallback
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
