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
  options_json: string
  last_used_at: string | null
}

export interface Folder {
  id: number
  parent_id: number | null
  name: string
  sort_order: number
}

export interface HighlightRule {
  id: number
  name: string
  pattern: string
  is_regex: boolean
  fg_color: string | null
  bg_color: string | null
  bold: boolean
  underline: boolean
  enabled: boolean
  sort_order: number
}

export interface ColorScheme {
  id: number
  name: string
  palette_json: string
  is_builtin: boolean
}

export interface ColorPalette {
  background: string
  foreground: string
  cursor: string
  black: string
  red: string
  green: string
  yellow: string
  blue: string
  magenta: string
  cyan: string
  white: string
  brightBlack: string
  brightRed: string
  brightGreen: string
  brightYellow: string
  brightBlue: string
  brightMagenta: string
  brightCyan: string
  brightWhite: string
}

export interface SessionLog {
  id: number
  session_id: number | null
  started_at: string
  ended_at: string | null
  file_path: string
  bytes: number
  format: string
}
