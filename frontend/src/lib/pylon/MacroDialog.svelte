<script lang="ts">
  /// Centered modal to create or edit a macro from the sidebar Macros section.
  /// Hosts the shared MacroStepEditor plus name/colour. Saving is delegated to
  /// the caller (App.svelte) which persists via createSnippet/updateSnippet +
  /// setSnippetSteps.
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import type { PylonTheme } from '$lib/themes/index'
  import type { Snippet, LoginStep } from '$lib/bridge/types'
  import MacroStepEditor from './MacroStepEditor.svelte'

  interface Props {
    theme: PylonTheme
    /** Editing an existing macro, or null when creating one. */
    macro: Snippet | null
    /** Existing folder names, offered as autocomplete suggestions. */
    folders?: string[]
    onSave: (v: {
      name: string
      color: string | null
      folder: string | null
      steps: LoginStep[]
    }) => void
    onDelete?: () => void
    onClose: () => void
  }

  const { theme, macro, folders = [], onSave, onDelete, onClose }: Props = $props()

  // One-shot snapshot of the macro into editable local state (the dialog is
  // remounted via {#key} when the target changes).
  // svelte-ignore state_referenced_locally
  let name = $state(macro?.name ?? '')
  // svelte-ignore state_referenced_locally
  let color = $state<string | null>(macro?.color ?? null)
  // svelte-ignore state_referenced_locally
  let folder = $state(macro?.folder ?? '')
  // svelte-ignore state_referenced_locally
  let steps = $state<LoginStep[]>(initialSteps(macro?.steps_json ?? null))

  /// Parse the saved steps, seeding one empty step so a fresh macro isn't an
  /// empty list.
  function initialSteps(json: string | null): LoginStep[] {
    if (json) {
      try {
        const v = JSON.parse(json)
        if (Array.isArray(v) && v.length > 0) return v
      } catch {
        /* fall through to the seeded default */
      }
    }
    return [{ kind: 'expect', expect: '', is_regex: false, send: '', timeout_ms: 10000 }]
  }

  const SWATCHES = ['#5eb3b2', '#5b8fc9', '#9a91e8', '#3e8f60', '#b88528', '#c2410c', '#b13a3a']

  const canSave = $derived(!!name.trim() && steps.length > 0)

  function save() {
    if (!canSave) return
    onSave({ name: name.trim(), color, folder: folder.trim() || null, steps })
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation()
      onClose()
    }
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label={macro ? 'Edit macro' : 'New macro'}
  tabindex="-1"
  onkeydown={onKeydown}
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose()
  }}
  transition:fade={{ duration: 120 }}
>
  <div
    class="dialog"
    style:background={theme.sidebarBg}
    style:border="1px solid {theme.border}"
    style:box-shadow={theme.windowShadow}
    style:font-family={theme.fontUi}
    transition:scale={{ start: 0.96, duration: 160, easing: cubicOut }}
  >
    <h3 style:color={theme.textPrimary}>{macro ? 'Edit macro' : 'New macro'}</h3>

    <div class="row">
      <input
        class="name"
        placeholder="Macro name"
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
            aria-label={`Color ${c}`}
            onclick={() => (color = c)}
          ></button>
        {/each}
      </div>
    </div>

    <input
      class="folder"
      placeholder="Folder (optional)"
      list="macro-folder-list"
      bind:value={folder}
      style:background={theme.bodyBg}
      style:color={theme.textPrimary}
      style:border="1px solid {theme.border}"
    />
    <datalist id="macro-folder-list">
      {#each folders as f (f)}
        <option value={f}></option>
      {/each}
    </datalist>

    <MacroStepEditor {theme} bind:steps />

    <div class="footer">
      {#if onDelete}
        <button class="btn danger" style:color={theme.err} onclick={onDelete}>Delete</button>
      {/if}
      <span class="spacer"></span>
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
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
  }
  .dialog {
    width: 540px;
    max-width: 94vw;
    max-height: 88vh;
    overflow-y: auto;
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h3 {
    margin: 0;
    font-size: 0.95rem;
    font-weight: 600;
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
  .folder {
    border-radius: 5px;
    padding: 5px 8px;
    font-size: 12px;
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
  .footer {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
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
