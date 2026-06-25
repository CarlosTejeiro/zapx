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
    /// The session has a live connection open → show a status dot.
    connected?: boolean
    /// Status-dot colour when `connected` (theme.ok).
    ok?: string
    size?: number
  }

  const {
    name,
    color,
    paper,
    ink,
    accent,
    active = false,
    connected = false,
    ok = '#3ddc84',
    size = 22,
  }: Props = $props()

  const initials = $derived(deriveInitials(name))

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
  {#if connected}
    <span
      class="status-dot"
      style:width="{Math.max(7, Math.round(size * 0.34))}px"
      style:height="{Math.max(7, Math.round(size * 0.34))}px"
      style:background={ok}
      style:box-shadow="0 0 0 2px {paper}"
      title="Connected"
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
</style>
