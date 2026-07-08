<script lang="ts">
  import { onMount, onDestroy } from 'svelte'
  import Icon from '$lib/icons/Icon.svelte'
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { SearchAddon } from '@xterm/addon-search'
  import { invoke } from '@tauri-apps/api/core'
  import { writeText as clipboardWriteText } from '@tauri-apps/plugin-clipboard-manager'
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
  import { sanitizeDims, isSaneDim } from './dims'
  import HostKeyDialog from './HostKeyDialog.svelte'
  import TunnelsDialog from './TunnelsDialog.svelte'
  import SftpDialog from './SftpDialog.svelte'
  import {
    broadcast,
    broadcastTargets,
    registerSession,
    unregisterSession,
    registerFocus,
    unregisterFocus,
  } from '$lib/stores/sessionRuntime.svelte'
  import { matchAction } from '$lib/stores/keybindings.svelte'
  import { terminalSettings, colorSchemes, connectionSettings } from '$lib/stores/settings.svelte'
  import type { ColorPalette } from '$lib/bridge/types'
  import { HintController } from '$lib/hints/controller.svelte'
  import GhostText from '$lib/hints/GhostText.svelte'
  import HintPopup from '$lib/hints/HintPopup.svelte'
  import { hintsSettings } from '$lib/hints/store.svelte'
  import { showToast } from '$lib/ui/toast-store.svelte'
  import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'

  const DEFAULT_PALETTE: ColorPalette = {
    background: '#282c34',
    foreground: '#abb2bf',
    cursor: '#528bff',
    black: '#282c34',
    red: '#e06c75',
    green: '#98c379',
    yellow: '#e5c07b',
    blue: '#61afef',
    magenta: '#c678dd',
    cyan: '#56b6c2',
    white: '#abb2bf',
    brightBlack: '#5c6370',
    brightRed: '#e06c75',
    brightGreen: '#98c379',
    brightYellow: '#e5c07b',
    brightBlue: '#61afef',
    brightMagenta: '#c678dd',
    brightCyan: '#56b6c2',
    brightWhite: '#ffffff',
  }

  function parsePalette(json: string | null): ColorPalette {
    if (!json) return DEFAULT_PALETTE
    try {
      return JSON.parse(json) as ColorPalette
    } catch {
      return DEFAULT_PALETTE
    }
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
    parsePalette(colorSchemes.find((s) => s.name === activeSchemeName)?.palette_json ?? null),
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

    // Only "unknown" is approvable; persist trust before connecting. Pass the
    // fingerprint the user just approved so the backend can refuse to learn a
    // key that changed between preflight and now (TOCTOU guard).
    try {
      await sshTrustHostKey(host, port, status.fingerprint)
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

  let {
    ssh,
    telnet,
    savedSession,
    paneId,
    hideToolbar = false,
    pylonPalette,
    onGlobalShortcut,
    onSessionOpen,
    onSessionError,
    onSessionClose,
    onNeedPassword,
  }: Props = $props()

  // True when a connect failed because we have no usable credential and the
  // user should be offered the inline password form. Two shapes:
  //   • keyring entry gone ("keyring … No matching entry") — was cached then lost;
  //   • "missing credential" — a password/imported session with no stored secret
  //     (e.g. a MobaXterm import, which carries no exported password).
  function needsPasswordReentry(e: unknown): boolean {
    const raw =
      typeof e === 'string'
        ? e
        : e instanceof Error
          ? e.message
          : (() => {
              try {
                return JSON.stringify(e)
              } catch {
                return ''
              }
            })()
    let inner = raw
    try {
      const obj = JSON.parse(raw) as Record<string, unknown>
      if (typeof obj.Internal === 'string') inner = obj.Internal
    } catch {
      // not JSON — fall through with the raw string
    }
    return (
      (inner.includes('keyring') && inner.includes('No matching entry')) ||
      inner.includes('missing credential')
    )
  }

  // When a pylon theme palette is provided it takes precedence over user color scheme settings
  const effectivePalette = $derived(pylonPalette ?? activePalette)

  let container: HTMLDivElement
  let sessionId = $state<string | null>(null)
  let errorMsg = $state<string | null>(null)
  // Reconnect state: the remote dropped the link (vs. the user closing the tab).
  let disconnected = $state(false)
  let reconnecting = $state(false)
  let reconnectAttempt = 0
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null
  // Delayed retry of the post-open fit self-heal. Tracked so onDestroy can
  // cancel it — otherwise closing the tab within its window would fire
  // safeFit()/correctPtySize() on an already-disposed terminal.
  let selfHealTimer: ReturnType<typeof setTimeout> | null = null
  // Held for $effect theme/font reactivity — set inside onMount.
  let term: InstanceType<typeof Terminal> | null = null
  // Held at component scope so safeFit() and the post-open correction can reach
  // it (populated inside onMount).
  let fitAddon: FitAddon | null = null
  let hintController = $state<HintController | null>(null)
  // Teardown resources populated during the async onMount. They're released by
  // a synchronously-registered onDestroy (see the note there): registering the
  // teardown inside the async onMount — after its awaits — meant Svelte never
  // tied it to the component lifecycle, so it silently never ran and every tab
  // close leaked the xterm instance, the event listeners and the backend PTY.
  let resizeObserver: ResizeObserver | null = null
  let unlisteners: UnlistenFn[] = []

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
  let searchInput = $state<HTMLInputElement | null>(null)
  let searchCaseSensitive = $state(false)
  let searchWholeWord = $state(false)
  let searchRegex = $state(false)
  // Total matches in the buffer and the 1-based position of the active match.
  // We count ourselves rather than read the SearchAddon's result event, whose
  // decoration-based count is debounced and was reporting a stale 0 ("No
  // results") right after a search. The active match is xterm's own selection.
  let matchCount = $state(0)
  let activeMatch = $state(0)

  function searchOptions(incremental: boolean) {
    return {
      incremental,
      caseSensitive: searchCaseSensitive,
      wholeWord: searchWholeWord,
      regex: searchRegex,
    }
  }

  // Count matches in the whole buffer (scrollback included) for the current
  // query + toggles. Deterministic, so the counter is never wrongly empty.
  function countMatches(): number {
    if (!term || !searchQuery) return 0
    const buf = term.buffer.active
    let text = ''
    for (let i = 0; i < buf.length; i++) {
      text += (buf.getLine(i)?.translateToString(true) ?? '') + '\n'
    }
    try {
      const flags = searchCaseSensitive ? 'g' : 'gi'
      let source: string
      if (searchRegex) {
        source = searchQuery
      } else {
        const esc = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
        source = searchWholeWord ? `\\b${esc}\\b` : esc
      }
      return (text.match(new RegExp(source, flags)) ?? []).length
    } catch {
      return 0 // malformed regex while typing
    }
  }

  // Fresh search (typing / toggles changed): recount and jump to the first
  // match. `incremental` keeps the selection following the query as it grows.
  function refreshSearch() {
    if (!searchAddon) return
    matchCount = countMatches()
    if (!searchQuery || matchCount === 0) {
      activeMatch = 0
      return
    }
    try {
      searchAddon.findNext(searchQuery, searchOptions(true))
      activeMatch = 1
    } catch {
      matchCount = 0
      activeMatch = 0
    }
  }

  // Step to the next/previous match (arrows / Enter). xterm wraps around; we
  // keep a 1-based counter in lockstep.
  function navigate(direction: 'next' | 'prev') {
    if (!searchAddon || !searchQuery || matchCount === 0) return
    try {
      if (direction === 'prev') {
        searchAddon.findPrevious(searchQuery, searchOptions(false))
        activeMatch = activeMatch <= 1 ? matchCount : activeMatch - 1
      } else {
        searchAddon.findNext(searchQuery, searchOptions(false))
        activeMatch = activeMatch >= matchCount ? 1 : activeMatch + 1
      }
    } catch {
      /* invalid regex — ignore */
    }
  }

  function closeSearch() {
    showSearch = false
    searchAddon?.clearDecorations()
    searchQuery = ''
    matchCount = 0
    activeMatch = 0
  }

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
    const raw =
      typeof e === 'string'
        ? e
        : e instanceof Error
          ? e.message
          : (() => {
              try {
                return JSON.stringify(e)
              } catch {
                return 'Unknown error'
              }
            })()
    try {
      const obj = JSON.parse(raw) as Record<string, unknown>
      const inner = typeof obj.Internal === 'string' ? obj.Internal : raw
      if (inner.includes('keyring') && inner.includes('No matching entry')) {
        return 'Saved credentials not found in keyring.\n\nFix: delete this session and recreate it — you will be prompted to re-enter your password.\n\nIf running in WSL, ensure gnome-keyring is running:\n  eval $(gnome-keyring-daemon --start --components=secrets)'
      }
      if (inner.includes('Connection refused'))
        return 'Connection refused — check the host and port.'
      if (inner.includes('timeout') || inner.includes('timed out'))
        return 'Connection timed out — the host is unreachable.'
      if (inner.includes('Authentication') || inner.includes('authentication'))
        return 'Authentication failed — check your username and password.'
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

  // Wait until fonts are loaded and the browser has laid the container out, so
  // xterm's cell measurement is valid before the first fit. On some WebKitGTK
  // builds, fitting before the terminal font is ready yields a bogus (1×1 /
  // 0-cell) measurement. Guarded for environments without `document.fonts`.
  async function waitForLayout(): Promise<void> {
    try {
      if (typeof document !== 'undefined' && document.fonts?.ready) {
        await document.fonts.ready
      }
    } catch {
      /* font loading API unavailable/unsupported — proceed anyway */
    }
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
  }

  // Fit the terminal to its container and validate the result. If xterm still
  // reports a degenerate size (< 2 cols/rows or non-finite) — the WebKitGTK
  // cell-measurement bug — retry a couple of times on the next frames / a short
  // timeout. Returns true once a sane size is measured.
  async function safeFit(retries = 3): Promise<boolean> {
    for (let attempt = 0; attempt <= retries; attempt++) {
      // Re-check each iteration: an await below yields, during which the
      // component may be destroyed (term/fitAddon nulled). Never touch a
      // disposed terminal.
      if (!term || !fitAddon) return false
      try {
        fitAddon.fit()
      } catch {
        /* fit can throw if the element was detached — treat as invalid */
      }
      if (isSaneDim(term.cols) && isSaneDim(term.rows)) return true
      if (attempt < retries) {
        await new Promise<void>((resolve) => {
          requestAnimationFrame(() => setTimeout(() => resolve(), 30))
        })
      }
    }
    return term != null && isSaneDim(term.cols) && isSaneDim(term.rows)
  }

  // Push the currently-measured (sane) size to the live PTY. No-op if there's
  // no session or the size is still invalid. Used to correct a bad initial fit
  // after the terminal has settled, so the shell isn't stuck on the 80×24
  // fallback when the pane is actually a different size.
  async function correctPtySize(): Promise<void> {
    if (!term || !sessionId) return
    if (!isSaneDim(term.cols) || !isSaneDim(term.rows)) return
    await invoke('resize_terminal', {
      sessionId,
      cols: term.cols,
      rows: term.rows,
    }).catch(console.error)
  }

  // Open the backend session for this pane's parameters and return its id.
  // Factored out so reconnect can re-run it. Dimensions are sanitized so a
  // failed fit never opens the PTY at 1×1 — it falls back to 80×24.
  async function doOpen(): Promise<string> {
    if (!term) throw new Error('terminal not ready')
    const { cols, rows } = sanitizeDims(term.cols, term.rows)
    if (savedSession) {
      // Surface backend connection progress (jump-host hops → target tunnel)
      // in the pane while we wait, so a slow/failed ProxyJump isn't a blank
      // screen. Tagged by saved-session id (the live id doesn't exist yet).
      const sid = savedSession.id
      const unlistenProgress = await listen<{ saved_session_id: number; line: string }>(
        'connection-progress',
        (event) => {
          if (event.payload.saved_session_id !== sid) return
          term?.write(`\x1b[2m${event.payload.line}\x1b[0m\r\n`)
        },
      )
      try {
        return await openSavedSession(sid, cols, rows)
      } finally {
        unlistenProgress()
      }
    }
    if (ssh) return await openSshSession(ssh.host, ssh.port, ssh.user, ssh.auth, cols, rows)
    if (telnet) return await openTelnetSession(telnet.host, telnet.port, cols, rows)
    return await invoke<string>('open_local_session')
  }

  function clearReconnectTimer() {
    if (reconnectTimer != null) {
      clearTimeout(reconnectTimer)
      reconnectTimer = null
    }
  }

  function clearSelfHealTimer() {
    if (selfHealTimer != null) {
      clearTimeout(selfHealTimer)
      selfHealTimer = null
    }
  }

  // Re-open the session in place, reusing the same xterm (scrollback is kept).
  async function reconnect() {
    if (!term || reconnecting) return
    clearReconnectTimer()
    reconnecting = true
    errorMsg = null
    try {
      const id = await doOpen()
      sessionId = id
      disconnected = false
      reconnecting = false
      reconnectAttempt = 0
      if (paneId != null) registerSession(paneId, id)
      registerFocus(id, () => term?.focus())
      onSessionOpen?.()
      showToast({ kind: 'success', title: 'Reconnected' })
      term.focus()
    } catch (e) {
      reconnecting = false
      errorMsg = fmtError(e)
      scheduleReconnect()
    }
  }

  // Force a reconnect from the toolbar: tear down the live session, then
  // re-open in place. Works whether or not the link is currently up.
  async function manualReconnect() {
    if (reconnecting) return
    const old = sessionId
    sessionId = null
    if (paneId != null) unregisterSession(paneId)
    if (old) {
      unregisterFocus(old)
      await invoke('close_session', { sessionId: old }).catch(() => {})
    }
    await reconnect()
  }

  // Clear the terminal scrollback (and screen).
  function clearScrollback() {
    term?.clear()
  }

  // Dump the whole scrollback as plain text (trailing blank lines trimmed).
  function dumpBuffer(): string {
    if (!term) return ''
    const buf = term.buffer.active
    const lines: string[] = []
    for (let i = 0; i < buf.length; i++) {
      const line = buf.getLine(i)
      lines.push(line ? line.translateToString(true) : '')
    }
    while (lines.length && lines[lines.length - 1] === '') lines.pop()
    return lines.length ? lines.join('\n') + '\n' : ''
  }

  // Save the scrollback to a text file the user picks.
  async function saveBuffer() {
    const label = savedSession?.name ?? ssh?.host ?? telnet?.host ?? 'session'
    try {
      const path = await saveFileDialog({
        title: 'Save terminal output',
        defaultPath: `${label}.txt`,
        filters: [{ name: 'Text', extensions: ['txt', 'log'] }],
      })
      if (!path) return
      await invoke('save_text_file', { path, content: dumpBuffer() })
      showToast({ kind: 'success', title: 'Saved', detail: path })
    } catch (e) {
      showToast({ kind: 'error', title: 'Save failed', detail: fmtError(e) })
    }
  }

  // Whether to AUTO-reconnect this session on an unexpected drop. Manual
  // reconnect (toolbar / banner) always works; this only gates the automatic
  // retry. Quick-connect and local sessions (no saved row) never auto-reconnect.
  // A saved session opts out via `options_json.auto_reconnect = false`;
  // otherwise it follows the global default.
  function autoReconnectEnabled(): boolean {
    if (!savedSession) return false
    try {
      const o = JSON.parse(savedSession.options_json || '{}') as { auto_reconnect?: unknown }
      if (o.auto_reconnect === false) return false
    } catch {
      /* malformed options_json — fall through to the global default */
    }
    return connectionSettings.autoReconnect
  }

  // Auto-reconnect with linear backoff (2s, 4s … capped 15s), a few attempts.
  function scheduleReconnect() {
    if (!autoReconnectEnabled()) return
    if (reconnectAttempt >= 6) return
    reconnectAttempt += 1
    const delay = Math.min(2000 * reconnectAttempt, 15000)
    clearReconnectTimer()
    reconnectTimer = setTimeout(() => {
      void reconnect()
    }, delay)
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

    fitAddon = new FitAddon()
    searchAddon = new SearchAddon()
    term.loadAddon(fitAddon)
    term.loadAddon(searchAddon)
    term.open(container)
    // Wait for fonts + one layout frame before the first fit so xterm's cell
    // measurement is valid; then fit with retry. Without this, some WebKitGTK
    // builds measure a 1×1 grid and the PTY would be opened at that bad size.
    await waitForLayout()
    await safeFit()
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
    // selection, push it to the clipboard. We write through Tauri's native
    // clipboard rather than navigator.clipboard because WebKitGTK (Linux)
    // drops the newlines from the browser clipboard API — multi-line output
    // would paste as a single line. The native clipboard preserves them, and
    // it's the same path on macOS/Windows. Fall back to the browser API if the
    // native write fails; ignore failures silently either way.
    container.addEventListener('mouseup', () => {
      if (term?.hasSelection()) {
        const sel = term.getSelection()
        if (sel) {
          clipboardWriteText(sel).catch(() => navigator.clipboard.writeText(sel).catch(() => {}))
        }
      }
    })

    // Ctrl+F opens search bar; user-bound global shortcuts are forwarded to
    // App so xterm doesn't consume them as terminal control characters.
    term.attachCustomKeyEventHandler((e: KeyboardEvent) => {
      if (e.type !== 'keydown') return true
      if (e.ctrlKey && e.key === 'f') {
        if (showSearch) closeSearch()
        else showSearch = true
        return false
      }
      if (e.key === 'Escape' && showSearch) {
        closeSearch()
        return false
      }

      // ── Hint keybindings ────────────────────────────────────────────────
      const hc = hintController
      if (hc) {
        if (hc.popupOpen) {
          if (e.key === 'ArrowDown') {
            hc.movePopup(1)
            return false
          }
          if (e.key === 'ArrowUp') {
            hc.movePopup(-1)
            return false
          }
          if (e.key === 'Escape') {
            hc.closePopup()
            return false
          }
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
          term = null
          return
        }
      } else if (ssh) {
        if (!(await confirmHostKey(ssh.host, ssh.port))) {
          if (!errorMsg) errorMsg = 'Host key not trusted'
          onSessionError?.()
          term.dispose()
          term = null
          return
        }
      }

      sessionId = await doOpen()
    } catch (e) {
      if (needsPasswordReentry(e)) {
        onNeedPassword?.()
        term?.dispose()
        term = null
        return
      }
      errorMsg = fmtError(e)
      onSessionError?.()
      term?.dispose()
      term = null
      return
    }

    // Make this session discoverable by the snippets / broadcast machinery.
    if (paneId != null && sessionId) registerSession(paneId, sessionId)
    if (sessionId) registerFocus(sessionId, () => term?.focus())

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

    // Self-heal a bad initial fit without waiting for a user resize. If the
    // first measurement was degenerate (opened the PTY at the 80×24 fallback),
    // re-fit once fonts/layout are settled and, if we now get a valid size that
    // differs from what the PTY was opened with, push it to the backend.
    void (async () => {
      await waitForLayout()
      if (await safeFit()) await correctPtySize()
      // One more delayed retry for slow WebKitGTK layout passes. Tracked so
      // onDestroy can cancel it if the tab closes inside the 150ms window.
      clearSelfHealTimer()
      selfHealTimer = setTimeout(() => {
        selfHealTimer = null
        void (async () => {
          if (await safeFit()) await correctPtySize()
        })()
      }, 150)
    })()

    // Forward PTY output to xterm.
    unlisteners.push(
      await listen<TerminalDataPayload>('terminal-data', (event) => {
        if (!term || event.payload.session_id !== sessionId) return
        term.write(new Uint8Array(event.payload.data), () => {
          hintController?.onIncomingFlushed()
        })
      }),
    )

    // Login-script progress badge (auto-clears a couple of seconds after
    // the script completes; sticks on timeout so the user notices).
    unlisteners.push(
      await listen<{
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
      }),
    )

    // Output triggers that fired a notify/bell action.
    unlisteners.push(
      await listen<{
        session_id: string
        kind: string
        text: string
      }>('trigger-fired', (event) => {
        if (event.payload.session_id !== sessionId) return
        const isBell = event.payload.kind === 'bell'
        if (isBell) term?.write('\x07') // ring the terminal bell
        showToast({
          kind: isBell ? 'warning' : 'info',
          title: isBell ? '🔔 Trigger' : 'Trigger',
          detail: event.payload.text,
        })
      }),
    )

    // Remote dropped the link (server closed, or keepalives went unanswered).
    // Mark the pane disconnected and kick off auto-reconnect if enabled; the
    // banner offers a manual retry either way.
    unlisteners.push(
      await listen<{ session_id: string }>('session-disconnected', (event) => {
        if (event.payload.session_id !== sessionId) return
        if (paneId != null) unregisterSession(paneId)
        if (sessionId) unregisterFocus(sessionId)
        sessionId = null
        disconnected = true
        onSessionError?.()
        scheduleReconnect()
      }),
    )

    // Forward keyboard input to the PTY (and to every other registered
    // session when MultiExec broadcast is on).
    term.onData((data: string) => {
      if (!sessionId) return
      const u8 = new TextEncoder().encode(data)
      // Feed the hint buffer so it sees the bytes BEFORE the PTY round-trip.
      // onOutgoing already persists a flushed command line (with the prev→next
      // transition), so we must NOT record it again here or the frequency
      // stats double-count.
      hintController?.onOutgoing(u8)
      const bytes = Array.from(u8)
      invoke('send_input', { sessionId, data: bytes }).catch(console.error)
      if (broadcast.enabled) {
        for (const otherId of broadcastTargets(sessionId)) {
          invoke('send_input', { sessionId: otherId, data: bytes }).catch(console.error)
        }
      }
    })

    // Resize terminal when the container size changes. safeFit re-validates the
    // measurement (with retry), and we only forward the resize when the fitted
    // size is sane — never send a 1×1 / NaN resize to the PTY.
    resizeObserver = new ResizeObserver(() => {
      if (!term) return
      void safeFit().then((ok) => {
        if (!term || !sessionId) return
        if (ok && isSaneDim(term.cols) && isSaneDim(term.rows)) {
          invoke('resize_terminal', {
            sessionId,
            cols: term.cols,
            rows: term.rows,
          }).catch(console.error)
        }
      })
    })
    resizeObserver.observe(container)
  })

  // Teardown. Registered synchronously at component init — NOT inside the async
  // onMount, where it would run after an `await` and never bind to the
  // lifecycle. Reads the refs onMount populated; everything is null-guarded so
  // it's safe even if the component is destroyed before onMount finishes.
  onDestroy(() => {
    clearReconnectTimer()
    clearSelfHealTimer()
    resizeObserver?.disconnect()
    for (const off of unlisteners) off()
    unlisteners = []
    hintController?.clear()
    hintController = null
    term?.dispose()
    // Null the ref so any in-flight safeFit()/correctPtySize() (e.g. the
    // deferred self-heal) short-circuits instead of touching a disposed term.
    term = null
    if (paneId != null) unregisterSession(paneId)
    if (sessionId) {
      const id = sessionId
      sessionId = null
      unregisterFocus(id)
      // onDestroy can't await; do the async teardown fire-and-forget but keep
      // the order (stop logging before closing the backend session/PTY).
      void (async () => {
        if (isLogging) await stopSessionLogging(id).catch(console.error)
        await invoke('close_session', { sessionId: id }).catch(console.error)
      })()
    }
    onSessionClose?.()
  })

  // Reactive search: recount + jump to first match whenever the query or any
  // toggle changes while the bar is open (so flipping case/word/regex updates).
  $effect(() => {
    if (showSearch && searchAddon) refreshSearch()
  })

  // Focus the input when the bar opens.
  $effect(() => {
    if (showSearch) searchInput?.focus()
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
    <TunnelsDialog {sessionId} onClose={() => (showTunnels = false)} />
  {/if}
  {#if showSftp && sessionId}
    <SftpDialog {sessionId} onClose={() => (showSftp = false)} />
  {/if}
  {#if pastePending != null}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div
      class="paste-confirm-overlay"
      role="dialog"
      aria-modal="true"
      tabindex="-1"
      onclick={(e) => {
        if (e.target === e.currentTarget) pastePending = null
      }}
    >
      <div class="paste-confirm">
        <h3>Paste {pastePending.split(/\r?\n/).length} lines?</h3>
        <p class="paste-confirm-hint">
          The clipboard contains line breaks. Each line will be sent — that may execute multiple
          commands on the remote side.
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
            }}>Send {pastePending.split(/\r?\n/).length} lines</button
          >
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

      <button
        class="toolbar-btn"
        onclick={manualReconnect}
        disabled={reconnecting}
        title="Reconnect this session"
      >
        ⟳
      </button>
      <button class="toolbar-btn" onclick={clearScrollback} title="Clear scrollback"> ⌫ </button>
      <button class="toolbar-btn" onclick={saveBuffer} title="Save output to file…"> ⭳ </button>

      <!-- search toggle -->
      <button
        class="toolbar-btn"
        onclick={() => {
          if (showSearch) closeSearch()
          else showSearch = true
        }}
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
        class:no-match={searchQuery !== '' && matchCount === 0}
        bind:this={searchInput}
        bind:value={searchQuery}
        placeholder="Search…"
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault()
            navigate(e.shiftKey ? 'prev' : 'next')
          }
          if (e.key === 'Escape') closeSearch()
        }}
      />
      <span class="search-count">
        {#if searchQuery}
          {matchCount === 0 ? 'No results' : `${activeMatch || 1}/${matchCount}`}
        {/if}
      </span>
      <button
        class="search-toggle"
        class:active={searchCaseSensitive}
        title="Match case"
        onclick={() => (searchCaseSensitive = !searchCaseSensitive)}>Aa</button
      >
      <button
        class="search-toggle"
        class:active={searchWholeWord}
        title="Whole word"
        onclick={() => (searchWholeWord = !searchWholeWord)}>W</button
      >
      <button
        class="search-toggle"
        class:active={searchRegex}
        title="Regular expression"
        onclick={() => (searchRegex = !searchRegex)}>.*</button
      >
      <button
        class="search-nav"
        title="Previous match (Shift+Enter)"
        onclick={() => navigate('prev')}>▲</button
      >
      <button class="search-nav" title="Next match (Enter)" onclick={() => navigate('next')}
        >▼</button
      >
      <button class="search-nav" title="Close (Esc)" onclick={closeSearch}
        ><Icon name="x" size={11} /></button
      >
    </div>
  {/if}

  {#if disconnected}
    <div class="disconnected-bar">
      <span class="disconnected-icon">⚠</span>
      <span class="disconnected-text">
        {#if reconnecting}Reconnecting…{:else}Connection lost{/if}
      </span>
      <button class="reconnect-btn" onclick={() => reconnect()} disabled={reconnecting}>
        Reconnect
      </button>
    </div>
  {/if}

  {#if errorMsg && !disconnected}
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
              <span class="log-path" title={log.file_path}
                >{log.file_path.split(/[\\/]/).at(-1)}</span
              >
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
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.3;
    }
  }

  .flex-1 {
    flex: 1;
  }

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

  .search-input.no-match {
    border-color: #ef4444;
  }

  .search-count {
    font-size: 0.7rem;
    color: #a1a1aa;
    font-variant-numeric: tabular-nums;
    min-width: 3.5rem;
    text-align: right;
    white-space: nowrap;
  }

  .search-toggle {
    background: transparent;
    border: 1px solid transparent;
    border-radius: 0.25rem;
    cursor: pointer;
    color: #71717a;
    padding: 0.05rem 0.3rem;
    font-size: 0.72rem;
    font-family: monospace;
    line-height: 1.4;
  }

  .search-toggle:hover {
    color: #e4e4e7;
  }

  .search-toggle.active {
    color: #e4e4e7;
    background: #3b82f6;
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

  .log-date {
    color: #71717a;
    flex-shrink: 0;
  }
  .log-size {
    color: #52525b;
    flex-shrink: 0;
  }
  .log-path {
    flex: 1;
    font-family: monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .log-active {
    color: #22c55e;
    font-size: 0.65rem;
    flex-shrink: 0;
  }

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

  /* Slim scrollbar per the design system: 4px track barely tinted, rounded
     thumb. Replaces the old hidden-scrollbar behaviour so scrollback
     position is visible at a glance. */
  :global(.xterm-viewport) {
    background-color: var(--term-bg, #09090b) !important;
    overflow-y: auto !important;
    scrollbar-width: thin;
  }

  :global(.xterm-viewport::-webkit-scrollbar) {
    width: 4px;
  }

  :global(.xterm-viewport::-webkit-scrollbar-track) {
    background: rgba(255, 255, 255, 0.04);
    border-radius: 2px;
  }

  :global(.xterm-viewport::-webkit-scrollbar-thumb) {
    background: rgba(255, 255, 255, 0.14);
    border-radius: 2px;
  }

  :global(.xterm-viewport::-webkit-scrollbar-thumb:hover) {
    background: rgba(255, 255, 255, 0.22);
  }

  :global(.xterm-screen) {
    background-color: var(--term-bg, #09090b);
  }

  .disconnected-bar {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0.75rem;
    background: rgba(245, 158, 11, 0.12);
    border-left: 3px solid #f59e0b;
    color: #fbbf24;
    font-size: 0.8rem;
    flex-shrink: 0;
  }

  .disconnected-icon {
    font-size: 0.95rem;
  }

  .disconnected-text {
    flex: 1;
  }

  .reconnect-btn {
    background: #f59e0b;
    color: #1c1917;
    border: none;
    border-radius: 0.25rem;
    padding: 0.2rem 0.7rem;
    font-size: 0.78rem;
    font-weight: 600;
    cursor: pointer;
  }

  .reconnect-btn:hover {
    background: #fbbf24;
  }

  .reconnect-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .error {
    display: flex;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    color: #ef4444;
    font-family: var(--zx-font-mono, 'JetBrains Mono', ui-monospace, monospace);
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
