<script lang="ts">
  import { fade, scale } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'

  interface Props {
    title: string
    placeholder?: string
    initial?: string
    submitLabel?: string
    onSubmit: (value: string) => void
    onCancel: () => void
  }

  let {
    title,
    placeholder = '',
    initial = '',
    submitLabel = 'OK',
    onSubmit,
    onCancel,
  }: Props = $props()

  let value = $state(initial)
  let inputEl = $state<HTMLInputElement | null>(null)

  $effect(() => {
    if (inputEl) {
      inputEl.focus()
      inputEl.select()
    }
  })

  function submit() {
    const v = value.trim()
    if (!v) return
    onSubmit(v)
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onCancel()
  }
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  tabindex="-1"
  onkeydown={onKey}
  onclick={(e) => { if (e.target === e.currentTarget) onCancel() }}
  transition:fade={{ duration: 120 }}
>
  <form
    class="dialog"
    onsubmit={(e) => { e.preventDefault(); submit() }}
    transition:scale={{ start: 0.96, duration: 160, easing: cubicOut }}
  >
    <h3>{title}</h3>
    <input
      bind:this={inputEl}
      bind:value
      type="text"
      {placeholder}
      autocomplete="off"
      spellcheck="false"
    />
    <div class="actions">
      <button type="button" class="cancel" onclick={onCancel}>Cancel</button>
      <button type="submit" class="ok" disabled={!value.trim()}>{submitLabel}</button>
    </div>
  </form>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1100;
  }
  .dialog {
    width: 24rem;
    max-width: 92vw;
    background: rgba(13, 18, 32, 0.96);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 10px;
    padding: 1rem 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    color: #d4dff0;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.55);
  }
  h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
  }
  input {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 5px;
    padding: 0.4rem 0.55rem;
    color: #fff;
    font-size: 0.85rem;
    font-family: inherit;
    outline: none;
  }
  input:focus { border-color: #22d3ee; }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.45rem;
    margin-top: 0.1rem;
  }
  .actions button {
    border: 0;
    border-radius: 5px;
    padding: 0.4rem 0.85rem;
    font-size: 0.8rem;
    font-family: inherit;
    cursor: pointer;
  }
  .cancel { background: rgba(255, 255, 255, 0.07); color: #d4dff0; }
  .cancel:hover { background: rgba(255, 255, 255, 0.12); }
  .ok { background: #22d3ee; color: #052029; font-weight: 600; }
  .ok:hover:not(:disabled) { filter: brightness(1.1); }
  .ok:disabled { opacity: 0.45; cursor: not-allowed; }
</style>
