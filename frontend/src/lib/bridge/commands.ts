import { invoke } from '@tauri-apps/api/core'
import type { SavedSession, Folder, HighlightRule, SessionLog, ColorScheme } from './types'

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
  password: string,
): Promise<number> {
  return invoke<number>('create_saved_session', { name, folderId, host, port, username, password })
}

export async function deleteSavedSession(id: number): Promise<void> {
  return invoke<void>('delete_saved_session', { id })
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

export type { SavedSession, Folder, HighlightRule, SessionLog, ColorScheme }
