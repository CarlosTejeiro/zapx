// Hidden → visible tracking for a terminal pane.
//
// Inactive tabs stay mounted but are hidden with `display: none`. xterm's DOM
// renderer measures each glyph's width (`offsetWidth`) to pad it to the cell
// with `letter-spacing`; inside a hidden subtree that width reads as 0, so a
// row drawn while the pane is hidden gets a full extra cell after every glyph
// that wasn't measured yet ("C isco I O S  X E"). xterm pauses rendering for
// hidden terminals only once its IntersectionObserver reports it, which
// happens after the next frame has already been drawn: output that lands in
// that window stays mis-spaced until the row is redrawn. The pane therefore
// redraws its viewport each time it becomes visible again.

/** A pane is hidden when its box has no area (`display: none` ancestors). */
export function isHiddenSize(width: number, height: number): boolean {
  return !(width > 0 && height > 0)
}

/**
 * Feed it the pane's size on every ResizeObserver callback; `update` returns
 * true exactly when the pane went from hidden to visible.
 */
export function createVisibilityTracker() {
  let hidden = false
  return {
    update(width: number, height: number): boolean {
      const nowHidden = isHiddenSize(width, height)
      const shown = hidden && !nowHidden
      hidden = nowHidden
      return shown
    },
    get hidden(): boolean {
      return hidden
    },
  }
}
