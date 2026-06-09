<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { SearchAddon } from '@xterm/addon-search'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import '@xterm/xterm/css/xterm.css'

  import {
    openSavedSession,
    openSshSession,
    openTelnetSession,
    sshPreflightHostKey,
    sshTrustHostKey,
    startSessionLogging,
    stopSessionLogging,
    listSessionLogs,
  } from '$lib/bridge/commands'
  import type { SavedSession, SessionLog, AuthMethod, HostKeyStatus } from '$lib/bridge/types'
  import HostKeyDialog from './HostKeyDialog.svelte'
  import TunnelsDialog from './TunnelsDialog.svelte'
  import SftpDialog from './SftpDialog.svelte'
  import {
    broadcast,
    broadcastTargets,
    registerSession,
    unregisterSession,
  } from '$lib/stores/sessionRuntime.svelte'
  import { matchAction } from '$lib/stores/keybindings.svelte'
  import { terminalSettings, colorSchemes } from '$lib/stores/settings.svelte'
  import type { ColorPalette } from '$lib/bridge/types'
  import { HintController } from '$lib/hints/controller.svelte'
  import GhostText from '$lib/hints/GhostText.svelte'
  import HintPopup from '$lib/hints/HintPopup.svelte'
  import { hintsSettings } from '$lib/hints/store.svelte'
  import { recordCommand } from '$lib/bridge/commands'
  import { showToast } from '$lib/ui/toast-store.svelte'

  const DEFAULT_PALETTE: ColorPalette = {
    background: '#282c34', foreground: '#abb2bf', cursor: '#528bff',
    black: '#282c34', red: '#e06c75', green: '#98c379', yellow: '#e5c07b',
    blue: '#61afef', magenta: '#c678dd', cyan: '#56b6c2', white: '#abb2bf',
    brightBlack: '#5c6370', brightRed: '#e06c75', brightGreen: '#98c379',
    brightYellow: '#e5c07b', brightBlue: '#61afef', brightMagenta: '#c678dd',
    brightCyan: '#56b6c2', brightWhite: '#ffffff',
  }

  function parsePalette(json: string | null): ColorPalette {
    if (!json) return DEFAULT_PALETTE
    try { return JSON.parse(json) as ColorPalette } catch { return DEFAULT_PALETTE }
  }

  /// Per-session color-scheme override stored in `savedSession.options_json`
  /// under the `color_scheme` key. Falls back to the global active scheme.
  const sessionColorScheme = $derived.by<string | null>(() => {
    if (!savedSession?.options_json) return null
    try {
      const opts = JSON.parse(savedSession.options_json) as { color_scheme?: string }
      return typeof opts.color_scheme === 'string' && opts.color_scheme ? opts.color_scheme : null
    } catch {
      return null
    }
  })

  const activeSchemeName = $derived(sessionColorScheme ?? terminalSettings.activeColorScheme)

  const activePalette = $derived<ColorPalette>(
    parsePalette(
      colorSchemes.find((s) => s.name === activeSchemeName)?.palette_json ?? null,
    ),
  )

  // Host-key (known_hosts) confirmation prompt state.
  let hostKeyPrompt = $state<{
    host: string
    port: number
    fingerprint: string
    changed: boolean
  } | null>(null)
  let hostKeyResolve: ((ok: boolean) => void) | null = null

  /// Verify a host key before connecting. Returns false if the user declined
  /// (or the key changed), in which case the connection must be aborted.
  async function confirmHostKey(host: string, port: number): Promise<boolean> {
    let status: HostKeyStatus
    try {
      status = await sshPreflightHostKey(host, port)
    } catch {
      // Preflight failed (e.g. host unreachable) — let the real connect surface it.
      return true
    }
    if (status.status === 'known') return true

    const changed = status.status === 'changed'
    const approved = await new Promise<boolean>((resolve) => {
      hostKeyResolve = resolve
      hostKeyPrompt = { host, port, fingerprint: status.fingerprint, changed }
    })
    hostKeyPrompt = null
    hostKeyResolve = null
    if (!approved) return false

    // Only "unknown" is approvable; persist trust before connecting.
    try {
      await sshTrustHostKey(host, port)
    } catch (e) {
      errorMsg = fmtError(e)
      return false
    }
    return true
  }

  interface SshParams {
    host: string
    port: number
    user: string
    auth: AuthMethod
  }

  interface TelnetParams {
    host: string
    port: number
  }

  interface Props {
    ssh?: SshParams
    telnet?: TelnetParams
    savedSession?: SavedSession
    /** Pane id from the host. Used to track focused/broadcasting sessions. */
    paneId?: number
    hideToolbar?: boolean
    pylonPalette?: ColorPalette
    onGlobalShortcut?: (key: string, e: KeyboardEvent) => void
    onSessionOpen?: () => void
    onSessionError?: () => void
    onSessionClose?: () => void
    onNeedPassword?: () => void
  }

  let { ssh, telnet, savedSession, paneId, hideToolbar = false, pylonPalette, onGlobalShortcut, onSessionOpen, onSessionError, onSessionClose, onNeedPassword }: Props = $props()

  function isKeyringMissing(e: unknown): boolean {
    const raw = typeof e === 'string' ? e : (e instanceof Error ? e.message : (() => { try { return JSON.stringify(e) } catch { return '' } })())
    try {
      const obj = JSON.parse(raw) as Record<string, unknown>
      const inner = typeof obj.Internal === 'string' ? obj.Internal : raw
      return inner.includes('keyring') && inner.includes('No matching entry')
    } catch {
      return raw.includes('keyring') && raw.includes('No matching entry')
    }
  }

  // When a pylon theme palette is provided it takes precedence over user color scheme settings
  const effectivePalette = $derived(pylonPalette ?? activePalette)

  let container: HTMLDivElement
  let sessionId = $state<string | null>(null)
  let errorMsg = $state<string | null>(null)
  // Held for $effect theme/font reactivity — set inside onMount.
  let term: InstanceType<typeof Terminal> | null = null
  let hintController = $state<HintController | null>(null)

  // Logging state
  let isLogging = $state(false)
  let logPath = $state<string | null>(null)
  let logError = $state<string | null>(null)

  // Log history panel
  let showLogs = $state(false)
  let pastLogs = $state<SessionLog[]>([])
  let logsLoading = $state(false)

  // Search state
  let showSearch = $state(false)
  let searchQuery = $state('')
  let searchAddon: SearchAddon | null = null

  // Port-forwards + SFTP browser (only for SSH sessions)
  let showTunnels = $state(false)
  let showSftp = $state(false)
  const isSsh = $derived(savedSession?.protocol === 'ssh' || ssh != null)

  // Login-automation progress (filled by the `login-script-progress` event).
  let loginProgress = $state<{ current: number; total: number; status: string } | null>(null)

  /// Pending paste — shown when the clipboard text has more than one line.
  /// The user explicitly approves before bytes go to the remote shell.
  let pastePending = $state<string | null>(null)

  interface TerminalDataPayload {
    session_id: string
    data: number[]
  }

  function fmtError(e: unknown): string {
    // Tauri errors arrive as JSON strings like {"Internal":"keyring error: ..."}
    const raw = typeof e === 'string' ? e : (e instanceof Error ? e.message : (() => { try { return JSON.stringify(e) } catch { return 'Unknown error' } })())
    try {
      const obj = JSON.parse(raw) as Record<string, unknown>
      const inner = typeof obj.Internal === 'string' ? obj.Internal : raw
      if (inner.includes('keyring') && inner.includes('No matching entry')) {
        return 'Saved credentials not found in keyring.\n\nFix: delete this session and recreate it — you will be prompted to re-enter your password.\n\nIf running in WSL, ensure gnome-keyring is running:\n  eval $(gnome-keyring-daemon --start --components=secrets)'
      }
      if (inner.includes('Connection refused')) return 'Connection refused — check the host and port.'
      if (inner.includes('timeout') || inner.includes('timed out')) return 'Connection timed out — the host is unreachable.'
      if (inner.includes('Authentication') || inner.includes('authentication')) return 'Authentication failed — check your username and password.'
      return inner
    } catch {
      return raw
    }
  }

  async function waitForTauri(timeoutMs = 5000): Promise<void> {
    const deadline = Date.now() + timeoutMs
    while (Date.now() < deadline) {
      if ('__TAURI_INTERNALS__' in window) return
      await new Promise((r) => setTimeout(r, 50))
    }
    throw new Error('Tauri IPC not available after ' + timeoutMs + 'ms')
  }

  async function toggleLogging() {
    if (!sessionId) return
    logError = null
    if (isLogging) {
      try {
        await stopSessionLogging(sessionId)
        isLogging = false
        logPath = null
      } catch (e) {
        logError = String(e)
      }
    } else {
      const name = savedSession?.name ?? ssh?.host ?? 'local'
      const saved_session_id = savedSession?.id ?? null
      try {
        logPath = await startSessionLogging(sessionId, saved_session_id, name)
        isLogging = true
      } catch (e) {
        logError = String(e)
      }
    }
  }

  async function loadLogs() {
    if (!savedSession) return
    logsLoading = true
    try {
      pastLogs = await listSessionLogs(savedSession.id)
    } catch {
      pastLogs = []
    } finally {
      logsLoading = false
    }
  }

  function toggleLogs() {
    showLogs = !showLogs
    if (showLogs) loadLogs()
  }

  function formatBytes(b: number): string {
    if (b < 1024) return `${b} B`
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`
    return `${(b / (1024 * 1024)).toFixed(1)} MB`
  }

  function formatDate(iso: string): string {
    return new Date(iso).toLocaleString()
  }

  onMount(async () => {
    term = new Terminal({
      cursorBlink: terminalSettings.cursorBlink,
      cursorStyle: terminalSettings.cursorStyle,
      fontSize: terminalSettings.fontSize,
      fontFamily: terminalSettings.fontFamily,
      lineHeight: terminalSettings.lineHeight,
      theme: effectivePalette,
    })

    const fitAddon = new FitAddon()
    searchAddon = new SearchAddon()
    term.loadAddon(fitAddon)
    term.loadAddon(searchAddon)
    term.open(container)
    fitAddon.fit()
    // Focus immediately so the user can start typing the moment the tab
    // mounts. Without this, the hidden xterm textarea has no focus until
    // the user clicks the surface — which feels like the connection toast
    // "blocks" input even though it's just that nothing is listening.
    term.focus()

    // ── Clipboard polish ──────────────────────────────────────────────────
    // Intercept paste at the container level (capture phase, so we run before
    // xterm's own handler). If the clipboard payload is multi-line, surface
    // a confirmation dialog — typical foot-gun is pasting a config block and
    // having every line execute as a separate command.
    container.addEventListener(
      'paste',
      (e: ClipboardEvent) => {
        const text = e.clipboardData?.getData('text') ?? ''
        if (!text) return
        if (/\r|\n/.test(text)) {
          e.preventDefault()
          e.stopPropagation()
          pastePending = text
        }
        // Single-line: let xterm handle it (bracketed paste mode is honoured
        // automatically when the remote shell enables it).
      },
      { capture: true },
    )

    // Copy-on-select: when the user releases the mouse with a non-empty
    // selection, push it to the clipboard. Silently ignore failures (e.g.
    // browser denying clipboard access).
    container.addEventListener('mouseup', () => {
      if (term?.hasSelection()) {
        const sel = term.getSelection()
        if (sel) navigator.clipboard.writeText(sel).catch(() => {})
      }
    })

    // Ctrl+F opens search bar; user-bound global shortcuts are forwarded to
    // App so xterm doesn't consume them as terminal control characters.
    term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.type !== 'keydown') return true
      if (e.ctrlKey && e.key === 'f') {
        showSearch = !showSearch
        if (!showSearch) searchAddon?.clearDecorations()
        return false
      }
      if (e.key === 'Escape' && showSearch) {
        showSearch = false
        searchAddon?.clearDecorations()
        return false
      }

      // ── Hint keybindings ────────────────────────────────────────────────
      const hc = hintController
      if (hc) {
        if (hc.popupOpen) {
          if (e.key === 'ArrowDown') { hc.movePopup(1); return false }
          if (e.key === 'ArrowUp')   { hc.movePopup(-1); return false }
          if (e.key === 'Escape')    { hc.closePopup(); return false }
          if (e.key === 'Tab' || e.key === 'Enter') {
            hc.acceptIndex(hc.selected)
            return false
          }
        }
        // Ctrl+Space opens the popup; some terminals send Ctrl+@ on macOS.
        if ((e.ctrlKey || e.metaKey) && e.code === 'Space') {
          hc.openPopup()
          return false
        }
        if ((e.key === 'ArrowRight' || e.key === 'End') && hc.ghost) {
          hc.acceptGhost()
          return false
        }
        if (e.key === 'Escape' && hc.ghost) {
          hc.clear()
          return false
        }
      }

      const action = matchAction(e)
      if (action) {
        onGlobalShortcut?.(action, e)
        return false
      }
      // Quick-fire snippet by index (Ctrl+Shift+1..9). Forwarded to App as
      // `snippet:N` so xterm doesn't swallow the keys as control characters.
      if (e.ctrlKey && e.shiftKey && e.code.startsWith('Digit')) {
        const d = parseInt(e.code.slice('Digit'.length), 10)
        if (d >= 1 && d <= 9) {
          onGlobalShortcut?.(`snippet:${d - 1}`, e)
          return false
        }
      }
      return true
    })

    // Open the backend session.
    try {
      await waitForTauri()

      // Verify the SSH host key (known_hosts) before connecting.
      if (savedSession?.protocol === 'ssh' && savedSession.host) {
        if (!(await confirmHostKey(savedSession.host, savedSession.port ?? 22))) {
          if (!errorMsg) errorMsg = 'Host key not trusted'
          onSessionError?.()
          term.dispose()
          return
        }
      } else if (ssh) {
        if (!(await confirmHostKey(ssh.host, ssh.port))) {
          if (!errorMsg) errorMsg = 'Host key not trusted'
          onSessionError?.()
          term.dispose()
          return
        }
      }

      if (savedSession) {
        sessionId = await openSavedSession(savedSession.id, term.cols, term.rows)
      } else if (ssh) {
        sessionId = await openSshSession(ssh.host, ssh.port, ssh.user, ssh.auth, term.cols, term.rows)
      } else if (telnet) {
        sessionId = await openTelnetSession(telnet.host, telnet.port, term.cols, term.rows)
      } else {
        sessionId = await invoke<string>('open_local_session')
      }
    } catch (e) {
      if (isKeyringMissing(e)) {
        onNeedPassword?.()
        term.dispose()
        return
      }
      errorMsg = fmtError(e)
      onSessionError?.()
      term.dispose()
      return
    }

    // Make this session discoverable by the snippets / broadcast machinery.
    if (paneId != null && sessionId) registerSession(paneId, sessionId)

    // Wire up the hint controller now that the session is live.
    hintController = new HintController({
      savedSessionId: savedSession?.id ?? null,
      sendBytes: (data: Uint8Array) => {
        if (!sessionId) return
        invoke('send_input', { sessionId, data: Array.from(data) }).catch(console.error)
      },
      term,
      ghostEnabled: () => hintsSettings.ghostEnabled,
      popupEnabled: () => hintsSettings.popupEnabled,
    })

    {
      const label = savedSession?.name ?? ssh?.host ?? telnet?.host ?? 'local'
      showToast({ kind: 'success', title: 'Conectado', detail: label })
    }
    onSessionOpen?.()
    // Re-focus after the connect — xterm may have lost focus during the
    // host-key dialog, password prompt, or while the await chain ran.
    // Without this, the first key after "Conectado" can land nowhere.
    term.focus()

    // Forward PTY output to xterm.
    const unlisten: UnlistenFn = await listen<TerminalDataPayload>(
      'terminal-data',
      (event) => {
        if (!term || event.payload.session_id !== sessionId) return
        term.write(new Uint8Array(event.payload.data), () => {
          hintController?.onIncomingFlushed()
        })
      },
    )

    // Login-script progress badge (auto-clears a couple of seconds after
    // the script completes; sticks on timeout so the user notices).
    const unlistenLogin: UnlistenFn = await listen<{
      session_id: string
      current: number
      total: number
      status: string
    }>('login-script-progress', (event) => {
      if (event.payload.session_id !== sessionId) return
      loginProgress = {
        current: event.payload.current,
        total: event.payload.total,
        status: event.payload.status,
      }
      if (event.payload.status === 'complete') {
        setTimeout(() => {
          if (loginProgress?.status === 'complete') loginProgress = null
        }, 2500)
      }
    })

    // Forward keyboard input to the PTY (and to every other registered
    // session when MultiExec broadcast is on).
    term.onData((data: string) => {
      if (!sessionId) return
      const u8 = new TextEncoder().encode(data)
      // Feed the hint buffer first so it sees the bytes BEFORE the PTY
      // round-trip. The returned non-null value is a flushed command line
      // which we record asynchronously.
      const submitted = hintController?.onOutgoing(u8) ?? null
      if (submitted) {
        recordCommand(savedSession?.id ?? null, submitted).catch(console.error)
      }
      const bytes = Array.from(u8)
      invoke('send_input', { sessionId, data: bytes }).catch(console.error)
      if (broadcast.enabled) {
        for (const otherId of broadcastTargets(sessionId)) {
          invoke('send_input', { sessionId: otherId, data: bytes }).catch(console.error)
        }
      }
    })

    // Resize terminal when the container size changes.
    const observer = new ResizeObserver(() => {
      if (!term) return
      fitAddon.fit()
      if (sessionId) {
        invoke('resize_terminal', {
          sessionId,
          cols: term.cols,
          rows: term.rows,
        }).catch(console.error)
      }
    })
    observer.observe(container)

    onDestroy(async () => {
      observer.disconnect()
      unlisten()
      unlistenLogin()
      hintController?.clear()
      hintController = null
      term?.dispose()
      if (paneId != null) unregisterSession(paneId)
      if (sessionId) {
        if (isLogging) {
          await stopSessionLogging(sessionId).catch(console.error)
        }
        await invoke('close_session', { sessionId }).catch(console.error)
        sessionId = null
      }
      onSessionClose?.()
    })
  })

  // Reactive search: run whenever query changes and search bar is open
  $effect(() => {
    if (showSearch && searchAddon) {
      if (searchQuery) {
        searchAddon.findNext(searchQuery, { incremental: true })
      } else {
        searchAddon.clearDecorations()
      }
    }
  })

  // Reactive appearance: push new theme/font settings to the live terminal
  $effect(() => {
    if (!term) return
    term.options.theme = effectivePalette
    term.options.fontSize = terminalSettings.fontSize
    term.options.fontFamily = terminalSettings.fontFamily
    term.options.cursorStyle = terminalSettings.cursorStyle
    term.options.cursorBlink = terminalSettings.cursorBlink
  })
</script>

<div class="terminal-wrapper">
  {#if hostKeyPrompt}
    <HostKeyDialog
      host={hostKeyPrompt.host}
      port={hostKeyPrompt.port}
      fingerprint={hostKeyPrompt.fingerprint}
      changed={hostKeyPrompt.changed}
      onTrust={() => hostKeyResolve?.(true)}
      onCancel={() => hostKeyResolve?.(false)}
    />
  {/if}
  {#if showTunnels && sessionId}
    <TunnelsDialog
      sessionId={sessionId}
      onClose={() => (showTunnels = false)}
    />
  {/if}
  {#if showSftp && sessionId}
    <SftpDialog
      sessionId={sessionId}
      onClose={() => (showSftp = false)}
    />
  {/if}
  {#if pastePending != null}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="paste-confirm-overlay"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => { if (e.target === e.currentTarget) pastePending = null }}
    >
      <div class="paste-confirm">
        <h3>Paste {pastePending.split(/\r?\n/).length} lines?</h3>
        <p class="paste-confirm-hint">
          The clipboard contains line breaks. Each line will be sent — that may
          execute multiple commands on the remote side.
        </p>
        <pre class="paste-confirm-preview">{pastePending.length > 1500
          ? pastePending.slice(0, 1500) + `\n… (${pastePending.length - 1500} more chars)`
          : pastePending}</pre>
        <div class="paste-confirm-actions">
          <button class="btn" type="button" onclick={() => (pastePending = null)}>Cancel</button>
          <button
            class="btn primary"
            type="button"
            onclick={() => {
              const text = pastePending ?? ''
              pastePending = null
              term?.paste(text)
            }}
          >Send {pastePending.split(/\r?\n/).length} lines</button>
        </div>
      </div>
    </div>
  {/if}
  <!-- toolbar -->
  {#if !hideToolbar}
  <div class="toolbar">
    <!-- logging button -->
    <button
      class="toolbar-btn"
      class:recording={isLogging}
      onclick={toggleLogging}
      disabled={!sessionId}
      title={isLogging ? `Logging to ${logPath}` : 'Start logging'}
    >
      {#if isLogging}
        <span class="rec-dot"></span> Stop log
      {:else}
        ● Log
      {/if}
    </button>

    {#if savedSession}
      <button class="toolbar-btn" onclick={toggleLogs} title="Session log history">
        Logs {showLogs ? '▲' : '▼'}
      </button>
    {/if}

    {#if isSsh}
      <button
        class="toolbar-btn"
        onclick={() => (showSftp = true)}
        disabled={!sessionId}
        title="SFTP file browser"
      >
        📁 SFTP
      </button>
      <button
        class="toolbar-btn"
        onclick={() => (showTunnels = true)}
        disabled={!sessionId}
        title="Port forwards (-L / -D)"
      >
        ⇆ Tunnels
      </button>
    {/if}

    {#if logError}
      <span class="toolbar-error">{logError}</span>
    {/if}

    {#if loginProgress}
      <span class="login-badge" class:err={loginProgress.status === 'timeout'}>
        {#if loginProgress.status === 'complete'}
          ✓ Login complete
        {:else if loginProgress.status === 'timeout'}
          ⏱ Login timeout (step {loginProgress.current + 1})
        {:else}
          ⟳ Login {loginProgress.current}/{loginProgress.total}
        {/if}
      </span>
    {/if}

    <span class="flex-1"></span>

    <!-- search toggle -->
    <button
      class="toolbar-btn"
      onclick={() => { showSearch = !showSearch; if (!showSearch) { searchAddon?.clearDecorations(); searchQuery = '' } }}
      title="Search (Ctrl+F)"
    >
      🔍
    </button>
  </div>
  {/if}

  <!-- search bar -->
  {#if showSearch}
    <div class="search-bar">
      <input
        class="search-input"
        bind:value={searchQuery}
        placeholder="Search…"
        oninput={() => searchAddon?.findNext(searchQuery, { incremental: true })}
        onkeydown={(e) => {
          if (e.key === 'Enter') searchAddon?.findNext(searchQuery)
          if (e.key === 'Escape') { showSearch = false; searchAddon?.clearDecorations(); searchQuery = '' }
        }}
      />
      <button class="search-nav" onclick={() => searchAddon?.findNext(searchQuery)}>▼</button>
      <button class="search-nav" onclick={() => searchAddon?.findPrevious(searchQuery)}>▲</button>
      <button class="search-nav" onclick={() => { showSearch = false; searchAddon?.clearDecorations(); searchQuery = '' }}>✕</button>
    </div>
  {/if}

  {#if errorMsg}
    <div class="error">
      <span class="error-icon">⚠</span>
      <pre class="error-text">{errorMsg}</pre>
    </div>
  {/if}

  <!-- past logs panel -->
  {#if showLogs}
    <div class="logs-panel">
      {#if logsLoading}
        <p class="logs-empty">Loading…</p>
      {:else if pastLogs.length === 0}
        <p class="logs-empty">No logs yet for this session.</p>
      {:else}
        <ul class="logs-list">
          {#each pastLogs as log (log.id)}
            <li class="log-entry">
              <span class="log-date">{formatDate(log.started_at)}</span>
              <span class="log-size">{formatBytes(log.bytes)}</span>
              <span class="log-path" title={log.file_path}>{log.file_path.split(/[\\/]/).at(-1)}</span>
              {#if log.ended_at === null}
                <span class="log-active">active</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}

  <div class="terminal-stage">
    <div bind:this={container} class="terminal-container"></div>
    {#if hintController}
      <GhostText
        controller={hintController}
        color={effectivePalette.brightBlack || 'rgba(255,255,255,0.32)'}
        fontFamily={terminalSettings.fontFamily}
        fontSize={terminalSettings.fontSize}
      />
      <HintPopup
        controller={hintController}
        fontFamily={terminalSettings.fontFamily}
        accentColor={pylonPalette?.cursor ?? '#22d3ee'}
      />
    {/if}
  </div>
</div>

<style>
  .terminal-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--term-bg, #09090b);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.5rem;
    background: #18181b;
    border-bottom: 1px solid #27272a;
    flex-shrink: 0;
    min-height: 1.75rem;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.1rem 0.5rem;
    font-size: 0.75rem;
    background: transparent;
    border: none;
    cursor: pointer;
    color: #71717a;
    border-radius: 0.2rem;
    font-family: inherit;
  }

  .toolbar-btn:hover:not(:disabled) {
    color: #e4e4e7;
    background: #27272a;
  }

  .toolbar-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .toolbar-btn.recording {
    color: #ef4444;
  }

  .rec-dot {
    display: inline-block;
    width: 0.5rem;
    height: 0.5rem;
    background: #ef4444;
    border-radius: 50%;
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }

  .flex-1 { flex: 1; }

  .toolbar-error {
    font-size: 0.7rem;
    color: #ef4444;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .login-badge {
    font-size: 0.7rem;
    color: #34d399;
    background: rgba(52, 211, 153, 0.1);
    border: 1px solid rgba(52, 211, 153, 0.3);
    padding: 0.15rem 0.45rem;
    border-radius: 0.25rem;
    white-space: nowrap;
  }

  .login-badge.err {
    color: #f87171;
    background: rgba(239, 68, 68, 0.12);
    border-color: rgba(239, 68, 68, 0.4);
  }

  /* Multi-line paste confirmation */
  .paste-confirm-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 140;
  }
  .paste-confirm {
    background: #18181b;
    border: 1px solid #3f3f46;
    border-radius: 0.5rem;
    padding: 1rem 1.2rem 1.1rem;
    width: 34rem;
    max-width: 95vw;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .paste-confirm h3 {
    margin: 0;
    font-size: 0.9rem;
    font-weight: 600;
    color: #fbbf24;
  }
  .paste-confirm-hint {
    margin: 0;
    font-size: 0.78rem;
    color: #a1a1aa;
    line-height: 1.4;
  }
  .paste-confirm-preview {
    background: #09090b;
    border: 1px solid #27272a;
    border-radius: 0.3rem;
    padding: 0.6rem 0.7rem;
    font-size: 0.78rem;
    color: #e4e4e7;
    overflow: auto;
    max-height: 40vh;
    white-space: pre-wrap;
    margin: 0;
  }
  .paste-confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 0.2rem;
  }
  .paste-confirm-actions .btn {
    padding: 0.4rem 0.85rem;
    font-size: 0.82rem;
    border-radius: 0.25rem;
    border: 1px solid #3f3f46;
    background: #27272a;
    color: #e4e4e7;
    cursor: pointer;
    font-family: inherit;
  }
  .paste-confirm-actions .btn:hover {
    background: #3f3f46;
  }
  .paste-confirm-actions .btn.primary {
    background: #3b82f6;
    border-color: #3b82f6;
    color: #fff;
  }
  .paste-confirm-actions .btn.primary:hover {
    background: #2563eb;
  }

  .search-bar {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.2rem 0.5rem;
    background: #18181b;
    border-bottom: 1px solid #27272a;
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: #27272a;
    border: 1px solid #3f3f46;
    border-radius: 0.25rem;
    padding: 0.1rem 0.4rem;
    color: #e4e4e7;
    font-size: 0.8rem;
    outline: none;
    font-family: monospace;
  }

  .search-input:focus {
    border-color: #3b82f6;
  }

  .search-nav {
    background: transparent;
    border: none;
    cursor: pointer;
    color: #71717a;
    padding: 0.1rem 0.3rem;
    font-size: 0.75rem;
  }

  .search-nav:hover {
    color: #e4e4e7;
  }

  .logs-panel {
    background: #18181b;
    border-bottom: 1px solid #27272a;
    max-height: 8rem;
    overflow-y: auto;
    flex-shrink: 0;
  }

  .logs-empty {
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    color: #71717a;
  }

  .logs-list {
    list-style: none;
    padding: 0.25rem 0;
    margin: 0;
  }

  .log-entry {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.2rem 0.75rem;
    font-size: 0.72rem;
    color: #a1a1aa;
  }

  .log-entry:hover {
    background: #27272a;
  }

  .log-date { color: #71717a; flex-shrink: 0; }
  .log-size { color: #52525b; flex-shrink: 0; }
  .log-path { flex: 1; font-family: monospace; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .log-active { color: #22c55e; font-size: 0.65rem; flex-shrink: 0; }

  .terminal-stage {
    position: relative;
    flex: 1;
    overflow: hidden;
    background: var(--term-bg, #09090b);
  }

  .terminal-container {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: var(--term-bg, #09090b);
  }

  :global(.xterm) {
    height: 100%;
    padding: 0;
  }

  :global(.xterm-viewport) {
    background-color: var(--term-bg, #09090b) !important;
    overflow-y: hidden !important;
  }

  :global(.xterm-screen) {
    background-color: var(--term-bg, #09090b);
  }

  .error {
    display: flex;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    color: #ef4444;
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 0.8rem;
    background: rgba(239, 68, 68, 0.07);
    border-left: 3px solid #ef4444;
    flex-shrink: 0;
    line-height: 1.6;
  }

  .error-icon {
    font-size: 1rem;
    flex-shrink: 0;
    margin-top: 0.1rem;
  }

  .error-text {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    font-size: inherit;
    color: inherit;
  }
</style>
