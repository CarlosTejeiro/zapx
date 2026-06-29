<script lang="ts">
  import type { PylonTheme } from '$lib/themes/index'
  import type { SavedSession, Folder } from '$lib/bridge/types'
  import Icon from '$lib/icons/Icon.svelte'
  import SessionAvatar from '$lib/pylon/SessionAvatar.svelte'

  interface Props {
    theme: PylonTheme
    sessions: SavedSession[]
    folders: Folder[]
    activeSessionId?: number
    /** Live status per saved-session id → status dot on the avatar
     *  (connecting = amber, connected = green, error = red; absent = no dot).
     *  The focused session additionally gets the accent ring. */
    sessionStatuses?: Map<number, 'connecting' | 'connected' | 'error'>
    onSelect: (session: SavedSession) => void
    onEdit?: (session: SavedSession) => void
    onDuplicate?: (session: SavedSession) => void
    onDelete?: (session: SavedSession) => void
    onCreateFolder?: () => void
    onRenameFolder?: (folder: Folder) => void
    onDeleteFolder?: (folder: Folder) => void
    /** Reparent a session via drag-and-drop. `folderId = null` drops at root. */
    onMove?: (sessionId: number, folderId: number | null) => void
    /** Reorder a session inside its parent. `targetIndex` is the 0-based slot
     *  among the destination's children AFTER the move. Also used for cross-
     *  folder drops onto a specific row. */
    onReorder?: (sessionId: number, folderId: number | null, targetIndex: number) => void
    onAddSession?: () => void
    onSettings?: () => void
    onToggleTheme?: () => void
  }

  const {
    theme,
    sessions,
    folders,
    activeSessionId,
    sessionStatuses,
    onSelect,
    onEdit,
    onDuplicate,
    onDelete,
    onCreateFolder,
    onRenameFolder,
    onDeleteFolder,
    onMove,
    onReorder,
    onAddSession,
    onSettings,
    onToggleTheme,
  }: Props = $props()

  // ── Drag-and-drop (native HTML5) ────────────────────────────────────────
  /// Session id currently being dragged. `dragOver` is the drop target's id
  /// (`null` = root). Both reset on dragend.
  let draggingSessionId = $state<number | null>(null)
  let dragOver = $state<number | null | 'none'>('none')
  /// When hovering a session row, this records whether the indicator should
  /// be drawn above (before) or below (after) it. Cleared on dragleave/drop.
  let rowDropTarget = $state<{ sessionId: number; position: 'before' | 'after' } | null>(null)

  function onDragStart(e: DragEvent, sessionId: number) {
    draggingSessionId = sessionId
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move'
      // Pass via DataTransfer too so we can survive cross-frame edge cases.
      e.dataTransfer.setData('application/x-zapx-session', String(sessionId))
    }
  }

  function onDragOver(e: DragEvent, targetFolderId: number | null) {
    if (draggingSessionId == null) return
    e.preventDefault()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
    dragOver = targetFolderId
  }

  /// Per-row dragover handler: pick "insert before" vs "insert after" based
  /// on whether the cursor is in the upper or lower half of the row, and
  /// render an indicator line. Stops propagation so the parent section's
  /// handler doesn't overwrite the visual feedback.
  function onRowDragOver(e: DragEvent, sessionId: number) {
    if (draggingSessionId == null || draggingSessionId === sessionId) return
    e.preventDefault()
    e.stopPropagation()
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
    const position: 'before' | 'after' = e.clientY < rect.top + rect.height / 2 ? 'before' : 'after'
    if (rowDropTarget?.sessionId !== sessionId || rowDropTarget?.position !== position) {
      rowDropTarget = { sessionId, position }
    }
  }

  function onRowDragLeave(sessionId: number) {
    if (rowDropTarget?.sessionId === sessionId) rowDropTarget = null
  }

  /// Per-row drop: dispatch a reorder request with the explicit target index.
  /// Falls back to `onMove` (legacy) when no reorder consumer is attached.
  function onRowDrop(e: DragEvent, targetFolderId: number | null, targetSessionId: number) {
    if (draggingSessionId == null) return
    e.preventDefault()
    e.stopPropagation()
    const id = draggingSessionId
    const position = rowDropTarget?.position ?? 'after'

    // Compute peers in the destination AFTER removing the dragged row.
    const peers = sessions
      .filter((s) => (s.folder_id ?? null) === targetFolderId && s.id !== id)
      .map((s) => s.id)
    let idx = peers.indexOf(targetSessionId)
    if (idx < 0) idx = peers.length
    const targetIndex = position === 'before' ? idx : idx + 1

    if (onReorder) {
      onReorder(id, targetFolderId, targetIndex)
    } else {
      // Backwards-compat fallback: reparent only.
      const current = sessions.find((s) => s.id === id)?.folder_id ?? null
      if (current !== targetFolderId) onMove?.(id, targetFolderId)
    }
    resetDrag()
  }

  function onDrop(e: DragEvent, targetFolderId: number | null) {
    if (draggingSessionId == null) return
    e.preventDefault()
    const id = draggingSessionId
    const current = sessions.find((s) => s.id === id)?.folder_id ?? null
    // Dropped on the section background (not on a specific row) → append.
    if (onReorder) {
      const peers = sessions.filter((s) => (s.folder_id ?? null) === targetFolderId && s.id !== id)
      onReorder(id, targetFolderId, peers.length)
    } else if (current !== targetFolderId) {
      onMove?.(id, targetFolderId)
    }
    resetDrag()
  }

  function onDragEnd() {
    resetDrag()
  }

  function resetDrag() {
    draggingSessionId = null
    dragOver = 'none'
    rowDropTarget = null
  }

  let search = $state('')
  let expandedSections = $state<Set<string>>(new Set(['pinned', 'recent', 'sessions']))
  let expandedFolders = $state<Set<number>>(new Set())

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

  function sessionColor(s: SavedSession): string {
    return SESSION_COLORS[s.id % SESSION_COLORS.length] as string
  }

  /// Tags + notes live inside the per-session `options_json` blob (no schema
  /// change). Parse defensively — a malformed blob just yields no metadata.
  function sessionMeta(s: SavedSession): { tags: string[]; notes: string } {
    try {
      const o = JSON.parse(s.options_json || '{}')
      const tags = Array.isArray(o.tags)
        ? o.tags.filter((t: unknown): t is string => typeof t === 'string')
        : []
      return { tags, notes: typeof o.notes === 'string' ? o.notes : '' }
    } catch {
      return { tags: [], notes: '' }
    }
  }

  const query = $derived(search.toLowerCase())

  /// Match a session by name, tags or notes so a search like "prod" surfaces
  /// every tagged host even when the word isn't in the name.
  function matchesQuery(s: SavedSession): boolean {
    if (!query) return true
    if (s.name.toLowerCase().includes(query)) return true
    const { tags, notes } = sessionMeta(s)
    return tags.some((t) => t.toLowerCase().includes(query)) || notes.toLowerCase().includes(query)
  }

  const rootSessions = $derived(sessions.filter((s) => s.folder_id === null && matchesQuery(s)))

  /// Most-recently-opened sessions (by `last_used_at`, bumped on every open),
  /// surfaced as a quick-access section. Hidden while searching — the main
  /// list already covers search. ISO-ish timestamps sort lexicographically.
  const recentSessions = $derived(
    query
      ? []
      : [...sessions]
          .filter((s) => s.last_used_at)
          .sort((a, b) => (b.last_used_at ?? '').localeCompare(a.last_used_at ?? ''))
          .slice(0, 5),
  )

  function sessionsInFolder(id: number): SavedSession[] {
    return sessions.filter((s) => s.folder_id === id && matchesQuery(s))
  }

  function toggleSection(key: string) {
    const next = new Set(expandedSections)
    if (next.has(key)) next.delete(key)
    else next.add(key)
    expandedSections = next
  }

  function toggleFolder(id: number) {
    const next = new Set(expandedFolders)
    if (next.has(id)) next.delete(id)
    else next.add(id)
    expandedFolders = next
  }

  // User initials chip (deterministic from username)
  const username = 'admin'
  const initials = username.slice(0, 2).toUpperCase()
</script>

<aside
  class="sidebar"
  style:background={theme.sidebarBg}
  style:border-right="1px solid {theme.border}"
  style:font-family={theme.fontUi}
  style:--item-hover-bg={theme.itemHoverBg}
  style:--text-primary={theme.textPrimary}
  style:--accent={theme.accent}
  style:--err={theme.err}
  style:--radius={theme.radius}
>
  <!-- Search -->
  <div class="sb-search-wrap">
    <div
      class="sb-search-inner"
      style:background={theme.appBg}
      style:border="1px solid {theme.border}"
      style:border-radius={theme.radius}
      style:color={theme.textDim}
    >
      <Icon name="search" size={13} />
      <input
        class="sb-search"
        type="text"
        placeholder="Search sessions…"
        bind:value={search}
        style:color={theme.textPrimary}
        style:font-family={theme.fontUi}
      />
      <kbd class="sb-kbd" style:border="1px solid {theme.border}" style:font-family={theme.fontMono}
        >⌘K</kbd
      >
    </div>
  </div>

  <!-- Tree -->
  <div class="sb-tree">
    <!-- Recent: quick access to the last-opened sessions. Simplified rows
         (avatar + name + status), no drag/actions — those live in the main
         list. Hidden while searching. -->
    {#if recentSessions.length > 0}
      <div class="sb-section">
        <div class="sb-section-row" style:color={theme.textDim}>
          <button
            class="sb-section-header"
            style:color={theme.textDim}
            onclick={() => toggleSection('recent')}
          >
            <Icon name="chevron" size={11} open={expandedSections.has('recent')} />
            RECENT
            <span class="sb-count" style:color={theme.textDim}>{recentSessions.length}</span>
          </button>
        </div>
        {#if expandedSections.has('recent')}
          <div class="sb-droparea">
            {#each recentSessions as s (s.id)}
              {@const isActive = s.id === activeSessionId}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="sb-row"
                class:active={isActive}
                role="button"
                tabindex="0"
                style:background={isActive ? theme.itemActiveBg : ''}
                title={sessionMeta(s).notes || undefined}
                onclick={() => onSelect(s)}
              >
                <SessionAvatar
                  name={s.name}
                  color={sessionColor(s)}
                  paper={theme.sidebarBg}
                  ink={theme.textPrimary}
                  accent={theme.accent}
                  active={isActive}
                  status={sessionStatuses?.get(s.id)}
                  ok={theme.ok}
                  warn={theme.warn}
                  err={theme.err}
                  size={22}
                />
                <span
                  class="sb-name"
                  class:active={isActive}
                  style:color={isActive ? theme.textPrimary : theme.textMuted}>{s.name}</span
                >
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Root sessions. Drop handlers live on the outer .sb-section so
         dropping at root works even when the section is collapsed. -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="sb-section"
      class:dragover={dragOver === null && draggingSessionId != null}
      ondragover={(e) => onDragOver(e, null)}
      ondrop={(e) => onDrop(e, null)}
      ondragenter={() => {
        // Auto-expand on hover so the user sees their drop registered.
        if (draggingSessionId != null && !expandedSections.has('sessions')) {
          const next = new Set(expandedSections)
          next.add('sessions')
          expandedSections = next
        }
      }}
    >
      <div class="sb-section-row" style:color={theme.textDim}>
        <button
          class="sb-section-header"
          style:color={theme.textDim}
          onclick={() => toggleSection('sessions')}
        >
          <Icon name="chevron" size={11} open={expandedSections.has('sessions')} />
          SESSIONS
          <span class="sb-count" style:color={theme.textDim}>{rootSessions.length}</span>
        </button>
        {#if onCreateFolder}
          <button
            class="sb-add-btn"
            title="New folder"
            style:color={theme.textDim}
            onclick={onCreateFolder}><Icon name="folder" size={13} /></button
          >
        {/if}
        <button
          class="sb-add-btn"
          title="New session"
          style:color={theme.textDim}
          onclick={onAddSession}><Icon name="plus" size={13} /></button
        >
      </div>

      {#if expandedSections.has('sessions')}
        <div class="sb-droparea">
          {#each rootSessions as s (s.id)}
            {@const color = sessionColor(s)}
            {@const isActive = s.id === activeSessionId}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
            <div
              class="sb-row"
              class:active={isActive}
              class:dragging={draggingSessionId === s.id}
              class:drop-before={rowDropTarget?.sessionId === s.id &&
                rowDropTarget?.position === 'before'}
              class:drop-after={rowDropTarget?.sessionId === s.id &&
                rowDropTarget?.position === 'after'}
              draggable="true"
              role="button"
              tabindex="0"
              ondragstart={(e) => onDragStart(e, s.id)}
              ondragend={onDragEnd}
              ondragover={(e) => onRowDragOver(e, s.id)}
              ondragleave={() => onRowDragLeave(s.id)}
              ondrop={(e) => onRowDrop(e, null, s.id)}
              style:background={isActive ? theme.itemActiveBg : ''}
              onclick={() => onSelect(s)}
            >
              <SessionAvatar
                name={s.name}
                {color}
                paper={theme.sidebarBg}
                ink={theme.textPrimary}
                accent={theme.accent}
                active={isActive}
                status={sessionStatuses?.get(s.id)}
                ok={theme.ok}
                warn={theme.warn}
                err={theme.err}
                size={22}
              />
              <span
                class="sb-name"
                class:active={isActive}
                style:color={isActive ? theme.textPrimary : theme.textMuted}
                title={sessionMeta(s).notes || undefined}>{s.name}</span
              >
              {#each sessionMeta(s).tags.slice(0, 3) as tag (tag)}
                <span
                  class="sb-tag sb-tagchip"
                  style:color={theme.accent}
                  style:border-color="color-mix(in srgb, {theme.accent} 40%, transparent)"
                  style:font-family={theme.fontUi}>{tag}</span
                >
              {/each}
              {#if s.protocol !== 'local' && s.protocol !== 'ssh'}
                <span
                  class="sb-tag"
                  style:color={theme.textDim}
                  style:border-color={theme.border}
                  style:font-family={theme.fontMono}
                >
                  {s.protocol.toUpperCase()}
                </span>
              {/if}
              {#if onEdit}
                <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                <span
                  class="sb-edit"
                  role="button"
                  title="Edit session"
                  style:color={theme.textDim}
                  onclick={(e) => {
                    e.stopPropagation()
                    onEdit?.(s)
                  }}><Icon name="pencil" size={12} /></span
                >
              {/if}
              {#if onDuplicate}
                <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                <span
                  class="sb-edit"
                  role="button"
                  title="Duplicate session"
                  style:color={theme.textDim}
                  onclick={(e) => {
                    e.stopPropagation()
                    onDuplicate?.(s)
                  }}><Icon name="copy" size={12} /></span
                >
              {/if}
              {#if onDelete}
                <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                <span
                  class="sb-edit sb-del"
                  role="button"
                  title="Delete session"
                  style:color={theme.textDim}
                  onclick={(e) => {
                    e.stopPropagation()
                    onDelete?.(s)
                  }}><Icon name="x" size={12} /></span
                >
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Folders -->
    {#each folders as folder (folder.id)}
      {@const folderSessions = sessionsInFolder(folder.id)}
      {#if !query || folderSessions.length > 0}
        <div
          class="sb-section"
          class:dragover={dragOver === folder.id && draggingSessionId != null}
          ondragover={(e) => onDragOver(e, folder.id)}
          ondrop={(e) => onDrop(e, folder.id)}
          ondragenter={() => {
            // Auto-expand on hover so the user can see the drop is registering.
            if (draggingSessionId != null && !expandedFolders.has(folder.id)) {
              const next = new Set(expandedFolders)
              next.add(folder.id)
              expandedFolders = next
            }
          }}
          role="group"
        >
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
          <div
            class="sb-folder-header"
            role="button"
            tabindex="0"
            style:color={theme.textDim}
            onclick={() => toggleFolder(folder.id)}
          >
            <Icon name="chevron" size={11} open={expandedFolders.has(folder.id)} />
            <Icon name="folder" size={13} />
            <span class="sb-folder-name">{folder.name.toUpperCase()}</span>
            <span class="sb-count" style:color={theme.textDim}>{folderSessions.length}</span>
            {#if onRenameFolder}
              <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
              <span
                class="sb-edit"
                role="button"
                title="Rename folder"
                style:color={theme.textDim}
                onclick={(e) => {
                  e.stopPropagation()
                  onRenameFolder?.(folder)
                }}><Icon name="pencil" size={12} /></span
              >
            {/if}
            {#if onDeleteFolder}
              <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
              <span
                class="sb-edit sb-del"
                role="button"
                title="Delete folder"
                style:color={theme.textDim}
                onclick={(e) => {
                  e.stopPropagation()
                  onDeleteFolder?.(folder)
                }}><Icon name="x" size={12} /></span
              >
            {/if}
          </div>

          {#if expandedFolders.has(folder.id)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="sb-droparea"
              class:dragover={dragOver === folder.id && draggingSessionId != null}
              ondragover={(e) => onDragOver(e, folder.id)}
              ondrop={(e) => onDrop(e, folder.id)}
            >
              {#each folderSessions as s (s.id)}
                {@const color = sessionColor(s)}
                {@const isActive = s.id === activeSessionId}
                <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
                <div
                  class="sb-row sb-row-indented"
                  class:active={isActive}
                  class:dragging={draggingSessionId === s.id}
                  class:drop-before={rowDropTarget?.sessionId === s.id &&
                    rowDropTarget?.position === 'before'}
                  class:drop-after={rowDropTarget?.sessionId === s.id &&
                    rowDropTarget?.position === 'after'}
                  draggable="true"
                  role="button"
                  tabindex="0"
                  ondragstart={(e) => onDragStart(e, s.id)}
                  ondragend={onDragEnd}
                  ondragover={(e) => onRowDragOver(e, s.id)}
                  ondragleave={() => onRowDragLeave(s.id)}
                  ondrop={(e) => onRowDrop(e, folder.id, s.id)}
                  style:background={isActive ? theme.itemActiveBg : ''}
                  onclick={() => onSelect(s)}
                >
                  <SessionAvatar
                    name={s.name}
                    {color}
                    paper={theme.sidebarBg}
                    ink={theme.textPrimary}
                    accent={theme.accent}
                    active={isActive}
                    status={sessionStatuses?.get(s.id)}
                    ok={theme.ok}
                    warn={theme.warn}
                    err={theme.err}
                    size={19}
                  />
                  <span
                    class="sb-name sb-name-sm"
                    class:active={isActive}
                    style:color={isActive ? theme.textPrimary : theme.textMuted}
                    title={sessionMeta(s).notes || undefined}>{s.name}</span
                  >
                  {#each sessionMeta(s).tags.slice(0, 3) as tag (tag)}
                    <span
                      class="sb-tag sb-tagchip"
                      style:color={theme.accent}
                      style:border-color="color-mix(in srgb, {theme.accent} 40%, transparent)"
                      style:font-family={theme.fontUi}>{tag}</span
                    >
                  {/each}
                  {#if s.protocol !== 'local' && s.protocol !== 'ssh'}
                    <span
                      class="sb-tag"
                      style:color={theme.textDim}
                      style:border-color={theme.border}
                      style:font-family={theme.fontMono}
                    >
                      {s.protocol.toUpperCase()}
                    </span>
                  {/if}
                  {#if onEdit}
                    <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                    <span
                      class="sb-edit"
                      role="button"
                      title="Edit session"
                      style:color={theme.textDim}
                      onclick={(e) => {
                        e.stopPropagation()
                        onEdit?.(s)
                      }}><Icon name="pencil" size={12} /></span
                    >
                  {/if}
                  {#if onDuplicate}
                    <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                    <span
                      class="sb-edit"
                      role="button"
                      title="Duplicate session"
                      style:color={theme.textDim}
                      onclick={(e) => {
                        e.stopPropagation()
                        onDuplicate?.(s)
                      }}><Icon name="copy" size={12} /></span
                    >
                  {/if}
                  {#if onDelete}
                    <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                    <span
                      class="sb-edit sb-del"
                      role="button"
                      title="Delete session"
                      style:color={theme.textDim}
                      onclick={(e) => {
                        e.stopPropagation()
                        onDelete?.(s)
                      }}><Icon name="x" size={12} /></span
                    >
                  {/if}
                </div>
              {/each}
              {#if folderSessions.length === 0}
                <span class="sb-empty" style:color={theme.textDim}>empty — drop a session here</span
                >
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    {/each}

    {#if sessions.length === 0 && !search}
      <p class="sb-hint" style:color={theme.textDim}>No sessions yet.</p>
    {/if}
  </div>

  <!-- Footer: user chip + theme + settings -->
  <div class="sb-footer" style:border-top="1px solid {theme.border}">
    <span
      class="sb-user-chip"
      style:background={theme.accent}
      style:color={theme.onAccent}
      style:font-family={theme.fontUi}>{initials}</span
    >
    <span class="sb-username" style:color={theme.textMuted} style:font-family={theme.fontUi}>
      {username}
    </span>
    <button
      class="sb-settings-btn"
      title="Cycle theme"
      onclick={onToggleTheme}
      style:color={theme.textDim}
    >
      <Icon name="contrast" size={14} />
    </button>
    <button
      class="sb-settings-btn"
      title="Settings"
      onclick={onSettings}
      style:color={theme.textDim}
    >
      <Icon name="gear" size={14} />
    </button>
  </div>
</aside>

<style>
  .sidebar {
    width: 248px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sb-search-wrap {
    padding: 12px 12px 8px;
    flex-shrink: 0;
  }

  .sb-search-inner {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 10px;
  }

  .sb-search {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    font-size: 12.5px;
    min-width: 0;
  }

  .sb-search::placeholder {
    color: inherit;
    opacity: 0.85;
  }

  .sb-kbd {
    font-size: 10.5px;
    border-radius: 4px;
    padding: 1px 5px;
    line-height: 1.4;
    flex-shrink: 0;
  }

  .sb-tree {
    flex: 1;
    overflow-y: auto;
    padding: 2px 0 4px;
  }

  .sb-tree::-webkit-scrollbar {
    width: 4px;
  }
  .sb-tree::-webkit-scrollbar-track {
    background: transparent;
  }
  .sb-tree::-webkit-scrollbar-thumb {
    background: rgba(127, 127, 127, 0.25);
    border-radius: 2px;
  }

  .sb-section {
    margin-bottom: 2px;
  }

  .sb-section-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px 4px;
  }

  .sb-add-btn {
    background: none;
    border: none;
    cursor: pointer;
    width: 22px;
    height: 22px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    padding: 0;
    flex-shrink: 0;
    transition:
      background 0.1s,
      color 0.1s;
    color: inherit;
  }

  .sb-add-btn:hover {
    background: var(--item-hover-bg, rgba(255, 255, 255, 0.06));
    color: var(--text-primary, #2c2924);
  }

  .sb-section-header {
    flex: 1;
    width: auto;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 1px;
    text-transform: uppercase;
    font-family: inherit;
    user-select: none;
    color: inherit;
  }

  .sb-folder-header {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 14px;
    cursor: pointer;
    user-select: none;
  }

  .sb-folder-name {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.8px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Reveal rename/delete icons on hover over the folder row. */
  .sb-folder-header:hover .sb-edit {
    opacity: 0.6;
  }

  .sb-count {
    margin-left: auto;
    font-size: 10.5px;
    font-weight: 400;
    letter-spacing: 0;
  }

  .sb-section-header .sb-count {
    margin-left: 2px;
  }

  .sb-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 8px;
    height: 34px;
    background: none;
    border: none;
    border-radius: var(--radius, 7px);
    cursor: pointer;
    font-family: inherit;
    text-align: left;
    transition: background 0.1s;
    position: relative;
  }

  .sb-row:hover {
    background: var(--item-hover-bg, rgba(255, 255, 255, 0.04)) !important;
  }

  .sb-row-indented {
    padding-left: 22px;
    height: 32px;
  }

  /* Drop indicators — a thin accent rule above (before) or below (after)
     the hovered row. Pure CSS, no extra DOM. */
  .sb-row.drop-before::before,
  .sb-row.drop-after::after {
    content: '';
    position: absolute;
    left: 8px;
    right: 8px;
    height: 2px;
    background: var(--accent);
    border-radius: 2px;
    pointer-events: none;
  }
  .sb-row.drop-before::before {
    top: -1px;
  }
  .sb-row.drop-after::after {
    bottom: -1px;
  }

  .sb-name {
    flex: 1;
    font-size: 13px;
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 0.1s;
  }

  .sb-name.active {
    font-weight: 500;
  }

  .sb-name-sm {
    font-size: 12.5px;
  }

  .sb-droparea {
    display: flex;
    flex-direction: column;
    border: 1px dashed transparent;
    border-radius: var(--radius, 7px);
    transition:
      border-color 0.1s,
      background 0.1s;
    padding: 1px;
    margin: 0 8px;
  }

  .sb-droparea.dragover {
    border-color: color-mix(in srgb, var(--accent) 60%, transparent);
    background: color-mix(in srgb, var(--accent) 8%, transparent);
  }

  /* Highlight the whole folder section when dragging over a collapsed
     folder so it's obvious you can drop on the header too. */
  .sb-section.dragover {
    background: color-mix(in srgb, var(--accent) 8%, transparent);
    border-radius: var(--radius, 7px);
  }

  .sb-row.dragging {
    opacity: 0.5;
  }

  .sb-edit {
    margin-left: 0;
    padding: 2px 3px;
    border-radius: 4px;
    cursor: pointer;
    opacity: 0;
    display: inline-flex;
    align-items: center;
    transition:
      opacity 0.1s,
      background 0.1s;
  }
  .sb-row:hover .sb-edit,
  .sb-row.active .sb-edit {
    opacity: 0.6;
  }
  .sb-edit:hover {
    opacity: 1 !important;
    background: var(--item-hover-bg, rgba(255, 255, 255, 0.08));
  }
  .sb-del:hover {
    color: var(--err) !important;
    background: color-mix(in srgb, var(--err) 12%, transparent) !important;
  }

  /* The protocol tag yields to the action icons on hover/active. */
  .sb-row:hover .sb-tag,
  .sb-row.active .sb-tag {
    display: none;
  }

  .sb-tag {
    font-size: 9.5px;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    padding: 1px 5px;
    border: 1px solid;
    border-radius: 4px;
    flex-shrink: 0;
    line-height: 1.4;
  }

  /* User tags: lower-case words, accent-tinted, capped width. Inherit the
     hide-on-hover from `.sb-tag` so the action icons get room. */
  .sb-tagchip {
    text-transform: none;
    letter-spacing: 0;
    font-size: 10px;
    border-radius: 5px;
    max-width: 80px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sb-empty {
    display: block;
    padding: 4px 22px;
    font-size: 11px;
    font-style: italic;
  }

  .sb-hint {
    font-size: 11px;
    padding: 12px 14px;
    margin: 0;
    line-height: 1.6;
  }

  .sb-footer {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 10px 14px;
    flex-shrink: 0;
  }

  .sb-user-chip {
    width: 24px;
    height: 24px;
    border-radius: 7px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
  }

  .sb-username {
    font-size: 12px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sb-settings-btn {
    background: none;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px;
    border-radius: 4px;
    transition:
      background 0.1s,
      color 0.1s;
    color: inherit;
  }

  .sb-settings-btn:hover {
    background: var(--item-hover-bg, rgba(255, 255, 255, 0.06));
    color: var(--text-primary, #2c2924);
  }
</style>
