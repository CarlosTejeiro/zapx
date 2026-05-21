<script lang="ts">
  import TerminalTab from '$lib/terminal/TerminalTab.svelte'
  import type { PylonTheme } from '$lib/themes/index'
  import type { SavedSession } from '$lib/bridge/types'

  interface SshParams { host: string; port: number; user: string; password: string }
  interface TelnetParams { host: string; port: number }

  export interface PaneData {
    id: number
    label: string
    color: string
    savedSession?: SavedSession
    ssh?: SshParams
    telnet?: TelnetParams
  }

  type PaneStatus = 'connecting' | 'connected' | 'error' | 'closed'

  interface Props {
    theme: PylonTheme
    pane: PaneData
    focused: boolean
    onFocus: () => void
    onStatusChange?: (status: PaneStatus) => void
    onGlobalShortcut?: (key: string, e: KeyboardEvent) => void
  }

  const { theme, pane, focused, onFocus, onStatusChange, onGlobalShortcut }: Props = $props()

  let status = $state<PaneStatus>('connecting')

  function handleStatus(s: PaneStatus) {
    status = s
    onStatusChange?.(s)
  }

  const protocol = $derived(
    pane.savedSession?.protocol ?? (pane.ssh ? 'ssh' : pane.telnet ? 'telnet' : 'local')
  )

  const hostLabel = $derived(
    pane.ssh?.host ?? pane.savedSession?.host ?? ''
  )
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="pane"
  style:box-shadow={focused ? `inset 0 0 0 1.5px ${theme.accent}88` : 'none'}
  onclick={onFocus}
>

  <!-- Pane header (22px) -->
  <div
    class="pane-header"
    style:background={theme.titlebarBg}
    style:border-bottom="1px solid {theme.border}"
    style:font-family={theme.fontUi}
  >
    <span class="ph-dot" style:background={pane.color}></span>
    <span class="ph-name" style:color={theme.textPrimary}>{pane.label}</span>
    {#if hostLabel}
      <span class="ph-host" style:color={theme.textDim}>{hostLabel}</span>
    {/if}
    <span class="ph-proto" style:color={theme.textDim}>{protocol.toUpperCase()}</span>
  </div>

  <!-- Terminal body -->
  <div class="pane-body">
    <TerminalTab
      savedSession={pane.savedSession}
      ssh={pane.ssh}
      telnet={pane.telnet}
      {onGlobalShortcut}
      onSessionOpen={() => handleStatus('connected')}
      onSessionError={() => handleStatus('error')}
      onSessionClose={() => handleStatus('closed')}
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
    transition: box-shadow 0.1s;
  }

  .pane-header {
    height: 22px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
    flex-shrink: 0;
    user-select: none;
    cursor: default;
  }

  .ph-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .ph-name {
    font-size: 11.5px;
    font-weight: 500;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ph-host {
    font-size: 10.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 140px;
  }

  .ph-proto {
    font-size: 9.5px;
    font-weight: 600;
    letter-spacing: 0.4px;
    flex-shrink: 0;
  }

  .pane-body {
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
</style>
