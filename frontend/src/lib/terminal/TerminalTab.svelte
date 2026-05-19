<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import type { UnlistenFn } from '@tauri-apps/api/event'
  import '@xterm/xterm/css/xterm.css'

  let container: HTMLDivElement
  let sessionId = $state<string | null>(null)
  let errorMsg = $state<string | null>(null)

  interface TerminalDataPayload {
    session_id: string
    data: number[]
  }

  onMount(async () => {
    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: 'Cascadia Code, JetBrains Mono, monospace',
      theme: {
        background: '#09090b',  // zinc-950
        foreground: '#e4e4e7',  // zinc-200
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
    term.loadAddon(fitAddon)
    term.open(container)
    fitAddon.fit()

    // Open the backend session.
    try {
      sessionId = await invoke<string>('open_local_session')
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
        await invoke('close_session', { sessionId }).catch(console.error)
        sessionId = null
      }
    })
  })
</script>

<div class="terminal-wrapper">
  {#if errorMsg}
    <div class="error">{errorMsg}</div>
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

  .terminal-container {
    flex: 1;
    overflow: hidden;
  }

  .error {
    padding: 0.5rem 1rem;
    color: #ef4444;
    font-family: monospace;
    font-size: 0.875rem;
    background: #1c0a09;
  }
</style>
