import { invoke } from '@tauri-apps/api/core'
import type { SavedSession, Folder } from './types'

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

export type { SavedSession, Folder }
