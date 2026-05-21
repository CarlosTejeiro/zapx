// Three theme variants — same PylonApp structure, very different vibes.

(function () {
  // ─────────────────────────────────────────────────────────────
  // V1 · NEON NOIR — deep slate, cyan + magenta neon accents
  // ─────────────────────────────────────────────────────────────
  const neon = {
    ui: '"Geist", "SF Pro Display", system-ui, sans-serif',
    mono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',
    radius: 5,
    tabRadius: "8px 8px 0 0",

    appBg: "#0a0d14",
    appBgPattern: `
      radial-gradient(800px 500px at 90% 10%, rgba(34,211,238,0.06), transparent 60%),
      radial-gradient(700px 500px at 10% 100%, rgba(244,114,182,0.05), transparent 60%)
    `,

    bodyBg: "#0d1119",
    titlebarBg: "linear-gradient(180deg, #11151e 0%, #0c1018 100%)",
    sidebarBg: "#0b0f17",
    tabBarBg: "#0a0d14",
    tabActiveBg: "#0d1119",
    tabActiveFg: "#e6edf3",
    tabIdleBg: "transparent",
    tabIdleFg: "#7a8694",
    statusBg: "linear-gradient(90deg, #0a0d14, #0d1119)",
    statusFg: "#a3b1c2",
    inputBg: "rgba(255,255,255,0.025)",
    paletteBg: "rgba(13,17,25,0.96)",

    divider: "rgba(255,255,255,0.06)",
    uiFg: "#e6edf3",
    uiDim: "#6b7785",
    itemHoverBg: "rgba(255,255,255,0.04)",
    itemActiveBg: "rgba(34,211,238,0.10)",
    itemActiveFg: "#e6edf3",

    accent: "#22d3ee",   // cyan
    accent2: "#f472b6",  // magenta
    ok: "#4ade80",
    warn: "#fbbf24",
    err: "#ef4444",

    terminal: {
      bg: "#05080d",
      bgPattern: `linear-gradient(rgba(34,211,238,0.025) 1px, transparent 1px), linear-gradient(90deg, rgba(34,211,238,0.025) 1px, transparent 1px)`,
      bgPatternOpacity: 1,
      fg: "#cbd5e1",
      dim: "#64748b",
      ok: "#4ade80",
      warn: "#fbbf24",
      err: "#ef4444",
      cmd: "#e2e8f0",
      prompt: "#22d3ee",
      cursor: "#f472b6",
      banner: "#22d3ee",
      bannerBg: "rgba(34,211,238,0.06)",
      head: "#22d3ee",
      accent: "#f472b6",
      mono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',
      ui: '"Geist", system-ui, sans-serif',
      fontSize: 12.5,
      lineHeight: 1.55,
      padding: "14px 18px 60px 18px",
      glow: true,
      divider: "rgba(255,255,255,0.06)",
      uiDim: "#64748b",
      uiFg: "#cbd5e1",
    },
  };

  // ─────────────────────────────────────────────────────────────
  // V2 · GRAPHITE — quiet dark slate, single muted teal accent, no glows.
  // The "boring is good" professional choice. Inspired by modern dev tools.
  // ─────────────────────────────────────────────────────────────
  const graphite = {
    ui: '"Geist", "SF Pro Text", system-ui, sans-serif',
    mono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',
    radius: 6,
    tabRadius: "6px 6px 0 0",

    appBg: "#1a1d23",
    appBgPattern: "",

    bodyBg: "#1e2127",
    titlebarBg: "#1a1d23",
    sidebarBg: "#181b20",
    tabBarBg: "#1a1d23",
    tabActiveBg: "#1e2127",
    tabActiveFg: "#e8eaed",
    tabIdleBg: "transparent",
    tabIdleFg: "#8c949e",
    statusBg: "#16191e",
    statusFg: "#9aa3ad",
    inputBg: "#13161a",
    paletteBg: "rgba(30,33,39,0.98)",

    divider: "rgba(255,255,255,0.07)",
    uiFg: "#e8eaed",
    uiDim: "#8c949e",
    itemHoverBg: "rgba(255,255,255,0.04)",
    itemActiveBg: "rgba(94,179,178,0.13)",
    itemActiveFg: "#e8eaed",

    accent: "#5eb3b2",   // muted teal
    accent2: "#c89b6b",  // warm tan (calm secondary)
    ok: "#7cb992",
    warn: "#d4a85a",
    err: "#d97070",

    terminal: {
      bg: "#15181d",
      fg: "#c9cdd3",
      dim: "#6b727c",
      ok: "#7cb992",
      warn: "#d4a85a",
      err: "#d97070",
      cmd: "#e8eaed",
      prompt: "#5eb3b2",
      cursor: "#5eb3b2",
      banner: "#5eb3b2",
      bannerBg: "rgba(94,179,178,0.05)",
      head: "#5eb3b2",
      accent: "#c89b6b",
      mono: '"JetBrains Mono", ui-monospace, monospace',
      ui: '"Geist", system-ui, sans-serif',
      fontSize: 12.5,
      lineHeight: 1.55,
      padding: "14px 18px 60px 18px",
      glow: false,
      divider: "rgba(255,255,255,0.07)",
      uiDim: "#6b727c",
      uiFg: "#c9cdd3",
    },
  };

  // ─────────────────────────────────────────────────────────────
  // V2-LEGACY · PHOSPHOR — kept for reference, not exported by default
  // ─────────────────────────────────────────────────────────────
  const phosphor = {
    ui: '"JetBrains Mono", "IBM Plex Mono", ui-monospace, monospace',
    mono: '"JetBrains Mono", "IBM Plex Mono", "VT323", ui-monospace, monospace',
    radius: 0,
    tabRadius: "0",

    appBg: "#000000",
    appBgPattern: `
      repeating-linear-gradient(0deg, rgba(74,222,128,0.025) 0 1px, transparent 1px 3px),
      radial-gradient(600px 400px at 50% 0%, rgba(74,222,128,0.06), transparent 70%)
    `,

    bodyBg: "#000000",
    titlebarBg: "#000000",
    sidebarBg: "#020401",
    tabBarBg: "#000000",
    tabActiveBg: "#0a1a0a",
    tabActiveFg: "#a3e635",
    tabIdleBg: "transparent",
    tabIdleFg: "#3f6b3f",
    statusBg: "#000000",
    statusFg: "#86c886",
    inputBg: "rgba(74,222,128,0.04)",
    paletteBg: "rgba(0,8,0,0.97)",

    divider: "rgba(74,222,128,0.12)",
    uiFg: "#a3e635",
    uiDim: "#4a7c4a",
    itemHoverBg: "rgba(74,222,128,0.06)",
    itemActiveBg: "rgba(74,222,128,0.14)",
    itemActiveFg: "#bef264",

    accent: "#4ade80",
    accent2: "#fbbf24",
    ok: "#86efac",
    warn: "#fbbf24",
    err: "#f87171",

    terminal: {
      bg: "#000000",
      fg: "#86efac",
      dim: "#3f6b3f",
      ok: "#a3e635",
      warn: "#fbbf24",
      err: "#f87171",
      cmd: "#bef264",
      prompt: "#4ade80",
      cursor: "#a3e635",
      banner: "#4ade80",
      bannerBg: "rgba(74,222,128,0.06)",
      head: "#fbbf24",
      accent: "#fbbf24",
      mono: '"JetBrains Mono", "VT323", ui-monospace, monospace',
      ui: '"JetBrains Mono", monospace',
      fontSize: 12.5,
      lineHeight: 1.6,
      padding: "14px 18px 60px 18px",
      glow: true,
      scanlines: true,
      scanlineColor: "rgba(74,222,128,0.035)",
      vignette: true,
      divider: "rgba(74,222,128,0.12)",
      uiDim: "#4a7c4a",
      uiFg: "#a3e635",
    },
  };

  // ─────────────────────────────────────────────────────────────
  // V3 · PARCHMENT — warm light shell, dark terminal panes embedded.
  // Same approach as VS Code light: chrome is paper-cream, terminal area
  // stays dark for readability. Single muted indigo accent.
  // ─────────────────────────────────────────────────────────────
  const parchment = {
    ui: '"Geist", "SF Pro Text", system-ui, sans-serif',
    mono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',
    radius: 6,
    tabRadius: "6px 6px 0 0",

    appBg: "#f5f2eb",
    appBgPattern: "",

    bodyBg: "#faf8f3",
    titlebarBg: "#efeae0",
    sidebarBg: "#f0ece2",
    tabBarBg: "#efeae0",
    tabActiveBg: "#faf8f3",
    tabActiveFg: "#2d2a24",
    tabIdleBg: "transparent",
    tabIdleFg: "#7d7466",
    statusBg: "#e7e1d3",
    statusFg: "#6a6151",
    inputBg: "#fdfbf6",
    paletteBg: "rgba(252,250,245,0.98)",

    divider: "rgba(85,70,40,0.10)",
    uiFg: "#2d2a24",
    uiDim: "#8a8174",
    itemHoverBg: "rgba(85,70,40,0.04)",
    itemActiveBg: "rgba(86,76,160,0.10)",
    itemActiveFg: "#2d2a24",

    accent: "#564ca0",     // muted indigo
    accent2: "#b3793a",    // warm amber
    ok: "#3e8f60",
    warn: "#b88528",
    err: "#b13a3a",

    terminal: {
      bg: "#1d1e26",
      fg: "#d4d6dc",
      dim: "#7e8090",
      ok: "#7fb89a",
      warn: "#e0b06c",
      err: "#e08a8a",
      cmd: "#eef0f4",
      prompt: "#8e85e0",
      cursor: "#8e85e0",
      banner: "#8e85e0",
      bannerBg: "rgba(142,133,224,0.10)",
      head: "#8e85e0",
      accent: "#d4a26c",
      mono: '"JetBrains Mono", ui-monospace, monospace',
      ui: '"Geist", system-ui, sans-serif',
      fontSize: 12.5,
      lineHeight: 1.55,
      padding: "14px 18px 60px 18px",
      glow: false,
      divider: "rgba(255,255,255,0.06)",
      uiDim: "#7e8090",
      uiFg: "#d4d6dc",
    },
  };

  // ─────────────────────────────────────────────────────────────
  // V3-LEGACY · VAPORWAVE — kept for reference, not exported by default
  // ─────────────────────────────────────────────────────────────
  const vapor = {
    ui: '"Space Grotesk", "Geist", system-ui, sans-serif',
    mono: '"JetBrains Mono", "Fira Code", ui-monospace, monospace',
    radius: 8,
    tabRadius: "10px 10px 0 0",

    appBg: "#0b0820",
    appBgPattern: `
      radial-gradient(900px 600px at 100% 0%, rgba(244,114,182,0.10), transparent 60%),
      radial-gradient(900px 600px at 0% 100%, rgba(56,189,248,0.10), transparent 60%),
      linear-gradient(135deg, rgba(168,85,247,0.04), transparent 50%)
    `,

    bodyBg: "#0e0a26",
    titlebarBg: "linear-gradient(180deg, rgba(168,85,247,0.08), rgba(11,8,32,0))",
    sidebarBg: "#0c0824",
    tabBarBg: "#0b0820",
    tabActiveBg: "#15103a",
    tabActiveFg: "#fbe7ff",
    tabIdleBg: "transparent",
    tabIdleFg: "#8b7fb8",
    statusBg: "linear-gradient(90deg, rgba(244,114,182,0.08), rgba(56,189,248,0.08))",
    statusFg: "#c4b8e8",
    inputBg: "rgba(255,255,255,0.04)",
    paletteBg: "rgba(15,10,40,0.96)",

    divider: "rgba(168,85,247,0.18)",
    uiFg: "#f0e7ff",
    uiDim: "#8b7fb8",
    itemHoverBg: "rgba(168,85,247,0.10)",
    itemActiveBg: "rgba(244,114,182,0.14)",
    itemActiveFg: "#fbe7ff",

    accent: "#f472b6",
    accent2: "#22d3ee",
    ok: "#5eead4",
    warn: "#fbbf24",
    err: "#fb7185",

    terminal: {
      bg: "#0a0620",
      bgPattern: `
        linear-gradient(rgba(244,114,182,0.04) 1px, transparent 1px),
        linear-gradient(90deg, rgba(56,189,248,0.04) 1px, transparent 1px)
      `,
      bgPatternOpacity: 1,
      fg: "#e0d8ff",
      dim: "#7a6fa3",
      ok: "#5eead4",
      warn: "#fbbf24",
      err: "#fb7185",
      cmd: "#fbe7ff",
      prompt: "#f472b6",
      cursor: "#22d3ee",
      banner: "#f472b6",
      bannerBg: "rgba(244,114,182,0.10)",
      head: "#22d3ee",
      accent: "#22d3ee",
      mono: '"JetBrains Mono", ui-monospace, monospace',
      ui: '"Space Grotesk", system-ui, sans-serif',
      fontSize: 12.5,
      lineHeight: 1.6,
      padding: "14px 18px 60px 18px",
      glow: true,
      divider: "rgba(168,85,247,0.18)",
      uiDim: "#7a6fa3",
      uiFg: "#e0d8ff",
    },
  };

  window.PYLON_THEMES = { neon, graphite, parchment, phosphor, vapor };
})();
