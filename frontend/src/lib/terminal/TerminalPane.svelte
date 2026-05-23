<script lang="ts">
  import TerminalTab from './TerminalTab.svelte'
  import type { SavedSession, AuthMethod } from '$lib/bridge/types'

  interface SshParams { host: string; port: number; user: string; auth: AuthMethod }
  interface TelnetParams { host: string; port: number }

  export interface PaneInfo {
    id: number
    label: string
    savedSession?: SavedSession
    ssh?: SshParams
    telnet?: TelnetParams
  }

  interface Props {
    pane: PaneInfo
    active: boolean
    canClose: boolean
    onActivate: () => void
    onSplitH: () => void
    onSplitV: () => void
    onClose: () => void
    onGlobalShortcut?: (key: string, e: KeyboardEvent) => void
    onStatusChange?: (paneId: number, status: PaneStatus) => void
  }

  export type PaneStatus = 'connecting' | 'connected' | 'error' | 'closed'

  let {
    pane,
    active,
    canClose,
    onActivate,
    onSplitH,
    onSplitV,
    onClose,
    onGlobalShortcut,
    onStatusChange,
  }: Props = $props()

  let status = $state<PaneStatus>('connecting')

  function handleStatusChange(s: PaneStatus) {
    status = s
    onStatusChange?.(pane.id, s)
  }

  const protocol = $derived(
    pane.savedSession?.protocol ?? (pane.ssh ? 'ssh' : pane.telnet ? 'telnet' : 'local'),
  )

  const statusColor = $derived(
    status === 'connected' ? '#22c55e'
    : status === 'error' ? '#ef4444'
    : status === 'closed' ? '#71717a'
    : '#f59e0b',
  )
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="pane"
  class:active
  onclick={onActivate}
>
  <div class="pane-header" class:active>
    <span class="status-dot" style:background={statusColor}></span>

    <span class="pane-protocol">{protocol.toUpperCase()}</span>
    <span class="pane-label">{pane.label}</span>

    <span class="pane-actions">
      <button
        class="pane-btn"
        onclick={(e) => { e.stopPropagation(); onSplitH() }}
        title="Split horizontal"
      >⊟</button>
      <button
        class="pane-btn"
        onclick={(e) => { e.stopPropagation(); onSplitV() }}
        title="Split vertical"
      >⊠</button>
      {#if canClose}
        <button
          class="pane-btn pane-btn-close"
          onclick={(e) => { e.stopPropagation(); onClose() }}
          title="Close pane"
        >✕</button>
      {/if}
    </span>
  </div>

  <div class="pane-body">
    <TerminalTab
      savedSession={pane.savedSession}
      ssh={pane.ssh}
      telnet={pane.telnet}
      paneId={pane.id}
      {onGlobalShortcut}
      onSessionOpen={() => handleStatusChange('connected')}
      onSessionError={() => handleStatusChange('error')}
      onSessionClose={() => handleStatusChange('closed')}
    />
  </div>
</div>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    outline: 2px solid transparent;
    outline-offset: -2px;
    transition: outline-color 0.1s;
  }

  .pane.active {
    outline-color: #3b82f6;
  }

  .pane-header {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0 0.5rem;
    height: 1.5rem;
    background: #1c1c1c;
    border-bottom: 1px solid #2a2a2a;
    flex-shrink: 0;
    cursor: default;
    user-select: none;
  }

  .pane-header.active {
    background: #1e2a3a;
    border-bottom-color: #3b82f6;
  }

  .status-dot {
    width: 0.45rem;
    height: 0.45rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .pane-protocol {
    font-size: 0.6rem;
    color: #52525b;
    font-weight: 600;
    letter-spacing: 0.05em;
    flex-shrink: 0;
  }

  .pane-label {
    font-size: 0.72rem;
    color: #a1a1aa;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pane-header.active .pane-label {
    color: #e4e4e7;
  }

  .pane-actions {
    display: flex;
    gap: 0.1rem;
    flex-shrink: 0;
  }

  .pane-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #52525b;
    font-size: 0.7rem;
    padding: 0.1rem 0.25rem;
    border-radius: 0.2rem;
    font-family: inherit;
    line-height: 1;
  }

  .pane-btn:hover {
    color: #e4e4e7;
    background: #3f3f46;
  }

  .pane-btn-close:hover {
    color: #ef4444;
    background: rgba(239, 68, 68, 0.15);
  }

  .pane-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
</style>
