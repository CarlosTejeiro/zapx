<script lang="ts">
  /// Inline editor popover for a button-bar button (a snippet). Used by
  /// SnippetButtonBar for both create ("+") and edit. Anchored above the bar.
  import type { PylonTheme } from '$lib/themes/index'
  import type { Snippet, LoginStep } from '$lib/bridge/types'
  import MacroStepEditor from './MacroStepEditor.svelte'

  interface Props {
    theme: PylonTheme
    /** Editing an existing button, or null when creating a new one. */
    snippet: Snippet | null
    /** Focused session's platform (null/'' = no recognised platform). When
     *  absent, the button can only be global. */
    platform: string | null
    onSave: (v: {
      name: string
      content: string
      color: string | null
      platformScoped: boolean
      /** Non-null when this button is a macro (expect/send/wait steps). */
      steps: LoginStep[] | null
    }) => void
    onDelete?: () => void
    onClose: () => void
  }

  const { theme, snippet, platform, onSave, onDelete, onClose }: Props = $props()

  // svelte-ignore state_referenced_locally
  const hasPlatform = !!platform && platform !== 'generic'

  // One-shot capture of the props into editable local state — intentional:
  // the parent wraps this in `{#key editing}` so a different target remounts
  // the component and re-runs these initializers with fresh props.
  // svelte-ignore state_referenced_locally
  let name = $state(snippet?.name ?? '')
  // svelte-ignore state_referenced_locally
  let content = $state(snippet?.content ?? '')
  // svelte-ignore state_referenced_locally
  let color = $state<string | null>(snippet?.color ?? null)
  // Existing snippet keeps its scope; new buttons default to the current
  // platform when there is one (that's what the bar is showing).
  // svelte-ignore state_referenced_locally
  let platformScoped = $state(snippet ? snippet.platform !== null : hasPlatform)

  // Macro mode: an expect/send/wait step list instead of plain text.
  // svelte-ignore state_referenced_locally
  let isMacro = $state(!!snippet?.steps_json)
  // svelte-ignore state_referenced_locally
  let steps = $state<LoginStep[]>(parseSteps(snippet?.steps_json ?? null))

  function parseSteps(json: string | null): LoginStep[] {
    if (!json) return []
    try {
      const v = JSON.parse(json)
      return Array.isArray(v) ? v : []
    } catch {
      return []
    }
  }
  // Palette tuned to read on both light and dark themes.
  const SWATCHES = ['#5eb3b2', '#5b8fc9', '#9a91e8', '#3e8f60', '#b88528', '#c2410c', '#b13a3a']

  const canSave = $derived(!!name.trim() && (isMacro ? steps.length > 0 : !!content.trim()))

  function save() {
    if (!canSave) return
    onSave({
      name: name.trim(),
      content,
      color,
      platformScoped: hasPlatform && platformScoped,
      steps: isMacro ? steps : null,
    })
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation()
      onClose()
    }
    // Ctrl/Cmd+Enter saves (the textarea owns plain Enter for newlines).
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault()
      save()
    }
  }
</script>

<div
  class="editor"
  style:background={theme.sidebarBg}
  style:border="1px solid {theme.border}"
  style:box-shadow={theme.windowShadow}
  style:font-family={theme.fontUi}
  role="dialog"
  aria-label={snippet ? 'Edit button' : 'New button'}
  tabindex="-1"
  onkeydown={onKeydown}
>
  <div class="row">
    <input
      class="name"
      placeholder="Button label"
      bind:value={name}
      style:background={theme.bodyBg}
      style:color={theme.textPrimary}
      style:border="1px solid {theme.border}"
    />
    <div class="swatches">
      <button
        class="swatch none"
        class:sel={color === null}
        title="No color (theme default)"
        style:border-color={theme.border}
        style:color={theme.textDim}
        onclick={() => (color = null)}>✕</button
      >
      {#each SWATCHES as c (c)}
        <button
          class="swatch"
          class:sel={color === c}
          style:background={c}
          style:box-shadow={color === c ? `0 0 0 2px ${theme.accent}` : 'none'}
          title={c}
          onclick={() => (color = c)}
          aria-label={`Color ${c}`}
        ></button>
      {/each}
    </div>
  </div>

  <div class="mode" style:color={theme.textMuted}>
    <button
      type="button"
      class="mode-btn"
      class:active={!isMacro}
      style:color={!isMacro ? theme.accent : theme.textDim}
      style:border-color={!isMacro ? theme.accent : theme.border}
      onclick={() => (isMacro = false)}>Text</button
    >
    <button
      type="button"
      class="mode-btn"
      class:active={isMacro}
      style:color={isMacro ? theme.accent : theme.textDim}
      style:border-color={isMacro ? theme.accent : theme.border}
      onclick={() => {
        isMacro = true
        if (steps.length === 0)
          steps.push({ kind: 'expect', expect: '', is_regex: false, send: '', timeout_ms: 10000 })
      }}>Macro</button
    >
  </div>

  {#if !isMacro}
    <textarea
      class="content"
      placeholder="Command(s) to send"
      bind:value={content}
      spellcheck="false"
      style:background={theme.bodyBg}
      style:color={theme.textPrimary}
      style:border="1px solid {theme.border}"
      style:font-family={theme.fontMono}
    ></textarea>
  {:else}
    <MacroStepEditor {theme} bind:steps />
  {/if}

  <div class="footer">
    {#if hasPlatform}
      <label
        class="scope"
        style:color={theme.textMuted}
        title="Show only in sessions of this platform"
      >
        <input type="checkbox" bind:checked={platformScoped} />
        {platform} only
      </label>
    {:else}
      <span class="scope" style:color={theme.textDim}>global</span>
    {/if}
    <span class="spacer"></span>
    {#if onDelete}
      <button class="btn danger" style:color={theme.err} onclick={onDelete}>Delete</button>
    {/if}
    <button
      class="btn"
      style:color={theme.textMuted}
      style:border="1px solid {theme.border}"
      onclick={onClose}>Cancel</button
    >
    <button
      class="btn primary"
      disabled={!canSave}
      style:background={theme.accent}
      style:color={theme.onAccent}
      onclick={save}>Save</button
    >
  </div>
</div>

<style>
  .editor {
    position: absolute;
    bottom: calc(100% + 6px);
    left: 8px;
    width: 460px;
    max-width: calc(100vw - 24px);
    border-radius: 7px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 400;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .name {
    flex: 1;
    min-width: 0;
    border-radius: 5px;
    padding: 5px 8px;
    font-size: 12.5px;
    font-family: inherit;
    outline: none;
  }

  .swatches {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .swatch {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    border: none;
    cursor: pointer;
    padding: 0;
    flex-shrink: 0;
    transition: transform 0.1s;
  }
  .swatch:hover {
    transform: scale(1.15);
  }
  .swatch.none {
    background: transparent;
    border: 1px solid;
    font-size: 9px;
    line-height: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .swatch.none.sel {
    font-weight: 700;
  }

  .content {
    min-height: 56px;
    max-height: 160px;
    resize: vertical;
    border-radius: 5px;
    padding: 6px 8px;
    font-size: 12px;
    line-height: 1.5;
    outline: none;
  }

  .mode {
    display: flex;
    gap: 4px;
  }
  .mode-btn {
    background: transparent;
    border: 1px solid;
    border-radius: 5px;
    padding: 2px 10px;
    font-size: 11.5px;
    font-family: inherit;
    cursor: pointer;
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .scope {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    cursor: pointer;
    user-select: none;
  }
  .scope input {
    accent-color: var(--zx-accent);
  }

  .spacer {
    flex: 1;
  }

  .btn {
    background: transparent;
    border: none;
    border-radius: 5px;
    padding: 5px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }
  .btn:hover {
    filter: brightness(1.1);
  }
  .btn.danger {
    margin-right: auto;
  }
  .btn.primary {
    font-weight: 600;
  }
  .btn.primary:disabled {
    opacity: 0.45;
    cursor: default;
    filter: none;
  }
</style>
