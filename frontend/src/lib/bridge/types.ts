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
  auth_method: string | null
  /** Optional ProxyJump: id of the saved SSH session used as the bastion. */
  via_session_id: number | null
}

// Discriminated union mirroring the backend `AuthMethodArg` (serde tag = "type").
export type AuthMethod =
  | { type: 'password'; password: string }
  | { type: 'key'; keyPath: string; passphrase: string | null }
  | { type: 'keyboard-interactive' }
  | { type: 'agent' }

// One server-driven keyboard-interactive prompt.
export interface KiPrompt {
  prompt: string
  echo: boolean
}

// Payload of the `ssh-ki-prompt` Tauri event.
export interface KiPromptEvent {
  interactionId: string
  name: string
  instructions: string
  prompts: KiPrompt[]
}

// One remote SFTP directory entry, mirroring core_transport::SftpEntry.
export interface SftpEntry {
  name: string
  /** "dir" | "file" | "symlink" | "other" */
  kind: string
  size: number | null
  permissions: number | null
  /** Unix seconds since epoch */
  mtime: number | null
}

// One step of an automated login script, mirroring app::login_script::LoginStep.
export interface LoginStep {
  expect: string
  is_regex: boolean
  send: string
  timeout_ms: number
}

// Payload of `login-script-progress` Tauri event, mirroring LoginProgress.
export interface LoginProgressEvent {
  session_id: string
  current: number
  total: number
  /** "running" | "complete" | "timeout" */
  status: string
}

// A command snippet mirroring core_persistence::Snippet.
export interface Snippet {
  id: number
  name: string
  content: string
  sort_order: number
  created_at: string
}

// Live port-forward metadata mirroring core_transport::ForwardInfo.
export interface ForwardInfo {
  id: string
  /** "local" or "dynamic" */
  kind: string
  bind_addr: string
  bind_port: number
  /** Present for "local"; null for "dynamic" (SOCKS5 chooses per connection). */
  target_host: string | null
  target_port: number | null
}

// Mirror of core-transport `HostKeyStatus` (serde tag = "status").
export type HostKeyStatus =
  | { status: 'known' }
  | { status: 'unknown'; fingerprint: string }
  | { status: 'changed'; fingerprint: string }

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
