// Typed event subscribers — populated in Bloque 1
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type { SessionId } from './types'

export interface TerminalDataPayload {
  sessionId: SessionId
  data: string
}

export async function onTerminalData(
  handler: (payload: TerminalDataPayload) => void,
): Promise<UnlistenFn> {
  return listen<TerminalDataPayload>('terminal-data', (event) => {
    handler(event.payload)
  })
}
