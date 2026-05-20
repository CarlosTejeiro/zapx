import { invoke } from '@tauri-apps/api/core'
import type { SavedSession, Folder, HighlightRule, SessionLog, ColorScheme } from './types'

export async function listSessions(): Promise<SavedSession[]> {
  return invoke<SavedSession[]>('list_sessions')
}

export async function listFolders(): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders')
}

export async function createFolder(name: string, parent_id: number | null): Promise<number> {
  return invoke<number>('create_folder', { name, parent_id })
}

export async function renameFolder(id: number, name: string): Promise<void> {
  return invoke<void>('rename_folder', { id, name })
}

export async function deleteFolder(id: number): Promise<void> {
  return invoke<void>('delete_folder', { id })
}

export async function createSavedSession(
  name: string,
  folder_id: number | null,
  host: string,
  port: number,
  username: string,
  password: string,
): Promise<number> {
  return invoke<number>('create_saved_session', { name, folder_id, host, port, username, password })
}

export async function deleteSavedSession(id: number): Promise<void> {
  return invoke<void>('delete_saved_session', { id })
}

export async function openSavedSession(
  saved_session_id: number,
  cols: number,
  rows: number,
): Promise<string> {
  return invoke<string>('open_saved_session', { saved_session_id, cols, rows })
}

export async function createTelnetSession(
  name: string,
  folder_id: number | null,
  host: string,
  port: number,
): Promise<number> {
  return invoke<number>('create_telnet_session', { name, folder_id, host, port })
}

export async function createSerialSession(
  name: string,
  folder_id: number | null,
  device: string,
  baud_rate: number,
): Promise<number> {
  return invoke<number>('create_serial_session', { name, folder_id, device, baud_rate })
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
  is_regex: boolean,
  fg_color: string | null,
  bg_color: string | null,
  bold: boolean,
  underline: boolean,
): Promise<number> {
  return invoke<number>('create_highlight_rule', {
    name,
    pattern,
    is_regex,
    fg_color,
    bg_color,
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
  session_id: string,
  saved_session_id: number | null,
  session_name: string,
): Promise<string> {
  return invoke<string>('start_session_logging', { session_id, saved_session_id, session_name })
}

export async function stopSessionLogging(session_id: string): Promise<void> {
  return invoke<void>('stop_session_logging', { session_id })
}

export async function listSessionLogs(saved_session_id: number): Promise<SessionLog[]> {
  return invoke<SessionLog[]>('list_session_logs', { saved_session_id })
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

export type { SavedSession, Folder, HighlightRule, SessionLog, ColorScheme }
