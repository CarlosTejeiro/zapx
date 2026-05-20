<script lang="ts">
  import ThemeSelector from './ThemeSelector.svelte'
  import {
    terminalSettings,
    colorSchemes,
    applyFontSize,
    applyFontFamily,
    applyCursorStyle,
    applyCursorBlink,
  } from '$lib/stores/settings.svelte'

  const FONT_FAMILIES = [
    'Cascadia Code, JetBrains Mono, monospace',
    'JetBrains Mono, monospace',
    'Fira Code, monospace',
    'Consolas, monospace',
    'monospace',
  ]

  const FONT_SIZES = [10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24]
</script>

<div class="panel">
  <section>
    <h3 class="section-title">Theme</h3>
    {#if colorSchemes.length === 0}
      <p class="hint">Loading…</p>
    {:else}
      <ThemeSelector />
    {/if}
  </section>

  <section>
    <h3 class="section-title">Font</h3>
    <label class="field">
      <span class="label">Family</span>
      <select
        class="select"
        value={terminalSettings.fontFamily}
        onchange={(e) => applyFontFamily((e.target as HTMLSelectElement).value)}
      >
        {#each FONT_FAMILIES as f}
          <option value={f}>{f.split(',')[0]}</option>
        {/each}
      </select>
    </label>
    <label class="field">
      <span class="label">Size</span>
      <select
        class="select"
        value={terminalSettings.fontSize}
        onchange={(e) => applyFontSize(parseInt((e.target as HTMLSelectElement).value, 10))}
      >
        {#each FONT_SIZES as s}
          <option value={s}>{s}px</option>
        {/each}
      </select>
    </label>
  </section>

  <section>
    <h3 class="section-title">Cursor</h3>
    <div class="cursor-row">
      {#each (['block', 'underline', 'bar'] as const) as style}
        <button
          class="cursor-btn"
          class:active={terminalSettings.cursorStyle === style}
          onclick={() => applyCursorStyle(style)}
        >
          {style}
        </button>
      {/each}
    </div>
    <label class="field">
      <span class="label">Blink</span>
      <input
        type="checkbox"
        checked={terminalSettings.cursorBlink}
        onchange={(e) => applyCursorBlink((e.target as HTMLInputElement).checked)}
        class="accent-blue-500"
      />
    </label>
  </section>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 0;
    font-size: 0.8rem;
    color: #e4e4e7;
  }

  section {
    border-bottom: 1px solid #27272a;
    padding-bottom: 0.5rem;
    margin-bottom: 0;
  }

  .section-title {
    font-size: 0.7rem;
    font-weight: 600;
    color: #71717a;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.5rem 0.75rem 0.25rem;
    margin: 0;
  }

  .hint {
    padding: 0.25rem 0.75rem;
    color: #52525b;
    font-size: 0.75rem;
  }

  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.2rem 0.75rem;
    gap: 0.5rem;
  }

  .label {
    color: #a1a1aa;
    flex-shrink: 0;
  }

  .select {
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #e4e4e7;
    font-size: 0.75rem;
    padding: 0.15rem 0.4rem;
    flex: 1;
    max-width: 160px;
  }

  .cursor-row {
    display: flex;
    gap: 0.25rem;
    padding: 0.2rem 0.75rem;
  }

  .cursor-btn {
    flex: 1;
    padding: 0.2rem 0;
    font-size: 0.7rem;
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    color: #71717a;
    cursor: pointer;
    font-family: inherit;
  }

  .cursor-btn:hover {
    color: #e4e4e7;
    border-color: #52525b;
  }

  .cursor-btn.active {
    background: #1e3a5f44;
    border-color: #3b82f6;
    color: #93c5fd;
  }
</style>
