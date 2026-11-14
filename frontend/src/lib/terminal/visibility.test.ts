import { describe, it, expect } from 'vitest'
import { createVisibilityTracker, isHiddenSize } from './visibility'

describe('isHiddenSize', () => {
  it('treats a box without area as hidden', () => {
    expect(isHiddenSize(0, 0)).toBe(true)
    expect(isHiddenSize(900, 0)).toBe(true)
    expect(isHiddenSize(0, 400)).toBe(true)
    expect(isHiddenSize(NaN, 400)).toBe(true)
  })

  it('treats a box with area as visible', () => {
    expect(isHiddenSize(900, 400)).toBe(false)
  })
})

describe('createVisibilityTracker', () => {
  it('does not report a pane that starts visible', () => {
    const t = createVisibilityTracker()
    expect(t.update(900, 400)).toBe(false)
    expect(t.update(1000, 400)).toBe(false) // plain resize
    expect(t.hidden).toBe(false)
  })

  it('reports each hidden → visible transition once', () => {
    const t = createVisibilityTracker()
    t.update(900, 400)
    expect(t.update(0, 0)).toBe(false) // tab switched away
    expect(t.hidden).toBe(true)
    expect(t.update(0, 0)).toBe(false)
    expect(t.update(900, 400)).toBe(true) // tab shown again
    expect(t.update(900, 400)).toBe(false)
    expect(t.update(0, 0)).toBe(false)
    expect(t.update(900, 400)).toBe(true)
  })

  it('reports a pane created in a background tab when first shown', () => {
    const t = createVisibilityTracker()
    expect(t.update(0, 0)).toBe(false)
    expect(t.update(900, 400)).toBe(true)
  })
})
