import { invoke } from '@tauri-apps/api/core'
import type {
  SavedSession,
  Folder,
  BroadcastGroup,
  HighlightRule,
  SessionLog,
  ColorScheme,
  AuthMethod,
  HostKeyStatus,
  ForwardInfo,
  SftpEntry,
  Snippet,
  RecentCommand,
  LoginStep,
} from './types'

export async function listSessions(): Promise<SavedSession[]> {
  return invoke<SavedSession[]>('list_sessions')
}

export async function listFolders(): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders')
}

export async function createFolder(name: string, parentId: number | null): Promise<number> {
  return invoke<number>('create_folder', { name, parentId })
}

export async function renameFolder(id: number, name: string): Promise<void> {
  return invoke<void>('rename_folder', { id, name })
}

export async function deleteFolder(id: number): Promise<void> {
  return invoke<void>('delete_folder', { id })
}

// ── Broadcast groups (multi-host orchestration) ──────────────────────────────

export async function listBroadcastGroups(): Promise<BroadcastGroup[]> {
  return invoke<BroadcastGroup[]>('list_broadcast_groups')
}

export async function createBroadcastGroup(name: string): Promise<number> {
  return invoke<number>('create_broadcast_group', { name })
}

export async function renameBroadcastGroup(id: number, name: string): Promise<void> {
  return invoke<void>('rename_broadcast_group', { id, name })
}

export async function deleteBroadcastGroup(id: number): Promise<void> {
  return invoke<void>('delete_broadcast_group', { id })
}

export async function setBroadcastGroupMembers(
  groupId: number,
  sessionIds: number[],
): Promise<void> {
  return invoke<void>('set_broadcast_group_members', { groupId, sessionIds })
}

/**
 * Whether `tail` ends at `platform`'s idle prompt — i.e. a command has
 * finished on that device. Used by the multi-host command runner to bound
 * captured output per host. Unknown platforms always return false (the runner
 * falls back to a time window).
 */
export async function promptReturned(platform: string, tail: string): Promise<boolean> {
  return invoke<boolean>('prompt_returned', { platform, tail })
}

export async function createSavedSession(
  name: string,
  folderId: number | null,
  host: string,
  port: number,
  username: string,
  auth: AuthMethod,
  viaSessionId: number | null = null,
): Promise<number> {
  return invoke<number>('create_saved_session', {
    name,
    folderId,
    host,
    port,
    username,
    auth,
    viaSessionId,
  })
}

export async function openSshSession(
  host: string,
  port: number,
  user: string,
  auth: AuthMethod,
  cols: number,
  rows: number,
): Promise<string> {
  return invoke<string>('open_ssh_session', { host, port, user, auth, cols, rows })
}

/** Fetch+classify a host key against ~/.ssh/known_hosts (run before connecting). */
export async function sshPreflightHostKey(host: string, port: number): Promise<HostKeyStatus> {
  return invoke<HostKeyStatus>('ssh_preflight_host_key', { host, port })
}

/** Persist trust for a host key (writes ~/.ssh/known_hosts). */
export async function sshTrustHostKey(host: string, port: number): Promise<void> {
  return invoke<void>('ssh_trust_host_key', { host, port })
}

/** Respond to a keyboard-interactive `InfoRequest` (one entry per prompt). */
export async function respondKeyboardInteractive(
  interactionId: string,
  responses: string[],
): Promise<void> {
  return invoke<void>('respond_keyboard_interactive', { interactionId, responses })
}

// ── port forwards ────────────────────────────────────────────────────────────

export async function addLocalForward(
  sessionId: string,
  bindAddr: string,
  bindPort: number,
  targetHost: string,
  targetPort: number,
): Promise<ForwardInfo> {
  return invoke<ForwardInfo>('add_local_forward', {
    sessionId,
    bindAddr,
    bindPort,
    targetHost,
    targetPort,
  })
}

export async function addDynamicForward(
  sessionId: string,
  bindAddr: string,
  bindPort: number,
): Promise<ForwardInfo> {
  return invoke<ForwardInfo>('add_dynamic_forward', { sessionId, bindAddr, bindPort })
}

/// Ask the SSH server to listen on `bindAddr:bindPort` and forward every
/// inbound connection to `targetHost:targetPort` on this machine.
/// Equivalent to `ssh -R bindAddr:bindPort:targetHost:targetPort`.
export async function addRemoteForward(
  sessionId: string,
  bindAddr: string,
  bindPort: number,
  targetHost: string,
  targetPort: number,
): Promise<ForwardInfo> {
  return invoke<ForwardInfo>('add_remote_forward', {
    sessionId,
    bindAddr,
    bindPort,
    targetHost,
    targetPort,
  })
}

export async function listForwards(sessionId: string): Promise<ForwardInfo[]> {
  return invoke<ForwardInfo[]>('list_forwards', { sessionId })
}

export async function removeForward(sessionId: string, forwardId: string): Promise<void> {
  return invoke<void>('remove_forward', { sessionId, forwardId })
}

// ── SFTP ─────────────────────────────────────────────────────────────────────

export async function sftpListDir(sessionId: string, path: string): Promise<SftpEntry[]> {
  return invoke<SftpEntry[]>('sftp_list_dir', { sessionId, path })
}

export async function sftpStat(sessionId: string, path: string): Promise<SftpEntry> {
  return invoke<SftpEntry>('sftp_stat', { sessionId, path })
}

export async function sftpCanonicalize(sessionId: string, path: string): Promise<string> {
  return invoke<string>('sftp_canonicalize', { sessionId, path })
}

export async function sftpMkdir(sessionId: string, path: string): Promise<void> {
  return invoke<void>('sftp_mkdir', { sessionId, path })
}

export async function sftpRemoveDir(sessionId: string, path: string): Promise<void> {
  return invoke<void>('sftp_remove_dir', { sessionId, path })
}

export async function sftpRemoveFile(sessionId: string, path: string): Promise<void> {
  return invoke<void>('sftp_remove_file', { sessionId, path })
}

export async function sftpRename(sessionId: string, from: string, to: string): Promise<void> {
  return invoke<void>('sftp_rename', { sessionId, from, to })
}

/// Streaming download — emits `sftp-progress { transferId, done, total,
/// direction }` events while running. `transferId` is the same string the
/// caller passes here, so multiple concurrent transfers can be filtered.
export async function sftpDownloadFile(
  sessionId: string,
  remotePath: string,
  localPath: string,
  transferId: string,
): Promise<number> {
  return invoke<number>('sftp_download_file', { sessionId, remotePath, localPath, transferId })
}

export async function sftpUploadFile(
  sessionId: string,
  localPath: string,
  remotePath: string,
  transferId: string,
): Promise<number> {
  return invoke<number>('sftp_upload_file', { sessionId, localPath, remotePath, transferId })
}

/// Flip the cancel flag for a live transfer. Best-effort — the streaming
/// loop checks the flag after each chunk so the call returns immediately.
export async function sftpCancelTransfer(transferId: string): Promise<void> {
  return invoke<void>('sftp_cancel_transfer', { transferId })
}

/// TCP MSS snapshot for an SSH session (taken right after connect). Either
/// direction may come back as null on platforms / topologies where the OS
/// doesn't expose it.
export async function getSessionTcpMss(sessionId: string): Promise<import('./types').TcpMss> {
  return invoke<import('./types').TcpMss>('get_session_tcp_mss', { sessionId })
}

export interface SftpProgressEvent {
  transfer_id: string
  done: number
  total: number | null
  direction: 'download' | 'upload'
}

// ── snippets ─────────────────────────────────────────────────────────────────

export async function listSnippets(): Promise<Snippet[]> {
  return invoke<Snippet[]>('list_snippets')
}

/// Subset visible to a session of `platform` — globals (platform IS NULL)
/// plus matches. On first call for a never-seen platform the backend seeds a
/// curated default set so the bar isn't empty out of the box.
export async function listSnippetsForPlatform(platform: string): Promise<Snippet[]> {
  return invoke<Snippet[]>('list_snippets_for_platform', { platform })
}

export async function listRecentCommandSnippets(
  savedSessionId: number | null,
  limit?: number,
): Promise<RecentCommand[]> {
  return invoke<RecentCommand[]>('list_recent_command_snippets', { savedSessionId, limit })
}

export async function createSnippet(
  name: string,
  content: string,
  platform: string | null = null,
): Promise<number> {
  return invoke<number>('create_snippet', { name, content, platform })
}

export async function updateSnippet(
  id: number,
  name: string,
  content: string,
  platform: string | null = null,
): Promise<void> {
  return invoke<void>('update_snippet', { id, name, content, platform })
}

export async function deleteSnippet(id: number): Promise<void> {
  return invoke<void>('delete_snippet', { id })
}

// ── login automation scripts ─────────────────────────────────────────────────

export async function getLoginScript(savedSessionId: number): Promise<LoginStep[]> {
  return invoke<LoginStep[]>('get_login_script', { savedSessionId })
}

export async function setLoginScript(savedSessionId: number, steps: LoginStep[]): Promise<void> {
  return invoke<void>('set_login_script', { savedSessionId, steps })
}

/** Send raw text to a live session (uses the existing `send_input` command). */
export async function sendInputText(sessionId: string, text: string): Promise<void> {
  const data = Array.from(new TextEncoder().encode(text))
  return invoke<void>('send_input', { sessionId, data })
}

export async function deleteSavedSession(id: number): Promise<void> {
  return invoke<void>('delete_saved_session', { id })
}

/** Move a session to another folder (`null` = root). */
export async function moveSavedSession(id: number, folderId: number | null): Promise<void> {
  return invoke<void>('move_saved_session', { id, folderId })
}

/// Reorder a session inside its parent (folder or root). `targetIndex` is the
/// 0-based slot among the destination's children AFTER the move; the backend
/// renumbers every sibling so the next drag has predictable inputs.
export async function reorderSavedSession(
  id: number,
  folderId: number | null,
  targetIndex: number,
): Promise<void> {
  return invoke<void>('reorder_saved_session', { id, folderId, targetIndex })
}

/**
 * Edit a saved session in place. Credentials (auth method / password / key
 * passphrase) are intentionally NOT touched — recreate the session to change
 * those. `optionsJson` overwrites the protocol blob when provided.
 */
export async function updateSavedSession(
  id: number,
  name: string,
  folderId: number | null,
  host: string | null,
  port: number | null,
  username: string | null,
  viaSessionId: number | null,
  optionsJson: string | null = null,
): Promise<void> {
  return invoke<void>('update_saved_session', {
    id,
    name,
    folderId,
    host,
    port,
    username,
    viaSessionId,
    optionsJson,
  })
}

export async function openSavedSession(
  savedSessionId: number,
  cols: number,
  rows: number,
): Promise<string> {
  return invoke<string>('open_saved_session', { savedSessionId, cols, rows })
}

export async function createTelnetSession(
  name: string,
  folderId: number | null,
  host: string,
  port: number,
): Promise<number> {
  return invoke<number>('create_telnet_session', { name, folderId, host, port })
}

export async function createSerialSession(
  name: string,
  folderId: number | null,
  device: string,
  baudRate: number,
): Promise<number> {
  return invoke<number>('create_serial_session', { name, folderId, device, baudRate })
}

export async function listSerialPorts(): Promise<string[]> {
  return invoke<string[]>('list_serial_ports')
}

export async function getSettings(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_settings')
}

export async function listHighlightRules(): Promise<HighlightRule[]> {
  return invoke<HighlightRule[]>('list_highlight_rules')
}

export async function createHighlightRule(
  name: string,
  pattern: string,
  isRegex: boolean,
  fgColor: string | null,
  bgColor: string | null,
  bold: boolean,
  underline: boolean,
): Promise<number> {
  return invoke<number>('create_highlight_rule', {
    name,
    pattern,
    isRegex,
    fgColor,
    bgColor,
    bold,
    underline,
  })
}

export async function toggleHighlightRule(id: number, enabled: boolean): Promise<void> {
  return invoke<void>('toggle_highlight_rule', { id, enabled })
}

export async function deleteHighlightRule(id: number): Promise<void> {
  return invoke<void>('delete_highlight_rule', { id })
}

export async function startSessionLogging(
  sessionId: string,
  savedSessionId: number | null,
  sessionName: string,
): Promise<string> {
  return invoke<string>('start_session_logging', { sessionId, savedSessionId, sessionName })
}

export async function stopSessionLogging(sessionId: string): Promise<void> {
  return invoke<void>('stop_session_logging', { sessionId })
}

export async function listSessionLogs(savedSessionId: number): Promise<SessionLog[]> {
  return invoke<SessionLog[]>('list_session_logs', { savedSessionId })
}

export async function listAllSessionLogs(): Promise<SessionLog[]> {
  return invoke<SessionLog[]>('list_all_session_logs')
}

export async function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>('get_setting', { key })
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>('set_setting', { key, value })
}

export async function listColorSchemes(): Promise<ColorScheme[]> {
  return invoke<ColorScheme[]>('list_color_schemes')
}

export async function openTelnetSession(
  host: string,
  port: number,
  cols: number,
  rows: number,
): Promise<string> {
  return invoke<string>('open_telnet_session', { host, port, cols, rows })
}

// ---------------------------------------------------------------------------
// Hints
// ---------------------------------------------------------------------------

import type { Hint, PlatformInfo } from './types'

export async function getHints(
  savedSessionId: number | null,
  prefix: string,
  limit: number = 5,
): Promise<Hint[]> {
  return invoke<Hint[]>('get_hints', { savedSessionId, prefix, limit })
}

export async function recordCommand(
  savedSessionId: number | null,
  command: string,
): Promise<void> {
  return invoke<void>('record_command', { savedSessionId, command })
}

export async function setSessionPlatform(
  savedSessionId: number,
  platform: string,
): Promise<void> {
  return invoke<void>('set_session_platform', { savedSessionId, platform })
}

export async function getSessionPlatform(savedSessionId: number): Promise<string | null> {
  return invoke<string | null>('get_session_platform', { savedSessionId })
}

export async function listPlatforms(): Promise<PlatformInfo[]> {
  return invoke<PlatformInfo[]>('list_platforms')
}

export async function clearCommandHistory(savedSessionId: number | null): Promise<void> {
  return invoke<void>('clear_command_history', { savedSessionId })
}

/// Re-scan the catalog override directory and return how many platforms
/// got their catalog replaced by a user file.
export async function reloadHintCatalogs(): Promise<number> {
  return invoke<number>('reload_hint_catalogs')
}

/// Absolute path of `<app_data>/catalogs/`. Creates the directory on first
/// call so the host file browser can open it cleanly. `null` when the host
/// did not configure a user-override dir (shouldn't happen in production).
export async function hintCatalogsDir(): Promise<string | null> {
  return invoke<string | null>('hint_catalogs_dir')
}

// Reveal the app's session-log directory in the host file explorer.
// Returns the absolute path (also useful for showing in a toast).
export async function openLogsDir(): Promise<string> {
  return invoke<string>('open_logs_dir')
}

// ── Export / import of the ZAPX environment ────────────────────────────────

export interface ExportSummary {
  path: string
  sessions: number
  folders: number
  groups: number
  snippets: number
  rules: number
}

export interface ImportSummary {
  sessions_added: number
  sessions_skipped: number
  folders_added: number
  groups_added: number
  snippets_added: number
  rules_added: number
  warnings: string[]
}

// Write the whole environment (sessions, folders, groups, snippets,
// highlight rules — never credentials) as versioned JSON at `path`.
export async function exportSessions(path: string): Promise<ExportSummary> {
  return invoke<ExportSummary>('export_sessions', { path })
}

// Merge a ZAPX export file into the current DB. Idempotent: existing items
// are skipped and counted in the summary.
export async function importSessions(path: string): Promise<ImportSummary> {
  return invoke<ImportSummary>('import_sessions', { path })
}

// ── Data directory (DB, logs, catalogs) ────────────────────────────────────

export interface DataDirInfo {
  path: string
  source: 'cli-flag' | 'env-var' | 'portable' | 'pointer' | 'default'
  portable: boolean
  default_path: string
  changeable: boolean
}

export async function getDataDirInfo(): Promise<DataDirInfo> {
  return invoke<DataDirInfo>('get_data_dir_info')
}

// Migrate data to `newDir`, write the startup pointer and return a summary.
// Takes effect on next launch — pair with `restartApp()`.
export async function setDataDir(newDir: string): Promise<string> {
  return invoke<string>('set_data_dir', { newDir })
}

// Revert to the platform default location on next launch (data stays put).
export async function resetDataDir(): Promise<void> {
  return invoke<void>('reset_data_dir')
}

export async function restartApp(): Promise<void> {
  return invoke<void>('restart_app')
}

// Persist the user-typed password into the backend in-memory cache. Used by
// the reconnect dialog when the OS keychain denies access on dev builds.
export async function cacheSessionPassword(
  savedSessionId: number,
  password: string,
): Promise<void> {
  return invoke<void>('cache_session_password', { savedSessionId, password })
}

export type {
  SavedSession,
  Folder,
  BroadcastGroup,
  HighlightRule,
  SessionLog,
  ColorScheme,
  AuthMethod,
  HostKeyStatus,
}
