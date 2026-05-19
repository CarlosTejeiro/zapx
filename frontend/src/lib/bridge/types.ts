// Type placeholders — generated bindings arrive in Bloque 1 (specta + ts-rs)

export type SessionId = string
export type FolderId = string
export type CredentialRef = string
export type HighlightRuleId = string

export interface Session {
  id: SessionId
  name: string
  host: string
  port: number
  folderId: FolderId | null
}

export interface Folder {
  id: FolderId
  name: string
  parentId: FolderId | null
}
