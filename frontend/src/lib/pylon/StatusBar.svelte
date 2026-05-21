<script lang="ts">
  import type { PylonTheme } from '$lib/themes/index'

  type ConnStatus = 'connecting' | 'connected' | 'error' | 'closed'

  interface Props {
    theme: PylonTheme
    status?: ConnStatus
    host?: string
    port?: number
    protocol?: string
    latencyMs?: number
    layout?: string
  }

  const {
    theme,
    status = 'connecting',
    host = '',
    port,
    protocol = 'SSH',
    latencyMs,
    layout = '1×1',
  }: Props = $props()

  const statusLabel = $derived(
    status === 'connected'  ? 'CONNECTED'
    : status === 'error'   ? 'FAILED'
    : status === 'closed'  ? 'DISCONNECTED'
    : 'CONNECTING…'
  )

  const statusColor = $derived(
    status === 'connected'  ? theme.ok
    : status === 'error'   ? theme.err
    : status === 'closed'  ? theme.textDim
    : theme.warn
  )
</script>

<footer
  class="statusbar"
  style:background={theme.statusbarBg}
  style:border-top="1px solid {theme.border}"
  style:font-family={theme.fontMono}
  style:color={theme.textDim}
>

  <!-- Left segments -->
  <div class="sb-left">
    <span class="sb-seg">
      <span
        class="sb-led"
        style:background={statusColor}
        style:animation={status === 'connected' ? 'pylon-glow-pulse 2s ease infinite' : 'none'}
      ></span>
      <span style:color={statusColor}>{statusLabel}</span>
    </span>

    {#if host}
      <span class="sb-divider">|</span>
      <span class="sb-seg" style:color={theme.textMuted}>{host}</span>
    {/if}

    {#if port !== undefined}
      <span class="sb-divider">|</span>
      <span class="sb-seg">
        <span style:color={theme.textDim}>{protocol.toLowerCase()}</span>
        <span style:color={theme.textMuted}> {port}</span>
      </span>
    {/if}

    {#if latencyMs !== undefined && status === 'connected'}
      <span class="sb-divider">|</span>
      <span class="sb-seg sb-mono" style:color={theme.textMuted}>{latencyMs}ms</span>
    {/if}
  </div>

  <!-- Right segments -->
  <div class="sb-right">
    <span class="sb-seg">utf-8</span>
    <span class="sb-divider">|</span>
    <span class="sb-seg">VT320</span>
    <span class="sb-divider">|</span>
    <span class="sb-seg">{layout}</span>
  </div>

</footer>

<style>
  .statusbar {
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px;
    font-size: 11px;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .sb-left,
  .sb-right {
    display: flex;
    align-items: center;
    gap: 0;
  }

  .sb-seg {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    white-space: nowrap;
  }

  .sb-divider {
    opacity: 0.3;
    padding: 0 2px;
  }

  .sb-led {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .sb-mono {
    font-variant-numeric: tabular-nums;
  }
</style>
