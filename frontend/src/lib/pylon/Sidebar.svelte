<script lang="ts">
  import type { PylonTheme } from '$lib/themes/index'
  import type { SavedSession, Folder } from '$lib/bridge/types'

  interface Props {
    theme: PylonTheme
    sessions: SavedSession[]
    folders: Folder[]
    activeSessionId?: number
    onSelect: (session: SavedSession) => void
    onEdit?: (session: SavedSession) => void
    onDelete?: (session: SavedSession) => void
    /** Reparent a session via drag-and-drop. `folderId = null` drops at root. */
    onMove?: (sessionId: number, folderId: number | null) => void
    onAddSession?: () => void
    onSettings?: () => void
    onToggleTheme?: () => void
  }

  const {
    theme,
    sessions,
    folders,
    activeSessionId,
    onSelect,
    onEdit,
    onDelete,
    onMove,
    onAddSession,
    onSettings,
    onToggleTheme,
  }: Props = $props()

  // ── Drag-and-drop (native HTML5) ────────────────────────────────────────
  /// Session id currently being dragged. `dragOver` is the drop target's id
  /// (`null` = root). Both reset on dragend.
  let draggingSessionId = $state<number | null>(null)
  let dragOver = $state<number | null | 'none'>('none')

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

  function onDrop(e: DragEvent, targetFolderId: number | null) {
    if (draggingSessionId == null) return
    e.preventDefault()
    const id = draggingSessionId
    // Skip when dropped on its current folder.
    const current = sessions.find((s) => s.id === id)?.folder_id ?? null
    if (current !== targetFolderId) onMove?.(id, targetFolderId)
    draggingSessionId = null
    dragOver = 'none'
  }

  function onDragEnd() {
    draggingSessionId = null
    dragOver = 'none'
  }

  let search = $state('')
  let expandedSections = $state<Set<string>>(new Set(['pinned', 'sessions']))
  let expandedFolders = $state<Set<number>>(new Set())

  const SESSION_COLORS = [
    '#22d3ee','#f472b6','#a78bfa','#f59e0b',
    '#ef4444','#10b981','#f97316','#84cc16','#c89b6b','#5eb3b2',
  ]

  function sessionColor(s: SavedSession): string {
    return SESSION_COLORS[s.id % SESSION_COLORS.length] as string
  }

  const query = $derived(search.toLowerCase())

  const rootSessions = $derived(
    sessions.filter((s) => s.folder_id === null && (
      !query || s.name.toLowerCase().includes(query)
    ))
  )

  function sessionsInFolder(id: number): SavedSession[] {
    return sessions.filter((s) => s.folder_id === id && (
      !query || s.name.toLowerCase().includes(query)
    ))
  }

  function toggleSection(key: string) {
    const next = new Set(expandedSections)
    next.has(key) ? next.delete(key) : next.add(key)
    expandedSections = next
  }

  function toggleFolder(id: number) {
    const next = new Set(expandedFolders)
    next.has(id) ? next.delete(id) : next.add(id)
    expandedFolders = next
  }

  // User initials chip gradient (deterministic from username)
  const username = 'admin'
  const initials = username.slice(0, 2).toUpperCase()
</script>

<aside
  class="sidebar"
  style:background={theme.sidebarBg}
  style:border-right="1px solid {theme.accent}30"
  style:font-family={theme.fontUi}
  style:--item-hover-bg={theme.itemHoverBg}
  style:--text-primary={theme.textPrimary}
>

  <!-- Search -->
  <div class="sb-search-wrap" style:border-bottom="1px solid {theme.border}">
    <div class="sb-search-inner" style:border="1px solid {theme.border}">
      <svg class="sb-search-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke={theme.textDim} stroke-width="2.5">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
      </svg>
      <input
        class="sb-search"
        type="text"
        placeholder="Search sessions…"
        bind:value={search}
        style:color={theme.textPrimary}
        style:font-family={theme.fontUi}
        style:--focus-ring={theme.accent}
      />
    </div>
  </div>

  <!-- Tree -->
  <div class="sb-tree">

    <!-- Favorites / pinned (root sessions without folder) -->
    <div class="sb-section">
      <div class="sb-section-row">
        <button
          class="sb-section-header"
          style:color={theme.textDim}
          onclick={() => toggleSection('sessions')}
        >
          <span
            class="sb-caret"
            class:expanded={expandedSections.has('sessions')}
            style:color={theme.textDim}
          >▸</span>
          SESSIONS
          <span class="sb-count" style:color={theme.textDim}>{rootSessions.length}</span>
        </button>
        <button
          class="sb-add-btn"
          title="New session"
          style:color={theme.textDim}
          onclick={onAddSession}
        >+</button>
      </div>

      {#if expandedSections.has('sessions')}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="sb-droparea"
          class:dragover={dragOver === null && draggingSessionId != null}
          ondragover={(e) => onDragOver(e, null)}
          ondrop={(e) => onDrop(e, null)}
        >
          {#each rootSessions as s (s.id)}
            {@const color = sessionColor(s)}
            {@const isActive = s.id === activeSessionId}
            <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
            <div
              class="sb-row"
              class:active={isActive}
              class:dragging={draggingSessionId === s.id}
              draggable="true"
              role="button"
              tabindex="0"
              ondragstart={(e) => onDragStart(e, s.id)}
              ondragend={onDragEnd}
              style:background={isActive ? theme.itemActiveBg : ''}
              style:border-left={isActive ? `2px solid ${theme.itemActiveBorder}` : '2px solid transparent'}
              onclick={() => onSelect(s)}
            >
              <span class="sb-dot" style:background={color}></span>
              <span class="sb-name" style:color={isActive ? theme.textPrimary : theme.textMuted}>{s.name}</span>
              {#if s.protocol !== 'local'}
                <span class="sb-tag" style:color={theme.textDim} style:border-color={theme.border}>
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
                  onclick={(e) => { e.stopPropagation(); onEdit?.(s) }}
                >✎</span>
              {/if}
              {#if onDelete}
                <!-- svelte-ignore a11y_interactive_supports_focus a11y_click_events_have_key_events -->
                <span
                  class="sb-edit sb-del"
                  role="button"
                  title="Delete session"
                  style:color={theme.textDim}
                  onclick={(e) => { e.stopPropagation(); onDelete?.(s) }}
                >✕</span>
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
          <button
            class="sb-section-header"
            style:color={theme.textDim}
            onclick={() => toggleFolder(folder.id)}
          >
            <span
              class="sb-caret"
              class:expanded={expandedFolders.has(folder.id)}
              style:color={theme.textDim}
            >▸</span>
            {folder.name.toUpperCase()}
            <span class="sb-count" style:color={theme.textDim}>{folderSessions.length}</span>
          </button>

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
                  draggable="true"
                  role="button"
                  tabindex="0"
                  ondragstart={(e) => onDragStart(e, s.id)}
                  ondragend={onDragEnd}
                  style:background={isActive ? theme.itemActiveBg : ''}
                  style:border-left={isActive ? `2px solid ${theme.itemActiveBorder}` : '2px solid transparent'}
                  onclick={() => onSelect(s)}
                >
                  <span class="sb-dot" style:background={color}></span>
                  <span class="sb-name" style:color={isActive ? theme.textPrimary : theme.textMuted}>{s.name}</span>
                  {#if s.protocol !== 'local'}
                    <span class="sb-tag" style:color={theme.textDim} style:border-color={theme.border}>
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
                      onclick={(e) => { e.stopPropagation(); onEdit?.(s) }}
                    >✎</span>
                  {/if}
                </div>
              {/each}
              {#if folderSessions.length === 0}
                <span class="sb-empty" style:color={theme.textDim}>empty — drop a session here</span>
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

  <!-- Footer: user chip + settings -->
  <div class="sb-footer" style:border-top="1px solid {theme.border}">
    <div
      class="sb-user-chip"
      style:background="linear-gradient(135deg, {theme.accent}99, {theme.accent2}99)"
      style:border-radius={theme.radius}
    >
      <span style:font-family={theme.fontUi}>{initials}</span>
    </div>
    <span class="sb-username" style:color={theme.textMuted} style:font-family={theme.fontUi}>
      {username}
    </span>
    <button class="sb-settings-btn" title="Toggle theme" onclick={onToggleTheme} style:color={theme.accent}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
        <circle cx="12" cy="12" r="5"/>
        <path d="M12 1v3M12 20v3M4.22 4.22l2.12 2.12M17.66 17.66l2.12 2.12M1 12h3M20 12h3M4.22 19.78l2.12-2.12M17.66 6.34l2.12-2.12" stroke="currentColor" stroke-width="2" stroke-linecap="round" fill="none"/>
      </svg>
    </button>
    <button class="sb-settings-btn" title="Settings" onclick={onSettings} style:color={theme.textDim}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
      </svg>
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
    padding: 8px 10px;
    flex-shrink: 0;
  }

  .sb-search-inner {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    border-radius: 5px;
    padding: 0 8px;
    background: rgba(255,255,255,0.03);
    transition: border-color 0.15s;
  }

  .sb-search-inner:focus-within {
    border-color: var(--focus-ring, #5eb3b2) !important;
  }

  .sb-search-icon {
    flex-shrink: 0;
  }

  .sb-search {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    font-size: 12px;
    min-width: 0;
  }

  .sb-search::placeholder {
    color: #4b5563;
  }

  .sb-tree {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .sb-tree::-webkit-scrollbar {
    width: 4px;
  }
  .sb-tree::-webkit-scrollbar-track { background: transparent; }
  .sb-tree::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.08); border-radius: 2px; }

  .sb-section {
    margin-bottom: 2px;
  }

  .sb-section-row {
    display: flex;
    align-items: center;
  }

  .sb-add-btn {
    background: none;
    border: none;
    cursor: pointer;
    width: 22px;
    height: 22px;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    padding: 0;
    line-height: 1;
    flex-shrink: 0;
    margin-right: 6px;
    transition: background 0.1s, color 0.1s;
    color: inherit;
  }

  .sb-add-btn:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.06));
    color: var(--text-primary, #c9cdd3);
  }

  .sb-section-header {
    flex: 1;
    width: auto;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 10px;
    height: 24px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    font-family: inherit;
  }

  .sb-caret {
    font-size: 9px;
    transition: transform 0.12s ease;
    display: inline-block;
  }

  .sb-caret.expanded {
    transform: rotate(90deg);
  }

  .sb-count {
    margin-left: auto;
    font-size: 10px;
    font-weight: 400;
    letter-spacing: 0;
  }

  .sb-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    height: 30px;
    background: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
    text-align: left;
    transition: background 0.1s;
  }

  .sb-row:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.04)) !important;
  }

  .sb-row-indented {
    padding-left: 22px;
  }

  .sb-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .sb-name {
    flex: 1;
    font-size: 12.5px;
    font-weight: 400;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 0.1s;
  }

  .sb-droparea {
    display: flex;
    flex-direction: column;
    border: 1px dashed transparent;
    border-radius: 4px;
    transition: border-color 0.1s, background 0.1s;
    padding: 1px;
    margin: 0 4px;
  }

  .sb-droparea.dragover {
    border-color: rgba(59, 130, 246, 0.6);
    background: rgba(59, 130, 246, 0.08);
  }

  /* Highlight the whole folder section when dragging over a collapsed
     folder so it's obvious you can drop on the header too. */
  .sb-section.dragover {
    background: rgba(59, 130, 246, 0.08);
    border-radius: 4px;
  }

  .sb-row.dragging {
    opacity: 0.5;
  }

  .sb-edit {
    margin-left: 4px;
    font-size: 11px;
    line-height: 1;
    padding: 2px 4px;
    border-radius: 3px;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.1s, background 0.1s;
  }
  .sb-row:hover .sb-edit {
    opacity: 0.6;
  }
  .sb-edit:hover {
    opacity: 1 !important;
    background: rgba(255, 255, 255, 0.08);
  }
  .sb-del:hover {
    color: #ef4444 !important;
    background: rgba(239, 68, 68, 0.1) !important;
  }

  .sb-tag {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    padding: 1px 4px;
    border: 1px solid;
    border-radius: 3px;
    flex-shrink: 0;
    line-height: 1.4;
  }

  .sb-empty {
    display: block;
    padding: 4px 22px;
    font-size: 11px;
    font-style: italic;
  }

  .sb-hint {
    font-size: 11px;
    padding: 12px 12px;
    margin: 0;
    line-height: 1.6;
  }

  .sb-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    height: 40px;
    flex-shrink: 0;
  }

  .sb-user-chip {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    font-size: 10px;
    font-weight: 700;
    color: rgba(255,255,255,0.9);
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
    transition: background 0.1s, color 0.1s;
    color: inherit;
  }

  .sb-settings-btn:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.06));
    color: var(--text-primary, #c9cdd3);
  }
</style>
