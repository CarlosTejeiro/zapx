<script lang="ts">
  /// Identity tile for a saved session: a rounded square tinted with the
  /// session's colour, showing 1–2 initials derived from its name. Replaces the
  /// old 7px colour dot — far more scannable when the sidebar holds many hosts
  /// (Termius-style). Colours are computed with `color-mix` so the tile stays
  /// legible on every theme without per-theme palettes.
  interface Props {
    name: string
    /// The session's identity colour (`#RRGGBB`).
    color: string
    /// Surface the tile sits on, used as the mix base (theme.sidebarBg).
    paper: string
    /// Foreground ink, mixed into the initials colour (theme.textPrimary).
    ink: string
    /// Accent ring colour when `active` (theme.accent).
    accent?: string
    /// The session is the one currently focused → add an accent ring.
    active?: boolean
    /// Live connection status → status dot. `undefined` = idle (no dot).
    status?: 'connecting' | 'connected' | 'error'
    /// Status-dot colours (theme.ok / theme.warn / theme.err).
    ok?: string
    warn?: string
    err?: string
    size?: number
  }

  const {
    name,
    color,
    paper,
    ink,
    accent,
    active = false,
    status,
    ok = '#3ddc84',
    warn = '#d8b16a',
    err = '#e07474',
    size = 22,
  }: Props = $props()

  const initials = $derived(deriveInitials(name))

  const dotColor = $derived(
    status === 'connected' ? ok : status === 'connecting' ? warn : status === 'error' ? err : null,
  )

  /// First letters of the first two name parts (split on - _ . space), else the
  /// first two characters. Always 1–2 uppercase chars; never empty.
  function deriveInitials(n: string): string {
    const parts = n.split(/[-_.\s]+/).filter((p) => p.length > 0)
    if (parts.length >= 2) {
      return ((parts[0]?.charAt(0) ?? '') + (parts[1]?.charAt(0) ?? '')).toUpperCase()
    }
    return ((parts[0] ?? n).slice(0, 2) || '?').toUpperCase()
  }
</script>

<span class="avatar-wrap" style:width="{size}px" style:height="{size}px">
  <span
    class="avatar"
    class:active
    style:border-radius="{Math.round(size * 0.3)}px"
    style:font-size="{Math.round(size * 0.4)}px"
    style:background="color-mix(in srgb, {color} 24%, {paper})"
    style:color="color-mix(in srgb, {color} 70%, {ink})"
    style:border="1px solid color-mix(in srgb, {color} 40%, transparent)"
    style:box-shadow={active && accent
      ? `0 0 0 2px color-mix(in srgb, ${accent} 55%, transparent)`
      : 'none'}
  >
    {initials}
  </span>
  {#if dotColor}
    <span
      class="status-dot"
      class:pulse={status === 'connecting'}
      style:width="{Math.max(7, Math.round(size * 0.34))}px"
      style:height="{Math.max(7, Math.round(size * 0.34))}px"
      style:background={dotColor}
      style:box-shadow="0 0 0 2px {paper}"
      title={status}
    ></span>
  {/if}
</span>

<style>
  .avatar-wrap {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
  }

  .avatar {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    letter-spacing: 0.3px;
    font-family: var(--zx-font-mono);
    line-height: 1;
    user-select: none;
    transition: box-shadow 0.12s;
  }

  .status-dot {
    position: absolute;
    right: -2px;
    bottom: -2px;
    border-radius: 50%;
    pointer-events: none;
  }

  /* Connecting: gentle opacity pulse so the amber dot reads as "in progress". */
  .status-dot.pulse {
    animation: dot-pulse 1.3s ease-in-out infinite;
  }

  @keyframes dot-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .status-dot.pulse {
      animation: none;
    }
  }
</style>
