<script lang="ts">
  import TerminalSettingsPanel from './TerminalSettingsPanel.svelte'
  import HighlightRulesPanel from '$lib/sessions/HighlightRulesPanel.svelte'
  import {
    SHORTCUT_ACTIONS,
    bindings,
    eventToCombo,
    resetBinding,
    saveBinding,
    type ShortcutAction,
  } from '$lib/stores/keybindings.svelte'

  interface Props {
    onClose: () => void
  }

  let { onClose }: Props = $props()

  type Tab = 'appearance' | 'highlight' | 'shortcuts'
  let activeTab = $state<Tab>('appearance')

  /// While recording, swallow keys instead of dispatching them globally.
  let recordingAction = $state<ShortcutAction | null>(null)

  function beginRecord(id: ShortcutAction) {
    recordingAction = id
  }

  async function onRecordKeydown(e: KeyboardEvent) {
    if (!recordingAction) return
    // Escape exits recording without changing the binding.
    if (e.key === 'Escape' && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey) {
      e.preventDefault()
      e.stopPropagation()
      recordingAction = null
      return
    }
    const combo = eventToCombo(e)
    if (!combo) return // modifier-only press, keep waiting
    e.preventDefault()
    e.stopPropagation()
    const id = recordingAction
    recordingAction = null
    await saveBinding(id, combo)
  }

  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose()
  }

  function handleKeydown(e: KeyboardEvent) {
    if (recordingAction) return // recorder handles its own keys
    if (e.key === 'Escape') onClose()
  }
</script>

<svelte:window onkeydown={onRecordKeydown} />

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label="Settings"
  tabindex="-1"
  onclick={handleOverlayClick}
  onkeydown={handleKeydown}
>
  <div class="modal">
    <div class="modal-header">
      <h2 class="modal-title">Settings</h2>
      <button class="close-btn" onclick={onClose} aria-label="Close">✕</button>
    </div>

    <div class="modal-body">
      <nav class="tab-nav">
        <button
          class="tab-btn"
          class:active={activeTab === 'appearance'}
          onclick={() => (activeTab = 'appearance')}
        >
          Appearance
        </button>
        <button
          class="tab-btn"
          class:active={activeTab === 'highlight'}
          onclick={() => (activeTab = 'highlight')}
        >
          Highlight Rules
        </button>
        <button
          class="tab-btn"
          class:active={activeTab === 'shortcuts'}
          onclick={() => (activeTab = 'shortcuts')}
        >
          Shortcuts
        </button>
      </nav>

      <div class="tab-content">
        {#if activeTab === 'appearance'}
          <TerminalSettingsPanel />
        {:else if activeTab === 'highlight'}
          <HighlightRulesPanel />
        {:else}
          <div class="shortcuts-list">
            <p class="hint">
              Click a binding to record a new keystroke. Press <kbd class="kbd">Esc</kbd> to cancel.
            </p>
            {#each SHORTCUT_ACTIONS as a (a.id)}
              <div class="shortcut-row">
                <span class="shortcut-desc">{a.label}</span>
                {#if recordingAction === a.id}
                  <kbd class="kbd recording">press a key…</kbd>
                {:else}
                  <button class="kbd kbd-btn" onclick={() => beginRecord(a.id)}>
                    {bindings[a.id] || '(unbound)'}
                  </button>
                {/if}
                <button
                  class="reset-btn"
                  title="Reset to default ({a.default})"
                  onclick={() => resetBinding(a.id)}
                >↺</button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 200;
  }

  .modal {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    width: 42rem;
    max-width: 95vw;
    max-height: 85vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid #27272a;
    flex-shrink: 0;
  }

  .modal-title {
    font-size: 0.875rem;
    font-weight: 600;
    color: #e4e4e7;
    margin: 0;
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: #71717a;
    font-size: 1rem;
    padding: 0.1rem 0.3rem;
    border-radius: 0.2rem;
    font-family: inherit;
  }
  .close-btn:hover {
    color: #e4e4e7;
    background: #27272a;
  }

  .modal-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .tab-nav {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    padding: 0.5rem;
    border-right: 1px solid #27272a;
    min-width: 9rem;
    flex-shrink: 0;
  }

  .tab-btn {
    text-align: left;
    background: transparent;
    border: none;
    border-radius: 0.25rem;
    padding: 0.35rem 0.6rem;
    color: #71717a;
    font-size: 0.8rem;
    cursor: pointer;
    font-family: inherit;
  }
  .tab-btn:hover {
    color: #e4e4e7;
    background: #27272a;
  }
  .tab-btn.active {
    color: #e4e4e7;
    background: #27272a;
  }

  .tab-content {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .shortcuts-list {
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.25rem 0;
    border-bottom: 1px solid #27272a;
  }
  .shortcut-row:last-child {
    border-bottom: none;
  }

  .kbd {
    font-family: monospace;
    font-size: 0.72rem;
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.2rem;
    padding: 0.15rem 0.4rem;
    color: #e4e4e7;
    white-space: nowrap;
    min-width: 7rem;
    text-align: center;
    flex-shrink: 0;
  }

  .shortcut-desc {
    font-size: 0.78rem;
    color: #a1a1aa;
    line-height: 1.4;
    flex: 1;
  }

  .hint {
    font-size: 0.74rem;
    color: #71717a;
    margin: 0 0 0.5rem;
  }

  .kbd-btn {
    cursor: pointer;
    font-family: monospace;
  }

  .kbd-btn:hover {
    background: #3f3f46;
    border-color: #52525b;
  }

  .kbd.recording {
    background: #1e3a8a;
    border-color: #3b82f6;
    color: #fff;
    animation: pulse 1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  .reset-btn {
    background: transparent;
    border: none;
    color: #52525b;
    font-size: 0.95rem;
    cursor: pointer;
    padding: 0.2rem 0.4rem;
    border-radius: 0.2rem;
    flex-shrink: 0;
    font-family: inherit;
  }

  .reset-btn:hover {
    color: #e4e4e7;
    background: #27272a;
  }
</style>
