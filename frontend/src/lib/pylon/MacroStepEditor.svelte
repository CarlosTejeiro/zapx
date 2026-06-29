<script lang="ts">
  /// Reusable expect/send/wait step list for a macro. Shared by the button-bar
  /// inline editor (ButtonEditor) and the sidebar macro dialog (MacroDialog).
  /// `steps` is bindable so the parent owns the array.
  import type { PylonTheme } from '$lib/themes/index'
  import type { LoginStep } from '$lib/bridge/types'
  import Icon from '$lib/icons/Icon.svelte'

  interface Props {
    theme: PylonTheme
    steps: LoginStep[]
  }

  let { theme, steps = $bindable() }: Props = $props()

  export function addStep() {
    steps.push({ kind: 'expect', expect: '', is_regex: false, send: '', timeout_ms: 10000 })
  }
  function removeStep(i: number) {
    steps.splice(i, 1)
  }
  /// Swap a step with its neighbour (dir = -1 up, +1 down). No-op at the ends.
  function moveStep(i: number, dir: -1 | 1) {
    const j = i + dir
    if (j < 0 || j >= steps.length) return
    const a = steps[i]
    const b = steps[j]
    if (!a || !b) return
    steps[i] = b
    steps[j] = a
  }
</script>

<div class="steps">
  {#each steps as step, i (i)}
    <div class="mstep">
      <span class="m-move">
        <button
          type="button"
          class="m-arrow"
          style:color={theme.textDim}
          disabled={i === 0}
          title="Move up"
          aria-label="Move step up"
          onclick={() => moveStep(i, -1)}>▲</button
        >
        <button
          type="button"
          class="m-arrow"
          style:color={theme.textDim}
          disabled={i === steps.length - 1}
          title="Move down"
          aria-label="Move step down"
          onclick={() => moveStep(i, 1)}>▼</button
        >
      </span>
      <select
        class="m-kind"
        bind:value={step.kind}
        title="Step type"
        style:background={theme.bodyBg}
        style:color={theme.textPrimary}
        style:border="1px solid {theme.border}"
      >
        <option value="expect">expect</option>
        <option value="send">send</option>
        <option value="wait">wait</option>
      </select>
      {#if step.kind === 'expect'}
        <input
          class="m-in"
          placeholder="expect"
          bind:value={step.expect}
          spellcheck="false"
          style:background={theme.bodyBg}
          style:color={theme.textPrimary}
          style:border="1px solid {theme.border}"
        />
        <input
          class="m-in"
          placeholder="send"
          bind:value={step.send}
          spellcheck="false"
          style:background={theme.bodyBg}
          style:color={theme.textPrimary}
          style:border="1px solid {theme.border}"
        />
        <input
          class="m-ms"
          type="number"
          min={500}
          step={500}
          bind:value={step.timeout_ms}
          title="timeout (ms)"
          style:background={theme.bodyBg}
          style:color={theme.textPrimary}
          style:border="1px solid {theme.border}"
        />
      {:else if step.kind === 'send'}
        <input
          class="m-in m-wide"
          placeholder={'send (\\r for bare Enter)'}
          bind:value={step.send}
          spellcheck="false"
          style:background={theme.bodyBg}
          style:color={theme.textPrimary}
          style:border="1px solid {theme.border}"
        />
      {:else}
        <input
          class="m-ms m-waitms"
          type="number"
          min={100}
          step={100}
          bind:value={step.timeout_ms}
          title="pause (ms)"
          style:background={theme.bodyBg}
          style:color={theme.textPrimary}
          style:border="1px solid {theme.border}"
        />
        <span class="m-hint" style:color={theme.textDim}>ms pause</span>
      {/if}
      <button
        type="button"
        class="m-rm"
        style:color={theme.textDim}
        title="Remove step"
        onclick={() => removeStep(i)}
      >
        <Icon name="x" size={11} />
      </button>
    </div>
  {/each}
  <button
    type="button"
    class="m-add"
    style:color={theme.textDim}
    style:border="1px solid {theme.border}"
    onclick={addStep}>+ Add step</button
  >
</div>

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .mstep {
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .m-move {
    display: inline-flex;
    flex-direction: column;
    flex-shrink: 0;
    line-height: 0.7;
  }
  .m-arrow {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0 2px;
    font-size: 8px;
    line-height: 1;
  }
  .m-arrow:disabled {
    opacity: 0.3;
    cursor: default;
  }
  .m-kind {
    width: 4.6rem;
    flex-shrink: 0;
    border-radius: 5px;
    padding: 4px 4px;
    font-size: 11px;
    font-family: var(--zx-font-mono);
    outline: none;
  }
  .m-in {
    flex: 1;
    min-width: 0;
    border-radius: 5px;
    padding: 4px 6px;
    font-size: 11.5px;
    font-family: var(--zx-font-mono);
    outline: none;
  }
  .m-ms {
    width: 4.2rem;
    flex-shrink: 0;
    border-radius: 5px;
    padding: 4px 4px;
    font-size: 11.5px;
    outline: none;
  }
  .m-hint {
    font-size: 10.5px;
    flex-shrink: 0;
  }
  .m-rm {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 2px;
    display: inline-flex;
    flex-shrink: 0;
  }
  .m-add {
    align-self: flex-start;
    background: transparent;
    border-radius: 5px;
    padding: 3px 10px;
    font-size: 11.5px;
    font-family: inherit;
    cursor: pointer;
  }
</style>
