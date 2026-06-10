<script lang="ts">
  import { fly } from 'svelte/transition'
  import { cubicOut } from 'svelte/easing'
  import type { PylonTheme } from '$lib/themes/index'
  import Icon from '$lib/icons/Icon.svelte'

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
  style:--err={theme.err}
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
        style:border={active ? `1px solid ${theme.border}` : '1px solid transparent'}
        style:border-radius={theme.radius}
        style:box-shadow={active ? `inset 0 -2px 0 ${theme.accent}` : 'none'}
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
          ><Icon name="x" size={11} /></button>
        {/if}
      </div>
    {/each}

    <!-- New tab button -->
    <button
      class="tab-add"
      style:color={theme.textDim}
      style:border-radius={theme.radius}
      onclick={onAdd}
      title="New tab"
    ><Icon name="plus" size={14} /></button>
  </div>

  <!-- Right controls: Split + Multi -->
  <div class="tab-actions">
    <button
      class="tab-action-btn"
      class:active={splitOn}
      style:color={splitOn ? theme.accent : theme.textMuted}
      style:background={splitOn ? `${theme.accent}18` : 'transparent'}
      style:border="1px solid {splitOn ? `${theme.accent}55` : theme.border}"
      style:border-radius={theme.radius}
      onclick={onToggleSplit}
      title="Toggle split pane (Ctrl+\)"
    >
      <Icon name="split" size={13} />
      Split
    </button>
    <button
      class="tab-action-btn"
      class:active={multiOn}
      style:color={multiOn ? theme.accent2 : theme.textMuted}
      style:background={multiOn ? `${theme.accent2}18` : 'transparent'}
      style:border="1px solid {multiOn ? `${theme.accent2}55` : theme.border}"
      style:border-radius={theme.radius}
      onclick={onToggleMulti}
      title="Toggle MultiExec broadcast (Ctrl+Shift+M)"
    >
      <Icon name="cast" size={13} />
      Multi
    </button>
  </div>

</div>

<style>
  .tabbar {
    height: 42px;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    overflow: hidden;
    padding: 0 10px;
    gap: 4px;
  }

  .tab-strip {
    display: flex;
    align-items: center;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    gap: 4px;
    scrollbar-width: none;
  }

  .tab-strip::-webkit-scrollbar { display: none; }

  .tab {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 12px;
    min-width: 140px;
    max-width: 220px;
    cursor: pointer;
    flex-shrink: 0;
    position: relative;
    transition: background 0.1s;
  }

  .tab-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .tab-label {
    font-size: 12.5px;
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
    padding: 2px;
    border-radius: 4px;
    color: inherit;
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
  }

  .tab-close:hover {
    background: color-mix(in srgb, var(--err) 15%, transparent);
    color: var(--err);
  }

  .tab-add {
    background: none;
    border: none;
    cursor: pointer;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    transition: background 0.1s, color 0.1s;
    color: inherit;
  }

  .tab-add:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.06));
    color: var(--text-primary, #2c2924);
  }

  .tab-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .tab-action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    cursor: pointer;
    font-size: 11.5px;
    font-weight: 500;
    padding: 0 10px;
    height: 28px;
    font-family: inherit;
    transition: background 0.1s, color 0.1s, border-color 0.1s;
    color: inherit;
  }

  .tab-action-btn:hover {
    background: var(--item-hover-bg, rgba(255,255,255,0.06));
  }
</style>
