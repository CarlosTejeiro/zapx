// PylonApp — the full SSH client mock, themed by props. Used by all
// three variants on the design canvas.
//
// Layout: titlebar / toolbar+quickconnect / [sidebar | tabs+splits | (rightpane)] / statusbar
//
// Each variant supplies a `theme` (colors, fonts, accents, terminal theme),
// plus a `chrome` config (which corner buttons, accent rendering, etc.).

(function () {
  const { useState, useEffect, useRef, useMemo, useCallback } = React;
  const T = window.PylonTerminal;
  const { sessions: SESSIONS, transcripts: TX, snippets: SNIPPETS, palette: PALETTE } = window.PYLON;

  // ─────────────────────────────────────────────────────────────
  // Tab model
  // ─────────────────────────────────────────────────────────────
  function makeTab(id, name, host, color, txKey, tag) {
    return { id: id + "-" + Math.random().toString(36).slice(2, 7), name, host, color, txKey, tag, ts: Date.now() };
  }

  const DEFAULT_TABS = [
    makeTab("fmg", "10.5.201.175",  "admin@10.5.201.175", "#22d3ee", "fortimanager", "fmg"),
    makeTab("core","core-sw-01",    "netops@10.5.0.1",    "#a78bfa", "ciscoCore", "cisco"),
    makeTab("edge","edge-rtr-01",   "netops@10.5.10.1",   "#f472b6", "ciscoEdge", "cisco"),
    makeTab("pve", "proxmox",       "root@192.168.1.50",  "#10b981", "linux", "pve"),
  ];

  // ─────────────────────────────────────────────────────────────
  // Sidebar (tree + favorites pinned at top)
  // ─────────────────────────────────────────────────────────────
  function Sidebar({ theme, onConnect, activeHost, onSearch, search }) {
    const [open, setOpen] = useState({ favorites: true, "lab-local": true, "lab-sophia": true, datacenter: true });
    const toggle = (id) => setOpen((o) => ({ ...o, [id]: !o[id] }));

    const filter = (it) => !search || it.name.toLowerCase().includes(search.toLowerCase()) || (it.tag || "").toLowerCase().includes(search.toLowerCase());

    return (
      <div style={{
        width: 248, flexShrink: 0, height: "100%",
        background: theme.sidebarBg,
        borderRight: `1px solid ${theme.divider}`,
        display: "flex", flexDirection: "column",
        fontFamily: theme.ui, color: theme.uiFg,
        fontSize: 12.5,
      }}>
        {/* Search */}
        <div style={{ padding: "10px 10px 8px", borderBottom: `1px solid ${theme.divider}` }}>
          <div style={{ position: "relative" }}>
            <span style={{ position: "absolute", left: 10, top: "50%", transform: "translateY(-50%)", color: theme.uiDim, fontSize: 11 }}>⌕</span>
            <input
              value={search}
              onChange={(e) => onSearch(e.target.value)}
              placeholder="Quick connect…"
              style={{
                width: "100%", boxSizing: "border-box",
                background: theme.inputBg, color: theme.uiFg,
                border: `1px solid ${theme.divider}`,
                borderRadius: theme.radius, fontSize: 12,
                padding: "7px 9px 7px 26px",
                outline: "none", fontFamily: theme.ui,
                transition: "border-color .12s, box-shadow .12s",
              }}
              onFocus={(e) => { e.target.style.borderColor = theme.accent; e.target.style.boxShadow = `0 0 0 2px ${theme.accent}22`; }}
              onBlur={(e) => { e.target.style.borderColor = theme.divider; e.target.style.boxShadow = "none"; }}
            />
          </div>
        </div>

        {/* Tree */}
        <div className="pylon-scroll" style={{ flex: 1, overflowY: "auto", padding: "6px 0" }}>
          {SESSIONS.map((group) => {
            const items = group.items.filter(filter);
            if (search && !items.length) return null;
            const isOpen = open[group.id] || search;
            return (
              <div key={group.id} style={{ marginBottom: 4 }}>
                <button
                  onClick={() => toggle(group.id)}
                  style={{
                    width: "100%", textAlign: "left", border: "none", cursor: "pointer",
                    background: "transparent", padding: "6px 12px 6px 10px",
                    color: group.pinned ? theme.accent : theme.uiDim,
                    fontFamily: theme.ui, fontSize: 11, fontWeight: 600,
                    letterSpacing: 0.6, textTransform: "uppercase",
                    display: "flex", alignItems: "center", gap: 6,
                  }}>
                  <span style={{
                    display: "inline-block", width: 8, transition: "transform .12s",
                    transform: isOpen ? "rotate(90deg)" : "rotate(0deg)",
                    opacity: 0.6, fontSize: 9,
                  }}>▶</span>
                  <span style={{ flex: 1 }}>{group.label}</span>
                  <span style={{ opacity: 0.5, fontWeight: 400 }}>{items.length}</span>
                </button>
                {isOpen && (
                  <div>
                    {items.map((it) => {
                      const active = activeHost === it.host;
                      return (
                        <div
                          key={it.id}
                          onClick={() => onConnect(it)}
                          style={{
                            display: "flex", alignItems: "center", gap: 8,
                            padding: "5px 12px 5px 26px",
                            cursor: "pointer", fontSize: 12.5,
                            background: active ? theme.itemActiveBg : "transparent",
                            color: active ? theme.itemActiveFg : theme.uiFg,
                            borderLeft: `2px solid ${active ? it.color : "transparent"}`,
                            transition: "background .08s",
                          }}
                          onMouseEnter={(e) => { if (!active) e.currentTarget.style.background = theme.itemHoverBg; }}
                          onMouseLeave={(e) => { if (!active) e.currentTarget.style.background = "transparent"; }}
                        >
                          <span style={{
                            width: 8, height: 8, borderRadius: 4,
                            background: it.color,
                            boxShadow: it.status === "active" ? `0 0 8px ${it.color}` : "none",
                            animation: it.status === "active" ? "pylon-glow-pulse 2s infinite" : "none",
                          }}/>
                          <span style={{ flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{it.name}</span>
                          <span style={{
                            fontSize: 9.5, opacity: 0.7, color: it.color, fontWeight: 600,
                            padding: "1px 5px", border: `1px solid ${it.color}44`,
                            borderRadius: 3, textTransform: "uppercase", letterSpacing: 0.4,
                          }}>{it.tag}</span>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            );
          })}
        </div>

        {/* User footer */}
        <div style={{
          padding: "10px 12px", borderTop: `1px solid ${theme.divider}`,
          display: "flex", alignItems: "center", gap: 8, fontSize: 11.5,
        }}>
          <div style={{
            width: 22, height: 22, borderRadius: theme.radius,
            background: `linear-gradient(135deg, ${theme.accent}, ${theme.accent2})`,
            display: "flex", alignItems: "center", justifyContent: "center",
            color: "#0a0a0a", fontWeight: 700, fontSize: 11,
          }}>N</div>
          <div style={{ flex: 1, lineHeight: 1.2 }}>
            <div style={{ fontWeight: 600 }}>netops</div>
            <div style={{ color: theme.uiDim, fontSize: 10 }}>{SESSIONS.reduce((a,g)=>a+g.items.length,0)} hosts</div>
          </div>
          <button style={iconBtn(theme)} title="Settings">⚙</button>
        </div>
      </div>
    );
  }

  function iconBtn(theme) {
    return {
      border: "none", background: "transparent", cursor: "pointer",
      color: theme.uiDim, padding: 4, borderRadius: 4,
      fontSize: 13, lineHeight: 1, transition: "background .1s, color .1s",
    };
  }

  // ─────────────────────────────────────────────────────────────
  // Tabs — chrome-style, draggable
  // ─────────────────────────────────────────────────────────────
  function TabBar({ theme, tabs, activeId, onActivate, onClose, onReorder, onNew, splitOn, onToggleSplit, multiOn, onToggleMulti }) {
    const [dragging, setDragging] = useState(null);

    const onDragStart = (e, id) => { setDragging(id); e.dataTransfer.effectAllowed = "move"; };
    const onDragOver = (e, overId) => {
      e.preventDefault();
      if (!dragging || dragging === overId) return;
      const ids = tabs.map((t) => t.id);
      const from = ids.indexOf(dragging);
      const to = ids.indexOf(overId);
      if (from < 0 || to < 0) return;
      const next = ids.slice();
      next.splice(from, 1);
      next.splice(to, 0, dragging);
      onReorder(next);
    };
    const onDragEnd = () => setDragging(null);

    return (
      <div style={{
        display: "flex", alignItems: "flex-end",
        background: theme.tabBarBg,
        height: 36, paddingLeft: 8, gap: 1,
        borderBottom: `1px solid ${theme.divider}`,
        fontFamily: theme.ui,
      }}>
        {tabs.map((tb) => {
          const active = tb.id === activeId;
          return (
            <div
              key={tb.id}
              draggable
              onDragStart={(e) => onDragStart(e, tb.id)}
              onDragOver={(e) => onDragOver(e, tb.id)}
              onDragEnd={onDragEnd}
              onClick={() => onActivate(tb.id)}
              style={{
                display: "flex", alignItems: "center", gap: 8,
                padding: "0 10px 0 12px",
                height: active ? 32 : 28,
                marginTop: active ? 4 : 8,
                minWidth: 130, maxWidth: 220,
                background: active ? theme.tabActiveBg : theme.tabIdleBg,
                color: active ? theme.tabActiveFg : theme.tabIdleFg,
                borderRadius: theme.tabRadius || "8px 8px 0 0",
                borderTop: active ? `1.5px solid ${tb.color}` : `1.5px solid transparent`,
                borderLeft: `1px solid ${active ? theme.divider : "transparent"}`,
                borderRight: `1px solid ${active ? theme.divider : "transparent"}`,
                position: "relative", cursor: "pointer",
                opacity: dragging === tb.id ? 0.4 : 1,
                fontSize: 12, fontWeight: active ? 600 : 400,
                transition: "background .1s, color .1s",
                boxShadow: active ? `0 -2px 8px ${tb.color}22` : "none",
              }}>
              <span style={{
                width: 7, height: 7, borderRadius: 4, flexShrink: 0,
                background: tb.color,
                boxShadow: active ? `0 0 6px ${tb.color}` : "none",
              }}/>
              <span style={{
                flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                fontFamily: theme.ui,
              }}>{tb.name}</span>
              <button
                onClick={(e) => { e.stopPropagation(); onClose(tb.id); }}
                style={{
                  border: "none", background: "transparent", cursor: "pointer",
                  color: theme.uiDim, padding: 0, width: 14, height: 14,
                  borderRadius: 3, fontSize: 11, lineHeight: 1,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  opacity: active ? 0.7 : 0.4,
                  transition: "background .1s, opacity .1s",
                }}
                onMouseEnter={(e) => { e.currentTarget.style.background = theme.itemHoverBg; e.currentTarget.style.opacity = 1; }}
                onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.opacity = active ? 0.7 : 0.4; }}
              >✕</button>
            </div>
          );
        })}
        <button onClick={onNew} title="New tab" style={{
          ...iconBtn(theme), marginLeft: 4, marginBottom: 6, fontSize: 16, width: 24, height: 24,
        }} onMouseEnter={(e) => e.currentTarget.style.color = theme.accent}
           onMouseLeave={(e) => e.currentTarget.style.color = theme.uiDim}>+</button>

        <div style={{ flex: 1 }} />

        {/* Layout toggles */}
        <div style={{ display: "flex", alignItems: "center", gap: 4, padding: "0 10px", marginBottom: 4 }}>
          <ToolBtn theme={theme} active={splitOn} onClick={onToggleSplit} title="Split vertical (Ctrl+\\)">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5">
              <rect x="1.5" y="2" width="11" height="10" rx="1"/><path d="M7 2v10"/>
            </svg>
          </ToolBtn>
          <ToolBtn theme={theme} active={multiOn} onClick={onToggleMulti} title="MultiExec (Ctrl+Shift+M)">
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" strokeWidth="1.5">
              <path d="M2 4l5 4 5-4M2 9l5 4 5-4"/>
            </svg>
          </ToolBtn>
        </div>
      </div>
    );
  }

  function ToolBtn({ theme, active, onClick, title, children }) {
    return (
      <button onClick={onClick} title={title} style={{
        border: `1px solid ${active ? theme.accent : "transparent"}`,
        background: active ? `${theme.accent}18` : "transparent",
        color: active ? theme.accent : theme.uiDim,
        cursor: "pointer", padding: "4px 6px", borderRadius: theme.radius,
        display: "flex", alignItems: "center", justifyContent: "center",
        transition: "background .1s, color .1s, border-color .1s",
        boxShadow: active ? `0 0 8px ${theme.accent}33` : "none",
      }}
        onMouseEnter={(e) => { if (!active) { e.currentTarget.style.color = theme.uiFg; e.currentTarget.style.background = theme.itemHoverBg; } }}
        onMouseLeave={(e) => { if (!active) { e.currentTarget.style.color = theme.uiDim; e.currentTarget.style.background = "transparent"; } }}
      >{children}</button>
    );
  }

  // ─────────────────────────────────────────────────────────────
  // Title bar (Windows 11 style)
  // ─────────────────────────────────────────────────────────────
  function TitleBar({ theme, title, brand, subBrand }) {
    return (
      <div style={{
        display: "flex", alignItems: "center",
        height: 32, paddingLeft: 12, paddingRight: 0,
        background: theme.titlebarBg, color: theme.uiFg,
        fontFamily: theme.ui, fontSize: 12,
        borderBottom: `1px solid ${theme.divider}`,
        userSelect: "none", flexShrink: 0,
      }}>
        {/* brand mark */}
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div style={{
            width: 18, height: 18, position: "relative",
          }}>
            <svg viewBox="0 0 18 18" width="18" height="18">
              <defs>
                <linearGradient id="pl-brand" x1="0" y1="0" x2="1" y2="1">
                  <stop offset="0" stopColor={theme.accent}/>
                  <stop offset="1" stopColor={theme.accent2 || theme.accent}/>
                </linearGradient>
              </defs>
              <path d="M3 2 L9 2 L15 8 L9 16 L3 16 L9 9 Z" fill="url(#pl-brand)"/>
            </svg>
          </div>
          <span style={{ fontWeight: 700, letterSpacing: 1, color: theme.uiFg }}>{brand}</span>
          <span style={{ color: theme.uiDim, opacity: 0.6 }}>—</span>
          <span style={{ color: theme.uiDim }}>{title}</span>
        </div>

        {/* drag area */}
        <div style={{ flex: 1, height: "100%" }} />

        {/* menu buttons */}
        <div style={{ display: "flex", alignItems: "center" }}>
          {["File", "Edit", "View", "Session", "Tools", "Help"].map((m) => (
            <button key={m} style={{
              border: "none", background: "transparent", color: theme.uiDim,
              cursor: "pointer", padding: "6px 10px", fontSize: 12,
              fontFamily: theme.ui, transition: "background .1s, color .1s",
            }}
              onMouseEnter={(e) => { e.currentTarget.style.background = theme.itemHoverBg; e.currentTarget.style.color = theme.uiFg; }}
              onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = theme.uiDim; }}
            >{m}</button>
          ))}
        </div>

        {/* Windows traffic lights — right side */}
        <div style={{ display: "flex", marginLeft: 12 }}>
          {[
            { glyph: "—",   hover: theme.itemHoverBg, color: theme.uiDim },
            { glyph: "▢",   hover: theme.itemHoverBg, color: theme.uiDim },
            { glyph: "✕",   hover: "#e81123",          color: theme.uiDim },
          ].map((b, i) => (
            <button key={i} style={{
              width: 44, height: 32, border: "none", background: "transparent",
              color: b.color, cursor: "pointer", fontSize: 12,
              display: "flex", alignItems: "center", justifyContent: "center",
              transition: "background .1s, color .1s",
            }}
              onMouseEnter={(e) => { e.currentTarget.style.background = b.hover; if (i === 2) e.currentTarget.style.color = "#fff"; }}
              onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = b.color; }}
            >{b.glyph}</button>
          ))}
        </div>
      </div>
    );
  }

  // ─────────────────────────────────────────────────────────────
  // Status bar
  // ─────────────────────────────────────────────────────────────
  function StatusBar({ theme, tab, multiOn, splitOn, panelCount }) {
    const [t, setT] = useState(0);
    const [rx, setRx] = useState(48312);
    const [tx, setTx] = useState(8124);
    const [ping, setPing] = useState(8);
    useEffect(() => {
      const id = setInterval(() => {
        setT((x) => x + 1);
        setRx((x) => x + Math.floor(Math.random() * 240));
        setTx((x) => x + Math.floor(Math.random() * 60));
        setPing(() => 6 + Math.floor(Math.random() * 8));
      }, 1000);
      return () => clearInterval(id);
    }, []);

    const fmtTime = (n) => {
      const h = Math.floor(n / 3600), m = Math.floor((n % 3600) / 60), s = n % 60;
      return [h, m, s].map((x) => String(x).padStart(2, "0")).join(":");
    };
    const fmtKB = (n) => (n > 1024 * 1024) ? (n / 1024 / 1024).toFixed(2) + " MB"
                       : (n > 1024)        ? (n / 1024).toFixed(1) + " KB"
                       :                     n + " B";

    const dot = (color, pulse) => (
      <span style={{
        width: 6, height: 6, borderRadius: 3,
        background: color, boxShadow: pulse ? `0 0 8px ${color}` : "none",
        animation: pulse ? "pylon-glow-pulse 1.8s infinite" : "none",
      }} />
    );

    return (
      <div style={{
        display: "flex", alignItems: "center", gap: 12,
        height: 24, padding: "0 12px",
        background: theme.statusBg, color: theme.statusFg,
        borderTop: `1px solid ${theme.divider}`,
        fontFamily: theme.mono, fontSize: 11,
        flexShrink: 0,
      }}>
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          {dot(theme.ok, true)}
          <span style={{ color: theme.ok, fontWeight: 600 }}>CONNECTED</span>
        </div>
        <Sep theme={theme} />
        <span><span style={{ color: theme.uiDim }}>host </span>{tab.host}</span>
        <Sep theme={theme} />
        <span><span style={{ color: theme.uiDim }}>ssh </span>22</span>
        <Sep theme={theme} />
        <span style={{ color: ping < 10 ? theme.ok : ping < 20 ? theme.warn : theme.err }}>
          {ping}ms
        </span>
        <Sep theme={theme} />
        <span><span style={{ color: theme.uiDim }}>rx </span>{fmtKB(rx)}</span>
        <span><span style={{ color: theme.uiDim }}>tx </span>{fmtKB(tx)}</span>
        <Sep theme={theme} />
        <span><span style={{ color: theme.uiDim }}>up </span>{fmtTime(2934 + t)}</span>

        <div style={{ flex: 1 }} />

        {multiOn && (
          <>
            <div style={{
              display: "flex", alignItems: "center", gap: 6,
              padding: "1px 8px", borderRadius: 3,
              background: `${theme.accent2}22`, color: theme.accent2,
              border: `1px solid ${theme.accent2}55`,
              fontWeight: 600, letterSpacing: 0.4, fontSize: 10,
            }}>
              {dot(theme.accent2, true)}
              MULTIEXEC · {panelCount} panes
            </div>
            <Sep theme={theme} />
          </>
        )}
        <span><span style={{ color: theme.uiDim }}>utf-8</span></span>
        <Sep theme={theme} />
        <span><span style={{ color: theme.uiDim }}>VT320</span></span>
        <Sep theme={theme} />
        <span><span style={{ color: theme.uiDim }}>{splitOn ? "split-v" : "single"}</span></span>
      </div>
    );
  }
  function Sep({ theme }) {
    return <span style={{ color: theme.uiDim, opacity: 0.4 }}>│</span>;
  }

  // ─────────────────────────────────────────────────────────────
  // Command palette overlay
  // ─────────────────────────────────────────────────────────────
  function CommandPalette({ theme, open, onClose }) {
    const [q, setQ] = useState("");
    const [idx, setIdx] = useState(0);
    const inputRef = useRef(null);

    useEffect(() => {
      if (open) { setQ(""); setIdx(0); setTimeout(() => inputRef.current && inputRef.current.focus(), 30); }
    }, [open]);

    const items = useMemo(() => {
      if (!q) return PALETTE;
      const ql = q.toLowerCase();
      return PALETTE.filter((p) => p.label.toLowerCase().includes(ql) || p.sub.toLowerCase().includes(ql));
    }, [q]);

    useEffect(() => {
      if (!open) return;
      const onKey = (e) => {
        if (e.key === "Escape") onClose();
        else if (e.key === "ArrowDown") { setIdx((i) => Math.min(items.length - 1, i + 1)); e.preventDefault(); }
        else if (e.key === "ArrowUp")   { setIdx((i) => Math.max(0, i - 1)); e.preventDefault(); }
      };
      window.addEventListener("keydown", onKey);
      return () => window.removeEventListener("keydown", onKey);
    }, [open, items.length]);

    if (!open) return null;
    return (
      <div onClick={onClose} style={{
        position: "absolute", inset: 0, zIndex: 50,
        background: "rgba(5,5,12,.55)",
        backdropFilter: "blur(8px)",
        display: "flex", alignItems: "flex-start", justifyContent: "center",
        paddingTop: 110,
      }}>
        <div onClick={(e) => e.stopPropagation()} style={{
          width: 520, maxWidth: "85%",
          background: theme.paletteBg,
          border: `1px solid ${theme.accent}66`,
          borderRadius: 10,
          boxShadow: `0 24px 80px rgba(0,0,0,.6), 0 0 60px ${theme.accent}22, inset 0 0 0 1px ${theme.divider}`,
          overflow: "hidden",
          fontFamily: theme.ui,
        }}>
          <div style={{ display: "flex", alignItems: "center", padding: "12px 14px", gap: 10,
            borderBottom: `1px solid ${theme.divider}`,
          }}>
            <span style={{ color: theme.accent, fontSize: 14 }}>⌘</span>
            <input
              ref={inputRef}
              value={q}
              onChange={(e) => { setQ(e.target.value); setIdx(0); }}
              placeholder="Type a command, host, or snippet…"
              style={{
                flex: 1, background: "transparent", border: "none",
                color: theme.uiFg, fontSize: 14, fontFamily: theme.ui,
                outline: "none",
              }}
            />
            <span style={{
              fontSize: 10, color: theme.uiDim,
              padding: "2px 6px", border: `1px solid ${theme.divider}`,
              borderRadius: 3,
            }}>ESC</span>
          </div>
          <div style={{ maxHeight: 360, overflowY: "auto", padding: 6 }} className="pylon-scroll">
            {items.map((it, i) => (
              <div key={it.id} onMouseEnter={() => setIdx(i)}
                style={{
                  display: "flex", alignItems: "center", gap: 12,
                  padding: "9px 10px", borderRadius: 6, cursor: "pointer",
                  background: i === idx ? `${theme.accent}22` : "transparent",
                  border: i === idx ? `1px solid ${theme.accent}55` : "1px solid transparent",
                }}>
                <span style={{ color: theme.accent, fontSize: 14, width: 18, textAlign: "center" }}>{it.icon}</span>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ color: theme.uiFg, fontSize: 13 }}>{it.label}</div>
                  <div style={{ color: theme.uiDim, fontSize: 11, marginTop: 1 }}>{it.sub}</div>
                </div>
                {it.kbd && (
                  <span style={{
                    fontSize: 10, color: theme.uiDim,
                    padding: "2px 6px", border: `1px solid ${theme.divider}`,
                    borderRadius: 3, fontFamily: theme.mono,
                  }}>{it.kbd}</span>
                )}
              </div>
            ))}
            {!items.length && (
              <div style={{ padding: 30, textAlign: "center", color: theme.uiDim, fontSize: 12 }}>No matches.</div>
            )}
          </div>
          <div style={{
            display: "flex", gap: 16, padding: "8px 14px",
            borderTop: `1px solid ${theme.divider}`,
            fontSize: 10.5, color: theme.uiDim,
            background: theme.itemHoverBg,
          }}>
            <span><kbd style={kbdStyle(theme)}>↑↓</kbd> navigate</span>
            <span><kbd style={kbdStyle(theme)}>↵</kbd> select</span>
            <span><kbd style={kbdStyle(theme)}>Tab</kbd> snippets</span>
            <div style={{ flex: 1 }} />
            <span style={{ color: theme.accent }}>{items.length} results</span>
          </div>
        </div>
      </div>
    );
  }
  function kbdStyle(theme) {
    return {
      padding: "1px 5px", border: `1px solid ${theme.divider}`,
      borderRadius: 3, fontFamily: theme.mono, fontSize: 10,
      color: theme.uiDim, marginRight: 4,
    };
  }

  // ─────────────────────────────────────────────────────────────
  // The whole app
  // ─────────────────────────────────────────────────────────────
  function PylonApp({ theme, brand = "PYLON", subBrand = "SSH Client" }) {
    const [tabs, setTabs] = useState(DEFAULT_TABS);
    const [activeId, setActiveId] = useState(DEFAULT_TABS[0].id);
    const [splitOn, setSplitOn] = useState(true);
    const [multiOn, setMultiOn] = useState(false);
    const [paletteOpen, setPaletteOpen] = useState(false);
    const [search, setSearch] = useState("");
    const [searchInTerm, setSearchInTerm] = useState("");
    const [focusedPane, setFocusedPane] = useState(0);

    const activeTab = tabs.find((t) => t.id === activeId) || tabs[0];
    // For split, pick the secondary pane — second tab if present, else linux fallback
    const secondTab = tabs.find((t) => t.id !== activeId) || tabs[0];

    const onConnect = (it) => {
      const txKey =
        /fmg|forti.?manager/i.test(it.tag) ? "fortimanager" :
        /pve|ubuntu|proxmox/i.test(it.tag) ? "linux" :
        /cisco|core/i.test(it.tag) ? "ciscoCore" : "ciscoEdge";
      const existing = tabs.find((t) => t.host.includes(it.host));
      if (existing) { setActiveId(existing.id); return; }
      const tb = makeTab(it.id, it.name, `${it.user}@${it.host}`, it.color, txKey, it.tag);
      setTabs((ts) => [...ts, tb]);
      setActiveId(tb.id);
    };

    const onClose = (id) => {
      setTabs((ts) => {
        const next = ts.filter((t) => t.id !== id);
        if (id === activeId && next.length) setActiveId(next[0].id);
        return next;
      });
    };

    const onNew = () => {
      const tb = makeTab("new", "new session", "—", theme.accent, "ciscoCore", "new");
      setTabs((ts) => [...ts, tb]);
      setActiveId(tb.id);
    };

    const onReorder = (orderIds) => {
      const byId = Object.fromEntries(tabs.map((t) => [t.id, t]));
      setTabs(orderIds.map((id) => byId[id]).filter(Boolean));
    };

    // Keyboard
    useEffect(() => {
      const onKey = (e) => {
        if ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "K")) { e.preventDefault(); setPaletteOpen((o) => !o); }
        if ((e.ctrlKey || e.metaKey) && e.key === "\\") { e.preventDefault(); setSplitOn((s) => !s); }
        if ((e.ctrlKey || e.metaKey) && e.shiftKey && (e.key === "m" || e.key === "M")) { e.preventDefault(); setMultiOn((m) => !m); }
      };
      window.addEventListener("keydown", onKey);
      return () => window.removeEventListener("keydown", onKey);
    }, []);

    const termTheme = theme.terminal;
    const paneCount = splitOn ? 2 : 1;
    const multiPaneCount = multiOn ? Math.max(2, paneCount) : paneCount;

    return (
      <div style={{
        position: "relative", width: "100%", height: "100%",
        background: theme.appBg,
        color: theme.uiFg, fontFamily: theme.ui,
        overflow: "hidden",
        display: "flex", flexDirection: "column",
      }}>
        {theme.appBgPattern && (
          <div style={{ position: "absolute", inset: 0,
            background: theme.appBgPattern, pointerEvents: "none", opacity: theme.appBgPatternOpacity ?? 1 }}/>
        )}

        <TitleBar theme={theme} title={`${activeTab.name} — ${activeTab.host}`} brand={brand} subBrand={subBrand} />

        <div style={{ flex: 1, display: "flex", minHeight: 0, position: "relative", zIndex: 1 }}>
          <Sidebar theme={theme} onConnect={onConnect} activeHost={activeTab.host.split("@")[1]} search={search} onSearch={setSearch} />

          <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0, background: theme.bodyBg }}>
            <TabBar
              theme={theme} tabs={tabs} activeId={activeId}
              onActivate={setActiveId} onClose={onClose} onReorder={onReorder} onNew={onNew}
              splitOn={splitOn} onToggleSplit={() => setSplitOn((s) => !s)}
              multiOn={multiOn} onToggleMulti={() => setMultiOn((m) => !m)}
            />

            {/* Panes */}
            <div style={{ flex: 1, display: "flex", minHeight: 0, position: "relative" }}>
              <div style={{ flex: 1, display: "flex", minWidth: 0 }}>
                <Pane
                  theme={termTheme} tab={activeTab}
                  multiOn={multiOn} focused={focusedPane === 0}
                  onFocus={() => setFocusedPane(0)}
                />
                {splitOn && (
                  <>
                    <Resizer theme={theme} />
                    <Pane
                      theme={termTheme} tab={secondTab}
                      multiOn={multiOn} focused={focusedPane === 1}
                      onFocus={() => setFocusedPane(1)}
                    />
                  </>
                )}
              </div>
            </div>
          </div>
        </div>

        <StatusBar theme={theme} tab={activeTab} multiOn={multiOn} splitOn={splitOn} panelCount={multiPaneCount} />

        <CommandPalette theme={theme} open={paletteOpen} onClose={() => setPaletteOpen(false)} />
      </div>
    );
  }

  function Resizer({ theme }) {
    return (
      <div style={{
        width: 5, cursor: "col-resize", flexShrink: 0,
        background: theme.divider, opacity: 0.6,
        position: "relative",
      }}>
        <div style={{
          position: "absolute", left: -2, right: -2, top: 0, bottom: 0,
        }}/>
      </div>
    );
  }

  function Pane({ theme, tab, multiOn, focused, onFocus }) {
    const transcript = window.PYLON.transcripts[tab.txKey] || window.PYLON.transcripts.ciscoCore;
    return (
      <div style={{
        flex: 1, minWidth: 0, position: "relative",
        boxShadow: focused ? `inset 0 0 0 1.5px ${theme.accent}88` : "none",
      }} onClick={onFocus}>
        {/* per-pane header */}
        <div style={{
          height: 22, display: "flex", alignItems: "center", gap: 6,
          padding: "0 10px", fontSize: 10.5, fontFamily: theme.ui,
          background: theme.paneHeaderBg || "rgba(0,0,0,.25)",
          borderBottom: `1px solid ${theme.divider}`,
          color: theme.fg, letterSpacing: 0.3,
        }}>
          <span style={{
            width: 7, height: 7, borderRadius: 4, background: tab.color,
            boxShadow: `0 0 6px ${tab.color}`,
          }}/>
          <span style={{ color: theme.fg, opacity: 0.85, fontWeight: 600 }}>{tab.name}</span>
          <span style={{ color: theme.dim, opacity: 0.5 }}>·</span>
          <span style={{ color: theme.dim, opacity: 0.7 }}>{tab.host}</span>
          <div style={{ flex: 1 }} />
          {multiOn && (
            <span style={{
              fontSize: 9, color: theme.accent, fontWeight: 700, letterSpacing: 0.5,
              padding: "1px 5px", border: `1px solid ${theme.accent}66`, borderRadius: 3,
            }}>MULTI</span>
          )}
        </div>
        <div style={{ position: "absolute", top: 22, left: 0, right: 0, bottom: 0 }}>
          <T transcript={transcript} theme={theme} animate={focused}
            onClickFocus={onFocus} focus={focused} />
        </div>
      </div>
    );
  }

  window.PylonApp = PylonApp;
})();
