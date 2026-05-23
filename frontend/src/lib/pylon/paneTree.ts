// Recursive split-pane tree for the workspace.
//
// Each `AppTab` owns one [`PaneTreeNode`] root. A leaf wraps a [`PaneData`]
// (the connection / metadata for a single terminal). A `split` node holds two
// children plus a direction and a 0..1 ratio for the divider position. The
// renderer ([`SplitTree.svelte`]) walks the tree recursively.

import type { PaneData } from '$lib/pylon/Pane.svelte'

export type SplitDirection = 'h' | 'v'

export type PaneTreeNode =
  | { kind: 'leaf'; pane: PaneData }
  | {
      kind: 'split'
      direction: SplitDirection
      /** Fraction of the split given to child `a` (`0.1..0.9` clamped at draw-time). */
      ratio: number
      a: PaneTreeNode
      b: PaneTreeNode
    }

/** Build a leaf node. */
export function leaf(pane: PaneData): PaneTreeNode {
  return { kind: 'leaf', pane }
}

/** Apply `visit` to every leaf in the tree (depth-first, left-to-right). */
export function eachLeaf(node: PaneTreeNode, visit: (p: PaneData) => void): void {
  if (node.kind === 'leaf') {
    visit(node.pane)
  } else {
    eachLeaf(node.a, visit)
    eachLeaf(node.b, visit)
  }
}

/** Locate the [`PaneData`] for a given pane id, or `null` if not in the tree. */
export function findLeaf(node: PaneTreeNode, paneId: number): PaneData | null {
  if (node.kind === 'leaf') return node.pane.id === paneId ? node.pane : null
  return findLeaf(node.a, paneId) ?? findLeaf(node.b, paneId)
}

/** Id of the first (left/top) leaf in the tree — used to refocus after a close. */
export function firstLeafId(node: PaneTreeNode): number {
  return node.kind === 'leaf' ? node.pane.id : firstLeafId(node.a)
}

/** Total leaf count in the subtree. */
export function leafCount(node: PaneTreeNode): number {
  return node.kind === 'leaf' ? 1 : leafCount(node.a) + leafCount(node.b)
}

/**
 * Replace the leaf identified by `paneId` with a split node. The original
 * leaf becomes child `a` (top/left); `newPane` becomes child `b` (bottom/right).
 * If `paneId` isn't found the tree is returned unchanged.
 */
export function splitLeaf(
  node: PaneTreeNode,
  paneId: number,
  direction: SplitDirection,
  newPane: PaneData,
): PaneTreeNode {
  if (node.kind === 'leaf') {
    if (node.pane.id !== paneId) return node
    return { kind: 'split', direction, ratio: 0.5, a: node, b: leaf(newPane) }
  }
  const a = splitLeaf(node.a, paneId, direction, newPane)
  if (a !== node.a) return { ...node, a }
  const b = splitLeaf(node.b, paneId, direction, newPane)
  if (b !== node.b) return { ...node, b }
  return node
}

/**
 * Remove the leaf identified by `paneId`. If the leaf is the root, returns
 * `null` (caller should drop the tab). Otherwise the leaf's sibling replaces
 * the parent split node, collapsing the tree as expected.
 */
export function removeLeaf(node: PaneTreeNode, paneId: number): PaneTreeNode | null {
  if (node.kind === 'leaf') return node.pane.id === paneId ? null : node
  if (node.a.kind === 'leaf' && node.a.pane.id === paneId) return node.b
  if (node.b.kind === 'leaf' && node.b.pane.id === paneId) return node.a
  const a = removeLeaf(node.a, paneId)
  if (a === null) return node.b
  if (a !== node.a) return { ...node, a }
  const b = removeLeaf(node.b, paneId)
  if (b === null) return node.a
  if (b !== node.b) return { ...node, b }
  return node
}

/**
 * Update the ratio of the split node at `path` (array of 0/1 children indices
 * walked from the root). No-op if the path leads to a leaf.
 */
export function setRatio(node: PaneTreeNode, path: number[], ratio: number): PaneTreeNode {
  if (node.kind === 'leaf') return node
  if (path.length === 0) return { ...node, ratio }
  const [head, ...rest] = path
  if (head === 0) {
    const a = setRatio(node.a, rest, ratio)
    return a === node.a ? node : { ...node, a }
  }
  const b = setRatio(node.b, rest, ratio)
  return b === node.b ? node : { ...node, b }
}
