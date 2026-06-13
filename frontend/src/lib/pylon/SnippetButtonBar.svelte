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
  } from '$lib/bridge/commands'
  import {
    getFocusedSessionId,
    broadcast,
    broadcastTargets,
  } from '$lib/stores/sessionRuntime.svelte'
  import { showToast } from '$lib/ui/toast-store.svelte'
  import Icon from '$lib/icons/Icon.svelte'
  import ButtonEditor from './ButtonEditor.svelte'
  import type { Snippet, RecentCommand } from '$lib/bridge/types'
  import type { PylonTheme } from '$lib/themes/index'

  interface Props {
    theme: PylonTheme
  }

  let { theme }: Props = $props()

  // Inline button editor: `null` = closed, `'new'` = create, Snippet = edit.
  let editing = $state<Snippet | 'new' | null>(null)

  async function saveButton(v: {
    name: string
    content: string
    color: string | null
    platformScoped: boolean
  }) {
    const platform = v.platformScoped ? barContext.platform : null
    try {
      if (editing && editing !== 'new') {
        await updateSnippet(editing.id, v.name, v.content, platform, v.color)
      } else {
        await createSnippet(v.name, v.content, platform, v.color)
      }
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
      showToast({ kind: 'info', title: 'Botón borrado', detail: s.name })
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

  function fireSnippet(s: Snippet) {
    fireText(s.content, s.name)
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
  <div class="bar collapsed" style:background={theme.tabBarBg} style:border-color={theme.border}>
    <button
      class="collapse-toggle"
      style:color={theme.textDim}
      onclick={() => (collapsed = false)}
      title="Show button bar"
    >▴ Botones ({visibleSnippets.length}){recents.length ? ` · ${recents.length} recientes` : ''}</button>
  </div>
{:else}
  <div class="bar" style:background={theme.tabBarBg} style:border-color={theme.border}>
    <button
      class="collapse-toggle"
      style:color={theme.textDim}
      onclick={() => (collapsed = true)}
      title="Hide button bar"
    >▾</button>

    <div class="snippets">
      {#each visibleSnippets as s, i (s.id)}
        <!-- Each button: click fires it; a hover pencil opens the editor. The
             optional color paints a left accent stripe. -->
        <span class="btn-wrap">
          <button
            class="snippet-btn"
            style:color={theme.textPrimary}
            style:background={s.color ? `color-mix(in srgb, ${s.color} 18%, transparent)` : theme.itemActiveBg}
            style:border-color={s.color ?? theme.border}
            style:border-left={s.color ? `3px solid ${s.color}` : `1px solid ${theme.border}`}
            onclick={() => fireSnippet(s)}
            title={`${i < SNIPPET_BAR_LIMIT ? `Ctrl+Shift+${i + 1} — ` : ''}${s.content}`}
          >
            {#if i < SNIPPET_BAR_LIMIT}
              <span class="key-pill" style:color={theme.accent2}>{i + 1}</span>
            {/if}
            <span class="snippet-name">{s.name}</span>
          </button>
          <button
            class="edit-dot"
            style:color={theme.textDim}
            style:background={theme.tabBarBg}
            title="Editar botón"
            aria-label="Editar botón"
            onclick={(e) => { e.stopPropagation(); editing = s }}
          ><Icon name="pencil" size={10} /></button>
        </span>
      {/each}

      <!-- Create a new button (SecureCRT-style). -->
      <button
        class="add-btn"
        style:color={theme.textDim}
        style:border-color={theme.border}
        onclick={() => (editing = 'new')}
        title="Nuevo botón"
        aria-label="Nuevo botón"
      ><Icon name="plus" size={12} /></button>

      {#if recents.length > 0}
        <span class="divider" style:background={theme.border}></span>
      {/if}

      {#each recents as r (r.text)}
        <button
          class="snippet-btn recent"
          style:color={theme.textPrimary}
          style:border-color={theme.border}
          onclick={() => fireRecent(r)}
          title={`Reciente: ${r.text}`}
        >
          <span class="recent-mark" title="De tu historial reciente">⏱</span>
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
  .btn-wrap:hover .edit-dot { display: inline-flex; }
  .edit-dot:hover { filter: brightness(1.3); }

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
  .add-btn:hover { filter: brightness(1.3); }

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
    background: rgba(255, 255, 255, 0.06);
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
    background: rgba(255, 255, 255, 0.12);
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
    transition: border-color 0.1s, background 0.1s;
  }

  .snippet-btn:hover {
    border-color: rgba(59, 130, 246, 0.6);
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
    background: rgba(255, 255, 255, 0.08);
    border-radius: 3px;
    padding: 1px 5px;
    line-height: 1;
  }

  .recent-mark {
    font-size: 11px;
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
    color: #34d399;
    padding: 2px 8px;
    border-radius: 3px;
    background: rgba(52, 211, 153, 0.1);
    flex-shrink: 0;
    white-space: nowrap;
  }

  .status.err {
    color: #f87171;
    background: rgba(239, 68, 68, 0.12);
  }
</style>
