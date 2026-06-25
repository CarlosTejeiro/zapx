<script lang="ts">
  import Pane from './Pane.svelte'
  import Self from './SplitTree.svelte'
  import type { PaneTreeNode, SplitDirection } from './paneTree'
  import type { PylonTheme } from '$lib/themes/index'

  type PaneStatus = 'connecting' | 'connected' | 'error' | 'closed'

  interface Props {
    node: PaneTreeNode
    theme: PylonTheme
    focusedPaneId: number
    /** When true, each leaf shows a close button — disable when only one leaf left. */
    canClosePanes: boolean
    onFocus: (paneId: number) => void
    onSplit: (paneId: number, direction: SplitDirection) => void
    onClosePane: (paneId: number) => void
    onStatusChange: (paneId: number, status: PaneStatus) => void
    onGlobalShortcut?: (key: string, e: KeyboardEvent) => void
    onResize: (path: number[], ratio: number) => void
    /** Path from the root to this node (array of 0/1 indices). Internal. */
    path?: number[]
  }

  const {
    node,
    theme,
    focusedPaneId,
    canClosePanes,
    onFocus,
    onSplit,
    onClosePane,
    onStatusChange,
    onGlobalShortcut,
    onResize,
    path = [],
  }: Props = $props()

  /// Track the split-container element so we can compute ratios from mouse coords.
  let containerEl = $state<HTMLDivElement | null>(null)

  function startResize(e: MouseEvent, direction: SplitDirection) {
    e.preventDefault()
    e.stopPropagation()
    const container = containerEl
    if (!container) return
    const rect = container.getBoundingClientRect()
    function onMove(me: MouseEvent) {
      const ratio =
        direction === 'h'
          ? (me.clientX - rect.left) / rect.width
          : (me.clientY - rect.top) / rect.height
      onResize(path, Math.max(0.1, Math.min(0.9, ratio)))
    }
    function onUp() {
      window.removeEventListener('mousemove', onMove)
      window.removeEventListener('mouseup', onUp)
    }
    window.addEventListener('mousemove', onMove)
    window.addEventListener('mouseup', onUp)
  }
</script>

{#if node.kind === 'leaf'}
  <Pane
    {theme}
    pane={node.pane}
    focused={focusedPaneId === node.pane.id}
    canClose={canClosePanes}
    onFocus={() => onFocus(node.pane.id)}
    onSplitH={() => onSplit(node.pane.id, 'h')}
    onSplitV={() => onSplit(node.pane.id, 'v')}
    onClosePane={() => onClosePane(node.pane.id)}
    onStatusChange={(s) => onStatusChange(node.pane.id, s)}
    {onGlobalShortcut}
  />
{:else}
  <div
    class="split-container"
    class:vertical={node.direction === 'v'}
    bind:this={containerEl}
  >
    <div class="split-side" style:flex={node.ratio}>
      <Self
        node={node.a}
        {theme}
        {focusedPaneId}
        {canClosePanes}
        {onFocus}
        {onSplit}
        {onClosePane}
        {onStatusChange}
        {onGlobalShortcut}
        {onResize}
        path={[...path, 0]}
      />
    </div>
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="split-handle"
      class:vertical={node.direction === 'v'}
      style:--accent={theme.accent}
      role="separator"
      aria-orientation={node.direction === 'h' ? 'vertical' : 'horizontal'}
      onmousedown={(e) => startResize(e, node.direction)}
    ></div>
    <div class="split-side" style:flex={1 - node.ratio}>
      <Self
        node={node.b}
        {theme}
        {focusedPaneId}
        {canClosePanes}
        {onFocus}
        {onSplit}
        {onClosePane}
        {onStatusChange}
        {onGlobalShortcut}
        {onResize}
        path={[...path, 1]}
      />
    </div>
  </div>
{/if}

<style>
  .split-container {
    display: flex;
    flex-direction: row;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }

  .split-container.vertical {
    flex-direction: column;
  }

  .split-side {
    display: flex;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  /* Floating-card layout: the handle is the transparent 10px gap between
     terminal cards; an accent stripe appears while hovering/dragging. */
  .split-handle {
    flex: 0 0 10px;
    cursor: col-resize;
    background: transparent;
    transition: background 0.1s;
    user-select: none;
    border-radius: 3px;
  }

  .split-handle:hover {
    background: color-mix(in srgb, var(--accent) 35%, transparent);
  }

  .split-handle.vertical {
    cursor: row-resize;
  }
</style>
