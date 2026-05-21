import { parchment, themes } from './index'
import type { PylonTheme } from './index'

let active = $state<PylonTheme>(parchment)

export function getTheme(): PylonTheme {
  return active
}

export function setTheme(name: string) {
  const t = themes[name]
  if (t) active = t
}
