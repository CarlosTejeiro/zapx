<script lang="ts">
  import type { HintController } from './controller.svelte'
  import type { Hint } from '$lib/bridge/types'

  interface Props {
    controller: HintController
    fontFamily?: string
    accentColor?: string
  }

  let { controller, fontFamily, accentColor = '#22d3ee' }: Props = $props()

  function sourceIcon(h: Hint): string {
    switch (h.source.kind) {
      case 'snippet':
        return '⭐'
      case 'history':
        return '⟳'
      case 'catalog':
        return '📘'
      case 'sequence':
        return '↪'
    }
  }

  function sourceLabel(h: Hint): string {
    switch (h.source.kind) {
      case 'snippet':
        return h.label ? `snippet · ${h.label}` : 'snippet'
      case 'history':
        return 'history'
      case 'catalog':
        return `catalog · ${h.source.platform}`
      case 'sequence':
        return 'usual next'
    }
  }

  function highlightPrefix(text: string, prefix: string): { head: string; tail: string } {
    if (prefix.length === 0) return { head: '', tail: text }
    const lower = text.toLowerCase()
    if (lower.startsWith(prefix.toLowerCase())) {
      return { head: text.slice(0, prefix.length), tail: text.slice(prefix.length) }
    }
    return { head: '', tail: text }
  }

  const prefix = $derived(controller.buffer.snapshot.text)
</script>

{#if controller.popupOpen && controller.hints.length > 0}
  <div
    class="popup"
    style:left="{controller.cursor.left}px"
    style:top="{controller.cursor.top + controller.cursor.cellHeight + 2}px"
    style:font-family={fontFamily}
    style:--accent={accentColor}
    role="listbox"
  >
    {#each controller.hints as hint, i (hint.text)}
      {@const { head, tail } = highlightPrefix(hint.text, prefix)}
      <button
        type="button"
        class="row"
        class:selected={i === controller.selected}
        onclick={() => controller.acceptIndex(i)}
        onmouseenter={() => (controller.selected = i)}
        role="option"
        aria-selected={i === controller.selected}
      >
        <span class="icon">{sourceIcon(hint)}</span>
        <span class="text">
          {#if head}<span class="hl">{head}</span>{/if}<span>{tail}</span>
        </span>
        <span class="source">{sourceLabel(hint)}</span>
      </button>
    {/each}
    <div class="footer">
      <span>↑↓ navega</span>
      <span>Tab acepta</span>
      <span>Esc cierra</span>
    </div>
  </div>
{/if}

<style>
  .popup {
    position: absolute;
    z-index: 5;
    min-width: 320px;
    max-width: 560px;
    background: rgba(13, 18, 32, 0.96);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    box-shadow:
      0 12px 32px rgba(0, 0, 0, 0.55),
      0 0 0 1px rgba(0, 0, 0, 0.3);
    padding: 0.25rem;
    color: #d4dff0;
    font-size: 0.78rem;
    animation: popup-in 120ms cubic-bezier(0.2, 0.8, 0.3, 1);
    overflow: hidden;
  }

  @keyframes popup-in {
    from {
      opacity: 0;
      transform: translateY(-4px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.3rem 0.55rem;
    background: transparent;
    border: 0;
    text-align: left;
    color: inherit;
    cursor: pointer;
    border-radius: 4px;
    font: inherit;
  }

  .row.selected {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    box-shadow: inset 2px 0 0 var(--accent);
  }

  .icon {
    width: 1.1rem;
    text-align: center;
    opacity: 0.85;
    flex-shrink: 0;
  }

  .text {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-ligatures: none;
  }

  .hl {
    color: var(--accent);
    font-weight: 600;
  }

  .source {
    font-size: 0.68rem;
    opacity: 0.55;
    flex-shrink: 0;
    margin-left: auto;
  }

  .footer {
    display: flex;
    gap: 0.85rem;
    justify-content: flex-end;
    padding: 0.25rem 0.55rem 0.1rem;
    font-size: 0.65rem;
    color: rgba(212, 223, 240, 0.45);
    border-top: 1px solid rgba(255, 255, 255, 0.05);
    margin-top: 0.15rem;
  }
</style>
