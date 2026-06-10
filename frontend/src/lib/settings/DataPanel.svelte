<script lang="ts">
  import { onMount } from 'svelte'
  import { open as openDialog, ask } from '@tauri-apps/plugin-dialog'
  import {
    getDataDirInfo,
    setDataDir,
    resetDataDir,
    restartApp,
    type DataDirInfo,
  } from '$lib/bridge/commands'
  import { showToast } from '$lib/ui/toast-store.svelte'

  let info = $state<DataDirInfo | null>(null)
  let busy = $state(false)
  let pendingRestart = $state(false)

  const SOURCE_LABELS: Record<DataDirInfo['source'], string> = {
    'cli-flag': 'flag --data-dir',
    'env-var': 'variable ZAPX_DATA_DIR',
    portable: 'modo portable (marcador junto al ejecutable)',
    pointer: 'carpeta personalizada',
    default: 'ubicación por defecto del sistema',
  }

  onMount(async () => {
    try {
      info = await getDataDirInfo()
    } catch (e) {
      showToast({ kind: 'error', title: 'Data', detail: String(e) })
    }
  })

  async function promptRestart() {
    pendingRestart = true
    const yes = await ask(
      'El cambio se aplica al reiniciar ZAPX. ¿Reiniciar ahora?\n\nLas sesiones abiertas se cerrarán.',
      { title: 'Reiniciar ZAPX', kind: 'info' },
    )
    if (yes) await restartApp()
  }

  async function changeFolder() {
    if (busy) return
    const dir = await openDialog({ directory: true, title: 'Carpeta de datos de ZAPX' })
    if (typeof dir !== 'string' || !dir) return
    busy = true
    try {
      const summary = await setDataDir(dir)
      showToast({ kind: 'success', title: 'Carpeta de datos', detail: summary })
      await promptRestart()
    } catch (e) {
      showToast({ kind: 'error', title: 'Carpeta de datos', detail: String(e) })
    } finally {
      busy = false
    }
  }

  async function useDefault() {
    if (busy) return
    busy = true
    try {
      await resetDataDir()
      showToast({
        kind: 'success',
        title: 'Carpeta de datos',
        detail: 'A partir del próximo arranque se usará la ubicación por defecto. Los datos actuales no se mueven.',
      })
      await promptRestart()
    } catch (e) {
      showToast({ kind: 'error', title: 'Carpeta de datos', detail: String(e) })
    } finally {
      busy = false
    }
  }
</script>

<div class="panel">
  <section>
    <h3>Carpeta de datos</h3>
    <p class="hint">
      Dónde guarda ZAPX la base de datos de sesiones (<code>zapx.db</code>),
      los logs de sesión y los catálogos de snippets/hints. Útil para llevarla
      a una carpeta sincronizada (Drive, OneDrive) o a una unidad compartida.
    </p>

    {#if !info}
      <p class="hint">Cargando…</p>
    {:else}
      <code class="path">{info.path}</code>
      <p class="hint">
        Origen: <strong>{SOURCE_LABELS[info.source]}</strong>
        {#if pendingRestart}
          · <strong>pendiente de reinicio</strong>
        {/if}
      </p>

      {#if info.portable}
        <p class="notice">
          Estás en <strong>modo portable</strong>: los datos viven junto al
          ejecutable y las credenciales se guardan cifradas (AES-256-GCM) en la
          base de datos en lugar del llavero del sistema. Cualquiera con acceso
          a la carpeta puede usarlas — trátala como tratarías una llave física.
        </p>
      {:else if !info.changeable}
        <p class="notice">
          La ubicación está forzada por {SOURCE_LABELS[info.source]} y no puede
          cambiarse desde aquí.
        </p>
      {:else}
        <div class="actions">
          <button type="button" class="ok-btn" disabled={busy} onclick={changeFolder}>
            Cambiar carpeta…
          </button>
          {#if info.source === 'pointer'}
            <button type="button" class="ghost-btn" disabled={busy} onclick={useDefault}>
              Volver a la ubicación por defecto
            </button>
          {/if}
        </div>
        <p class="hint">
          Al cambiarla se copian la base de datos, los logs y los catálogos a la
          nueva carpeta (si ya contiene datos de ZAPX, se adoptan tal cual). El
          cambio se aplica tras reiniciar. Por defecto: <code>{info.default_path}</code>
        </p>
      {/if}
    {/if}
  </section>

  <section>
    <h3>Modo portable (Windows)</h3>
    <p class="hint">
      Descarga el <code>ZAPX_x64_portable.exe</code> de la release, ponlo en
      cualquier carpeta o USB y crea al lado un fichero vacío llamado
      <code>portable</code>. Todos los datos vivirán en <code>data/</code>
      junto al ejecutable y viajarán con él.
    </p>
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
  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--zx-text);
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .hint {
    font-size: 0.72rem;
    color: var(--zx-text-muted);
    margin: 0;
    line-height: 1.5;
  }
  .hint code {
    background: color-mix(in srgb, var(--zx-accent) 10%, transparent);
    color: var(--zx-accent);
    padding: 0.02rem 0.3rem;
    border-radius: 3px;
    font-size: 0.68rem;
    word-break: break-all;
  }
  .path {
    font-family: var(--zx-font-mono);
    font-size: 0.72rem;
    color: var(--zx-text-muted);
    background: color-mix(in srgb, var(--zx-text) 6%, transparent);
    padding: 0.3rem 0.5rem;
    border-radius: 3px;
    word-break: break-all;
  }
  .notice {
    font-size: 0.72rem;
    line-height: 1.5;
    margin: 0;
    padding: 0.45rem 0.6rem;
    border: 1px solid color-mix(in srgb, var(--zx-warn) 45%, transparent);
    background: color-mix(in srgb, var(--zx-warn) 10%, transparent);
    border-radius: var(--zx-radius);
    color: var(--zx-text);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .ok-btn {
    align-self: flex-start;
    background: var(--zx-accent);
    color: var(--zx-on-accent);
    border: 0;
    border-radius: 4px;
    font-size: 0.78rem;
    font-family: inherit;
    font-weight: 600;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
  }
  .ok-btn:hover { filter: brightness(1.1); }
  .ok-btn:disabled { opacity: 0.5; cursor: default; }
  .ghost-btn {
    background: transparent;
    color: var(--zx-text-muted);
    border: 1px solid var(--zx-border);
    border-radius: 4px;
    font-size: 0.78rem;
    font-family: inherit;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
  }
  .ghost-btn:hover { background: var(--zx-hover-bg); color: var(--zx-text); }
  .ghost-btn:disabled { opacity: 0.5; cursor: default; }
</style>
