<script lang="ts">
  import TerminalTab from '$lib/terminal/TerminalTab.svelte'
  import SshConnectDialog from '$lib/terminal/SshConnectDialog.svelte'
  import type { SshParams } from '$lib/terminal/SshConnectDialog.svelte'

  interface Tab {
    id: number
    label: string
    ssh?: SshParams
  }

  let nextId = 1
  const firstTab: Tab = { id: nextId++, label: 'shell' }
  let tabs = $state<Tab[]>([firstTab])
  let activeId = $state(firstTab.id)
  let showDialog = $state(false)

  function openSshTab(params: SshParams) {
    showDialog = false
    const tab: Tab = { id: nextId++, label: `${params.user}@${params.host}`, ssh: params }
    tabs = [...tabs, tab]
    activeId = tab.id
  }

  function closeTab(id: number) {
    tabs = tabs.filter((t) => t.id !== id)
    if (activeId === id) activeId = tabs.at(-1)?.id ?? 0
  }
</script>

<div class="layout">
  <header class="tab-bar">
    <span class="app-name">zapx</span>
    <div class="tabs">
      {#each tabs as tab (tab.id)}
        <button
          class="tab"
          class:active={activeId === tab.id}
          onclick={() => (activeId = tab.id)}
        >
          {tab.label}
          {#if tabs.length > 1}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <span
              class="close-btn"
              role="button"
              tabindex="-1"
              onclick={(e) => { e.stopPropagation(); closeTab(tab.id) }}
            >×</span>
          {/if}
        </button>
      {/each}

      <button class="tab new-tab" onclick={() => (showDialog = true)} title="New SSH session">+</button>
    </div>
  </header>

  <main class="terminal-area">
    {#each tabs as tab (tab.id)}
      <div class="tab-content" class:hidden={activeId !== tab.id}>
        <TerminalTab ssh={tab.ssh} />
      </div>
    {/each}
  </main>
</div>

{#if showDialog}
  <SshConnectDialog onConnect={openSshTab} onCancel={() => (showDialog = false)} />
{/if}

<style>
  .layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #09090b;
    color: #e4e4e7;
  }

  .tab-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    height: 2.25rem;
    padding: 0 0.75rem;
    background: #18181b;
    border-bottom: 1px solid #27272a;
    flex-shrink: 0;
    user-select: none;
  }

  .app-name {
    font-size: 0.75rem;
    font-weight: 600;
    color: #71717a;
    letter-spacing: 0.05em;
  }

  .tabs {
    display: flex;
    gap: 0.25rem;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.2rem 0.75rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: none;
    cursor: pointer;
    background: transparent;
    color: #71717a;
    transition: background 0.1s;
    font-family: inherit;
  }

  .tab:hover { background: #27272a; color: #e4e4e7; }
  .tab.active { background: #27272a; color: #e4e4e7; }

  .close-btn {
    font-size: 0.9rem;
    line-height: 1;
    color: #52525b;
    padding: 0 0.1rem;
  }

  .close-btn:hover { color: #ef4444; }

  .new-tab {
    font-size: 1rem;
    padding: 0.1rem 0.5rem;
  }

  .terminal-area {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .tab-content.hidden {
    display: none;
  }
</style>
