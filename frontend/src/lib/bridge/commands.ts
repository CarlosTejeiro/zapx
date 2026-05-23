import { invoke } from '@tauri-apps/api/core'
import type {
  SavedSession,
  Folder,
  HighlightRule,
  SessionLog,
  ColorScheme,
  AuthMethod,
  HostKeyStatus,
  ForwardInfo,
  SftpEntry,
  Snippet,
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

export async function sftpDownloadFile(
  sessionId: string,
  remotePath: string,
  localPath: string,
): Promise<number> {
  return invoke<number>('sftp_download_file', { sessionId, remotePath, localPath })
}

export async function sftpUploadFile(
  sessionId: string,
  localPath: string,
  remotePath: string,
): Promise<number> {
  return invoke<number>('sftp_upload_file', { sessionId, localPath, remotePath })
}

// ── snippets ─────────────────────────────────────────────────────────────────

export async function listSnippets(): Promise<Snippet[]> {
  return invoke<Snippet[]>('list_snippets')
}

export async function createSnippet(name: string, content: string): Promise<number> {
  return invoke<number>('create_snippet', { name, content })
}

export async function updateSnippet(id: number, name: string, content: string): Promise<void> {
  return invoke<void>('update_snippet', { id, name, content })
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

export type { SavedSession, Folder, HighlightRule, SessionLog, ColorScheme, AuthMethod, HostKeyStatus }
