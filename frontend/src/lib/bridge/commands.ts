// Typed invoke wrappers — populated in Bloque 1
import { invoke } from '@tauri-apps/api/core'
import type { Session, Folder, SessionId, FolderId } from './types'

export async function listSessions(): Promise<Session[]> {
  return invoke<Session[]>('list_sessions')
}

export async function listFolders(): Promise<Folder[]> {
  return invoke<Folder[]>('list_folders')
}

export async function getSettings(): Promise<Record<string, unknown>> {
  return invoke<Record<string, unknown>>('get_settings')
}

export type { Session, Folder, SessionId, FolderId }
