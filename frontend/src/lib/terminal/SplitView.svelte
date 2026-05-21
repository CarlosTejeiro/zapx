<script lang="ts">
  import SplitView from './SplitView.svelte'
  import TerminalPane from './TerminalPane.svelte'
  import type { PaneInfo, PaneStatus } from './TerminalPane.svelte'

  export type SplitNode =
    | { kind: 'leaf'; paneId: number }
    | { kind: 'h'; left: SplitNode; right: SplitNode; ratio: number }
    | { kind: 'v'; top: SplitNode; bottom: SplitNode; ratio: number }

  interface Props {
    node: SplitNode
    panes: Map<number, PaneInfo>
    activePaneId: number
    onActivate: (paneId: number) => void
    onSplitH: (paneId: number) => void
    onSplitV: (paneId: number) => void
    onClosePane: (paneId: number) => void
    onGlobalShortcut?: (key: string, e: KeyboardEvent) => void
    onStatusChange?: (paneId: number, status: PaneStatus) => void
    totalPanes: number
  }

  let {
    node,
    panes,
    activePaneId,
    onActivate,
    onSplitH,
    onSplitV,
    onClosePane,
    onGlobalShortcut,
    onStatusChange,
    totalPanes,
  }: Props = $props()

  // ── drag-to-resize ──────────────────────────────────────────────────────────

  let containerEl = $state<HTMLDivElement | null>(null)

  function startDrag(e: MouseEvent, isHorizontal: boolean) {
    e.preventDefault()
    const container = containerEl
    if (!container) return
    const rect = container.getBoundingClientRect()

    function onMove(me: MouseEvent) {
      if (isHorizontal) {
        node = { ...node, ratio: Math.max(0.15, Math.min(0.85, (me.clientX - rect.left) / rect.width)) } as typeof node
      } else {
        node = { ...node, ratio: Math.max(0.15, Math.min(0.85, (me.clientY - rect.top) / rect.height)) } as typeof node
      }
    }

    function onUp() {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
    }

    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
  }
</script>

{#if node.kind === 'leaf'}
  {@const pane = panes.get(node.paneId)}
  {#if pane}
    <TerminalPane
      {pane}
      active={activePaneId === pane.id}
      canClose={totalPanes > 1}
      onActivate={() => onActivate(pane.id)}
      onSplitH={() => onSplitH(pane.id)}
      onSplitV={() => onSplitV(pane.id)}
      onClose={() => onClosePane(pane.id)}
      {onGlobalShortcut}
      {onStatusChange}
    />
  {/if}

{:else if node.kind === 'h'}
  <div class="split-container split-h" bind:this={containerEl} style:--ratio={node.ratio}>
    <div class="split-child" style:flex={node.ratio}>
      <SplitView
        node={node.left}
        {panes}
        {activePaneId}
        {onActivate}
        {onSplitH}
        {onSplitV}
        {onClosePane}
        {onGlobalShortcut}
        {onStatusChange}
        {totalPanes}
      />
    </div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="divider divider-h"
      onmousedown={(e) => startDrag(e, true)}
    ></div>
    <div class="split-child" style:flex={1 - node.ratio}>
      <SplitView
        node={node.right}
        {panes}
        {activePaneId}
        {onActivate}
        {onSplitH}
        {onSplitV}
        {onClosePane}
        {onGlobalShortcut}
        {onStatusChange}
        {totalPanes}
      />
    </div>
  </div>

{:else}
  <div class="split-container split-v" bind:this={containerEl} style:--ratio={node.ratio}>
    <div class="split-child" style:flex={node.ratio}>
      <SplitView
        node={node.top}
        {panes}
        {activePaneId}
        {onActivate}
        {onSplitH}
        {onSplitV}
        {onClosePane}
        {onGlobalShortcut}
        {onStatusChange}
        {totalPanes}
      />
    </div>
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="divider divider-v"
      onmousedown={(e) => startDrag(e, false)}
    ></div>
    <div class="split-child" style:flex={1 - node.ratio}>
      <SplitView
        node={node.bottom}
        {panes}
        {activePaneId}
        {onActivate}
        {onSplitH}
        {onSplitV}
        {onClosePane}
        {onGlobalShortcut}
        {onStatusChange}
        {totalPanes}
      />
    </div>
  </div>
{/if}

<style>
  .split-container {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .split-h {
    flex-direction: row;
  }

  .split-v {
    flex-direction: column;
  }

  .split-child {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }

  .divider {
    background: #2a2a2a;
    flex-shrink: 0;
    transition: background 0.15s;
    position: relative;
  }

  .divider:hover,
  .divider:active {
    background: #3b82f6;
  }

  .divider-h {
    width: 3px;
    cursor: col-resize;
  }

  .divider-v {
    height: 3px;
    cursor: row-resize;
  }
</style>
