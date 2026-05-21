import { neonNoir, themes } from './index'
import type { PylonTheme } from './index'

let active = $state<PylonTheme>(neonNoir)

export function getTheme(): PylonTheme {
  return active
}

export function setTheme(name: string) {
  const t = themes[name]
  if (t) active = t
}
