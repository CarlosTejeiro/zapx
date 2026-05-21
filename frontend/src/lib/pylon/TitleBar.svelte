<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import type { PylonTheme } from '$lib/themes/index'

  interface Props {
    theme: PylonTheme
    sessionName?: string
  }

  const { theme, sessionName = '' }: Props = $props()

  const win = getCurrentWindow()
</script>

<header class="titlebar" style:background={theme.titlebarBg} style:color={theme.textMuted}>

  <!-- Left: brand + active session -->
  <div class="tb-left">
    <!-- PYLON glyph — stylised P lettermark -->
    <svg class="tb-glyph" width="16" height="16" viewBox="0 0 16 16" fill="none">
      <rect x="2" y="2" width="3" height="12" rx="1" fill={theme.accent}/>
      <rect x="5" y="2" width="6" height="3" rx="1" fill={theme.accent}/>
      <rect x="5" y="7" width="5" height="3" rx="1" fill={theme.accent} opacity="0.7"/>
    </svg>

    <span class="tb-wordmark" style:color={theme.textPrimary} style:font-family={theme.fontUi}>
      PYLON
    </span>

    {#if sessionName}
      <span class="tb-sep" style:color={theme.textDim}>·</span>
      <span class="tb-session" style:color={theme.textMuted} style:font-family={theme.fontUi}>
        {sessionName}
      </span>
    {/if}
  </div>

  <!-- Center: drag region -->
  <div class="tb-drag" data-tauri-drag-region></div>

  <!-- Right: menu strip + window controls -->
  <div class="tb-right">
    <nav class="tb-menu" style:font-family={theme.fontUi}>
      {#each ['File', 'Edit', 'View', 'Session', 'Tools', 'Help'] as item}
        <button class="tb-menu-item" style:color={theme.textMuted}>{item}</button>
      {/each}
    </nav>

    <div class="tb-controls">
      <button
        class="tb-ctrl"
        title="Minimize"
        style:color={theme.textDim}
        onclick={() => win.minimize()}
      >─</button>
      <button
        class="tb-ctrl"
        title="Maximize"
        style:color={theme.textDim}
        onclick={() => win.toggleMaximize()}
      >□</button>
      <button
        class="tb-ctrl tb-ctrl-close"
        title="Close"
        style:color={theme.textDim}
        onclick={() => win.close()}
      >✕</button>
    </div>
  </div>

</header>

<style>
  .titlebar {
    height: 32px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
    border-bottom: 1px solid rgba(255,255,255,0.05);
  }

  .tb-left {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    flex-shrink: 0;
  }

  .tb-glyph {
    flex-shrink: 0;
  }

  .tb-wordmark {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 1px;
  }

  .tb-sep {
    font-size: 12px;
    opacity: 0.4;
  }

  .tb-session {
    font-size: 12px;
    font-weight: 400;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tb-drag {
    flex: 1;
    height: 100%;
  }

  .tb-right {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .tb-menu {
    display: flex;
    align-items: center;
    padding: 0 8px;
  }

  .tb-menu-item {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 11px;
    font-weight: 400;
    padding: 0 8px;
    height: 32px;
    color: inherit;
    transition: color 0.1s, background 0.1s;
    border-radius: 0;
  }

  .tb-menu-item:hover {
    background: rgba(255,255,255,0.06);
    color: #c9cdd3;
  }

  .tb-controls {
    display: flex;
    height: 100%;
  }

  .tb-ctrl {
    background: none;
    border: none;
    cursor: pointer;
    width: 46px;
    height: 32px;
    font-size: 11px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s, color 0.1s;
    color: inherit;
  }

  .tb-ctrl:hover {
    background: rgba(255,255,255,0.08);
    color: #c9cdd3;
  }

  .tb-ctrl-close:hover {
    background: #e81123;
    color: #ffffff;
  }
</style>
