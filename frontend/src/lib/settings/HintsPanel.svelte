<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { listen, type UnlistenFn } from '@tauri-apps/api/event'
  import {
    listPlatforms,
    clearCommandHistory,
    reloadHintCatalogs,
    hintCatalogsDir,
  } from '$lib/bridge/commands'
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

  // Custom catalog UI state.
  let catalogStatus = $state<{ kind: 'ok' | 'err'; text: string } | null>(null)
  let catalogDir = $state<string | null>(null)
  let reloading = $state(false)

  function flashCatalog(kind: 'ok' | 'err', text: string) {
    catalogStatus = { kind, text }
    setTimeout(() => {
      if (catalogStatus?.text === text) catalogStatus = null
    }, 3500)
  }

  async function openCatalogsDir() {
    try {
      const path = await hintCatalogsDir()
      catalogDir = path
      flashCatalog('ok', path ? `Carpeta abierta: ${path}` : 'Carpeta no configurada.')
    } catch (e) {
      flashCatalog('err', e instanceof Error ? e.message : String(e))
    }
  }

  async function reloadCatalogs() {
    reloading = true
    try {
      const n = await reloadHintCatalogs()
      flashCatalog('ok', n === 0
        ? 'Sin ficheros de usuario — usando los catálogos integrados.'
        : `Recargado: ${n} catálogo${n === 1 ? '' : 's'} sustituido${n === 1 ? '' : 's'} por tus ficheros.`)
    } catch (e) {
      flashCatalog('err', e instanceof Error ? e.message : String(e))
    } finally {
      reloading = false
    }
  }

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

  let unlistenReload: UnlistenFn | null = null
  let unlistenError: UnlistenFn | null = null

  onMount(async () => {
    await refresh()
    // Surface auto-reloads emitted by the backend file watcher so the user
    // sees that their edit took effect without pressing the button.
    unlistenReload = await listen<number>('hint-catalogs-reloaded', (e) => {
      flashCatalog('ok', e.payload === 0
        ? 'Cambios detectados — usando los catálogos integrados.'
        : `Recargado automáticamente: ${e.payload} catálogo${e.payload === 1 ? '' : 's'} en uso.`)
    })
    unlistenError = await listen<string>('hint-catalogs-error', (e) => {
      flashCatalog('err', `Recarga fallida: ${e.payload}`)
    })
  })

  onDestroy(() => {
    unlistenReload?.()
    unlistenError?.()
  })
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
    <h3>Catálogos personalizados</h3>
    <p class="hint-help">
      Sustituye los comandos integrados por tus propias listas: deja un fichero
      <code>&lt;plataforma&gt;.json</code> (p.ej. <code>fortigate.json</code>)
      en la carpeta de catálogos. Se recarga <strong>automáticamente</strong>
      al guardar el fichero — el botón es solo por si quieres forzarlo.
    </p>
    <div class="catalog-actions">
      <button type="button" class="catalog-btn" onclick={openCatalogsDir}>
        📁 Abrir carpeta de catálogos
      </button>
      <button type="button" class="catalog-btn" onclick={reloadCatalogs} disabled={reloading}>
        {reloading ? '↻ Recargando…' : '↻ Recargar catálogos'}
      </button>
    </div>
    {#if catalogStatus}
      <p class="catalog-msg" class:err={catalogStatus.kind === 'err'}>{catalogStatus.text}</p>
    {/if}
    {#if catalogDir}
      <p class="catalog-path" title={catalogDir}>{catalogDir}</p>
    {/if}
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
    color: var(--zx-text);
    font-family: var(--zx-font-ui);
  }

  section {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  h3 {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--zx-text);
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
    color: var(--zx-text);
  }

  .toggle input { accent-color: var(--zx-accent); }

  .hint-help {
    font-size: 0.72rem;
    color: var(--zx-text-muted);
    margin: 0 0 0.25rem 0;
    line-height: 1.45;
  }

  select {
    width: 16rem;
    background: var(--zx-surface-2);
    border: 1px solid var(--zx-border);
    border-radius: 0.25rem;
    padding: 0.3rem 0.4rem;
    color: var(--zx-text);
    font-size: 0.82rem;
  }

  .danger-btn {
    background: color-mix(in srgb, var(--zx-err) 15%, transparent);
    color: var(--zx-err);
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
    background: color-mix(in srgb, var(--zx-err) 25%, transparent);
  }
  .danger-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .catalog-actions {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
  }

  .catalog-btn {
    background: color-mix(in srgb, var(--zx-accent) 15%, transparent);
    color: var(--zx-accent);
    border: 1px solid color-mix(in srgb, var(--zx-accent) 30%, transparent);
    border-radius: 0.25rem;
    padding: 0.3rem 0.65rem;
    font-size: 0.78rem;
    cursor: pointer;
    font-family: inherit;
  }

  .catalog-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--zx-accent) 25%, transparent);
  }

  .catalog-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .catalog-msg {
    font-size: 0.72rem;
    color: var(--zx-ok);
    margin: 0;
  }

  .catalog-msg.err {
    color: var(--zx-err);
  }

  .catalog-path {
    font-size: 0.7rem;
    color: var(--zx-text-muted);
    font-family: var(--zx-font-mono);
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  code {
    background: var(--zx-surface-2);
    border-radius: 0.2rem;
    padding: 0 0.3rem;
    font-size: 0.72rem;
  }
</style>
