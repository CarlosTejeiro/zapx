<script lang="ts">
  import { fly } from 'svelte/transition'
  import { toasts, dismissToast, type Toast } from './toast-store.svelte'

  function colorFor(kind: Toast['kind']): { fg: string; bg: string; border: string; icon: string } {
    switch (kind) {
      case 'success': return { fg: '#4ade80', bg: 'rgba(74,222,128,0.10)', border: 'rgba(74,222,128,0.45)', icon: '✓' }
      case 'warning': return { fg: '#fbbf24', bg: 'rgba(251,191,36,0.10)', border: 'rgba(251,191,36,0.45)', icon: '⚠' }
      case 'error':   return { fg: '#ef4444', bg: 'rgba(239,68,68,0.10)',  border: 'rgba(239,68,68,0.45)',  icon: '⨯' }
      default:        return { fg: '#22d3ee', bg: 'rgba(34,211,238,0.10)', border: 'rgba(34,211,238,0.40)', icon: 'ℹ' }
    }
  }
</script>

<div class="stack">
  {#each toasts as toast (toast.id)}
    {@const c = colorFor(toast.kind)}
    <div
      class="toast"
      style:background={c.bg}
      style:border-color={c.border}
      transition:fly={{ x: 40, duration: 200 }}
      role="status"
      aria-live="polite"
    >
      <span class="icon" style:color={c.fg}>{c.icon}</span>
      <div class="body">
        <div class="title">{toast.title}</div>
        {#if toast.detail}<div class="detail">{toast.detail}</div>{/if}
      </div>
      <button
        type="button"
        class="close"
        onclick={() => dismissToast(toast.id)}
        aria-label="Cerrar"
      >✕</button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    top: 56px;
    right: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 950;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 0.55rem;
    background: rgba(13, 18, 32, 0.95);
    backdrop-filter: blur(10px);
    -webkit-backdrop-filter: blur(10px);
    border: 1px solid;
    border-radius: 8px;
    padding: 0.55rem 0.7rem 0.55rem 0.65rem;
    min-width: 260px;
    max-width: 360px;
    color: #d4dff0;
    font-size: 0.78rem;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
    pointer-events: auto;
  }

  .icon {
    font-size: 0.95rem;
    margin-top: 1px;
    flex-shrink: 0;
  }

  .body {
    flex: 1;
    line-height: 1.4;
  }

  .title {
    font-weight: 600;
    color: #e4e4e7;
  }

  .detail {
    margin-top: 2px;
    color: rgba(212, 223, 240, 0.65);
    font-size: 0.72rem;
  }

  .close {
    background: transparent;
    border: 0;
    color: rgba(212, 223, 240, 0.45);
    cursor: pointer;
    padding: 0 0.2rem;
    font-size: 0.85rem;
  }
  .close:hover { color: #e4e4e7; }
</style>
