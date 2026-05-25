<script lang="ts">
  import { onMount } from 'svelte'
  import { getSetting, setSetting, openLogsDir } from '$lib/bridge/commands'
  import { showToast } from '$lib/ui/toast-store.svelte'

  type Format = 'plain' | 'raw'
  let format = $state<Format>('plain')
  let logsPath = $state<string | null>(null)

  onMount(async () => {
    try {
      const v = await getSetting('logging.format')
      if (v === 'raw') format = 'raw'
    } catch {
      // keep default
    }
  })

  async function update(next: Format) {
    format = next
    try {
      await setSetting('logging.format', next)
    } catch (e) {
      showToast({ kind: 'error', title: 'Setting', detail: String(e) })
    }
  }

  async function reveal() {
    try {
      logsPath = await openLogsDir()
    } catch (e) {
      showToast({ kind: 'error', title: 'Logs', detail: String(e) })
    }
  }
</script>

<div class="panel">
  <section>
    <h3>Formato del archivo</h3>
    <p class="hint">
      Controla cómo se guarda el contenido del terminal cuando activas el
      botón de grabación (●) en la cabecera de un pane.
    </p>

    <label class="row">
      <input
        type="radio"
        name="logfmt"
        value="plain"
        checked={format === 'plain'}
        onchange={() => update('plain')}
      />
      <div>
        <div class="row-title">Texto plano <span class="default-tag">por defecto</span></div>
        <div class="row-help">
          Elimina los códigos ANSI (colores, títulos de ventana, modos de paste)
          y deja sólo texto. Igual que la opción <em>Log Session</em> de
          SecureCRT. Recomendado para auditoría y para abrir el log en cualquier
          editor.
        </div>
      </div>
    </label>

    <label class="row">
      <input
        type="radio"
        name="logfmt"
        value="raw"
        checked={format === 'raw'}
        onchange={() => update('raw')}
      />
      <div>
        <div class="row-title">Raw</div>
        <div class="row-help">
          Guarda los bytes exactos que envía el remoto, incluyendo escape
          sequences. Útil para reproducir la sesión con
          <code>cat logfile</code> en otra terminal, o para depurar problemas
          de renderizado.
        </div>
      </div>
    </label>
  </section>

  <section>
    <h3>Ubicación</h3>
    <p class="hint">Los archivos rotan automáticamente cada 50&nbsp;MB.</p>
    <button type="button" class="ok-btn" onclick={reveal}>📂 Abrir carpeta de logs</button>
    {#if logsPath}
      <code class="path">{logsPath}</code>
    {/if}
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
  section { display: flex; flex-direction: column; gap: 0.5rem; }
  h3 {
    font-size: 0.78rem;
    font-weight: 600;
    color: #d4d4d8;
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .hint {
    font-size: 0.72rem;
    color: #71717a;
    margin: 0 0 0.25rem;
    line-height: 1.5;
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
    padding: 0.45rem 0.6rem;
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 4px;
    cursor: pointer;
  }
  .row input { accent-color: #22d3ee; margin-top: 3px; }
  .row-title { font-size: 0.82rem; color: #d4d4d8; }
  .row-help {
    font-size: 0.72rem;
    color: #71717a;
    line-height: 1.45;
    margin-top: 0.2rem;
  }
  .row-help code {
    background: rgba(34, 211, 238, 0.1);
    color: #67e8f9;
    padding: 0.02rem 0.3rem;
    border-radius: 3px;
    font-size: 0.68rem;
  }
  .default-tag {
    display: inline-block;
    margin-left: 0.35rem;
    padding: 0.03rem 0.35rem;
    background: rgba(34, 211, 238, 0.12);
    color: #67e8f9;
    font-size: 0.6rem;
    border-radius: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    font-weight: 600;
    vertical-align: middle;
  }
  .ok-btn {
    align-self: flex-start;
    background: #22d3ee;
    color: #052029;
    border: 0;
    border-radius: 4px;
    font-size: 0.78rem;
    font-family: inherit;
    font-weight: 600;
    padding: 0.35rem 0.85rem;
    cursor: pointer;
  }
  .ok-btn:hover { filter: brightness(1.1); }
  .path {
    font-family: 'JetBrains Mono', ui-monospace, monospace;
    font-size: 0.72rem;
    color: #a1a1aa;
    background: rgba(0, 0, 0, 0.2);
    padding: 0.3rem 0.5rem;
    border-radius: 3px;
    word-break: break-all;
  }
</style>
