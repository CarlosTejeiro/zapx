// Terminal — renders a transcript array (lines of typed tokens) using a
// theme object. The same renderer is used by all three Pylon variants;
// only the theme + chrome around it differ.
//
// theme:
//   { fg, bg, dim, ok, warn, err, cmd, prompt, banner, head, accent,
//     mono, fontSize, lineHeight, padding, scanlines? }
//
// transcript line types: banner, prompt(+cmd, +cur), ok, warn, err, dim,
//   out, kv(+k,+v,+vt), table({head,rows}), h2, blank

(function () {
  const { useRef, useEffect, useState, useMemo, useLayoutEffect } = React;

  function Cursor({ color }) {
    return (
      <span style={{
        display: "inline-block", width: "0.55em", height: "1em",
        background: color, marginLeft: 1, verticalAlign: "text-bottom",
        animation: "pylon-cursor-blink 1.05s steps(2) infinite",
        boxShadow: `0 0 8px ${color}`,
      }} />
    );
  }

  // Typewriter-render text into target string. Returns a substring up to
  // `chars` characters; rest is held back.
  function typeUpTo(s, chars) {
    if (chars >= s.length) return [s, chars - s.length];
    return [s.slice(0, chars), 0];
  }

  function KV({ k, v, vt, theme }) {
    const color = vt === "ok" ? theme.ok : vt === "warn" ? theme.warn : vt === "dim" ? theme.dim : theme.fg;
    return (
      <div style={{ display: "flex", gap: 0 }}>
        <span style={{ color: theme.fg, opacity: 0.78, minWidth: 220, display: "inline-block" }}>{k}</span>
        <span style={{ color: theme.dim, opacity: 0.6 }}>: </span>
        <span style={{ color, fontWeight: vt === "ok" || vt === "warn" ? 600 : 400 }}>{v}</span>
      </div>
    );
  }

  function Table({ head, rows, theme }) {
    const cols = head.length;
    const widths = head.map((_, i) => Math.max(head[i].length, ...rows.map((r) => (r[i] || "").length)) + 2);
    const pad = (s, w) => (s + " ".repeat(Math.max(0, w - s.length)));
    return (
      <div style={{ marginTop: 2 }}>
        <div style={{ color: theme.accent, opacity: 0.95, fontWeight: 600 }}>
          {head.map((h, i) => pad(h, widths[i])).join("")}
        </div>
        <div style={{ color: theme.dim, opacity: 0.4 }}>
          {widths.map((w) => "─".repeat(w - 1) + " ").join("")}
        </div>
        {rows.map((r, ri) => (
          <div key={ri} style={{ color: theme.fg, opacity: 0.92 }}>
            {r.map((c, i) => {
              const isStatus = (i === cols - 1 || i === cols - 2) && /up|YES|running|Idle|stopped|down/.test(c);
              const col =
                /^(up|running|YES)$/.test(c) ? theme.ok :
                /^(down|Idle|stopped)$/.test(c) ? theme.err :
                /^!+$/.test(c) ? theme.ok :
                theme.fg;
              return <span key={i} style={{ color: col }}>{pad(c, widths[i])}</span>;
            })}
          </div>
        ))}
      </div>
    );
  }

  function Line({ line, theme, typed }) {
    const t = line.t;
    if (t === "banner") {
      return (
        <div style={{
          color: theme.banner, fontWeight: 600,
          padding: "4px 10px", marginBottom: 6,
          borderLeft: `2px solid ${theme.banner}`,
          background: theme.bannerBg || "transparent",
          opacity: 0.95,
        }}>{line.s}</div>
      );
    }
    if (t === "ok")    return <div style={{ color: theme.ok,   textShadow: theme.glow ? `0 0 6px ${theme.ok}55` : "none" }}>{line.s}</div>;
    if (t === "warn")  return <div style={{ color: theme.warn, textShadow: theme.glow ? `0 0 6px ${theme.warn}66` : "none" }}>{line.s}</div>;
    if (t === "err")   return <div style={{ color: theme.err }}>{line.s}</div>;
    if (t === "dim")   return <div style={{ color: theme.dim, opacity: 0.7 }}>{line.s}</div>;
    if (t === "out")   return <div style={{ color: theme.fg, opacity: 0.92 }}>{line.s}</div>;
    if (t === "h2")    return <div style={{ color: theme.accent, fontWeight: 600, marginTop: 4 }}>{line.s}</div>;
    if (t === "blank") return <div style={{ height: "1em" }}></div>;
    if (t === "kv")    return <KV k={line.k} v={line.v} vt={line.vt} theme={theme} />;
    if (t === "table") return <Table head={line.head} rows={line.rows} theme={theme} />;
    if (t === "prompt") {
      const promptColor = theme.prompt;
      const cmdColor = theme.cmd;
      const showCmd = line.cmd ? (typed != null ? typed : line.cmd) : "";
      return (
        <div style={{ display: "flex", alignItems: "baseline", flexWrap: "wrap" }}>
          <span style={{ color: promptColor, fontWeight: 600, textShadow: theme.glow ? `0 0 6px ${promptColor}88` : "none" }}>{line.s}</span>
          {line.cmd && <span style={{ color: cmdColor }}>{showCmd}</span>}
          {line.cur && <Cursor color={theme.cursor || theme.cmd} />}
        </div>
      );
    }
    return <div>{line.s}</div>;
  }

  // Terminal — wraps lines in a scroll container with optional scanlines,
  // a sticky bottom cursor line for the "live" prompt, and a typed-in
  // animation for the most recent command.
  function Terminal({ transcript, theme, liveInput, onInput, title, height = "100%", focus, onClickFocus, animate = true, scrollRef }) {
    const ref = useRef(null);
    // Find the last prompt line that has cmd — animate it via char-by-char
    const lastCmdIdx = useMemo(() => {
      for (let i = transcript.length - 1; i >= 0; i--) {
        if (transcript[i].t === "prompt" && transcript[i].cmd) return i;
      }
      return -1;
    }, [transcript]);

    const [typed, setTyped] = useState(animate ? "" : null);
    useEffect(() => {
      if (!animate || lastCmdIdx < 0) return;
      const target = transcript[lastCmdIdx].cmd;
      setTyped("");
      let i = 0;
      const id = setInterval(() => {
        i++;
        setTyped(target.slice(0, i));
        if (i >= target.length) clearInterval(id);
      }, 35 + Math.random() * 25);
      return () => clearInterval(id);
    }, [lastCmdIdx, animate]);

    // Autoscroll on transcript grow
    useEffect(() => {
      if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
    }, [transcript.length, typed]);

    return (
      <div
        onClick={onClickFocus}
        style={{
          position: "relative", height,
          background: theme.bg,
          color: theme.fg,
          fontFamily: theme.mono,
          fontSize: theme.fontSize,
          lineHeight: theme.lineHeight,
          overflow: "hidden",
          boxShadow: focus ? `inset 0 0 0 1.5px ${theme.accent}` : "none",
          transition: "box-shadow .15s",
        }}
      >
        {theme.bgPattern && (
          <div style={{ position: "absolute", inset: 0, pointerEvents: "none",
            background: theme.bgPattern, opacity: theme.bgPatternOpacity ?? 1 }}/>
        )}
        {theme.scanlines && (
          <div style={{
            position: "absolute", inset: 0, pointerEvents: "none",
            backgroundImage: `repeating-linear-gradient(0deg, ${theme.scanlineColor || "rgba(255,255,255,.03)"} 0 1px, transparent 1px 3px)`,
            mixBlendMode: "screen", opacity: 0.5,
          }} />
        )}
        {theme.vignette && (
          <div style={{ position: "absolute", inset: 0, pointerEvents: "none",
            boxShadow: "inset 0 0 120px rgba(0,0,0,.7)" }} />
        )}
        <div ref={ref || scrollRef} style={{
          position: "absolute", inset: 0, overflow: "auto",
          padding: theme.padding || "14px 18px 60px 18px",
        }}>
          {transcript.map((ln, i) => (
            <Line key={i} line={ln} theme={theme}
              typed={i === lastCmdIdx ? typed : null}
            />
          ))}
        </div>
      </div>
    );
  }

  // Insert keyframes once
  if (typeof document !== "undefined" && !document.getElementById("pylon-term-styles")) {
    const s = document.createElement("style");
    s.id = "pylon-term-styles";
    s.textContent = `
      @keyframes pylon-cursor-blink { 50% { opacity: 0; } }
      @keyframes pylon-glow-pulse { 0%,100% { opacity:.6 } 50% { opacity:1 } }
      @keyframes pylon-scan { 0% { transform: translateY(-100%) } 100% { transform: translateY(100vh) } }
      .pylon-scroll::-webkit-scrollbar { width: 6px; height: 6px; }
      .pylon-scroll::-webkit-scrollbar-track { background: transparent; }
      .pylon-scroll::-webkit-scrollbar-thumb { background: rgba(255,255,255,.12); border-radius: 3px; }
      .pylon-scroll::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,.22); }
    `;
    document.head.appendChild(s);
  }

  window.PylonTerminal = Terminal;
})();
