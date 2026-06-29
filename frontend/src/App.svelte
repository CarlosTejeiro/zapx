<script lang="ts">
  import { onMount } from 'svelte'
  import { getTheme, setTheme } from '$lib/themes/store.svelte'
  import { themes, themeLabels } from '$lib/themes/index'
  import MarkTile from '$lib/icons/MarkTile.svelte'
  import Icon from '$lib/icons/Icon.svelte'
  import TitleBar from '$lib/pylon/TitleBar.svelte'
  import Sidebar from '$lib/pylon/Sidebar.svelte'
  import TabBar from '$lib/pylon/TabBar.svelte'
  import type { TabEntry } from '$lib/pylon/TabBar.svelte'
  import type { PaneData } from '$lib/pylon/Pane.svelte'
  import SplitTree from '$lib/pylon/SplitTree.svelte'
  import GridView from '$lib/pylon/GridView.svelte'
  import MasterInputBar from '$lib/pylon/MasterInputBar.svelte'
  import ComparisonPanel from '$lib/orchestrator/ComparisonPanel.svelte'
  import { CommandRunner, type RunnerHost } from '$lib/orchestrator/commandRunner.svelte'
  import CommandListDialog from '$lib/orchestrator/CommandListDialog.svelte'
  import TunnelsManagerDialog from '$lib/terminal/TunnelsManagerDialog.svelte'
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
  import VariablesDialog from '$lib/snippets/VariablesDialog.svelte'
  import { resolveSnippet } from '$lib/snippets/variables.svelte'
  import { showToast } from '$lib/ui/toast-store.svelte'
  import PromptDialog from '$lib/ui/PromptDialog.svelte'
  import { ask, open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog'
  import {
    exportSessions,
    importSessions,
    importSshConfig,
    importPutty,
    importMobaXterm,
    importSecureCrt,
  } from '$lib/bridge/commands'
  import type { ImportSummary } from '$lib/bridge/commands'
  import { loadHintsSettings } from '$lib/hints/store.svelte'
  import { broadcast, sessionRuntime, paneToSession } from '$lib/stores/sessionRuntime.svelte'
  import { loadBindings, matchAction, type ShortcutAction } from '$lib/stores/keybindings.svelte'
  import {
    listSessions,
    listFolders,
    moveSavedSession,
    reorderSavedSession,
    deleteSavedSession,
    cloneSavedSession,
    createFolder,
    renameFolder,
    deleteFolder,
    sendInputText,
    openLogsDir,
  } from '$lib/bridge/commands'
  import {
    getFocusedSessionId,
    broadcastTargets,
    focusSession,
  } from '$lib/stores/sessionRuntime.svelte'
  import { visibleSnippets, loadSnippets, setBarContext } from '$lib/stores/snippets.svelte'
  import { getSessionPlatform, getSessionTcpMss, openExternal } from '$lib/bridge/commands'
  import SnippetButtonBar from '$lib/pylon/SnippetButtonBar.svelte'
  import { getVersion } from '@tauri-apps/api/app'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import { loadSettings } from '$lib/stores/settings.svelte'
  import type { SavedSession, Folder, BroadcastGroup } from '$lib/bridge/types'
  import { groups, loadGroups } from '$lib/stores/groups.svelte'
  import GroupsDialog from '$lib/sessions/GroupsDialog.svelte'

  // ── types ───────────────────────────────────────────────────────────────────

  type PaneStatus = 'connecting' | 'connected' | 'error' | 'closed'

  /**
   * A tab is either a split-pane tree (the default, recursive binary splits)
   * or a flat grid of panes (multi-host orchestration). The grid carries its
   * own ordered pane list + column count and an optional source group id; it
   * is rendered by GridView and is NOT splittable.
   */
  type TabLayout =
    | { kind: 'split'; root: PaneTreeNode }
    | { kind: 'grid'; panes: PaneData[]; cols: number; groupId: number | null }

  interface AppTab {
    id: number
    layout: TabLayout
    /** Pane status keyed by paneId. Reactive map for fine-grained updates. */
    statuses: SvelteMap<number, PaneStatus>
    /** Stable label/colour for the tab (mirrors the first leaf at creation). */
    label: string
    color: string
  }

  /** All panes in a tab, regardless of layout. */
  function tabPanes(tab: AppTab): PaneData[] {
    if (tab.layout.kind === 'grid') return tab.layout.panes
    const out: PaneData[] = []
    eachLeaf(tab.layout.root, (p) => out.push(p))
    return out
  }

  /** First pane id in a tab (for focus + tab-strip summary). */
  function firstPaneId(tab: AppTab): number {
    if (tab.layout.kind === 'grid') return tab.layout.panes[0]?.id ?? -1
    return firstLeafId(tab.layout.root)
  }

  // ── theme ────────────────────────────────────────────────────────────────────

  const theme = $derived(getTheme())

  /// Cycle through the theme registry in declaration order.
  function toggleTheme() {
    const keys = Object.keys(themes)
    const idx = keys.indexOf(theme.name)
    setTheme(keys[(idx + 1) % keys.length] ?? 'parchment')
  }

  // Publish the active theme as global `--zx-*` CSS custom properties on the
  // document root. This lets fixed-position overlays (dialogs, toasts, the
  // command palette) — which render outside the prop-threaded layout tree —
  // pick up theme colors via plain `var(--zx-…)` in their own styles, with
  // no prop plumbing. Re-runs whenever `theme` changes because it reads the
  // derived value. Fallback values live in app.css so first paint never flashes.
  $effect(() => {
    const root = document.documentElement.style
    const set = (k: string, v: string) => root.setProperty(k, v)
    set('--zx-app-bg', theme.appBg)
    set('--zx-body-bg', theme.bodyBg)
    set('--zx-surface', theme.sidebarBg)
    set('--zx-surface-2', theme.titlebarBg)
    set('--zx-text', theme.textPrimary)
    set('--zx-text-muted', theme.textMuted)
    set('--zx-text-dim', theme.textDim)
    set('--zx-accent', theme.accent)
    set('--zx-accent-2', theme.accent2)
    set('--zx-on-accent', theme.onAccent)
    set('--zx-border', theme.border)
    set('--zx-radius', theme.radius)
    set('--zx-hover-bg', theme.itemHoverBg)
    set('--zx-active-bg', theme.itemActiveBg)
    set('--zx-active-border', theme.itemActiveBorder)
    set('--zx-ok', theme.ok)
    set('--zx-warn', theme.warn)
    set('--zx-err', theme.err)
    set('--zx-font-ui', theme.fontUi)
    set('--zx-font-mono', theme.fontMono)
    set('--zx-shadow', theme.windowShadow)
    set('--zx-term-bg', theme.terminal.bg)
    set('--zx-term-fg', theme.terminal.fg)
  })

  // ── state ────────────────────────────────────────────────────────────────────

  let nextId = 1

  const SESSION_COLORS = [
    '#22d3ee',
    '#f472b6',
    '#a78bfa',
    '#f59e0b',
    '#ef4444',
    '#10b981',
    '#f97316',
    '#84cc16',
    '#c89b6b',
    '#5eb3b2',
  ]

  function pickColor(id: number): string {
    return SESSION_COLORS[id % SESSION_COLORS.length] as string
  }

  function mkPane(
    label: string,
    opts?: Partial<Omit<PaneData, 'id' | 'label' | 'color'>>,
  ): PaneData {
    const id = nextId++
    return { id, label, color: pickColor(id), ...opts }
  }

  function mkTab(pane: PaneData): AppTab {
    return {
      id: nextId++,
      layout: { kind: 'split', root: leafNode(pane) },
      statuses: new SvelteMap<number, PaneStatus>([[pane.id, 'connecting']]),
      label: pane.label,
      color: pane.color,
    }
  }

  function mkGridTab(panes: PaneData[], label: string, groupId: number | null): AppTab {
    const cols = Math.max(1, Math.ceil(Math.sqrt(panes.length)))
    const statuses = new SvelteMap<number, PaneStatus>()
    for (const p of panes) statuses.set(p.id, 'connecting')
    return {
      id: nextId++,
      layout: { kind: 'grid', panes, cols, groupId },
      statuses,
      label,
      color: panes[0]?.color ?? pickColor(nextId),
    }
  }

  const firstPane = mkPane('shell')
  const firstTab = mkTab(firstPane)

  let tabs = $state<AppTab[]>([firstTab])
  let activeTabId = $state(firstTab.id)
  let focusedPaneId = $state(firstPane.id)
  let sessions = $state<SavedSession[]>([])
  let folders = $state<Folder[]>([])
  let showNewSession = $state(false)
  let showQuickConnect = $state(false)
  let showSettings = $state(false)
  let showSnippets = $state(false)
  let showGroups = $state(false)
  let showCommandList = $state(false)
  let showTunnelsManager = $state(false)

  /// Live session UUID → human label (pane name), for the tunnels manager.
  const sessionLabels = $derived<Record<string, string>>(
    Object.fromEntries(
      tabs.flatMap((t) =>
        tabPanes(t)
          .map((p) => [paneToSession.get(p.id), p.label] as const)
          .filter((e): e is [string, string] => typeof e[0] === 'string'),
      ),
    ),
  )
  let showAbout = $state(false)
  // Bundle version from tauri.conf.json — single source of truth.
  let appVersion = $state('')
  getVersion()
    .then((v) => (appVersion = v))
    .catch(() => {})
  let showPalette = $state(false)
  // Lightweight text-prompt dialog (replaces window.prompt which Tauri blocks).
  let prompt = $state<{
    title: string
    initial?: string
    placeholder?: string
    submitLabel?: string
    onSubmit: (v: string) => void
  } | null>(null)
  function openPrompt(p: NonNullable<typeof prompt>) {
    prompt = p
  }

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

  // Tell the bottom snippet bar which session is focused so it can filter
  // snippets by platform AND show that session's recent-commands. Effects
  // re-run when `focusedPaneId` flips; we resolve the saved-session id
  // inside so a folder/local tab cleanly resets to "no platform".
  $effect(() => {
    void focusedPaneId // explicit dep
    const pane = focusedPaneData
    const sid = pane?.savedSession?.id ?? null
    if (sid != null) {
      getSessionPlatform(sid)
        .then((p) => setBarContext(p, sid))
        .catch(() => setBarContext(null, sid))
    } else if (pane && !pane.ssh && !pane.telnet) {
      // Local shell tab (no saved row, not SSH/Telnet). It's a unix shell, so
      // give the bar the Linux/bash catalog — useful local commands instead of
      // every vendor's snippets. This is also what makes a local session count
      // as "recognised" for hints.
      setBarContext('linux', null).catch(() => {})
    } else {
      setBarContext(null, null).catch(() => {})
    }
  })

  // ── derived ──────────────────────────────────────────────────────────────────

  const activeTab = $derived(tabs.find((t) => t.id === activeTabId))

  /// Returns the first leaf's status as a coarse summary for the tab strip.
  function summaryStatus(tab: AppTab): PaneStatus {
    const id = firstPaneId(tab)
    return tab.statuses.get(id) ?? 'connecting'
  }

  const tabEntries = $derived<TabEntry[]>(
    tabs.map((t) => ({
      id: t.id,
      label: t.label,
      color: t.color,
      status: summaryStatus(t),
    })),
  )

  /// Find a pane by id in the active tab (split tree or grid).
  function findActiveLeaf(paneId: number): PaneData | null {
    if (!activeTab) return null
    return tabPanes(activeTab).find((p) => p.id === paneId) ?? null
  }

  const focusedPaneData = $derived<PaneData | null>(findActiveLeaf(focusedPaneId))

  const focusedStatus = $derived<PaneStatus>(activeTab?.statuses.get(focusedPaneId) ?? 'connecting')

  /** Aggregated live status per saved-session id, across every open pane in
   *  every tab — drives the status dot on the sidebar avatars. A session with
   *  no open pane (or only `closed` ones) is absent from the map and shows no
   *  dot (idle / not opened). When a session has several panes open, the most
   *  reassuring status wins: connected > connecting > error. */
  const sessionStatuses = $derived.by<Map<number, 'connecting' | 'connected' | 'error'>>(() => {
    const m = new Map<number, 'connecting' | 'connected' | 'error'>()
    const rank = { error: 1, connecting: 2, connected: 3 } as const
    for (const t of tabs) {
      for (const p of tabPanes(t)) {
        const sid = p.savedSession?.id
        if (sid == null) continue
        const st = t.statuses.get(p.id)
        if (st !== 'connecting' && st !== 'connected' && st !== 'error') continue
        const cur = m.get(sid)
        if (!cur || rank[st] > rank[cur]) m.set(sid, st)
      }
    }
    return m
  })

  const splitOn = $derived(
    !!activeTab && activeTab.layout.kind === 'split' && leafCount(activeTab.layout.root) > 1,
  )

  /// Live session UUIDs of the active tab's panes — feeds the master input
  /// bar on regular (split) tabs when multi-exec is on. Grid tabs render
  /// their own bar inside GridView.
  const activeTabSessionIds = $derived(
    activeTab
      ? tabPanes(activeTab)
          .map((p) => paneToSession.get(p.id))
          .filter((id): id is string => typeof id === 'string')
      : [],
  )

  // ── Run & compare on a regular (split) tab ──────────────────────────────────
  // Mirrors GridView's runner so "Compare" works outside broadcast-group grids:
  // send one command to every live pane of the active tab and diff the outputs.
  const compareRunner = new CommandRunner()
  let showComparePanel = $state(false)

  async function runCompareActiveTab(command: string) {
    if (!activeTab) return
    const hosts: RunnerHost[] = []
    for (const pane of tabPanes(activeTab)) {
      const sid = paneToSession.get(pane.id)
      if (!sid) continue
      const savedId = pane.savedSession?.id
      const platform = savedId != null ? await getSessionPlatform(savedId).catch(() => null) : null
      hosts.push({ sessionId: sid, label: pane.label, platform })
    }
    showComparePanel = true
    await compareRunner.run(hosts, command)
  }

  /// Every live session across all tabs, labelled — target list for the
  /// command-list dialog.
  const liveTargets = $derived(
    tabs.flatMap((t) =>
      tabPanes(t)
        .map((p) => ({
          sessionId: paneToSession.get(p.id),
          label: p.label,
          tabLabel: t.label,
          active: t.id === activeTabId,
        }))
        .filter(
          (x): x is { sessionId: string; label: string; tabLabel: string; active: boolean } =>
            typeof x.sessionId === 'string',
        ),
    ),
  )
  const canClosePanes = $derived(splitOn)

  const statusHost = $derived(
    focusedPaneData?.ssh?.host ?? focusedPaneData?.savedSession?.host ?? '',
  )

  const statusPort = $derived(
    focusedPaneData?.ssh?.port ?? focusedPaneData?.savedSession?.port ?? undefined,
  )

  const statusProtocol = $derived(
    focusedPaneData?.savedSession?.protocol?.toUpperCase() ??
      (focusedPaneData?.ssh ? 'SSH' : focusedPaneData?.telnet ? 'TELNET' : 'LOCAL'),
  )

  const activeSessionName = $derived(activeTab?.label ?? '')

  // ── data loading ─────────────────────────────────────────────────────────────

  async function load() {
    // Each call independent so a single failure doesn't blank the whole tree.
    listSessions()
      .then((v) => (sessions = v))
      .catch((e) => console.error('listSessions', e))
    listFolders()
      .then((v) => (folders = v))
      .catch((e) => console.error('listFolders', e))
    loadSnippets().catch((e) => console.error('loadSnippets', e))
    loadGroups().catch((e) => console.error('loadGroups', e))
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

  /// Open every saved session in `group` as a tiled grid in a single new tab,
  /// driven by the master input bar. Missing members (deleted sessions) are
  /// skipped. Each pane connects independently via the existing path.
  function openGroupAsGrid(group: BroadcastGroup) {
    const members = group.session_ids
      .map((sid) => sessions.find((s) => s.id === sid))
      .filter((s): s is SavedSession => !!s)
    if (members.length === 0) {
      showToast({ kind: 'warning', title: group.name, detail: 'This group has no sessions.' })
      return
    }
    const panes = members.map((s) =>
      mkPane(s.name, { savedSession: s, color: pickColor(s.id) } as Partial<PaneData>),
    )
    const tab = mkGridTab(panes, group.name, group.id)
    tabs = [...tabs, tab]
    activeTabId = tab.id
    focusedPaneId = panes[0]!.id
  }

  function closeTab(id: number) {
    if (tabs.length <= 1) return
    const idx = tabs.findIndex((t) => t.id === id)
    tabs = tabs.filter((t) => t.id !== id)
    if (activeTabId === id) {
      activeTabId = (tabs[idx] ?? tabs[idx - 1])?.id ?? tabs[0]!.id
    }
  }

  function renameTab(id: number, label: string) {
    const t = tabs.find((x) => x.id === id)
    if (t) t.label = label.trim() || t.label
  }

  function setTabColor(id: number, color: string) {
    const t = tabs.find((x) => x.id === id)
    if (t) t.color = color
  }

  // Open a new tab with the same connection parameters as `id`'s first pane.
  function duplicateTab(id: number) {
    const tab = tabs.find((t) => t.id === id)
    if (!tab) return
    const first = tabPanes(tab)[0]
    if (!first) return
    if (first.savedSession) {
      openSavedSessionTab(first.savedSession)
      return
    }
    const pane = mkPane(first.label, {
      ssh: first.ssh,
      telnet: first.telnet,
    } as Partial<PaneData>)
    const newTab = mkTab(pane)
    newTab.color = tab.color
    tabs = [...tabs, newTab]
    activeTabId = newTab.id
    focusedPaneId = pane.id
  }

  function activateTab(id: number) {
    activeTabId = id
    const tab = tabs.find((t) => t.id === id)
    if (tab) focusedPaneId = firstPaneId(tab)
  }

  function handleStatusChange(tabId: number, paneId: number, status: PaneStatus) {
    const tab = tabs.find((t) => t.id === tabId)
    if (!tab) return
    tab.statuses.set(paneId, status)
  }

  /// Split the focused leaf in the active tab in `direction`. No-op on grids.
  function handleSplit(paneId: number, direction: SplitDirection) {
    const tab = tabs.find((t) => t.id === activeTabId)
    if (!tab || tab.layout.kind !== 'split') return
    const newPane = mkPane('shell')
    tab.layout.root = splitLeaf(tab.layout.root, paneId, direction, newPane)
    tab.statuses.set(newPane.id, 'connecting')
    focusedPaneId = newPane.id
  }

  /// Close a single pane. On a split tab, collapses the tree (closing the tab
  /// if it was the last leaf). On a grid tab, drops the pane from the grid
  /// (closing the tab when the last pane goes).
  function handleClosePane(paneId: number) {
    const tab = tabs.find((t) => t.id === activeTabId)
    if (!tab) return
    if (tab.layout.kind === 'grid') {
      const remaining = tab.layout.panes.filter((p) => p.id !== paneId)
      tab.statuses.delete(paneId)
      if (remaining.length === 0) {
        closeTab(tab.id)
        return
      }
      tab.layout.panes = remaining
      tab.layout.cols = Math.max(1, Math.ceil(Math.sqrt(remaining.length)))
      if (focusedPaneId === paneId) focusedPaneId = firstPaneId(tab)
      return
    }
    const next = removeLeaf(tab.layout.root, paneId)
    tab.statuses.delete(paneId)
    if (next === null) {
      closeTab(tab.id)
      return
    }
    tab.layout.root = next
    if (focusedPaneId === paneId) {
      focusedPaneId = firstPaneId(tab)
    }
  }

  /// Drag-resize a divider — `path` walks the tree from the root.
  function handleResize(path: number[], ratio: number) {
    const tab = tabs.find((t) => t.id === activeTabId)
    if (!tab || tab.layout.kind !== 'split') return
    tab.layout.root = setRatio(tab.layout.root, path, ratio)
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
      case 'new-session':
        showNewSession = true
        break
      case 'new-tab':
        addLocalTab()
        break
      case 'close-tab':
        closeTab(activeTabId)
        break
      case 'next-tab':
      case 'prev-tab': {
        const dir = action === 'prev-tab' ? -1 : 1
        const idx = tabs.findIndex((t) => t.id === activeTabId)
        const next = tabs[(idx + dir + tabs.length) % tabs.length]
        if (next) activateTab(next.id)
        break
      }
      case 'settings':
        showSettings = true
        break
      case 'snippets':
        showSnippets = true
        break
      case 'split-h':
        handleSplit(focusedPaneId, 'h')
        break
      case 'split-v':
        handleSplit(focusedPaneId, 'v')
        break
      case 'multi-exec':
        multiOn = !multiOn
        break
      case 'quick-connect':
        showQuickConnect = true
        break
    }
  }

  /// Check the configured update endpoint, ask the user, and install on confirm.
  /// Requires the updater plugin to be `active` in tauri.conf.json with a valid
  /// signing pubkey + endpoint — until then this surfaces a clear error.
  /// In-app auto-update isn't wired (the repo/releases are private, so there's
  /// no public manifest a client could read). Open the releases page instead,
  /// where the latest build can be downloaded.
  async function handleCheckUpdates() {
    const releasesUrl = 'https://github.com/CarlosTejeiro/zapx/releases'
    try {
      await openExternal(releasesUrl)
      showToast({
        kind: 'info',
        title: 'Releases',
        detail: `Current version ${appVersion}. Opening the releases page…`,
      })
    } catch (e) {
      showToast({ kind: 'error', title: 'Releases', detail: String(e) })
    }
  }

  // ── export / import ──────────────────────────────────────────────────────

  /// Export the whole environment to a user-chosen JSON file. Warns that
  /// login scripts travel verbatim (their `send` steps may contain secrets);
  /// passwords themselves never leave the keyring.
  async function handleExport() {
    const proceed = await ask(
      'This exports sessions, folders, groups, snippets and highlight rules.\n\n' +
        'Passwords are NOT exported. Note: login scripts ARE exported verbatim ' +
        '(their "send" steps may contain secrets you typed).',
      { title: 'Export sessions', kind: 'info' },
    )
    if (!proceed) return
    const path = await saveFileDialog({
      title: 'Export ZAPX sessions',
      defaultPath: 'zapx-sessions.json',
      filters: [{ name: 'ZAPX export', extensions: ['json'] }],
    })
    if (!path) return
    try {
      const s = await exportSessions(path)
      showToast({
        kind: 'success',
        title: 'Export complete',
        detail: `${s.sessions} sessions, ${s.folders} folders, ${s.groups} groups, ${s.snippets} snippets, ${s.rules} rules → ${s.path}`,
      })
    } catch (e) {
      showToast({ kind: 'error', title: 'Export failed', detail: String(e) })
    }
  }

  /// Shared toast reporting for every import source.
  async function reportImport(s: ImportSummary) {
    showToast({
      kind: 'success',
      title: 'Import complete',
      detail:
        `${s.sessions_added} new sessions (${s.sessions_skipped} already existed), ` +
        `${s.folders_added} folders, ${s.groups_added} groups, ` +
        `${s.snippets_added} snippets, ${s.rules_added} rules`,
    })
    if (s.warnings.length > 0) {
      showToast({
        kind: 'warning',
        title: `Import: ${s.warnings.length} warning(s)`,
        detail: s.warnings.slice(0, 3).join(' · ') + (s.warnings.length > 3 ? ' …' : ''),
      })
    }
    await load()
  }

  /// Merge a ZAPX export file into the current environment (idempotent —
  /// existing items are skipped) and refresh every affected store.
  async function handleImport() {
    const path = await openFileDialog({
      title: 'Import ZAPX sessions',
      multiple: false,
      directory: false,
      filters: [{ name: 'ZAPX export', extensions: ['json'] }],
    })
    if (typeof path !== 'string' || !path) return
    try {
      await reportImport(await importSessions(path))
    } catch (e) {
      showToast({ kind: 'error', title: 'Import failed', detail: String(e) })
    }
  }

  /// Import hosts from the OpenSSH client config. Defaults to ~/.ssh/config;
  /// declining the prompt opens a file picker for a custom location.
  async function handleImportSshConfig() {
    const useDefault = await ask(
      'Import hosts from ~/.ssh/config?\n\nChoose "No" to pick a different SSH config file.',
      { title: 'Import from SSH config', kind: 'info' },
    )
    let path: string | undefined
    if (!useDefault) {
      const picked = await openFileDialog({
        title: 'Choose ssh_config file',
        multiple: false,
        directory: false,
      })
      if (typeof picked !== 'string' || !picked) return
      path = picked
    }
    try {
      await reportImport(await importSshConfig(path))
    } catch (e) {
      showToast({ kind: 'error', title: 'SSH config import failed', detail: String(e) })
    }
  }

  /// PuTTY: on Windows offer the registry directly; otherwise (or on "No")
  /// pick a `.reg` export. The backend errors clearly if the registry is
  /// asked for off-Windows.
  async function handleImportPutty() {
    const fromRegistry = await ask(
      'Import from the Windows registry?\n\nChoose "No" to pick an exported .reg file (required on macOS/Linux).',
      { title: 'Import from PuTTY', kind: 'info' },
    )
    let path: string | undefined
    if (!fromRegistry) {
      const picked = await openFileDialog({
        title: 'Choose PuTTY .reg export',
        multiple: false,
        directory: false,
        filters: [{ name: 'Registry export', extensions: ['reg'] }],
      })
      if (typeof picked !== 'string' || !picked) return
      path = picked
    }
    try {
      await reportImport(await importPutty(path))
    } catch (e) {
      showToast({ kind: 'error', title: 'PuTTY import failed', detail: String(e) })
    }
  }

  /// MobaXterm: pick a MobaXterm.ini or .mxtsessions file.
  async function handleImportMobaXterm() {
    const picked = await openFileDialog({
      title: 'Choose MobaXterm.ini',
      multiple: false,
      directory: false,
      filters: [{ name: 'MobaXterm', extensions: ['ini', 'mxtsessions'] }],
    })
    if (typeof picked !== 'string' || !picked) return
    try {
      await reportImport(await importMobaXterm(picked))
    } catch (e) {
      showToast({ kind: 'error', title: 'MobaXterm import failed', detail: String(e) })
    }
  }

  /// SecureCRT: pick the "Sessions" directory of its Config folder.
  async function handleImportSecureCrt() {
    const picked = await openFileDialog({
      title: 'Choose SecureCRT Sessions folder',
      directory: true,
      multiple: false,
    })
    if (typeof picked !== 'string' || !picked) return
    try {
      await reportImport(await importSecureCrt(picked))
    } catch (e) {
      showToast({ kind: 'error', title: 'SecureCRT import failed', detail: String(e) })
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

  /// Fire the i-th snippet (0-indexed) at the focused session. Used by the
  /// Ctrl+Shift+1..9 quick-launch shortcut wired in TerminalTab. We address
  /// `visibleSnippets` (what the bar currently shows for this session)
  /// rather than the global list — so the [N] pill on screen matches the
  /// key the user pressed.
  async function fireSnippetByIndex(idx: number) {
    const s = visibleSnippets[idx]
    if (!s) return
    const focused = getFocusedSessionId()
    if (!focused) return
    // Resolve any {{variables}} (prompts the user) before sending.
    const text = await resolveSnippet(s.content)
    if (text === null) return
    await sendInputText(focused, text).catch(console.error)
    // Variable prompts steal focus; restore it so the user can keep typing
    // after a snippet that carries no trailing newline.
    focusSession(focused)
    if (broadcast.enabled) {
      for (const id of broadcastTargets(focused)) {
        await sendInputText(id, text).catch(() => {})
      }
    }
  }

  /// xterm forwards `(action, e)` here when its custom key handler matches a
  /// binding. `action` is the [`ShortcutAction`] id (typed as string for the
  /// existing callback signature). The `snippet:N` namespace is handled here
  /// to avoid widening the typed action union.
  function handleGlobalShortcut(action: string, _e: KeyboardEvent) {
    if (action.startsWith('snippet:')) {
      const idx = parseInt(action.slice('snippet:'.length), 10)
      if (!Number.isNaN(idx)) fireSnippetByIndex(idx)
      return
    }
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
      icon: 'term' as const,
      section: 'Sessions' as const,
      run: () => openSavedSessionTab(s),
    })),
    {
      id: 'act-new-tab',
      label: 'New local tab',
      icon: 'plus' as const,
      section: 'Actions' as const,
      run: addLocalTab,
    },
    {
      id: 'act-new-session',
      label: 'New saved session',
      icon: 'file' as const,
      section: 'Actions' as const,
      run: () => (showNewSession = true),
    },
    {
      id: 'act-quick',
      label: 'Quick Connect',
      icon: 'bolt' as const,
      section: 'Actions' as const,
      run: () => (showQuickConnect = true),
    },
    {
      id: 'act-snippets',
      label: 'Open Snippets',
      icon: 'book' as const,
      section: 'Actions' as const,
      run: () => (showSnippets = true),
    },
    {
      id: 'act-groups',
      label: 'Broadcast groups…',
      icon: 'cast' as const,
      section: 'Actions' as const,
      run: () => (showGroups = true),
    },
    {
      id: 'act-cmdlist',
      label: 'Send command list…',
      icon: 'file' as const,
      section: 'Actions' as const,
      run: () => (showCommandList = true),
    },
    {
      id: 'act-tunnels',
      label: 'Active tunnels…',
      icon: 'tunnel' as const,
      section: 'Actions' as const,
      run: () => (showTunnelsManager = true),
    },
    {
      id: 'act-settings',
      label: 'Open Settings',
      icon: 'gear' as const,
      section: 'Actions' as const,
      run: () => (showSettings = true),
    },
    ...groups.map((g) => ({
      id: `group-${g.id}`,
      label: `Open group: ${g.name}`,
      subtitle: `${g.session_ids.length} session${g.session_ids.length === 1 ? '' : 's'} · grid`,
      icon: 'cast' as const,
      section: 'Groups' as const,
      run: () => openGroupAsGrid(g),
    })),
    {
      id: 'act-split-h',
      label: 'Split horizontally',
      icon: 'split' as const,
      section: 'Actions' as const,
      run: () => handleSplit(focusedPaneId, 'h'),
    },
    {
      id: 'act-split-v',
      label: 'Split vertically',
      icon: 'splitV' as const,
      section: 'Actions' as const,
      run: () => handleSplit(focusedPaneId, 'v'),
    },
    {
      id: 'act-multi',
      label: 'Toggle multi-exec',
      icon: 'cast' as const,
      section: 'Actions' as const,
      run: () => (multiOn = !multiOn),
    },
    {
      id: 'act-close-tab',
      label: 'Close current tab',
      icon: 'x' as const,
      section: 'Actions' as const,
      run: () => closeTab(activeTabId),
    },
    {
      id: 'act-open-logs',
      label: 'Open logs folder',
      icon: 'folder' as const,
      section: 'Actions' as const,
      run: async () => {
        try {
          const path = await openLogsDir()
          showToast({ kind: 'info', title: 'Logs', detail: path })
        } catch (e) {
          showToast({
            kind: 'error',
            title: 'Logs',
            detail: e instanceof Error ? e.message : String(e),
          })
        }
      },
    },
    {
      id: 'act-export',
      label: 'Export sessions…',
      icon: 'transfer' as const,
      section: 'Actions' as const,
      run: handleExport,
    },
    {
      id: 'act-import',
      label: 'Import sessions…',
      icon: 'transfer' as const,
      section: 'Actions' as const,
      run: handleImport,
    },
    {
      id: 'act-import-ssh',
      label: 'Import from SSH config…',
      icon: 'transfer' as const,
      section: 'Actions' as const,
      run: handleImportSshConfig,
    },
    {
      id: 'act-import-putty',
      label: 'Import from PuTTY…',
      icon: 'transfer' as const,
      section: 'Actions' as const,
      run: handleImportPutty,
    },
    {
      id: 'act-import-moba',
      label: 'Import from MobaXterm…',
      icon: 'transfer' as const,
      section: 'Actions' as const,
      run: handleImportMobaXterm,
    },
    {
      id: 'act-import-scrt',
      label: 'Import from SecureCRT…',
      icon: 'transfer' as const,
      section: 'Actions' as const,
      run: handleImportSecureCrt,
    },
    {
      id: 'act-about',
      label: 'About ZAPX',
      icon: 'book' as const,
      section: 'Actions' as const,
      run: () => (showAbout = true),
    },
    ...Object.entries(themeLabels).map(([key, label]) => ({
      id: `theme-${key}`,
      label: `Tema · ${label}`,
      icon: 'contrast' as const,
      section: 'Themes' as const,
      run: () => setTheme(key),
    })),
  ])

  // Throughput in the StatusBar reflects the runtime session of the focused
  // pane. paneToSession is the runtime-store map updated by TerminalTab.
  const focusedRuntimeSessionId = $derived(paneToSession.get(focusedPaneId) ?? null)

  // TCP MSS for the focused SSH session. Initial value comes from the
  // snapshot the backend captured at connect; live updates arrive via the
  // `session-mss-updated` Tauri event the backend polling task emits
  // (every ~5s, only when the value actually changes).
  let focusedMss = $state<{ send: number | null; recv: number | null } | null>(null)
  // Keyed cache so the listener doesn't blow away the snapshot for a
  // session that isn't currently focused.
  const mssBySession = new Map<string, { send: number | null; recv: number | null }>()

  $effect(() => {
    const sid = focusedRuntimeSessionId
    if (!sid) {
      focusedMss = null
      return
    }
    const cached = mssBySession.get(sid)
    if (cached) {
      focusedMss = cached
    } else {
      getSessionTcpMss(sid)
        .then((m) => {
          mssBySession.set(sid, m)
          if (focusedRuntimeSessionId === sid) focusedMss = m
        })
        .catch(() => {
          focusedMss = null
        })
    }
  })

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
    }).then((fn) => {
      unlistenStats = fn
    })

    // Live MSS updates — backend emits when the value changes (typically
    // because Path-MTU Discovery revised the negotiated size). We cache by
    // session so navigating away and back retains the latest known value.
    let unlistenMss: UnlistenFn | null = null
    listen<{ session_id: string; mss: { send: number | null; recv: number | null } }>(
      'session-mss-updated',
      (e) => {
        mssBySession.set(e.payload.session_id, e.payload.mss)
        if (focusedRuntimeSessionId === e.payload.session_id) {
          focusedMss = e.payload.mss
        }
      },
    ).then((fn) => {
      unlistenMss = fn
    })

    // Platform auto-detect — backend emits once per session the first time
    // it latches onto a recognisable prompt. Surface a toast and refresh
    // the saved-session list so the new platform shows up in EditSession
    // without the user having to reload the panel.
    let unlistenPlatform: UnlistenFn | null = null
    listen<{ session_id: string; platform: string; display_name: string }>(
      'session-platform-detected',
      async (e) => {
        showToast({
          kind: 'info',
          title: 'Platform detected',
          detail: e.payload.display_name,
        })
        // Refresh sessions so the updated platform is visible immediately.
        listSessions()
          .then((v) => (sessions = v))
          .catch(() => {})
        // If the detection landed on the focused session, refresh the bar with
        // the new platform — that's what surfaces the seeded defaults. Match by
        // RUNTIME session id so this also works for unsaved/quick-connect
        // sessions (which have no saved-session row).
        const focusedRuntimeSid = paneToSession.get(focusedPaneId) ?? null
        if (focusedRuntimeSid === e.payload.session_id) {
          const savedSid = focusedPaneData?.savedSession?.id ?? null
          await setBarContext(e.payload.platform, savedSid)
        }
      },
    ).then((fn) => {
      unlistenPlatform = fn
    })

    // SFTP "edit remote file": the backend re-uploads on each save and emits
    // these so the user gets feedback that their edit landed.
    let unlistenEditSaved: UnlistenFn | null = null
    listen<string>('sftp-edit-saved', (e) => {
      showToast({ kind: 'success', title: 'Uploaded', detail: e.payload })
    }).then((fn) => {
      unlistenEditSaved = fn
    })
    let unlistenEditError: UnlistenFn | null = null
    listen<string>('sftp-edit-error', (e) => {
      showToast({ kind: 'error', title: 'Upload failed', detail: e.payload })
    }).then((fn) => {
      unlistenEditError = fn
    })

    return () => {
      document.removeEventListener('keydown', onKeydown)
      if (unlistenStats) unlistenStats()
      if (unlistenPlatform) unlistenPlatform()
      if (unlistenMss) unlistenMss()
      if (unlistenEditSaved) unlistenEditSaved()
      if (unlistenEditError) unlistenEditError()
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
    onNewSession={() => (showNewSession = true)}
    onQuickConnect={() => (showQuickConnect = true)}
    onSettings={() => (showSettings = true)}
    onSnippets={() => (showSnippets = true)}
    onCheckUpdates={handleCheckUpdates}
    onSetTheme={(k) => setTheme(k)}
    onAbout={() => (showAbout = true)}
    onExport={handleExport}
    onImport={handleImport}
    onImportSshConfig={handleImportSshConfig}
    onImportPutty={handleImportPutty}
    onImportMobaXterm={handleImportMobaXterm}
    onImportSecureCrt={handleImportSecureCrt}
    onCommandList={() => (showCommandList = true)}
    onTunnelsManager={() => (showTunnelsManager = true)}
  />

  <div class="pylon-body">
    <Sidebar
      {theme}
      {sessions}
      {folders}
      activeSessionId={focusedPaneData?.savedSession?.id}
      {sessionStatuses}
      onSelect={openSavedSessionTab}
      onEdit={(s) => (editingSession = s)}
      onDuplicate={async (s) => {
        try {
          await cloneSavedSession(s.id)
          await load()
          showToast({ kind: 'success', title: 'Session duplicated', detail: `${s.name} (copy)` })
        } catch (e) {
          showToast({
            kind: 'error',
            title: 'Duplicate failed',
            detail: e instanceof Error ? e.message : String(e),
          })
        }
      }}
      onDelete={async (s) => {
        const ok = await ask(`Delete "${s.name}"? This can't be undone.`, {
          title: 'Delete session',
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
          title: 'New folder',
          placeholder: 'My folder',
          submitLabel: 'Create',
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
          title: `Rename "${folder.name}"`,
          initial: folder.name,
          submitLabel: 'Rename',
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
        const ok = await ask(`Delete folder "${folder.name}"? Its sessions move to the root.`, {
          title: 'Delete folder',
          kind: 'warning',
        })
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
      onReorder={async (sessionId, folderId, targetIndex) => {
        try {
          await reorderSavedSession(sessionId, folderId, targetIndex)
          await load()
        } catch (e) {
          console.error('reorder session failed:', e)
        }
      }}
      onAddSession={() => (showNewSession = true)}
      onSettings={() => (showSettings = true)}
      onToggleTheme={toggleTheme}
    />

    <div class="pylon-workspace" style:background={theme.bodyBg}>
      <TabBar
        {theme}
        tabs={tabEntries}
        {activeTabId}
        {splitOn}
        {multiOn}
        onActivate={activateTab}
        onAdd={addLocalTab}
        onClose={closeTab}
        onRename={renameTab}
        onDuplicate={duplicateTab}
        onSetColor={setTabColor}
        onToggleSplit={handleSplitFocused}
        onToggleMulti={() => (multiOn = !multiOn)}
      />

      <div class="pylon-panes">
        {#each tabs as tab (tab.id)}
          <!-- All tabs stay mounted; inactive ones are hidden with CSS
               (display:none). This keeps each tab's xterm instance — and its
               backend session — alive across tab switches, so scrollback and
               the running shell survive instead of being torn down and
               re-opened (which looked like a `clear`). -->
          <div class="pane-slot" class:hidden={tab.id !== activeTabId}>
            {#if tab.layout.kind === 'grid'}
              <GridView
                panes={tab.layout.panes}
                cols={tab.layout.cols}
                {theme}
                {focusedPaneId}
                onFocus={(id) => (focusedPaneId = id)}
                onClosePane={(id) => handleClosePane(id)}
                onStatusChange={(id, s) => handleStatusChange(tab.id, id, s)}
                onGlobalShortcut={handleGlobalShortcut}
              />
            {:else}
              <SplitTree
                node={tab.layout.root}
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

      {#if multiOn && activeTab?.layout.kind === 'split'}
        <!-- Multi-exec on a regular tab: same master bar as grid mode,
             broadcasting to every live pane of the active tab, with the same
             run-and-compare audit. -->
        <MasterInputBar
          {theme}
          sessionIds={activeTabSessionIds}
          onRunCompare={runCompareActiveTab}
        />
      {/if}

      <SnippetButtonBar {theme} />

      <StatusBar
        {theme}
        status={focusedStatus}
        host={statusHost || undefined}
        port={statusPort ?? undefined}
        protocol={statusProtocol}
        bytesPerSec={focusedRuntimeSessionId
          ? bytesPerSecBySession[focusedRuntimeSessionId]
          : undefined}
        mss={focusedMss}
      />
    </div>
  </div>
</div>

<!-- ── Dialogs ──────────────────────────────────────────────────────────────── -->
{#if showNewSession}
  <NewSessionDialog
    {folders}
    onCancel={() => (showNewSession = false)}
    onCreated={(_id) => handleSessionAdded()}
  />
{/if}

{#if editingSession}
  <NewSessionDialog
    {folders}
    existing={editingSession}
    onCancel={() => (editingSession = null)}
    onCreated={() => {
      editingSession = null
      load()
    }}
  />
{/if}

{#if showQuickConnect}
  <QuickConnectDialog onCancel={() => (showQuickConnect = false)} onConnect={handleQuickConnect} />
{/if}

<KeyboardInteractiveDialog />

{#if showSnippets}
  <SnippetsDialog onClose={() => (showSnippets = false)} />
{/if}

{#if showGroups}
  <GroupsDialog
    {sessions}
    onClose={() => (showGroups = false)}
    onOpenGrid={(g) => {
      showGroups = false
      openGroupAsGrid(g)
    }}
  />
{/if}

{#if showCommandList}
  <CommandListDialog targets={liveTargets} onClose={() => (showCommandList = false)} />
{/if}

{#if showComparePanel}
  <ComparisonPanel
    {theme}
    command={compareRunner.command}
    hosts={compareRunner.hosts}
    running={compareRunner.running}
    onClose={() => {
      showComparePanel = false
      compareRunner.reset()
    }}
  />
{/if}

{#if showTunnelsManager}
  <TunnelsManagerDialog labels={sessionLabels} onClose={() => (showTunnelsManager = false)} />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
{/if}

{#if showPalette}
  <CommandPalette items={paletteItems} onClose={() => (showPalette = false)} />
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
<VariablesDialog />

{#if showAbout}
  <div
    class="about-backdrop"
    onclick={() => (showAbout = false)}
    onkeydown={(e) => {
      if (e.key === 'Escape') showAbout = false
    }}
    role="presentation"
    tabindex="-1"
  >
    <div
      class="about-modal"
      style:background={theme.sidebarBg}
      style:border="1px solid {theme.border}"
      style:font-family={theme.fontUi}
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="about-header" style:border-bottom="1px solid {theme.border}">
        <MarkTile size={20} accent={theme.accent} paper={theme.appBg} />
        <span style:color={theme.textPrimary} style:font-size="15px" style:font-weight="600"
          >ZAPX</span
        >
        <button class="about-close" style:color={theme.textDim} onclick={() => (showAbout = false)}
          ><Icon name="x" size={13} /></button
        >
      </div>
      <div class="about-body">
        <p style:color={theme.textMuted} style:font-size="12.5px" style:line-height="1.7">
          A modern multiprotocol terminal for network engineers.<br />
          Built with Tauri · Svelte 5 · xterm.js · Rust.
        </p>
        <table class="about-table" style:color={theme.textMuted}>
          <tbody>
            <tr
              ><td style:color={theme.textDim}>Version</td><td style:color={theme.textPrimary}
                >{appVersion || '—'}</td
              ></tr
            >
            <tr><td style:color={theme.textDim}>Runtime</td><td>Tauri 2 · WebView</td></tr>
            <tr
              ><td style:color={theme.textDim}>Theme</td><td style:color={theme.accent}
                >{themeLabels[theme.name] ?? theme.name}</td
              ></tr
            >
          </tbody>
        </table>
        <p class="about-hint" style:color={theme.textDim} style:font-family={theme.fontMono}>
          Credentials stored in the OS keyring; portable mode encrypts them with a local key file
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

  /* Terminal zone — the cards float on the body background with a 14px
     frame around them (handoff: «terminal como tarjeta flotante»). */
  .pylon-panes {
    flex: 1;
    display: flex;
    min-height: 0;
    overflow: hidden;
    position: relative;
    padding: 14px;
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
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(4px);
    z-index: 300;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .about-modal {
    width: 360px;
    border-radius: 8px;
    box-shadow: var(--zx-shadow);
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

  .about-close:hover {
    background: var(--zx-hover-bg);
  }

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
