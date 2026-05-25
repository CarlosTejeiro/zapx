<script lang="ts">
  import { onMount } from 'svelte'
  import { listPlatforms, clearCommandHistory } from '$lib/bridge/commands'
  import type { PlatformInfo } from '$lib/bridge/types'
  import {
    hintsSettings,
    setGhostEnabled,
    setPopupEnabled,
    setDefaultPlatform,
  } from '$lib/hints/store.svelte'

  let platforms = $state<PlatformInfo[]>([])
  let clearingHistory = $state(false)
  let cleared = $state(false)

  async function refresh() {
    platforms = await listPlatforms()
  }

  async function clearAllHistory() {
    if (!confirm('Borrar todo el historial de comandos? Esta acción no se puede deshacer.')) return
    clearingHistory = true
    try {
      await clearCommandHistory(null)
      cleared = true
      setTimeout(() => (cleared = false), 2500)
    } finally {
      clearingHistory = false
    }
  }

  onMount(refresh)
</script>

<div class="panel">
  <section>
    <h3>Apariencia</h3>
    <label class="toggle">
      <input
        type="checkbox"
        checked={hintsSettings.ghostEnabled}
        onchange={(e) => setGhostEnabled((e.currentTarget as HTMLInputElement).checked)}
      />
      <span>Mostrar ghost text inline (estilo zsh-autosuggestions)</span>
    </label>
    <p class="hint-help">→ o End para aceptar la sugerencia.</p>

    <label class="toggle">
      <input
        type="checkbox"
        checked={hintsSettings.popupEnabled}
        onchange={(e) => setPopupEnabled((e.currentTarget as HTMLInputElement).checked)}
      />
      <span>Habilitar popup de sugerencias con Ctrl+Espacio</span>
    </label>
    <p class="hint-help">↑↓ para navegar, Tab/Enter para aceptar, Esc para cerrar.</p>
  </section>

  <section>
    <h3>Plataforma por defecto</h3>
    <p class="hint-help">Catálogo de comandos a usar cuando una sesión no tiene plataforma específica.</p>
    <select
      value={hintsSettings.defaultPlatform}
      onchange={(e) => setDefaultPlatform((e.currentTarget as HTMLSelectElement).value)}
    >
      {#each platforms as p (p.id)}
        <option value={p.id}>{p.name}</option>
      {/each}
    </select>
  </section>

  <section>
    <h3>Snippets</h3>
    <p class="hint-help">
      Los snippets que crees desde el diálogo de Snippets aparecen también como
      sugerencias prioritarias en el popup (badge ⭐).
    </p>
  </section>

  <section>
    <h3>Historial</h3>
    <p class="hint-help">
      Los comandos se guardan automáticamente por sesión. Los que parezcan contener
      contraseñas o tokens nunca entran al historial.
    </p>
    <button
      type="button"
      class="danger-btn"
      onclick={clearAllHistory}
      disabled={clearingHistory}
    >
      {#if cleared}✓ Historial borrado{:else if clearingHistory}Borrando…{:else}Borrar historial completo{/if}
    </button>
  </section>
</div>

<style>
  .panel {
    padding: 0.75rem 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    color: #e4e4e7;
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  h3 {
    font-size: 0.78rem;
    font-weight: 600;
    color: #d4d4d8;
    margin: 0 0 0.15rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.82rem;
    cursor: pointer;
    color: #d4d4d8;
  }

  .toggle input { accent-color: #22d3ee; }

  .hint-help {
    font-size: 0.72rem;
    color: #71717a;
    margin: 0 0 0.25rem 0;
    line-height: 1.45;
  }

  select {
    width: 16rem;
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    padding: 0.3rem 0.4rem;
    color: #e4e4e7;
    font-size: 0.82rem;
  }

  .danger-btn {
    background: rgba(239, 68, 68, 0.15);
    color: #fca5a5;
    border: none;
    border-radius: 0.25rem;
    padding: 0.3rem 0.7rem;
    font-size: 0.78rem;
    cursor: pointer;
    font-weight: 600;
    font-family: inherit;
    align-self: flex-start;
  }
  .danger-btn:hover:not(:disabled) {
    background: rgba(239, 68, 68, 0.25);
  }
  .danger-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
