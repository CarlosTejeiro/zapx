<script lang="ts">
  import TerminalTab from '$lib/terminal/TerminalTab.svelte'
</script>

<div class="layout">
  <header class="tab-bar">
    <span class="app-name">zapx</span>
    <div class="tabs">
      <button class="tab active">shell</button>
      <button class="tab new-tab" onclick={() => console.log('TODO: new tab')}>+</button>
    </div>
  </header>

  <main class="terminal-area">
    <TerminalTab />
  </main>
</div>

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
    padding: 0.2rem 0.75rem;
    font-size: 0.8rem;
    border-radius: 0.25rem;
    border: none;
    cursor: pointer;
    background: transparent;
    color: #71717a;
    transition: background 0.1s;
  }

  .tab:hover {
    background: #27272a;
    color: #e4e4e7;
  }

  .tab.active {
    background: #27272a;
    color: #e4e4e7;
  }

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
</style>
