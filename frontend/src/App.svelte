<script lang="ts">
  import { onMount } from 'svelte'
  import SplitView from '$lib/terminal/SplitView.svelte'
  import type { SplitNode } from '$lib/terminal/SplitView.svelte'
  import type { PaneInfo, PaneStatus } from '$lib/terminal/TerminalPane.svelte'
  import SessionTree from '$lib/sessions/SessionTree.svelte'
  import NewSessionDialog from '$lib/sessions/NewSessionDialog.svelte'
  import QuickConnectDialog from '$lib/sessions/QuickConnectDialog.svelte'
  import type { ConnectParams } from '$lib/sessions/QuickConnectDialog.svelte'
  import SettingsModal from '$lib/settings/SettingsModal.svelte'
  import { listFolders } from '$lib/bridge/commands'
  import { loadSettings } from '$lib/stores/settings.svelte'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  // ── types ───────────────────────────────────────────────────────────────────

  interface Tab {
    id: number
    label: string
    root: SplitNode
    panes: Map<number, PaneInfo>
    activePaneId: number
    paneStatuses: Map<number, PaneStatus>
  }

  // ── state ───────────────────────────────────────────────────────────────────

  let nextId = 1

  function mkPane(label: string, opts?: Partial<Omit<PaneInfo, 'id' | 'label'>>): PaneInfo {
    return { id: nextId++, label, ...opts }
  }

  function mkTab(pane: PaneInfo): Tab {
    return {
      id: nextId++,
      label: pane.label,
      root: { kind: 'leaf', paneId: pane.id },
      panes: new Map([[pane.id, pane]]),
      activePaneId: pane.id,
      paneStatuses: new Map([[pane.id, 'connecting']]),
    }
  }

  const firstPane = mkPane('shell')
  const firstTab = mkTab(firstPane)
  let tabs = $state<Tab[]>([firstTab])
  let activeTabId = $state(firstTab.id)
  let showNewSession = $state(false)
  let showQuickConnect = $state(false)
  let showSettings = $state(false)
  let folders = $state<Folder[]>([])
  let sessionTreeKey = $state(0)

  const activeTab = $derived(tabs.find((t) => t.id === activeTabId)!)

  // ── status helpers ──────────────────────────────────────────────────────────

  function tabStatusColor(tab: Tab): string {
    const statuses = [...tab.paneStatuses.values()]
    if (statuses.some((s) => s === 'error')) return '#ef4444'
    if (statuses.some((s) => s === 'connecting')) return '#f59e0b'
    if (statuses.every((s) => s === 'closed')) return '#52525b'
    return '#22c55e'
  }

  function handleStatusChange(tabId: number, paneId: number, status: PaneStatus) {
    const tab = tabs.find((t) => t.id === tabId)
    if (!tab) return
    tab.paneStatuses.set(paneId, status)
    tabs = tabs // trigger reactivity
  }

  // ── active pane status bar info ─────────────────────────────────────────────

  const activePaneInfo = $derived(activeTab?.panes.get(activeTab.activePaneId))
  const activePaneStatus = $derived(activeTab?.paneStatuses.get(activeTab.activePaneId) ?? 'connecting')

  const statusBarLeft = $derived(() => {
    if (activePaneStatus === 'connecting') return 'Connecting…'
    if (activePaneStatus === 'error') return 'Connection failed'
    if (activePaneStatus === 'closed') return 'Disconnected'
    return 'Ready'
  })

  const statusBarRight = $derived(() => {
    if (!activePaneInfo) return ''
    const p = activePaneInfo
    if (p.ssh) return `SSH: ${p.ssh.host}:${p.ssh.port}`
    if (p.telnet) return `Telnet: ${p.telnet.host}:${p.telnet.port}`
    if (p.savedSession) {
      const s = p.savedSession
      if (s.protocol === 'ssh' && s.host) return `SSH: ${s.host}:${s.port ?? 22}`
      if (s.protocol === 'telnet' && s.host) return `Telnet: ${s.host}:${s.port ?? 23}`
      if (s.protocol === 'serial') return `Serial`
    }
    return 'Local'
  })

  // ── settings ────────────────────────────────────────────────────────────────

  $effect(() => { loadSettings() })

  // ── keyboard shortcuts ──────────────────────────────────────────────────────

  onMount(() => {
    function handleKeydown(e: KeyboardEvent) {
      if (!e.ctrlKey) return
      switch (e.key) {
        case 'n': e.preventDefault(); showQuickConnect = true; break
        case 't': e.preventDefault(); addLocalTab(); break
        case 'w': e.preventDefault(); closeActiveTab(); break
        case 'Tab': e.preventDefault(); cycleTab(e.shiftKey ? -1 : 1); break
        case ',': e.preventDefault(); showSettings = true; break
      }
    }
    document.addEventListener('keydown', handleKeydown)
    return () => document.removeEventListener('keydown', handleKeydown)
  })

  function handleGlobalShortcut(key: string, e: KeyboardEvent) {
    switch (key) {
      case 'n': showQuickConnect = true; break
      case 't': addLocalTab(); break
      case 'w': closeActiveTab(); break
      case 'Tab': cycleTab(e.shiftKey ? -1 : 1); break
      case ',': showSettings = true; break
    }
  }

  function cycleTab(dir: 1 | -1) {
    const idx = tabs.findIndex((t) => t.id === activeTabId)
    const next = tabs[(idx + dir + tabs.length) % tabs.length]
    if (next) activeTabId = next.id
  }

  // ── tab management ──────────────────────────────────────────────────────────

  function addLocalTab() {
    const p = mkPane('shell')
    const t = mkTab(p)
    tabs = [...tabs, t]
    activeTabId = t.id
  }

  function closeActiveTab() {
    if (tabs.length <= 1) return
    closeTab(activeTabId)
  }

  function closeTab(id: number) {
    tabs = tabs.filter((t) => t.id !== id)
    if (activeTabId === id) activeTabId = tabs.at(-1)?.id ?? 0
  }

  function openSavedSession(s: SavedSession) {
    const p = mkPane(s.name, { savedSession: s })
    const t = mkTab(p)
    t.label = s.name
    tabs = [...tabs, t]
    activeTabId = t.id
  }

  function handleQuickConnect(params: ConnectParams) {
    showQuickConnect = false
    let p: PaneInfo
    if (params.type === 'local') {
      p = mkPane('shell')
    } else if (params.type === 'ssh') {
      p = mkPane(params.host, { ssh: { host: params.host, port: params.port, user: params.user, password: params.password } })
    } else {
      p = mkPane(`telnet:${params.host}`, { telnet: { host: params.host, port: params.port } })
    }
    const t = mkTab(p)
    tabs = [...tabs, t]
    activeTabId = t.id
  }

  // ── split management ────────────────────────────────────────────────────────

  function countLeaves(node: SplitNode): number {
    if (node.kind === 'leaf') return 1
    if (node.kind === 'h') return countLeaves(node.left) + countLeaves(node.right)
    return countLeaves(node.top) + countLeaves(node.bottom)
  }

  function splitPane(tabId: number, paneId: number, dir: 'h' | 'v') {
    const tab = tabs.find((t) => t.id === tabId)
    if (!tab) return
    const newPane = mkPane('shell')
    tab.panes.set(newPane.id, newPane)
    tab.paneStatuses.set(newPane.id, 'connecting')
    tab.root = insertSplit(tab.root, paneId, newPane.id, dir)
    tab.activePaneId = newPane.id
    tabs = [...tabs]
  }

  function insertSplit(node: SplitNode, targetId: number, newId: number, dir: 'h' | 'v'): SplitNode {
    if (node.kind === 'leaf') {
      if (node.paneId !== targetId) return node
      const newLeaf: SplitNode = { kind: 'leaf', paneId: newId }
      if (dir === 'h') return { kind: 'h', left: node, right: newLeaf, ratio: 0.5 }
      return { kind: 'v', top: node, bottom: newLeaf, ratio: 0.5 }
    }
    if (node.kind === 'h') return { ...node, left: insertSplit(node.left, targetId, newId, dir), right: insertSplit(node.right, targetId, newId, dir) }
    return { ...node, top: insertSplit(node.top, targetId, newId, dir), bottom: insertSplit(node.bottom, targetId, newId, dir) }
  }

  function closePane(tabId: number, paneId: number) {
    const tab = tabs.find((t) => t.id === tabId)
    if (!tab) return
    if (tab.panes.size <= 1) {
      closeTab(tabId)
      return
    }
    const [newRoot, siblingId] = removeLeaf(tab.root, paneId)
    tab.panes.delete(paneId)
    tab.paneStatuses.delete(paneId)
    if (newRoot) tab.root = newRoot
    if (tab.activePaneId === paneId && siblingId !== null) tab.activePaneId = siblingId
    tabs = [...tabs]
  }

  function removeLeaf(node: SplitNode, targetId: number): [SplitNode | null, number | null] {
    if (node.kind === 'leaf') {
      return node.paneId === targetId ? [null, null] : [node, null]
    }
    if (node.kind === 'h') {
      if (hasLeaf(node.left, targetId)) {
        const siblingId = firstLeafId(node.right)
        return [node.right, siblingId]
      }
      if (hasLeaf(node.right, targetId)) {
        const siblingId = firstLeafId(node.left)
        return [node.left, siblingId]
      }
      const [newLeft, sid1] = removeLeaf(node.left, targetId)
      const [newRight, sid2] = removeLeaf(node.right, targetId)
      return [{ ...node, left: newLeft ?? node.left, right: newRight ?? node.right }, sid1 ?? sid2]
    }
    // v split
    if (hasLeaf(node.top, targetId)) {
      const siblingId = firstLeafId(node.bottom)
      return [node.bottom, siblingId]
    }
    if (hasLeaf(node.bottom, targetId)) {
      const siblingId = firstLeafId(node.top)
      return [node.top, siblingId]
    }
    const [newTop, sid3] = removeLeaf(node.top, targetId)
    const [newBottom, sid4] = removeLeaf(node.bottom, targetId)
    return [{ ...node, top: newTop ?? node.top, bottom: newBottom ?? node.bottom }, sid3 ?? sid4]
  }

  function hasLeaf(node: SplitNode, id: number): boolean {
    if (node.kind === 'leaf') return node.paneId === id
    if (node.kind === 'h') return hasLeaf(node.left, id) || hasLeaf(node.right, id)
    return hasLeaf(node.top, id) || hasLeaf(node.bottom, id)
  }

  function firstLeafId(node: SplitNode): number {
    if (node.kind === 'leaf') return node.paneId
    if (node.kind === 'h') return firstLeafId(node.left)
    return firstLeafId(node.top)
  }

  // ── folders ─────────────────────────────────────────────────────────────────

  async function loadFolders() {
    try { folders = await listFolders() } catch { /* non-fatal */ }
  }

  function openNewSessionDialog() {
    loadFolders()
    showNewSession = true
  }

  function handleSessionCreated() {
    showNewSession = false
    sessionTreeKey += 1
  }
</script>

<div class="app">

  <!-- ── title bar ─────────────────────────────────────────────────────────── -->
  <header class="titlebar">
    <span class="app-brand">zapx</span>

    <div class="tab-strip">
      {#each tabs as tab (tab.id)}
        <button
          class="tab"
          class:tab-active={activeTabId === tab.id}
          onclick={() => (activeTabId = tab.id)}
        >
          <span class="tab-dot" style:background={tabStatusColor(tab)}></span>
          <span class="tab-label">{tab.label}</span>
          {#if tabs.length > 1}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <span
              class="tab-close"
              role="button"
              tabindex="-1"
              onclick={(e) => { e.stopPropagation(); closeTab(tab.id) }}
            >×</span>
          {/if}
        </button>
      {/each}
      <button class="tab-new" onclick={addLocalTab} title="New tab (Ctrl+T)">+</button>
    </div>

    <div class="titlebar-actions">
      <button class="action-btn" onclick={() => (showQuickConnect = true)} title="Quick Connect (Ctrl+N)">
        ⚡ Connect
      </button>
      <button class="action-btn" onclick={() => (showSettings = true)} title="Settings (Ctrl+,)">
        ⚙
      </button>
    </div>
  </header>

  <!-- ── body ──────────────────────────────────────────────────────────────── -->
  <div class="body">

    <!-- Session Manager sidebar (SecureCRT style) -->
    <aside class="session-manager">
      <div class="sm-header">
        <span class="sm-title">Active Sessions</span>
        <button class="sm-add" onclick={openNewSessionDialog} title="New session">+</button>
      </div>

      {#key sessionTreeKey}
        <SessionTree onOpen={openSavedSession} onAdd={openNewSessionDialog} />
      {/key}
    </aside>

    <!-- Main terminal area -->
    <main class="terminal-area">
      {#each tabs as tab (tab.id)}
        <div class="tab-content" class:hidden={activeTabId !== tab.id}>
          {#if activeTabId === tab.id}
            <SplitView
              node={tab.root}
              panes={tab.panes}
              activePaneId={tab.activePaneId}
              totalPanes={countLeaves(tab.root)}
              onActivate={(pid) => { tab.activePaneId = pid; tabs = [...tabs] }}
              onSplitH={(pid) => splitPane(tab.id, pid, 'h')}
              onSplitV={(pid) => splitPane(tab.id, pid, 'v')}
              onClosePane={(pid) => closePane(tab.id, pid)}
              onGlobalShortcut={handleGlobalShortcut}
              onStatusChange={(pid, s) => handleStatusChange(tab.id, pid, s)}
            />
          {/if}
        </div>
      {/each}
    </main>

  </div>

  <!-- ── status bar (SecureCRT style) ─────────────────────────────────────── -->
  <footer class="statusbar">
    <span
      class="status-indicator"
      style:color={
        activePaneStatus === 'connected' ? '#22c55e'
        : activePaneStatus === 'error' ? '#ef4444'
        : activePaneStatus === 'closed' ? '#52525b'
        : '#f59e0b'
      }
    >●</span>
    <span class="status-text">{statusBarLeft()}</span>
    <span class="status-sep">|</span>
    <span class="status-session">{activePaneInfo?.label ?? '—'}</span>
    <span class="flex-1"></span>
    <span class="status-right">{statusBarRight()}</span>
  </footer>

</div>

<!-- ── dialogs ──────────────────────────────────────────────────────────────── -->

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
  /* ── global reset ────────────────────────────────────────────────────────── */
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    font-size: 13px;
    background: #0a0a0a;
    color: #d4d4d4;
    overflow: hidden;
  }

  /* ── app shell ───────────────────────────────────────────────────────────── */
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #0a0a0a;
    color: #d4d4d4;
  }

  /* ── title bar / tab strip ───────────────────────────────────────────────── */
  .titlebar {
    display: flex;
    align-items: center;
    height: 2rem;
    background: #161616;
    border-bottom: 1px solid #2a2a2a;
    flex-shrink: 0;
    user-select: none;
    gap: 0;
  }

  .app-brand {
    font-size: 0.75rem;
    font-weight: 700;
    color: #4a9eff;
    padding: 0 0.75rem;
    letter-spacing: 0.08em;
    flex-shrink: 0;
    border-right: 1px solid #2a2a2a;
    height: 100%;
    display: flex;
    align-items: center;
  }

  .tab-strip {
    display: flex;
    align-items: stretch;
    flex: 1;
    overflow: hidden;
    height: 100%;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0 0.65rem;
    height: 100%;
    font-size: 0.75rem;
    border: none;
    border-right: 1px solid #2a2a2a;
    cursor: pointer;
    background: transparent;
    color: #71717a;
    font-family: inherit;
    white-space: nowrap;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
  }

  .tab:hover {
    background: #1e1e1e;
    color: #d4d4d4;
  }

  .tab-active {
    background: #1a2a3a !important;
    color: #e4e4e7 !important;
    border-bottom: 2px solid #4a9eff;
  }

  .tab-dot {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tab-label {
    max-width: 8rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-close {
    font-size: 0.85rem;
    color: #52525b;
    padding: 0 0.1rem;
    margin-left: 0.1rem;
  }

  .tab-close:hover {
    color: #ef4444;
  }

  .tab-new {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #52525b;
    font-size: 1rem;
    padding: 0 0.5rem;
    height: 100%;
    font-family: inherit;
    border-right: 1px solid #2a2a2a;
  }

  .tab-new:hover {
    color: #d4d4d4;
    background: #1e1e1e;
  }

  .titlebar-actions {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0 0.5rem;
    height: 100%;
    border-left: 1px solid #2a2a2a;
    flex-shrink: 0;
  }

  .action-btn {
    background: transparent;
    border: 1px solid #2a2a2a;
    border-radius: 0.2rem;
    cursor: pointer;
    color: #71717a;
    font-size: 0.7rem;
    padding: 0.1rem 0.5rem;
    font-family: inherit;
    white-space: nowrap;
  }

  .action-btn:hover {
    color: #d4d4d4;
    border-color: #4a9eff;
    background: #1a2a3a;
  }

  /* ── body ────────────────────────────────────────────────────────────────── */
  .body {
    flex: 1;
    display: flex;
    overflow: hidden;
    min-height: 0;
  }

  /* ── session manager (SecureCRT left panel) ──────────────────────────────── */
  .session-manager {
    width: 180px;
    min-width: 180px;
    max-width: 180px;
    display: flex;
    flex-direction: column;
    background: #111111;
    border-right: 1px solid #2a2a2a;
    overflow: hidden;
    flex-shrink: 0;
  }

  .sm-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.3rem 0.5rem;
    background: #161616;
    border-bottom: 1px solid #2a2a2a;
    flex-shrink: 0;
  }

  .sm-title {
    font-size: 0.68rem;
    font-weight: 600;
    color: #71717a;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .sm-add {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #52525b;
    font-size: 1rem;
    line-height: 1;
    padding: 0 0.2rem;
    font-family: inherit;
  }

  .sm-add:hover {
    color: #4a9eff;
  }

  /* ── terminal area ───────────────────────────────────────────────────────── */
  .terminal-area {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .tab-content.hidden {
    display: none;
  }

  /* ── status bar (SecureCRT style) ────────────────────────────────────────── */
  .statusbar {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    height: 1.4rem;
    padding: 0 0.6rem;
    background: #161616;
    border-top: 1px solid #2a2a2a;
    flex-shrink: 0;
    font-size: 0.68rem;
    color: #52525b;
    user-select: none;
  }

  .status-indicator {
    font-size: 0.55rem;
    line-height: 1;
  }

  .status-text {
    color: #71717a;
  }

  .status-sep {
    color: #3a3a3a;
  }

  .status-session {
    color: #71717a;
  }

  .flex-1 {
    flex: 1;
  }

  .status-right {
    color: #52525b;
    font-family: 'Consolas', monospace;
  }
</style>
