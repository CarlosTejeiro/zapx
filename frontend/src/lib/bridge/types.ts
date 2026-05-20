// Mirror of core-persistence public model types

export interface SavedSession {
  id: number
  folder_id: number | null
  name: string
  protocol: string
  host: string | null
  port: number | null
  username: string | null
  credential_id: number | null
  last_used_at: string | null
}

export interface Folder {
  id: number
  parent_id: number | null
  name: string
  sort_order: number
}
