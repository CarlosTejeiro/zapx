<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import type { PylonTheme } from '$lib/themes/index'

  interface MenuItem {
    label: string
    action?: () => void
    divider?: boolean
    disabled?: boolean
  }

  interface Props {
    theme: PylonTheme
    sessionName?: string
    onNewSession?: () => void
    onSettings?: () => void
    onToggleTheme?: () => void
    themeName?: string
  }

  const {
    theme,
    sessionName = '',
    onNewSession,
    onSettings,
    onToggleTheme,
    themeName = 'neon-noir',
  }: Props = $props()

  const win = getCurrentWindow()

  let openMenu = $state<string | null>(null)

  const menus = $derived<Record<string, MenuItem[]>>({
    File: [
      { label: 'New Window', disabled: true },
      { divider: true, label: '' },
      { label: 'Quit', action: () => win.close() },
    ],
    Edit: [
      { label: 'Copy', disabled: true },
      { label: 'Paste', disabled: true },
      { divider: true, label: '' },
      { label: 'Find  Ctrl+F', disabled: true },
    ],
    View: [
      { label: themeName === 'graphite' ? '● Graphite' : '  Graphite', action: () => { onToggleTheme?.(); openMenu = null } },
      { label: themeName === 'neon-noir' ? '● Neon Noir' : '  Neon Noir', action: () => { onToggleTheme?.(); openMenu = null } },
      { divider: true, label: '' },
      { label: 'Status Bar', disabled: true },
    ],
    Session: [
      { label: 'New Session  Ctrl+N', action: () => { onNewSession?.(); openMenu = null } },
      { divider: true, label: '' },
      { label: 'Disconnect', disabled: true },
      { label: 'Reconnect', disabled: true },
    ],
    Tools: [
      { label: 'Settings  Ctrl+,', action: () => { onSettings?.(); openMenu = null } },
      { divider: true, label: '' },
      { label: 'Snippets', disabled: true },
    ],
    Help: [
      { label: 'About zapx', disabled: true },
    ],
  })

  function toggleMenu(name: string) {
    openMenu = openMenu === name ? null : name
  }

  function closeMenus() {
    openMenu = null
  }
</script>

<!-- close menus on outside click -->
<svelte:window onclick={(e) => {
  const target = e.target as Element
  if (!target.closest?.('.tb-menu-wrap')) closeMenus()
}} />

<header class="titlebar" style:background={theme.titlebarBg} style:color={theme.textMuted}>

  <!-- Left: brand + active session -->
  <div class="tb-left">
    <svg class="tb-glyph" width="16" height="16" viewBox="0 0 16 16" fill="none">
      <rect x="2" y="2" width="3" height="12" rx="1" fill={theme.accent}/>
      <rect x="5" y="2" width="6" height="3" rx="1" fill={theme.accent}/>
      <rect x="5" y="7" width="5" height="3" rx="1" fill={theme.accent} opacity="0.7"/>
    </svg>

    <span class="tb-wordmark" style:color={theme.textPrimary} style:font-family={theme.fontUi}>
      zapx
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
    <nav class="tb-menu-bar" style:font-family={theme.fontUi}>
      {#each Object.entries(menus) as [name, items]}
        <div class="tb-menu-wrap" style:position="relative">
          <button
            class="tb-menu-item"
            class:active={openMenu === name}
            style:color={openMenu === name ? theme.textPrimary : theme.textMuted}
            style:background={openMenu === name ? theme.itemActiveBg : 'transparent'}
            onclick={() => toggleMenu(name)}
          >{name}</button>

          {#if openMenu === name}
            <div
              class="tb-dropdown"
              style:background={theme.sidebarBg}
              style:border="1px solid {theme.border}"
            >
              {#each items as item}
                {#if item.divider}
                  <div class="dd-divider" style:background={theme.border}></div>
                {:else}
                  <button
                    class="dd-item"
                    disabled={item.disabled}
                    style:color={item.disabled ? theme.textDim : theme.textPrimary}
                    style:font-family={theme.fontUi}
                    onclick={item.action}
                  >{item.label}</button>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </nav>

    <div class="tb-controls">
      <button class="tb-ctrl" title="Minimize" style:color={theme.textDim} onclick={() => win.minimize()}>─</button>
      <button class="tb-ctrl" title="Maximize" style:color={theme.textDim} onclick={() => win.toggleMaximize()}>□</button>
      <button class="tb-ctrl tb-ctrl-close" title="Close" style:color={theme.textDim} onclick={() => win.close()}>✕</button>
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
    position: relative;
    z-index: 100;
  }

  .tb-left {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    flex-shrink: 0;
  }

  .tb-glyph { flex-shrink: 0; }

  .tb-wordmark {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 1px;
  }

  .tb-sep { font-size: 12px; opacity: 0.4; }

  .tb-session {
    font-size: 12px;
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tb-drag { flex: 1; height: 100%; }

  .tb-right {
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .tb-menu-bar {
    display: flex;
    align-items: center;
  }

  .tb-menu-wrap {
    position: relative;
  }

  .tb-menu-item {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 11px;
    font-weight: 400;
    padding: 0 9px;
    height: 32px;
    color: inherit;
    transition: color 0.1s, background 0.1s;
    border-radius: 0;
  }

  .tb-menu-item:hover,
  .tb-menu-item.active {
    background: rgba(255,255,255,0.06);
    color: #c9cdd3;
  }

  .tb-dropdown {
    position: absolute;
    top: 32px;
    left: 0;
    min-width: 180px;
    border-radius: 6px;
    padding: 4px 0;
    box-shadow: 0 8px 32px rgba(0,0,0,0.5);
    z-index: 200;
  }

  .dd-item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 12px;
    padding: 5px 14px;
    white-space: nowrap;
    transition: background 0.08s;
    font-family: inherit;
  }

  .dd-item:not(:disabled):hover {
    background: rgba(255,255,255,0.07);
  }

  .dd-item:disabled {
    cursor: default;
    opacity: 0.4;
  }

  .dd-divider {
    height: 1px;
    margin: 4px 0;
  }

  .tb-controls { display: flex; height: 100%; }

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
