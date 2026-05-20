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
    startSessionLogging,
    stopSessionLogging,
    listSessionLogs,
  } from '$lib/bridge/commands'
  import type { SavedSession, SessionLog } from '$lib/bridge/types'

  interface SshParams {
    host: string
    port: number
    user: string
    password: string
  }

  interface Props {
    ssh?: SshParams
    savedSession?: SavedSession
  }

  let { ssh, savedSession }: Props = $props()

  let container: HTMLDivElement
  let sessionId = $state<string | null>(null)
  let errorMsg = $state<string | null>(null)

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
    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Cascadia Code, JetBrains Mono, monospace',
      theme: {
        background: '#09090b',
        foreground: '#e4e4e7',
        cursor: '#a1a1aa',
        black: '#18181b',
        red: '#ef4444',
        green: '#22c55e',
        yellow: '#eab308',
        blue: '#3b82f6',
        magenta: '#a855f7',
        cyan: '#06b6d4',
        white: '#d4d4d8',
        brightBlack: '#3f3f46',
        brightRed: '#f87171',
        brightGreen: '#4ade80',
        brightYellow: '#fde047',
        brightBlue: '#60a5fa',
        brightMagenta: '#c084fc',
        brightCyan: '#22d3ee',
        brightWhite: '#f4f4f5',
      },
    })

    const fitAddon = new FitAddon()
    searchAddon = new SearchAddon()
    term.loadAddon(fitAddon)
    term.loadAddon(searchAddon)
    term.open(container)
    fitAddon.fit()

    // Ctrl+F opens search bar
    term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 'f' && e.type === 'keydown') {
        showSearch = !showSearch
        if (!showSearch) searchAddon?.clearDecorations()
        return false // prevent default xterm handling
      }
      if (e.key === 'Escape' && showSearch && e.type === 'keydown') {
        showSearch = false
        searchAddon?.clearDecorations()
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
      } else {
        sessionId = await invoke<string>('open_local_session')
      }
    } catch (e) {
      errorMsg = String(e)
      term.dispose()
      return
    }

    // Forward PTY output to xterm.
    const unlisten: UnlistenFn = await listen<TerminalDataPayload>(
      'terminal-data',
      (event) => {
        if (event.payload.session_id !== sessionId) return
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
      term.dispose()
      if (sessionId) {
        if (isLogging) {
          await stopSessionLogging(sessionId).catch(console.error)
        }
        await invoke('close_session', { sessionId }).catch(console.error)
        sessionId = null
      }
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
</script>

<div class="terminal-wrapper">
  <!-- toolbar -->
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
    <div class="error">{errorMsg}</div>
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
    padding: 0.5rem 1rem;
    color: #ef4444;
    font-family: monospace;
    font-size: 0.875rem;
    background: #1c0a09;
  }
</style>
