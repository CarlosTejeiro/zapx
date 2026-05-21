<script lang="ts">
  import TerminalTab from '$lib/terminal/TerminalTab.svelte'
  import type { PylonTheme } from '$lib/themes/index'
  import type { SavedSession, ColorPalette } from '$lib/bridge/types'
  import { getCachedPassword, setCachedPassword } from '$lib/credentialCache'

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

  // Password re-entry when keyring credential is missing
  let needsPassword = $state(false)
  let passwordInput = $state('')
  let sshOverride = $state<SshParams | null>(null)
  let termKey = $state(0)

  function handleNeedPassword() {
    const s = pane.savedSession
    if (s) {
      const cached = getCachedPassword(s.id)
      if (cached) {
        // Use cached password silently — no form shown
        sshOverride = { host: s.host ?? '', port: s.port ?? 22, user: s.username ?? '', password: cached }
        status = 'connecting'
        onStatusChange?.('connecting')
        termKey++
        return
      }
    }
    needsPassword = true
    status = 'error'
    onStatusChange?.('error')
  }

  function handlePasswordSubmit(e: Event) {
    e.preventDefault()
    if (!passwordInput.trim()) return
    const s = pane.savedSession
    if (!s) return
    const pw = passwordInput
    setCachedPassword(s.id, pw)
    sshOverride = { host: s.host ?? '', port: s.port ?? 22, user: s.username ?? '', password: pw }
    passwordInput = ''
    needsPassword = false
    status = 'connecting'
    onStatusChange?.('connecting')
    termKey++
  }

  const protocol = $derived(
    pane.savedSession?.protocol ?? (pane.ssh ? 'ssh' : pane.telnet ? 'telnet' : 'local')
  )

  const hostLabel = $derived(
    pane.ssh?.host ?? pane.savedSession?.host ?? ''
  )

  // Map PylonTheme terminal tokens → xterm ColorPalette
  const terminalPalette = $derived<ColorPalette>({
    background:    theme.terminal.bg,
    foreground:    theme.terminal.fg,
    cursor:        theme.terminal.cursor,
    black:         theme.terminal.black,
    red:           theme.terminal.red,
    green:         theme.terminal.green,
    yellow:        theme.terminal.yellow,
    blue:          theme.terminal.blue,
    magenta:       theme.terminal.magenta,
    cyan:          theme.terminal.cyan,
    white:         theme.terminal.white,
    brightBlack:   theme.terminal.brightBlack,
    brightRed:     theme.terminal.brightRed,
    brightGreen:   theme.terminal.brightGreen,
    brightYellow:  theme.terminal.brightYellow,
    brightBlue:    theme.terminal.brightBlue,
    brightMagenta: theme.terminal.brightMagenta,
    brightCyan:    theme.terminal.brightCyan,
    brightWhite:   theme.terminal.brightWhite,
  })
</script>

<!-- svelte-ignore a11y_no_static_element_interactions a11y_click_events_have_key_events -->
<div
  class="pane"
  style:box-shadow={focused ? `inset 0 0 0 1.5px ${theme.accent}88` : 'none'}
  onclick={onFocus}
>

  <!-- Pane header (22px) — always dark, floats on terminal surface -->
  <div
    class="pane-header"
    style:background={theme.terminal.bg}
    style:border-bottom="1px solid rgba(255,255,255,0.07)"
    style:font-family={theme.fontUi}
  >
    <span class="ph-dot" style:background={pane.color}></span>
    <span class="ph-name" style:color={theme.terminal.fg}>{pane.label}</span>
    {#if hostLabel}
      <span class="ph-host" style:color={theme.terminal.dim}>{hostLabel}</span>
    {/if}
    <span class="ph-proto" style:color={theme.terminal.dim}>{protocol.toUpperCase()}</span>
  </div>

  <!-- Terminal body -->
  <div class="pane-body" style:--term-bg={theme.terminal.bg}>
    {#if needsPassword}
      <!-- Inline re-auth form when keyring credential is missing -->
      <div class="pw-overlay" style:background={theme.terminal.bg}>
        <form class="pw-box" onsubmit={handlePasswordSubmit} style:font-family={theme.fontUi}>
          <svg class="pw-icon" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke={theme.terminal.cursor} stroke-width="2">
            <rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>
          </svg>
          <p class="pw-title" style:color={theme.terminal.fg}>
            {pane.savedSession?.username ?? 'user'}@{pane.savedSession?.host ?? 'host'}
          </p>
          <p class="pw-sub" style:color={theme.terminal.dim}>Credentials missing from keyring — enter password to reconnect</p>
          <input
            class="pw-input"
            type="password"
            placeholder="Password"
            bind:value={passwordInput}
            autocomplete="current-password"
            style:border="1px solid {theme.terminal.dim}"
            style:color={theme.terminal.fg}
          />
          <button
            class="pw-btn"
            type="submit"
            style:background={theme.terminal.cursor}
          >Connect</button>
        </form>
      </div>
    {:else}
      {#key termKey}
        <TerminalTab
          savedSession={sshOverride ? undefined : pane.savedSession}
          ssh={sshOverride ?? pane.ssh}
          telnet={pane.telnet}
          hideToolbar={true}
          pylonPalette={terminalPalette}
          {onGlobalShortcut}
          onNeedPassword={handleNeedPassword}
          onSessionOpen={() => handleStatus('connected')}
          onSessionError={() => handleStatus('error')}
          onSessionClose={() => handleStatus('closed')}
        />
      {/key}
    {/if}
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
    display: flex;
    flex-direction: column;
  }

  .pw-overlay {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .pw-box {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    width: 280px;
    padding: 28px 24px;
  }

  .pw-icon {
    margin-bottom: 4px;
  }

  .pw-title {
    font-size: 13px;
    font-weight: 600;
    margin: 0;
    text-align: center;
  }

  .pw-sub {
    font-size: 11px;
    text-align: center;
    margin: 0;
    line-height: 1.5;
    opacity: 0.7;
  }

  .pw-input {
    width: 100%;
    background: rgba(255,255,255,0.06);
    border-radius: 5px;
    padding: 7px 10px;
    font-size: 13px;
    outline: none;
    font-family: inherit;
    margin-top: 4px;
  }

  .pw-input:focus {
    border-color: rgba(255,255,255,0.35) !important;
  }

  .pw-btn {
    width: 100%;
    border: none;
    border-radius: 5px;
    padding: 8px;
    font-size: 12.5px;
    font-weight: 600;
    color: #fff;
    cursor: pointer;
    font-family: inherit;
    opacity: 0.9;
    transition: opacity 0.1s;
  }

  .pw-btn:hover {
    opacity: 1;
  }
</style>
