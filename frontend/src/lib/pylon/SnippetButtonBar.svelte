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
    SNIPPET_BAR_LIMIT,
  } from '$lib/stores/snippets.svelte'
  import { sendInputText } from '$lib/bridge/commands'
  import {
    getFocusedSessionId,
    broadcast,
    broadcastTargets,
  } from '$lib/stores/sessionRuntime.svelte'
  import type { Snippet, RecentCommand } from '$lib/bridge/types'
  import type { PylonTheme } from '$lib/themes/index'

  interface Props {
    theme: PylonTheme
  }

  let { theme }: Props = $props()

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

{#if visibleSnippets.length === 0 && recents.length === 0}
  <!-- Hide entirely when there's nothing to show. Tools→Snippets… still works. -->
{:else if collapsed}
  <div class="bar collapsed" style:background={theme.tabBarBg} style:border-color={theme.border}>
    <button
      class="collapse-toggle"
      style:color={theme.textDim}
      onclick={() => (collapsed = false)}
      title="Show snippet bar"
    >▴ Snippets ({visibleSnippets.length}){recents.length ? ` · ${recents.length} recent` : ''}</button>
  </div>
{:else}
  <div class="bar" style:background={theme.tabBarBg} style:border-color={theme.border}>
    <button
      class="collapse-toggle"
      style:color={theme.textDim}
      onclick={() => (collapsed = true)}
      title="Hide snippet bar"
    >▾</button>

    <div class="snippets">
      {#each visibleSnippets as s, i (s.id)}
        <button
          class="snippet-btn"
          style:color={theme.textPrimary}
          style:background={theme.itemActiveBg}
          style:border-color={theme.border}
          onclick={() => fireSnippet(s)}
          title={`${i < SNIPPET_BAR_LIMIT ? `Ctrl+Shift+${i + 1} — ` : ''}${s.content}`}
        >
          {#if i < SNIPPET_BAR_LIMIT}
            <span class="key-pill" style:color={theme.accent2}>{i + 1}</span>
          {/if}
          <span class="snippet-name">{s.name}</span>
        </button>
      {/each}

      {#if recents.length > 0 && visibleSnippets.length > 0}
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
          <span class="recent-mark" title="From your recent history">⏱</span>
          <span class="snippet-name">{shortLabel(r.text)}</span>
        </button>
      {/each}
    </div>

    {#if status}
      <span class="status" class:err={status.kind === 'err'}>{status.text}</span>
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
