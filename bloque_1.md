# Bloque 1 — PoC del terminal local

## Contexto

Bajada a tareas concretas del Bloque 1 de [plan_desarrollo_v2.md](plan_desarrollo_v2.md). El Bloque 0 está cerrado: scaffold compila limpio, push hecho.

El Bloque 1 entrega un terminal local funcional. Al terminar, el usuario puede abrir zapx, ver un tab con una shell del sistema (`cmd.exe` en Windows, `bash` en Linux), escribir comandos, ver la salida con colores correctos, redimensionar la ventana y cerrar la sesión. No hay persistencia, no hay SSH, no hay UI de sesiones — solo el bucle PTY↔xterm.js probado a fondo.

Convenciones igual que el Bloque 0: **Owner** AI = borrador del asistente revisado por el engineer; **Owner** Eng = tarea manual. **Done when** son criterios objetivos.

## Arquitectura del Bloque 1

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend                                                   │
│  App.svelte → TerminalTab.svelte → xterm.js                 │
│    • Terminal.write(bytes)  ← Tauri event "terminal-data"   │
│    • terminal.onData        → invoke("send_input", ...)     │
│    • ResizeObserver + fit   → invoke("resize_terminal", ...)│
└─────────────────────────────────────────────────────────────┘
                  │ Tauri IPC (JSON)
┌─────────────────────────────────────────────────────────────┐
│  App layer (crates/app)                                     │
│  • open_local_session() → SessionId                         │
│  • send_input(id, data: Vec<u8>)                            │
│  • resize_terminal(id, cols, rows)                          │
│  • close_session(id)                                        │
│  • AppState: HashMap<SessionId, ActiveSession>              │
│  • Background task: PTY reader → emit terminal-data event   │
└─────────────────────────────────────────────────────────────┘
                  │
┌─────────────────────────────────────────────────────────────┐
│  core-transport                                             │
│  • LocalPty: Transport trait implementation                 │
│  • Usa portable-pty para ConPTY (Win) / Unix PTY            │
└─────────────────────────────────────────────────────────────┘
                  │
┌─────────────────────────────────────────────────────────────┐
│  core-terminal (scaffold validado, no conectado a display)  │
│  • TerminalState wrapping wezterm-term                      │
│  • Procesa los mismos bytes que xterm.js                    │
│  • Usado en Bloque 5 para keyword highlighting              │
└─────────────────────────────────────────────────────────────┘
```

**Decisiones de diseño para este bloque:**

- Los bytes del PTY viajan como `Vec<u8>` serializado en JSON del evento Tauri. Base64 en el frontend para reconstruir el `Uint8Array` que xterm acepta.
- El `AppState` usa `Arc<Mutex<HashMap<SessionId, ActiveSession>>>`. `SessionId` es un UUID (String).
- `wezterm-term` se integra en `core-terminal` y se valida con un test unitario que parsea una secuencia VT simple, pero no conecta aún al pipeline de rendering — eso llega en Bloque 5.
- Shell por defecto: `COMSPEC` en Windows (cmd.exe), `SHELL` en Linux, fallback a `/bin/sh`.

---

## Tareas

### T-01 — Añadir dependencias de Bloque 1 al workspace
- **Owner**: AI
- **Effort**: 30 min
- **Depends on**: —
- **Done when**:
  - `Cargo.toml` workspace añade: `portable-pty = "0.8"`, `wezterm-term = "0.1"`, `uuid = { version = "1", features = ["v4"] }`.
  - `crates/core-transport/Cargo.toml` declara `portable-pty = { workspace = true }`.
  - `crates/core-terminal/Cargo.toml` declara `wezterm-term = { workspace = true }`.
  - `crates/app/Cargo.toml` declara `uuid = { workspace = true }`.
  - `cargo build --workspace` pasa (descarga las nuevas deps).

### T-02 — core-transport: implementar `LocalPty`
- **Owner**: AI
- **Effort**: 2 h
- **Depends on**: T-01
- **Done when**:
  - `crates/core-transport/src/local_pty.rs` implementa `LocalPty` con:
    - `spawn(cols, rows) -> Result<LocalPty, Error>` — lanza la shell del SO con `portable-pty`.
    - `LocalPty` implementa el `Transport` trait: `connect`, `disconnect`, `resize`.
    - `reader() -> PtyReader` y `writer() -> PtyWriter` (newtype wrappers sobre los tipos de portable-pty) que son `Send`.
    - Detección de shell: `COMSPEC` (Windows) / `SHELL` (Unix) / fallback `/bin/sh`.
  - `crates/core-transport/src/error.rs` añade variantes `SpawnFailed`, `IoClosed`, `ResizeFailed`.
  - Test en `crates/core-transport/tests/local_pty_test.rs`: spawn + write "echo hello" + read bytes que contengan "hello".
  - `cargo test -p core-transport` pasa.

### T-03 — core-terminal: integrar wezterm-term
- **Owner**: AI
- **Effort**: 1.5 h
- **Depends on**: T-01
- **Done when**:
  - `crates/core-terminal/src/lib.rs` expone `TerminalState` que wrappea `wezterm_term::Terminal`.
  - `TerminalState::new(cols, rows)` construye el terminal.
  - `TerminalState::advance_bytes(data: &[u8])` alimenta bytes al parser VT.
  - `TerminalState::screen()` devuelve una referencia al `wezterm_term::Screen`.
  - Test unitario: avanzar una secuencia `\x1b[32mOK\x1b[0m` y verificar que la celda contiene "O" con atributo fg verde.
  - `cargo test -p core-terminal` pasa.

### T-04 — App layer: estado y comandos de sesión
- **Owner**: AI
- **Effort**: 2 h
- **Depends on**: T-02
- **Done when**:
  - `crates/app/src/state.rs` contiene `AppState` con `sessions: Arc<Mutex<HashMap<String, ActiveSession>>>`.
  - `ActiveSession` tiene: `pty_writer` (para recibir input), `kill_tx` (canal para señalar cierre), `cols/rows` actuales.
  - `crates/app/src/commands/sessions.rs` implementa cuatro comandos Tauri:
    - `open_local_session(app: AppHandle) -> Result<String, AppError>` — lanza PTY, registra la sesión, devuelve UUID.
    - `send_input(app: AppHandle, session_id: String, data: Vec<u8>) -> Result<(), AppError>`.
    - `resize_terminal(app: AppHandle, session_id: String, cols: u16, rows: u16) -> Result<(), AppError>`.
    - `close_session(app: AppHandle, session_id: String) -> Result<(), AppError>`.
  - Los cuatro comandos registrados en `tauri::generate_handler!` en `lib.rs`.
  - Compilación limpia (`cargo build -p app`).

### T-05 — App layer: streaming de bytes PTY → frontend
- **Owner**: AI
- **Effort**: 1.5 h
- **Depends on**: T-04
- **Done when**:
  - Al abrir sesión, se lanza una `tokio::task` que lee del PTY reader en bucle.
  - Cada lectura emite un evento Tauri `terminal-data` con payload `{ sessionId: String, data: Vec<u8> }`.
  - Al cerrar la sesión (por `close_session` o por EOF del PTY), la task termina limpiamente y la sesión se elimina del `AppState`.
  - No hay leak de tasks ni de file descriptors (verificado con logs de tracing a nivel DEBUG).
  - `cargo clippy -p app -- -D warnings` pasa.

### T-06 — Frontend: instalar xterm.js y crear TerminalTab
- **Owner**: AI
- **Effort**: 2 h
- **Depends on**: T-05
- **Done when**:
  - `frontend/package.json` añade `@xterm/xterm`, `@xterm/addon-fit`.
  - `frontend/src/lib/terminal/TerminalTab.svelte` con:
    - Monta un `xterm.js Terminal` en un `<div>` con `onMount`.
    - Aplica `FitAddon` para rellenar el contenedor.
    - Al montar: invoca `open_local_session`, guarda el `sessionId`.
    - Escucha el evento Tauri `terminal-data` y filtra por `sessionId`; decodifica el `Vec<u8>` → `Uint8Array` y llama `terminal.write()`.
    - `terminal.onData(data => invoke("send_input", { sessionId, data: stringToBytes(data) }))`.
    - `ResizeObserver` → `fitAddon.fit()` → `invoke("resize_terminal", { sessionId, cols, rows })`.
    - Al destruir: invoca `close_session`.
  - `pnpm type-check` pasa con 0 errores.

### T-07 — Frontend: layout con tab y terminal a pantalla completa
- **Owner**: AI
- **Effort**: 1 h
- **Depends on**: T-06
- **Done when**:
  - `App.svelte` reemplaza el "hello world" por un layout de dos zonas: barra superior estrecha (nombre de la app + título del tab) + panel de terminal que ocupa el resto de la ventana.
  - El fondo del terminal es negro/muy oscuro; no hay padding que recorte la terminal.
  - Un botón "New tab" (sin funcionalidad real aún — `console.log` como placeholder) en la barra.
  - `pnpm lint` y `pnpm type-check` pasan.

### T-08 — Validación manual + medición de SLAs (gate)
- **Owner**: Eng (validación) + AI (setup de medición)
- **Effort**: 1-2 h
- **Depends on**: T-07
- **Done when**:
  - **Funcional**: abrir la app → shell aparece → escribir `echo "zapx test"` → ver salida → escribir `dir` (Windows) / `ls -la` (Linux) → ver colores ANSI → redimensionar la ventana → ver que el terminal se adapta → cerrar la ventana → sin crash.
  - **Sin regresión de Bloque 0**: `cargo build --workspace`, `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `pnpm type-check`, `pnpm lint` siguen pasando.
  - **SLAs medidos** (no deben pasar el umbral; si lo hacen, bloquear y abrir issue antes de continuar):
    - Arranque en frío hasta shell visible: ≤3 s (objetivo MVP ≤1.5 s, aquí sin optimizar).
    - RAM con 1 tab activo: ≤120 MB (baseline antes de optimizar).
    - Latencia visual de tecla a carácter en pantalla: perceptiblemente instantánea (sin medición formal; si hay lag visible, investigar antes de seguir).
  - Tag `v0.1.0-alpha.1` en el repo.

---

## Grafo de dependencias

```
T-01 ──┬──▶ T-02 ──▶ T-04 ──▶ T-05 ──▶ T-06 ──▶ T-07 ──▶ T-08
       └──▶ T-03                                         ◀──┘
```

T-02 y T-03 son paralelos una vez T-01 termina. Ruta crítica: **T-01 → T-02 → T-04 → T-05 → T-06 → T-07 → T-08**. Aprox. 10-12 h de trabajo focalizado.

---

## Riesgos específicos del Bloque 1

| Riesgo | Mitigación |
|---|---|
| `portable-pty` en Windows requiere MSVC + Visual C++ runtime; puede fallar en entornos sin ellos. | Verificar en T-02 con Windows nativo. En WSL el target es `x86_64-unknown-linux-gnu`, que es más sencillo. |
| `wezterm-term` cambia API entre versiones; es una crate interna de WezTerm, no su API pública primaria. | Pin de versión exacta en T-01. Si la API cambia, tenemos la versión pinneada que funcionó. |
| Serialización de `Vec<u8>` como array JSON en Tauri event es ineficiente para ráfagas grandes. | En Bloque 1 es aceptable. Si en T-08 el throughput falla, migrar a base64 string (cambio acotado). |
| El canal Tauri IPC añade latencia apreciable en el bucle PTY→xterm. | Medir en T-08. Tauri 2 con WRY tiene latencia de IPC de 1-3 ms en LAN — tolerable para ≤16 ms objetivo. Si falla, ver ADR-003 (alternativa WebSocket localhost). |
| `portable-pty` en WSL puede no tener ConPTY y caer en un PTY Unix estándar. | Esperado. El Bloque 1 valida en WSL primero (Linux PTY), Windows nativo viene en T-12 del plan. |

---

## Lo que el Bloque 1 explícitamente NO hace

- Cero SSH, Telnet, Serial — Bloques 2 y 4.
- Cero persistencia de sesiones en SQLite — Bloque 3.
- Cero keyword highlighting — Bloque 5.
- Cero logging de sesión — Bloque 6.
- El `TerminalState` de `core-terminal` se implementa y prueba unitariamente, pero **no** conecta al pipeline de rendering (eso es Bloque 5).
- Cero múltiples tabs reales — el botón "New tab" es un stub.
- Cero vault ni gestión de credenciales.

---

## Gate de cierre

El Bloque 1 está cerrado cuando:

1. `cargo xtask dev` arranca y aparece una shell (`cmd.exe` o `bash`) en el terminal de xterm.js.
2. Typing de comandos funciona: se ve la salida, los colores ANSI se renderizan correctamente.
3. Redimensionar la ventana → el terminal se ajusta sin artefactos.
4. Cerrar la app → sin crash ni proceso zombie.
5. Todos los checks de CI siguen verdes: `cargo build --workspace`, `cargo clippy`, `cargo fmt --check`, `pnpm type-check`, `pnpm lint`.
6. SLAs medidos y registrados en este documento (aunque no cumplan el objetivo MVP aún — lo importante es tener la baseline).
