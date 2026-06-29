<script lang="ts">
  /// Bottom snippet bar.
  ///
  /// Two zones:
  ///   • User snippets — capped at SNIPPET_BAR_LIMIT (9), filtered by the
  ///     focused session's platform (globals + matches). First 9 carry a
  ///     `[N]` pill so the user knows which Ctrl+Shift+N fires them.
  ///   • Recents — top frequently-typed commands surfaced from the history
  ///     table. Read-only, rotates as the user works.
  ///
  /// The bar refreshes when the focused session changes (App.svelte calls
  /// `setBarContext`) and when CRUD happens in the manage dialog.

  import {
    visibleSnippets,
    recents,
    barContext,
    loadVisibleSnippets,
    loadSnippets,
    SNIPPET_BAR_LIMIT,
  } from '$lib/stores/snippets.svelte'
  import {
    sendInputText,
    createSnippet,
    updateSnippet,
    deleteSnippet,
    setSnippetsOrder,
    setSnippetSteps,
  } from '$lib/bridge/commands'
  import { runMacro } from '$lib/orchestrator/macroRunner'
  import {
    getFocusedSessionId,
    focusSession,
    broadcast,
    broadcastTargets,
  } from '$lib/stores/sessionRuntime.svelte'
  import { showToast } from '$lib/ui/toast-store.svelte'
  import { resolveSnippet } from '$lib/snippets/variables.svelte'
  import Icon from '$lib/icons/Icon.svelte'
  import ButtonEditor from './ButtonEditor.svelte'
  import type { Snippet, RecentCommand, LoginStep } from '$lib/bridge/types'
  import type { PylonTheme } from '$lib/themes/index'

  interface Props {
    theme: PylonTheme
  }

  let { theme }: Props = $props()

  // Inline button editor: `null` = closed, `'new'` = create, Snippet = edit.
  let editing = $state<Snippet | 'new' | null>(null)

  // Drag-to-reorder state (index into visibleSnippets being dragged).
  let dragIndex = $state<number | null>(null)
  let dropIndex = $state<number | null>(null)

  async function reorderTo(target: number) {
    const from = dragIndex
    dragIndex = null
    dropIndex = null
    if (from === null || from === target) return
    const ids = visibleSnippets.map((s) => s.id)
    const [moved] = ids.splice(from, 1)
    if (moved === undefined) return
    ids.splice(target, 0, moved)
    try {
      await setSnippetsOrder(ids)
      await Promise.all([loadVisibleSnippets(), loadSnippets()])
    } catch (e) {
      flash('err', e instanceof Error ? e.message : String(e))
    }
  }

  async function saveButton(v: {
    name: string
    content: string
    color: string | null
    platformScoped: boolean
    steps: LoginStep[] | null
  }) {
    const platform = v.platformScoped ? barContext.platform : null
    const stepsJson = v.steps && v.steps.length > 0 ? JSON.stringify(v.steps) : null
    try {
      let id: number
      if (editing && editing !== 'new') {
        await updateSnippet(editing.id, v.name, v.content, platform, v.color)
        id = editing.id
      } else {
        id = await createSnippet(v.name, v.content, platform, v.color)
      }
      // Macro steps live in a separate column (set/cleared independently).
      await setSnippetSteps(id, stepsJson)
      editing = null
      await Promise.all([loadVisibleSnippets(), loadSnippets()])
    } catch (e) {
      flash('err', e instanceof Error ? e.message : String(e))
    }
  }

  async function removeButton(s: Snippet) {
    try {
      await deleteSnippet(s.id)
      editing = null
      await Promise.all([loadVisibleSnippets(), loadSnippets()])
      showToast({ kind: 'info', title: 'Button deleted', detail: s.name })
    } catch (e) {
      flash('err', e instanceof Error ? e.message : String(e))
    }
  }

  let status = $state<{ kind: 'ok' | 'err'; text: string } | null>(null)
  let statusTimer: ReturnType<typeof setTimeout> | null = null

  function flash(kind: 'ok' | 'err', text: string) {
    status = { kind, text }
    if (statusTimer) clearTimeout(statusTimer)
    statusTimer = setTimeout(() => (status = null), 2200)
  }

  /// Shared dispatch — accepts both Snippet and RecentCommand by extracting
  /// the text + display label.
  async function fireText(text: string, label: string) {
    const focused = getFocusedSessionId()
    if (!focused) {
      flash('err', 'No session focused — click a terminal first.')
      return
    }
    try {
      await sendInputText(focused, text)
      // Hand focus back to the terminal so the user can keep typing — snippets
      // like "ps -aux | grep " carry no newline and expect more input.
      focusSession(focused)
      if (broadcast.enabled) {
        const others = broadcastTargets(focused)
        for (const id of others) {
          await sendInputText(id, text).catch(() => {})
        }
        flash('ok', `Sent "${label}" to ${1 + others.length} sessions`)
      } else {
        flash('ok', `Sent "${label}"`)
      }
    } catch (e) {
      flash('err', e instanceof Error ? e.message : String(e))
    }
  }

  async function fireSnippet(s: Snippet) {
    // Macro snippets run an expect/send/wait sequence on the focused session;
    // plain snippets just send their (variable-resolved) text.
    if (s.steps_json) {
      await fireMacro(s)
      return
    }
    // Resolve any {{variables}} (prompts the user) before sending.
    const text = await resolveSnippet(s.content)
    if (text === null) return
    fireText(text, s.name)
  }

  async function fireMacro(s: Snippet) {
    const focused = getFocusedSessionId()
    if (!focused) {
      flash('err', 'No session focused — click a terminal first.')
      return
    }
    let steps: LoginStep[]
    try {
      steps = JSON.parse(s.steps_json ?? '[]')
    } catch {
      flash('err', `Macro "${s.name}" is corrupt`)
      return
    }
    flash('ok', `Running macro "${s.name}"…`)
    const res = await runMacro(focused, steps)
    focusSession(focused)
    if (res.ok) flash('ok', `Macro "${s.name}" done`)
    else flash('err', `Macro "${s.name}" timed out at step ${(res.failedStep ?? 0) + 1}`)
  }

  function fireRecent(r: RecentCommand) {
    // Recents come straight from command_history; user typed them without
    // a trailing newline. Append one so the command actually executes.
    const withNl = r.text.endsWith('\n') ? r.text : r.text + '\n'
    fireText(withNl, r.text)
  }

  /// Truncate a recent for display (the bar is horizontal — long commands
  /// blow up the layout). Tooltip carries the full text.
  function shortLabel(text: string, max = 28): string {
    return text.length > max ? text.slice(0, max - 1) + '…' : text
  }

  let collapsed = $state(false)
</script>

{#if collapsed}
  <div
    class="bar collapsed"
    style:background={theme.tabBarBg}
    style:border-color={theme.border}
    style:--hover={theme.itemHoverBg}
  >
    <button
      class="collapse-toggle"
      style:color={theme.textDim}
      onclick={() => (collapsed = false)}
      title="Show button bar"
      >▴ Buttons ({visibleSnippets.length}){recents.length
        ? ` · ${recents.length} recent`
        : ''}</button
    >
  </div>
{:else}
  <div
    class="bar"
    style:background={theme.tabBarBg}
    style:border-color={theme.border}
    style:--accent={theme.accent}
    style:--hover={theme.itemHoverBg}
    style:--ok={theme.ok}
    style:--err={theme.err}
    style:--pill-bg="color-mix(in srgb, {theme.textPrimary} 10%, transparent)"
    style:--scroll="color-mix(in srgb, {theme.textPrimary} 22%, transparent)"
  >
    <button
      class="collapse-toggle"
      style:color={theme.textDim}
      onclick={() => (collapsed = true)}
      title="Hide button bar">▾</button
    >

    <div class="snippets">
      {#each visibleSnippets as s, i (s.id)}
        <!-- Each button: click fires it; a hover pencil opens the editor;
             drag to reorder. The optional color paints a left accent stripe. -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span
          class="btn-wrap"
          class:dragging={dragIndex === i}
          class:drop-target={dropIndex === i && dragIndex !== i}
          ondragover={(e) => {
            if (dragIndex !== null) {
              e.preventDefault()
              dropIndex = i
            }
          }}
          ondrop={(e) => {
            e.preventDefault()
            reorderTo(i)
          }}
        >
          <button
            class="snippet-btn"
            draggable="true"
            ondragstart={() => {
              dragIndex = i
            }}
            ondragend={() => {
              dragIndex = null
              dropIndex = null
            }}
            style:color={theme.textPrimary}
            style:background={s.color
              ? `color-mix(in srgb, ${s.color} 18%, transparent)`
              : theme.itemActiveBg}
            style:border-color={s.color ?? theme.border}
            style:border-left={s.color ? `3px solid ${s.color}` : `1px solid ${theme.border}`}
            onclick={() => fireSnippet(s)}
            title={`${i < SNIPPET_BAR_LIMIT ? `Ctrl+Shift+${i + 1} — ` : ''}${s.content}`}
          >
            {#if i < SNIPPET_BAR_LIMIT}
              <span class="key-pill" style:color={theme.accent2}>{i + 1}</span>
            {/if}
            {#if s.steps_json}
              <span class="macro-badge" style:color={theme.accent} title="Macro (expect/send/wait)">
                <Icon name="bolt" size={10} />
              </span>
            {/if}
            <span class="snippet-name">{s.name}</span>
          </button>
          <button
            class="edit-dot"
            style:color={theme.textDim}
            style:background={theme.tabBarBg}
            title="Edit button"
            aria-label="Edit button"
            onclick={(e) => {
              e.stopPropagation()
              editing = s
            }}><Icon name="pencil" size={10} /></button
          >
        </span>
      {/each}

      <!-- Create a new button (SecureCRT-style). -->
      <button
        class="add-btn"
        style:color={theme.textDim}
        style:border-color={theme.border}
        onclick={() => (editing = 'new')}
        title="New button"
        aria-label="New button"><Icon name="plus" size={12} /></button
      >

      {#if recents.length > 0}
        <span class="divider" style:background={theme.border}></span>
      {/if}

      {#each recents as r (r.text)}
        <button
          class="snippet-btn recent"
          style:color={theme.textPrimary}
          style:border-color={theme.border}
          onclick={() => fireRecent(r)}
          title={`Recent: ${r.text}`}
        >
          <span class="recent-mark" title="From your recent history"
            ><Icon name="clock" size={11} /></span
          >
          <span class="snippet-name">{shortLabel(r.text)}</span>
        </button>
      {/each}
    </div>

    {#if status}
      <span class="status" class:err={status.kind === 'err'}>{status.text}</span>
    {/if}

    {#if editing}
      {#key editing}
        <ButtonEditor
          {theme}
          snippet={editing === 'new' ? null : editing}
          platform={barContext.platform}
          onSave={saveButton}
          onDelete={editing === 'new' ? undefined : () => removeButton(editing as Snippet)}
          onClose={() => (editing = null)}
        />
      {/key}
    {/if}
  </div>
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-top: 1px solid;
    border-bottom: 1px solid;
    min-height: 28px;
    flex-shrink: 0;
    user-select: none;
    position: relative; /* anchors the ButtonEditor popover */
  }

  /* Button + hover edit affordance. The edit dot floats at the top-right. */
  .btn-wrap {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
  }

  .edit-dot {
    position: absolute;
    top: -5px;
    right: -5px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: 1px solid;
    border-color: inherit;
    display: none;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    z-index: 2;
  }
  .btn-wrap:hover .edit-dot {
    display: inline-flex;
  }
  .btn-wrap.dragging {
    opacity: 0.45;
  }
  .btn-wrap.drop-target {
    box-shadow: -2px 0 0 0 var(--accent, #5eb3b2);
  }
  .snippet-btn[draggable='true'] {
    cursor: grab;
  }
  .snippet-btn[draggable='true']:active {
    cursor: grabbing;
  }
  .edit-dot:hover {
    filter: brightness(1.3);
  }

  .add-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 22px;
    border: 1px dashed;
    border-radius: 4px;
    cursor: pointer;
    background: transparent;
    flex-shrink: 0;
    transition: filter 0.1s;
  }
  .add-btn:hover {
    filter: brightness(1.3);
  }

  .bar.collapsed {
    padding: 2px 8px;
    min-height: 18px;
    border-bottom: none;
  }

  .collapse-toggle {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: inherit;
    flex-shrink: 0;
    opacity: 0.7;
  }

  .collapse-toggle:hover {
    opacity: 1;
    background: var(--hover);
  }

  .snippets {
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    flex: 1;
    padding: 0 2px;
    scrollbar-width: thin;
  }

  .snippets::-webkit-scrollbar {
    height: 4px;
  }

  .snippets::-webkit-scrollbar-thumb {
    background: var(--scroll);
    border-radius: 2px;
  }

  .snippet-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 9px;
    font-size: 11.5px;
    border: 1px solid;
    border-radius: 4px;
    cursor: pointer;
    font-family: inherit;
    line-height: 1.3;
    white-space: nowrap;
    flex-shrink: 0;
    transition:
      border-color 0.1s,
      background 0.1s;
  }

  .snippet-btn:hover {
    border-color: var(--accent);
    filter: brightness(1.15);
  }

  .snippet-btn:active {
    transform: translateY(1px);
  }

  /* Recents are visually subtler than user snippets — dotted border + no
     filled background. Conveys "this is auto-suggested, not your fixed list". */
  .snippet-btn.recent {
    background: transparent;
    border-style: dashed;
    opacity: 0.78;
  }

  .snippet-btn.recent:hover {
    opacity: 1;
  }

  .key-pill {
    font-family: monospace;
    font-size: 9.5px;
    font-weight: 700;
    background: var(--pill-bg);
    border-radius: 3px;
    padding: 1px 5px;
    line-height: 1;
  }

  /* Macro snippets (expect/send/wait) carry a small bolt so they read as more
     than plain text. */
  .macro-badge {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
  }

  .recent-mark {
    display: inline-flex;
    align-items: center;
    opacity: 0.7;
    line-height: 1;
  }

  .divider {
    width: 1px;
    align-self: stretch;
    margin: 3px 6px;
    opacity: 0.6;
    flex-shrink: 0;
  }

  .snippet-name {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 18ch;
  }

  .status {
    font-size: 11px;
    color: var(--ok);
    padding: 2px 8px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--ok) 12%, transparent);
    flex-shrink: 0;
    white-space: nowrap;
  }

  .status.err {
    color: var(--err);
    background: color-mix(in srgb, var(--err) 12%, transparent);
  }
</style>
