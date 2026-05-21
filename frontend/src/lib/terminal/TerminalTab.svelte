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
    openTelnetSession,
    startSessionLogging,
    stopSessionLogging,
    listSessionLogs,
  } from '$lib/bridge/commands'
  import type { SavedSession, SessionLog } from '$lib/bridge/types'
  import { terminalSettings, colorSchemes } from '$lib/stores/settings.svelte'
  import type { ColorPalette } from '$lib/bridge/types'

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

  const activePalette = $derived<ColorPalette>(
    parsePalette(
      colorSchemes.find((s) => s.name === terminalSettings.activeColorScheme)?.palette_json ?? null,
    ),
  )

  interface SshParams {
    host: string
    port: number
    user: string
    password: string
  }

  interface TelnetParams {
    host: string
    port: number
  }

  interface Props {
    ssh?: SshParams
    telnet?: TelnetParams
    savedSession?: SavedSession
    hideToolbar?: boolean
    onGlobalShortcut?: (key: string, e: KeyboardEvent) => void
    onSessionOpen?: () => void
    onSessionError?: () => void
    onSessionClose?: () => void
  }

  let { ssh, telnet, savedSession, hideToolbar = false, onGlobalShortcut, onSessionOpen, onSessionError, onSessionClose }: Props = $props()

  let container: HTMLDivElement
  let sessionId = $state<string | null>(null)
  let errorMsg = $state<string | null>(null)
  // Held for $effect theme/font reactivity — set inside onMount.
  let term: InstanceType<typeof Terminal> | null = null

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
      theme: activePalette,
    })

    const fitAddon = new FitAddon()
    searchAddon = new SearchAddon()
    term.loadAddon(fitAddon)
    term.loadAddon(searchAddon)
    term.open(container)
    fitAddon.fit()

    // Ctrl+F opens search bar; other Ctrl shortcuts bubble to App for global handling
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
      // Pass global shortcuts to App without xterm consuming them
      if (
        e.ctrlKey &&
        (e.key === 'n' || e.key === 't' || e.key === 'w' || e.key === 'Tab' || e.key === ',')
      ) {
        onGlobalShortcut?.(e.key, e)
        return false
      }
      return true
    })

    // Open the backend session.
    try {
      await waitForTauri()
      if (savedSession) {
        sessionId = await openSavedSession(savedSession.id, term.cols, term.rows)
      } else if (ssh) {
        sessionId = await invoke<string>('open_ssh_session', {
          host: ssh.host,
          port: ssh.port,
          user: ssh.user,
          password: ssh.password,
          cols: term.cols,
          rows: term.rows,
        })
      } else if (telnet) {
        sessionId = await openTelnetSession(telnet.host, telnet.port, term.cols, term.rows)
      } else {
        sessionId = await invoke<string>('open_local_session')
      }
    } catch (e) {
      errorMsg = fmtError(e)
      onSessionError?.()
      term.dispose()
      return
    }

    onSessionOpen?.()

    // Forward PTY output to xterm.
    const unlisten: UnlistenFn = await listen<TerminalDataPayload>(
      'terminal-data',
      (event) => {
        if (!term || event.payload.session_id !== sessionId) return
        term.write(new Uint8Array(event.payload.data))
      },
    )

    // Forward keyboard input to the PTY.
    term.onData((data: string) => {
      if (!sessionId) return
      const bytes = Array.from(new TextEncoder().encode(data))
      invoke('send_input', { sessionId, data: bytes }).catch(console.error)
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
      term?.dispose()
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
    term.options.theme = activePalette
    term.options.fontSize = terminalSettings.fontSize
    term.options.fontFamily = terminalSettings.fontFamily
    term.options.cursorStyle = terminalSettings.cursorStyle
    term.options.cursorBlink = terminalSettings.cursorBlink
  })
</script>

<div class="terminal-wrapper">
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

    {#if logError}
      <span class="toolbar-error">{logError}</span>
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

  <div bind:this={container} class="terminal-container"></div>
</div>

<style>
  .terminal-wrapper {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: #09090b;
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

  .terminal-container {
    flex: 1;
    overflow: hidden;
    background: #09090b;
  }

  :global(.xterm) {
    height: 100%;
    padding: 0;
  }

  :global(.xterm-viewport) {
    background-color: #09090b !important;
    overflow-y: hidden !important;
  }

  :global(.xterm-screen) {
    background-color: #09090b;
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
