<script lang="ts">
  /// Paste a MobaXterm macro (the line list from its "Macro edition" dialog)
  /// and import it as a ZAPX macro. Parsing is live so the user sees how many
  /// steps will be created before importing.
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import type { PylonTheme } from '$lib/themes/index'
  import type { LoginStep } from '$lib/bridge/types'
  import { parseMobaMacro } from '$lib/snippets/mobaMacro'

  interface Props {
    theme: PylonTheme
    onImport: (name: string, steps: LoginStep[]) => void
    onClose: () => void
  }

  const { theme, onImport, onClose }: Props = $props()

  let name = $state('Imported macro')
  let text = $state('')

  const steps = $derived(parseMobaMacro(text))
  const canImport = $derived(!!name.trim() && steps.length > 0)

  function doImport() {
    if (!canImport) return
    onImport(name.trim(), steps)
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
  aria-label="Import MobaXterm macro"
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
    <h3 style:color={theme.textPrimary}>Import macro from MobaXterm</h3>
    <p class="hint" style:color={theme.textDim}>
      Paste the lines from MobaXterm's macro editor (text, RETURN, SLEEP=ms, BACK…).
    </p>

    <input
      class="name"
      placeholder="Macro name"
      bind:value={name}
      style:background={theme.bodyBg}
      style:color={theme.textPrimary}
      style:border="1px solid {theme.border}"
    />

    <textarea
      class="paste"
      placeholder={'ssh user@host\nRETURN\nSLEEP=1200\nyes\nRETURN'}
      bind:value={text}
      spellcheck="false"
      style:background={theme.bodyBg}
      style:color={theme.textPrimary}
      style:border="1px solid {theme.border}"
      style:font-family={theme.fontMono}
    ></textarea>

    <div class="footer">
      <span class="count" style:color={theme.textMuted}>
        {steps.length}
        {steps.length === 1 ? 'step' : 'steps'}
      </span>
      <span class="spacer"></span>
      <button
        class="btn"
        style:color={theme.textMuted}
        style:border="1px solid {theme.border}"
        onclick={onClose}>Cancel</button
      >
      <button
        class="btn primary"
        disabled={!canImport}
        style:background={theme.accent}
        style:color={theme.onAccent}
        onclick={doImport}>Import</button
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
    width: 480px;
    max-width: 94vw;
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
  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.4;
  }
  .name {
    border-radius: 5px;
    padding: 5px 8px;
    font-size: 12.5px;
    font-family: inherit;
    outline: none;
  }
  .paste {
    min-height: 160px;
    max-height: 320px;
    resize: vertical;
    border-radius: 5px;
    padding: 7px 9px;
    font-size: 12px;
    line-height: 1.5;
    outline: none;
  }
  .footer {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .count {
    font-size: 11.5px;
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
  .btn.primary {
    font-weight: 600;
  }
  .btn.primary:disabled {
    opacity: 0.45;
    cursor: default;
    filter: none;
  }
</style>
