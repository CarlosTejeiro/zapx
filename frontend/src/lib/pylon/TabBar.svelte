<script lang="ts">
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import type { PylonTheme } from '$lib/themes/index'

  export interface TabEntry {
    id: number
    label: string
    color: string
    status: 'connecting' | 'connected' | 'error' | 'closed'
  }

  interface Props {
    theme: PylonTheme
    tabs: TabEntry[]
    activeTabId: number
    splitOn?: boolean
    multiOn?: boolean
    onActivate: (id: number) => void
    onAdd: () => void
    onClose: (id: number) => void
    onToggleSplit?: () => void
    onToggleMulti?: () => void
  }

  const {
    theme,
    tabs,
    activeTabId,
    splitOn = false,
    multiOn = false,
    onActivate,
    onAdd,
    onClose,
    onToggleSplit,
    onToggleMulti,
  }: Props = $props()

  function statusColor(status: TabEntry['status']): string {
    if (status === 'connected') return theme.ok
    if (status === 'error')     return theme.err
    if (status === 'closed')    return theme.textDim
    return theme.warn
  }

  let hoveredTab = $state<number | null>(null)
</script>

<div
  class="tabbar"
  style:background={theme.tabBarBg}
  style:border-bottom="1px solid {theme.border}"
  style:font-family={theme.fontUi}
  style:--item-hover-bg={theme.itemHoverBg}
  style:--text-primary={theme.textPrimary}
>

  <!-- Tabs strip -->
  <div class="tab-strip">
    {#each tabs as tab (tab.id)}
      {@const active = tab.id === activeTabId}
      {@const hovered = hoveredTab === tab.id}
      <div
        class="tab"
        class:tab-active={active}
        in:fly={{ y: -8, duration: 180, easing: cubicOut }}
        style:background={active ? theme.tabActiveBg : theme.tabIdleBg}
        style:border={active ? `1px solid ${theme.border}` : `1px solid transparent`}
        style:border-bottom={active ? `1px solid ${theme.tabActiveBg}` : 'none'}
        style:border-radius={theme.tabRadius}
        style:border-top={active ? `2px solid ${theme.accent}` : '2px solid transparent'}
        onclick={() => onActivate(tab.id)}
        onmouseenter={() => hoveredTab = tab.id}
        onmouseleave={() => hoveredTab = null}
        role="button"
        tabindex="0"
        onkeydown={(e) => e.key === 'Enter' && onActivate(tab.id)}
      >
        <span class="tab-dot" style:background={statusColor(tab.status)}></span>
        <span class="tab-label" style:color={active ? theme.textPrimary : theme.textMuted}>
          {tab.label}
        </span>
        {#if active || hovered}
          <button
            class="tab-close"
            style:color={theme.textDim}
            onclick={(e) => { e.stopPropagation(); onClose(tab.id) }}
            title="Close tab"
          >✕</button>
        {/if}
      </div>
    {/each}

    <!-- New tab button -->
    <button
      class="tab-add"
      style:color={theme.textDim}
      onclick={onAdd}
      title="New tab"
    >+</button>
  </div>

  <!-- Right controls: Split + Multi -->
  <div class="tab-actions">
    <button
      class="tab-action-btn"
      class:active={splitOn}
      style:color={splitOn ? theme.accent : theme.textDim}
      style:background={splitOn ? `${theme.accent}18` : 'transparent'}
      onclick={onToggleSplit}
      title="Toggle split pane (Ctrl+\)"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <rect x="1" y="1" width="5" height="12" rx="1" stroke="currentColor" stroke-width="1.5"/>
        <rect x="8" y="1" width="5" height="12" rx="1" stroke="currentColor" stroke-width="1.5"/>
      </svg>
      Split
    </button>
    <button
      class="tab-action-btn"
      class:active={multiOn}
      style:color={multiOn ? theme.accent2 : theme.textDim}
      style:background={multiOn ? `${theme.accent2}18` : 'transparent'}
      onclick={onToggleMulti}
      title="Toggle MultiExec broadcast (Ctrl+Shift+M)"
    >
      <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
        <path d="M1 7h12M7 1v12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
      Multi
    </button>
  </div>

</div>

<style>
  .tabbar {
    height: 36px;
    display: flex;
    align-items: flex-end;
    flex-shrink: 0;
    overflow: hidden;
  }

  .tab-strip {
    display: flex;
    align-items: flex-end;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    padding: 0 4px;
    gap: 2px;
    scrollbar-width: none;
  }

  .tab-strip::-webkit-scrollbar { display: none; }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 10px;
    min-width: 130px;
    max-width: 220px;
    cursor: pointer;
    flex-shrink: 0;
    position: relative;
    transition: background 0.1s;
    margin-bottom: -1px;
  }

  .tab-active {
    box-shadow: 0 1px 0 0 var(--tab-active-bg, #1e2127);
  }

  .tab-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tab-label {
    font-size: 12px;
    font-weight: 400;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    transition: color 0.1s;
  }

  .tab-active .tab-label {
    font-weight: 500;
  }

  .tab-close {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 10px;
    padding: 2px 3px;
    border-radius: 3px;
    color: inherit;
    line-height: 1;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
  }

  .tab-close:hover {
    background: rgba(239,68,68,0.15);
    color: #ef4444;
  }

  .tab-add {
    background: none;
    border: none;
    cursor: pointer;
    width: 26px;
    height: 26px;
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    margin-bottom: 2px;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
    color: inherit;
  }

  .tab-add:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.06));
    color: var(--text-primary, #c9cdd3);
  }

  .tab-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 8px 4px;
    flex-shrink: 0;
  }

  .tab-action-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 4px;
    height: 22px;
    font-family: inherit;
    transition: background 0.1s, color 0.1s;
    color: inherit;
  }

  .tab-action-btn:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.06));
  }
</style>
