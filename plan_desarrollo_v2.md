# Plan de desarrollo v2: Terminal multiprotocolo en Rust para network engineers

> Documento de trabajo para el desarrollo conjunto del proyecto. Sustituye al assessment v1 en su parte de planificación operativa; el v1 sigue siendo válido como análisis competitivo y de mercado.

## 1. Contexto del proyecto

### 1.1 Quiénes somos y cómo trabajamos
- **Equipo**: un network engineer con sólida experiencia en Python (tú) + un asistente IA para arquitectura, código, revisión y aprendizaje (yo).
- **Ritmo**: sin fecha, iterativo, calidad sobre velocidad.
- **Modelo**: software libre con donaciones (GitHub Sponsors / Open Collective).
- **Motivación primaria**: construir la herramienta que tú querrías usar a diario como network engineer, y que la comunidad pueda adoptar.

### 1.2 Nicho objetivo
Network engineers que hoy usan SecureCRT, MobaXterm o PuTTY para gestionar equipos Cisco, Juniper, Arista, Fortinet, MikroTik, Aruba, etc., por SSH/Telnet/Serial. Sysadmins que se conectan a servidores Linux/Unix desde Windows también caben en el target.

### 1.3 Requisitos explícitos (de la conversación)
1. **Gestión fácil de conexiones** con árbol de carpetas (similar al "Connections" de SecureCRT/MobaXterm).
2. **Logging de sesiones** completo, fiable, con rotación y búsqueda.
3. **Apariencia personalizable** y coloreado de texto en el stream (keyword highlighting tipo SecureCRT, esencial para parsear salidas de red).
4. **Aplicación liviana**: arranque rápido, footprint de RAM bajo, binario manejable.
5. **Estética moderna** como factor de atracción frente a la UI dated de los incumbentes.

### 1.4 Decisión de stack
**Rust como lenguaje principal**, decidido tras análisis comparado vs Python. Motivos:
- Cumple el requisito "liviano" sin discusión (binario único 15-25 MB, RAM idle ~70 MB).
- Rendimiento sostenido superior con 20+ sesiones simultáneas.
- Distribución como un único `.exe` sin runtime, sin antivirus marcando falsos positivos como hace con PyInstaller.
- Sin GIL, escalado real a múltiples cores.
- Type system riguroso: refactorings seguros años después.
- Backward compatibility extrema desde Rust 1.0 (2015).
- Memory-safety crítica para una herramienta que maneja credenciales.
- Aprender Rust es un activo de carrera transferible para un network engineer.

## 2. Stack técnico definitivo

### 2.1 Núcleo Rust

| Capa | Crate | Versión orientativa | Función |
|---|---|---|---|
| Async runtime | `tokio` | 1.40+ | Base de toda la concurrencia |
| SSH | `russh` + `russh-keys` + `russh-sftp` | 0.46+ | Cliente SSH, autenticación, SFTP |
| PTY local | `portable-pty` | 0.9+ | PTYs en Windows (ConPTY) / Unix |
| Emulación VT | `wezterm-term` | última | Parser y estado de terminal completo |
| Serial | `tokio-serial` + `serialport` | 5.x | Puerto serie cross-platform |
| Telnet | `telnet-codec` + `tokio-rustls` | última | Telnet y Telnet/TLS |
| TLS | `rustls` + `aws-lc-rs` | última | Stack TLS para Telnet/TLS y futuras necesidades |
| Persistencia | `rusqlite` con `bundled` | 0.32+ | SQLite embebido, sin dependencias del SO |
| Cifrado at-rest | `chacha20poly1305` + `argon2` | última | Cifrado de vault y configuración sensible |
| Credenciales OS | `keyring` | 3.x | Windows Credential Manager / Keychain / Secret Service |
| Serialización | `serde` + `serde_json` + `toml` | última | Configuración y APIs |
| Logging | `tracing` + `tracing-subscriber` | última | Logging estructurado |
| Errores | `thiserror` + `anyhow` | última | Manejo de errores |
| Regex | `regex` + `fancy-regex` | última | Keyword highlighting con lookbehind |
| Configuración | `directories` | 5.x | Paths estándar por SO |
| Async utils | `futures` + `tokio-util` | última | Streams, codec utilities |

### 2.2 Frontend (Tauri 2 + Svelte 5)

| Pieza | Tecnología | Función |
|---|---|---|
| Contenedor | **Tauri 2** | Webview nativo + bridge Rust↔JS |
| Framework UI | **Svelte 5** con runes | Reactividad fina, sintaxis limpia, bundle pequeño |
| Estilos | **Tailwind CSS 4** | Utility-first, tema oscuro/claro fácil |
| Componentes | **shadcn-svelte** + componentes propios | Botones, diálogos, dropdowns pulidos |
| Terminal renderer | **xterm.js 5** + addons (`fit`, `search`, `web-links`, `webgl`/`canvas`) | Renderizado de terminal, estándar de industria |
| Iconos | **Lucide Svelte** | Iconografía consistente y moderna |
| Animaciones | Transiciones nativas Svelte + `motion` cuando haga falta | Polish sin pesadez |
| Drag & drop | `svelte-dnd-action` | Reordenar carpetas, mover sesiones |
| Estado global | Svelte stores + runes | Sin Redux ni complejidad innecesaria |
| Build tool | **Vite** (incluido en Tauri) | HMR rápido en desarrollo |

### 2.3 Empaquetado y distribución

| SO | Empaquetado |
|---|---|
| Windows 11 | MSI + MSIX firmados con EV cert (cuando llegue el momento de releases públicas) |
| macOS (futuro) | .pkg / .dmg notarizado |
| Linux (futuro) | AppImage + .deb + .rpm + Flatpak |

Auto-update con **Tauri Updater** y firma Ed25519 propia adicional.

## 3. Arquitectura del software

### 3.1 Workspace Cargo (estructura del repositorio)

```
proyecto/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── core-transport/           # trait Transport + implementaciones
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ssh.rs            # russh wrapper
│   │   │   ├── telnet.rs
│   │   │   ├── serial.rs
│   │   │   └── local_pty.rs
│   ├── core-terminal/            # wezterm-term wrapper + highlighting engine
│   ├── core-session/             # Modelo de sesión, lifecycle, state
│   ├── core-persistence/         # SQLite + cifrado + migraciones
│   ├── core-vault/               # Credenciales: keyring + DPAPI envuelto
│   ├── core-logging/             # Session loggers (file rotating, search)
│   ├── core-highlight/           # Motor de keyword highlighting con regex
│   ├── core-config/              # Settings, themes, profiles
│   └── app/                      # Binario Tauri principal
│       ├── src-tauri/
│       └── src/                  # Frontend Svelte (también podría ir aparte)
├── frontend/                     # Frontend Svelte (alternativa a meter en app/)
│   ├── src/
│   ├── package.json
│   └── vite.config.ts
├── docs/                         # Docs internas y ADRs
│   └── adr/
├── scripts/                      # Scripts de build, dev, release
└── README.md
```

**Por qué workspace de crates desde el día 1**: aunque al principio toda la lógica cabría en un solo crate, separar capas con sus propios límites claros tiene dos beneficios enormes:
1. Te obliga a definir APIs explícitas entre módulos (no se cuelan dependencias circulares).
2. Permite test e iteración aisladas: `cargo test -p core-transport` no compila el resto.

### 3.2 Diagrama de capas

```
┌─────────────────────────────────────────────────────────────┐
│  Frontend (Svelte 5 + xterm.js)                             │
│  • SessionTree (árbol de carpetas y conexiones)             │
│  • TerminalPane (xterm.js + addons)                         │
│  • Tabs / Split panes                                       │
│  • Settings / Themes / Highlight rules editor               │
│  • Quick connect / Command palette                          │
└─────────────────────────────────────────────────────────────┘
                       │ Tauri commands + events (JSON)
┌─────────────────────────────────────────────────────────────┐
│  App layer (Rust, dentro de src-tauri)                      │
│  • Command handlers (#[tauri::command])                     │
│  • Event emitters (push de bytes del terminal al frontend)  │
│  • Session lifecycle orchestration                          │
│  • Window/menu/tray management                              │
└─────────────────────────────────────────────────────────────┘
                       │
┌─────────────────────────────────────────────────────────────┐
│  Core crates                                                │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │transport │ │ terminal │ │ session  │ │ logging  │        │
│  │ SSH      │ │ wezterm- │ │ lifecycle│ │ rotating │        │
│  │ Telnet   │ │ term     │ │ state    │ │ search   │        │
│  │ Serial   │ │          │ │          │ │          │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐        │
│  │persistence│ │  vault   │ │ highlight│ │  config  │        │
│  │ SQLite   │ │ keyring  │ │ regex    │ │ themes   │        │
│  │ encrypted│ │ DPAPI    │ │ engine   │ │ profiles │        │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘        │
└─────────────────────────────────────────────────────────────┘
                       │
┌─────────────────────────────────────────────────────────────┐
│  Platform adapters                                          │
│  Windows: ConPTY · DPAPI · WinCred · WiX/MSIX               │
│  (Mac/Linux: futuro)                                        │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 Decisiones arquitectónicas clave (ADRs iniciales)

**ADR-001: Backend Rust nativo, no sidecar.**
Toda la lógica de protocolos y persistencia vive dentro del binario Tauri. Sin procesos hijos Python ni Node. Razón: alineado con requisito de "liviano" y simplicidad de distribución.

**ADR-002: Frontend en webview Tauri, terminal renderer con xterm.js.**
La UI general va en Svelte (productividad y estética). El renderizado del terminal usa xterm.js por madurez del proyecto y porque reescribir un renderer de terminal con ligaduras, true color, OSC8, sixel, etc. es trabajo de años. xterm.js lo usan VS Code, Hyper, Tabby, Termius en producción.

**ADR-003: Comunicación Rust↔JS por Tauri events para streams de terminal, commands para el resto.**
Cada PTY/SSH channel emite un event tipado al frontend con los bytes recibidos. Los inputs de teclado y operaciones administrativas (abrir sesión, listar carpetas, guardar config) van por `invoke` (request/response). Si en el futuro el throughput satura, migrar el stream a un canal Tauri 2 o WebSocket localhost.

**ADR-004: SQLite con cifrado a nivel aplicación.**
SQLite con `rusqlite` feature `bundled` (compila SQLite contra Rust, sin dependencia del SO). El blob de la base puede ir cifrado con XChaCha20-Poly1305 usando una clave envuelta por DPAPI (Windows) / Keychain (macOS) / Secret Service (Linux). Credenciales sensibles (passwords, passphrases) no van en SQLite: van directas al keyring del SO, en SQLite solo queda una referencia opaca.

**ADR-005: Schema versionado con migraciones.**
Cualquier cambio de schema se hace por migración numerada con script SQL idempotente. Nunca borrar columnas en producción; marcar como deprecated y limpiar en migraciones posteriores.

**ADR-006: Keyword highlighting como capa post-emulación.**
El highlighting se aplica al estado de la grilla del terminal después de que `wezterm-term` haya procesado las secuencias VT, no al stream crudo. Esto permite resaltar resultados de comandos sin romper secuencias de escape.

**ADR-007: No abrir issues sobre features fuera del scope de la fase actual.**
Disciplina de scope. Anotamos ideas en un backlog (`docs/backlog.md`) pero solo se prioriza lo de la fase activa.

**ADR-008: Logs de la app (no de sesiones) con `tracing` + JSON estructurado.**
Útil para debug del propio binario. Distinto de los session logs, que son texto/HTML.

**ADR-009: Cross-platform desde el día 1 en CI, aunque solo distribuyamos Windows en Fase 1.**
GitHub Actions matrix compila para Windows/macOS/Linux desde el primer commit. Detectamos pronto cualquier código accidentalmente platform-specific.

**ADR-010: `#![forbid(unsafe_code)]` en todos los crates excepto los adaptadores de plataforma.**
Unsafe Rust solo donde es genuinamente necesario (FFI a DPAPI, llamadas Win32 que no cubren las crates), y siempre documentado y aislado.

## 4. Modelo de datos

### 4.1 Schema SQLite (v1, MVP)

```sql
-- Folders: árbol de carpetas para organizar sesiones
CREATE TABLE folders (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_id   INTEGER REFERENCES folders(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    color       TEXT,
    icon        TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions: una conexión guardada
CREATE TABLE sessions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id       INTEGER REFERENCES folders(id) ON DELETE SET NULL,
    name            TEXT NOT NULL,
    protocol        TEXT NOT NULL CHECK (protocol IN ('ssh', 'telnet', 'serial', 'local')),
    host            TEXT,
    port            INTEGER,
    serial_device   TEXT,
    serial_baud     INTEGER,
    serial_options  TEXT,
    username        TEXT,
    auth_method     TEXT,
    credential_id   INTEGER REFERENCES credentials(id) ON DELETE SET NULL,
    profile_id      INTEGER REFERENCES profiles(id) ON DELETE SET NULL,
    options_json    TEXT NOT NULL DEFAULT '{}',
    tags            TEXT,
    notes           TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at    TEXT
);

CREATE INDEX idx_sessions_folder ON sessions(folder_id);
CREATE INDEX idx_sessions_last_used ON sessions(last_used_at DESC);

-- Credentials: referencias a secrets en keyring (NUNCA el secreto real)
CREATE TABLE credentials (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL,
    kind            TEXT NOT NULL,
    username        TEXT,
    keyring_key     TEXT NOT NULL,
    key_file_path   TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Profiles: terminal appearance reutilizable
CREATE TABLE profiles (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    name              TEXT NOT NULL UNIQUE,
    font_family       TEXT NOT NULL DEFAULT 'Cascadia Code',
    font_size         INTEGER NOT NULL DEFAULT 14,
    line_height       REAL NOT NULL DEFAULT 1.2,
    cursor_style      TEXT NOT NULL DEFAULT 'block',
    cursor_blink      INTEGER NOT NULL DEFAULT 1,
    color_scheme_id   INTEGER REFERENCES color_schemes(id),
    scrollback_lines  INTEGER NOT NULL DEFAULT 50000,
    options_json      TEXT NOT NULL DEFAULT '{}',
    is_builtin        INTEGER NOT NULL DEFAULT 0,
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE color_schemes (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL UNIQUE,
    palette_json TEXT NOT NULL,
    is_builtin   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Highlight rules: reglas de coloreado de texto
CREATE TABLE highlight_rules (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL,
    pattern      TEXT NOT NULL,
    is_regex     INTEGER NOT NULL DEFAULT 1,
    fg_color     TEXT,
    bg_color     TEXT,
    bold         INTEGER NOT NULL DEFAULT 0,
    underline    INTEGER NOT NULL DEFAULT 0,
    enabled      INTEGER NOT NULL DEFAULT 1,
    scope        TEXT NOT NULL DEFAULT 'global',
    scope_ref    INTEGER,
    sort_order   INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_highlight_scope ON highlight_rules(scope, scope_ref);

-- Session logs metadata (los logs son ficheros en disco)
CREATE TABLE session_logs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  INTEGER REFERENCES sessions(id) ON DELETE CASCADE,
    started_at  TEXT NOT NULL,
    ended_at    TEXT,
    file_path   TEXT NOT NULL,
    bytes       INTEGER NOT NULL DEFAULT 0,
    encrypted   INTEGER NOT NULL DEFAULT 0,
    format      TEXT NOT NULL DEFAULT 'raw'
);

CREATE INDEX idx_session_logs_session ON session_logs(session_id, started_at DESC);

-- Settings globales (key-value)
CREATE TABLE settings (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL,
    type   TEXT NOT NULL DEFAULT 'string'
);

-- Migración tracking
CREATE TABLE schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### 4.2 Tipos Rust principales (sketch)

```rust
// crates/core-session/src/lib.rs
pub enum Protocol {
    Ssh(SshConfig),
    Telnet(TelnetConfig),
    Serial(SerialConfig),
    Local,
}

pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
    pub jump_host: Option<Box<SshConfig>>,
}

pub enum SshAuth {
    Password { credential_ref: CredentialRef },
    PublicKey { key_path: PathBuf, passphrase_ref: Option<CredentialRef> },
    Agent,
}

pub struct CredentialRef(pub String);  // opaque keyring key

pub struct Session {
    pub id: SessionId,
    pub name: String,
    pub folder_id: Option<FolderId>,
    pub protocol: Protocol,
    pub profile_id: Option<ProfileId>,
    pub tags: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait Transport: Send {
    async fn connect(&mut self) -> Result<(), TransportError>;
    async fn disconnect(&mut self) -> Result<(), TransportError>;
    fn writer(&self) -> Box<dyn AsyncWrite + Send + Unpin>;
    fn reader(&self) -> Box<dyn AsyncRead + Send + Unpin>;
    fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TransportError>;
}
```

## 5. Definición refinada del MVP (Fase 1)

### 5.1 Scope MVP - Ligeramente reducido vs v1

Tras la decisión de dúo + foco network engineer, el MVP se ajusta:

**Obligatorio en MVP**
- SSH2 con auth por password y public key (RSA, ECDSA, Ed25519).
- Telnet (RFC 854 + NAWS + ECHO + SGA + TTYPE).
- Serial / COM (Windows y Linux).
- Emulación xterm-256color con UTF-8 y true color.
- Tabs (no split panes inicialmente).
- Session manager con árbol de carpetas, drag-and-drop, búsqueda incremental.
- Quick connect (atajo de teclado).
- Vault local: credenciales en Windows Credential Manager.
- Keyword highlighting con regex (mínimo 10 reglas predefinidas para errores típicos: `%ERROR`, `down`, `up`, IPs, MAC addresses).
- Logging de sesiones a fichero con rotación por tamaño/fecha.
- Búsqueda en buffer (Ctrl+F).
- Copy/paste mejorado (paste con confirmación si hay newlines).
- ≥10 themes incluidos (One Dark, Solarized, Dracula, Tokyo Night, Nord, Gruvbox, Catppuccin, Monokai, GitHub).
- Fuentes incluidas: Cascadia Code, JetBrains Mono.
- Configuración persistente en SQLite.
- Atajos de teclado configurables (mínimo: nuevo tab, cerrar tab, siguiente/anterior, quick connect, búsqueda).
- Installer MSI sin firmar (firma viene cuando empecemos a distribuir públicamente).

**Aplazado a Fase 2**
- Split panes (horizontal/vertical, nested).
- SFTP browser.
- Jump hosts SSH (proxy jump).
- Auto-completado de hostnames recientes en quick connect.
- Snippets (envío de bloques de texto predefinido).
- Multi-execution (enviar el mismo comando a N tabs).

**Aplazado más allá de Fase 2**
- RDP / VNC / Mosh.
- X server.
- Scripting embebido (Lua/Python).
- Button bar.
- macOS / Linux native.
- Cloud sync.
- AI integration.

### 5.2 Métricas de aceptación del MVP

| Métrica | Objetivo |
|---|---|
| Arranque en frío (SSD NVMe, Win11) | ≤1.5 s |
| Latencia teclado→render local | ≤16 ms p99 |
| Throughput SSH sostenido LAN 1 Gbps | ≥30 MB/s |
| RAM idle | ≤80 MB |
| RAM con 5 tabs activas | ≤200 MB |
| RAM con 20 tabs activas | ≤400 MB |
| Tamaño instalador (Win x64) | ≤25 MB |
| Crash-free sessions (en uso propio) | ≥99% |

### 5.3 Casos de uso de validación

Para considerar el MVP "listo" tienen que funcionar bien estos tres flujos reales:

1. **El día normal del network engineer**: abrir la app, expandir la carpeta "Core routers", hacer doble click en `core-rtr-01` (SSH), entrar con clave pública, ejecutar `show interface description | include up`, ver IPs resaltadas en color, copiar una línea, abrir un tab nuevo con `show ip bgp summary`, salir.
2. **El troubleshooting con logging**: conectarse a un switch problemático, activar logging con un click, enviar varios comandos, dejar la sesión 30 minutos capturando, salir, abrir el fichero de log en otro editor, buscar la línea exacta donde apareció `%LINEPROTO-5-UPDOWN`.
3. **El equipo nuevo por consola**: enchufar un cable serial USB, crear una sesión Serial con baud 9600, abrir, configurar el equipo desde cero con copy/paste de un bloque de configuración generado con plantillas.

Si los tres funcionan con fluidez en Windows 11, el MVP está cerrado.

## 6. Plan de aprendizaje de Rust en paralelo al proyecto

Esta sección es específica para nuestro modo de trabajo. Tu Python te facilita el camino, pero hay conceptos de Rust que conviene asimilar en orden.

### 6.1 Roadmap de aprendizaje

**Bloque A - Fundamentos (semanas 1-3)**
- Sintaxis básica, variables, funciones, control de flujo.
- Ownership, borrowing, lifetimes (el corazón de Rust).
- Structs, enums, pattern matching.
- Traits (similar a clases abstractas o protocolos en Python).
- `Result<T, E>` y `Option<T>` (manejo de errores explícito).
- Módulos y crates.
- Material recomendado: "The Rust Book" oficial (capítulos 1-10).

**Bloque B - Práctico (semanas 4-6)**
- Async/await con Tokio (diferente al asyncio de Python en detalles importantes).
- Generics y trait bounds.
- Error handling profesional con `thiserror` + `anyhow`.
- Iteradores y closures.
- Smart pointers: `Box`, `Rc`, `Arc`, `Mutex`, `RwLock`.
- Cargo workspaces.
- Material: "Rust Book" caps 11-21, "Tokio Tutorial", "Async Book".

**Bloque C - Aplicado al proyecto (semanas 7+)**
- A medida que escribimos código real, te explico los patrones específicos que usamos.
- Code reviews: yo te reviso PRs/diffs con explicación pedagógica del porqué de cada cambio.
- "Rust by Example" como referencia rápida.

**Ritmo realista**: si dedicas 5-8 horas/semana a aprender + practicar, en 6-8 semanas tendrás autonomía para escribir Rust sin pelearte con el compilador en cada función. En 4-5 meses lo escribirás con fluidez.

### 6.2 Cómo trabajaremos en el código

- **Yo escribo el primer borrador** de los módulos complejos (transport, terminal, persistence con cifrado). Tú los lees, preguntas, y propones cambios.
- **Tú escribes** lo más cercano a tu dominio (configuración de session, lógica de highlight rules específicas de Cisco, formatos de log, profiles).
- **Pair programming asíncrono**: tú compartes el diff o el archivo, yo te explico cada bloque, dudas → resolvemos antes de avanzar.
- **Tests primero cuando sea posible**: muchos módulos del core se prestan a TDD y ahí Rust + Python se parecen.
- **Commits pequeños y atómicos** con mensaje claro. Esto importa mucho si volvemos al proyecto después de un parón.

### 6.3 Setup de desarrollo recomendado

- **OS**: Windows 11 para el desarrollo principal (alineado con la Fase 1).
- **Editor**: VS Code con extensiones:
  - `rust-analyzer` (LSP de Rust, fundamental)
  - `Even Better TOML`
  - `CodeLLDB` para debugging
  - `Tauri` extension
  - `Svelte for VS Code` + `Svelte 5 Snippets`
  - `Tailwind CSS IntelliSense`
- **Toolchain**:
  - Rust stable via `rustup` (última versión)
  - Node.js LTS
  - pnpm como gestor de paquetes JS (más rápido que npm, mejor en monorepos)
- **Utilidades Rust**:
  - `cargo-watch` (re-compila al guardar)
  - `cargo-edit` (gestión cómoda de deps)
  - `cargo-nextest` (test runner rápido)
  - `cargo-deny` (auditoría de licencias y vulns)
- **Git**: GitHub para el repo, con CI básico desde el día 1.

## 7. Plan de trabajo por bloques (sin fechas)

Sin fechas, pero con orden lógico. Cada bloque es una "milestone" cerrable con valor demostrable.

### Bloque 0 - Setup (1-2 sesiones nuestras)
- [ ] Decidir nombre del proyecto y reservar dominios/handles.
- [ ] Crear repo en GitHub con licencia (propuesta: **Apache 2.0**).
- [ ] Setup local: Rust toolchain, Node.js, VS Code con extensiones.
- [ ] Generar esqueleto Tauri 2 + Svelte 5 + Tailwind.
- [ ] Cargo workspace con los crates vacíos definidos en §3.1.
- [ ] GitHub Actions: matrix build Windows/macOS/Linux + tests + clippy.
- [ ] README inicial con propósito y estado.
- [ ] Primera "hello world" deployable: app que abre una ventana con "ProjectName v0.0.1".

### Bloque 1 - PoC del terminal local
- [ ] `core-transport`: implementar `Transport` trait con variante `Local` usando `portable-pty`.
- [ ] `core-terminal`: integrar `wezterm-term` y validar que parsea correctamente.
- [ ] App layer: command `open_local_session` que crea un PTY, lanza un shell, y stream bytes al frontend por eventos Tauri.
- [ ] Frontend: pestaña con xterm.js que recibe bytes y envía teclas al backend.
- [ ] Validar: abrir cmd/PowerShell, escribir comandos, ver salida con colores, cerrar.
- [ ] **Gate**: medir latencia y throughput. Si no cumple los SLAs, ajustar arquitectura antes de seguir.

### Bloque 2 - SSH funcional mínimo
- [ ] `core-transport`: variante `Ssh` con `russh`.
- [ ] Conexión a host con password (autenticación por usuario+password hardcoded primero, vault después).
- [ ] Conexión con clave pública desde fichero PEM/OpenSSH.
- [ ] Known_hosts handling con TOFU básico.
- [ ] Manejo de resize del PTY remoto.
- [ ] **Validar contra varios devices**: un Linux SSH, un Cisco, un Juniper, un MikroTik que tengas accesibles. Lista los quirks que aparezcan.

### Bloque 3 - Persistencia y session manager
- [ ] `core-persistence`: SQLite con migraciones, schema v1.
- [ ] `core-vault`: integración con Windows Credential Manager via `keyring`.
- [ ] Cifrado at-rest del SQLite (XChaCha20-Poly1305 + DPAPI envuelta).
- [ ] CRUD de folders y sessions desde el backend.
- [ ] Frontend: árbol de carpetas con drag-and-drop, dialog de "Nueva sesión".
- [ ] Doble click en sesión → abre tab con la conexión correspondiente.

### Bloque 4 - Telnet y Serial
- [ ] `core-transport`: variantes `Telnet` y `Serial`.
- [ ] UI de configuración de sesión Serial (device picker, baud, parity, etc.).
- [ ] Telnet con NAWS para reportar tamaño de terminal.
- [ ] **Validar**: conectarse a un equipo viejo por Telnet, configurar un router por consola serial.

### Bloque 5 - Keyword highlighting
- [ ] `core-highlight`: motor de reglas regex con prioridades.
- [ ] Aplicación al estado del terminal (post-emulación VT).
- [ ] Editor de reglas en el frontend (lista con regex tester en vivo).
- [ ] 10 reglas predefinidas útiles para network: errores Cisco/Juniper típicos, IPs IPv4/IPv6, MAC addresses, interfaces, números de VLAN, "up"/"down" en distintos contextos.
- [ ] Scope por sesión o global.

### Bloque 6 - Logging
- [ ] `core-logging`: logger por sesión con rotación por tamaño y fecha.
- [ ] Formato raw text con timestamps opcionales.
- [ ] UI: botón de "Activar logging" por tab, indicador visual, path al fichero.
- [ ] Búsqueda en buffer de sesión activa (Ctrl+F en xterm.js con `addon-search`).
- [ ] Listado de logs anteriores por sesión, con botón para abrir.

### Bloque 7 - Apariencia
- [ ] 10 themes integrados.
- [ ] UI de selección de tema y customización (font, size, line height, cursor).
- [ ] Per-session theme override.
- [ ] Modo claro/oscuro automático siguiendo el SO.
- [ ] Polish general de la UI: animaciones sutiles, iconografía Lucide, tipografía cuidada.

### Bloque 8 - Polish y release 0.1.0
- [ ] Atajos de teclado configurables.
- [ ] Quick connect (Ctrl+N).
- [ ] Settings dialog completo.
- [ ] Manual de usuario básico (MD en repo + website estático).
- [ ] Installer MSI con WiX (sin firmar inicialmente).
- [ ] Auto-update via Tauri Updater (apuntando a releases de GitHub).
- [ ] README pulido, screenshots, vídeo demo corto.
- [ ] **Release 0.1.0 público** en GitHub.

### Bloque 9 - Feedback y Fase 2
- Recoger feedback de la comunidad (issues, Discord/Matrix).
- Priorizar Fase 2: split panes, SFTP, jump hosts, snippets, multi-execution.

## 8. Modelo open source

### 8.1 Licencia
**Recomendación**: **Apache 2.0**.
- Permite uso comercial sin fricción (atrae contributors corporativos).
- Cláusula explícita de patentes (protege a contributors y usuarios).
- Compatible con casi todo el ecosistema Rust.
- Es lo que usan Tauri, tokio, la mayoría de crates que vamos a usar.

Alternativa: **MIT** si quieres máxima simplicidad sin cláusula de patentes.

Alternativa más restrictiva: **GPLv3** si te preocupa que un competidor comercial empaquete tu trabajo sin contribuir. Pero limitará contributors corporativos y la adopción.

### 8.2 Infraestructura

- **Repo**: GitHub (público desde el día 1, aunque al principio sea esqueleto).
- **Issues + Discussions** en GitHub para roadmap, bugs, ideas.
- **CI/CD**: GitHub Actions.
- **Documentation**: MkDocs Material o Docusaurus, deployado a GitHub Pages.
- **Comunidad**: empezar simple con GitHub Discussions; añadir Discord o Matrix si crece.
- **Donaciones**:
  - **GitHub Sponsors**: lo más visible y fácil de configurar.
  - **Open Collective**: más transparente, mejor si llegan donaciones corporativas.
  - **Liberapay**: Europa-friendly, sin intermediarios USA.
- **Aspectos legales**:
  - Declarar ingresos por donaciones como rendimientos en la AEAT.
  - Si llega a >10k€/año, plantearse una asociación o autónomo profesional para optimización.
  - GDPR: si la app envía cualquier dato a un servidor (telemetría opt-in, crash reports), DPIA y política de privacidad pública.

### 8.3 Branding mínimo

Para que el proyecto se distinga:
- **Nombre**: corto, memorable, .io o .dev disponible, no chocar con marcas registradas. Idealmente evocador del nicho (red, terminal, conexión) sin ser literal.
- **Logo**: SVG simple, monocromo + variante color. Una tarde con Figma o contratar a alguien por 50-150€.
- **Tagline**: una frase. Ej.: "El terminal moderno para network engineers" / "Modern terminal for network engineers".
- **Website**: una landing simple en Astro o Hugo, con descripción, screenshots, descarga, link a docs y GitHub. Hosting en Cloudflare Pages o GitHub Pages, gratis.

## 9. Estimaciones (honestas, sin equipo profesional)

Para nuestro modo de trabajo real (network engineer aprendiendo Rust + asistente IA, sin fecha límite, dedicación variable):

| Bloque | Esfuerzo aprendizaje | Esfuerzo desarrollo | Calendario realista |
|---|---|---|---|
| 0. Setup | Alto | Bajo | 2-4 semanas |
| 1. PoC terminal local | Alto (primer contacto Rust real) | Medio | 3-6 semanas |
| 2. SSH funcional | Medio | Alto | 4-8 semanas |
| 3. Persistencia + session manager | Medio | Alto | 4-8 semanas |
| 4. Telnet + Serial | Bajo | Medio | 3-5 semanas |
| 5. Keyword highlighting | Bajo | Medio | 3-5 semanas |
| 6. Logging | Bajo | Medio | 2-4 semanas |
| 7. Apariencia | Bajo (es frontend) | Medio | 3-5 semanas |
| 8. Release 0.1.0 | Bajo | Alto (todo el polish) | 4-8 semanas |
| **Total a MVP usable** | | | **~7-13 meses calendario** |

Asume dedicación de 8-15 horas/semana. Si trabajas más, comprime; si menos, dilata.

## 10. Riesgos específicos a nuestro setup

| Riesgo | Mitigación |
|---|---|
| **Pérdida de momentum por la curva Rust** | Bloques cortos con valor entregable; celebrar cada milestone; volver a Python para automatizaciones laterales si hace falta respirar |
| **Sobre-ingeniería** (workspace de 8 crates puede ser excesivo al inicio) | Empezar con 2-3 crates y separar más cuando duela; el compilador nos avisa cuando los límites están borrosos |
| **Scope creep** ("¿y si añadimos RDP ya?") | Backlog disciplinado; cada feature fuera de bloque actual va al backlog, no al código |
| **Que el proyecto compita con tu trabajo y se quede parado** | Aceptar parones; commits pequeños y READMEs claros para retomar fácil; documentar decisiones (ADRs) para no olvidar el porqué |
| **Que la comunidad no llegue** | Es un riesgo aceptable porque el primer usuario eres tú; si solo lo usas tú, el proyecto sigue teniendo valor |
| **Quemarse por aprender Rust en paralelo a hacer producto** | Aceptar que las primeras 50-100 líneas de Rust serán dolorosas; yo te ayudo con explicaciones detalladas; usar `rust-analyzer` y `clippy` como copiloto |
| **Dependencias del ecosistema que cambien** (russh, tauri 3, etc.) | Versiones pinneadas; Dependabot/Renovate para actualizaciones controladas |
| **Code-signing y distribución cuando llegue** | Esperar a tener 0.1.0 público para invertir en EV cert (~300-500€/año); inicialmente release sin firmar con instrucciones de "unblock" |

## 11. Resumen ejecutivo de la decisión

**Stack final**:
- **Backend**: Rust con `russh`, `portable-pty`, `wezterm-term`, `tokio`, `rusqlite`, `keyring`.
- **Frontend**: Tauri 2 + Svelte 5 + Tailwind + xterm.js.
- **Empaquetado**: MSI/MSIX para Windows; arquitectura preparada para macOS/Linux desde el día 1.
- **Licencia**: Apache 2.0.
- **Modelo**: Software libre con donaciones (GitHub Sponsors + Open Collective).

**MVP (Fase 1)**:
- SSH + Telnet + Serial + emulación terminal + tabs + session manager + vault + keyword highlighting + logging + themes + búsqueda + atajos.
- Windows 11.
- Métricas: arranque <1.5s, RAM idle <80 MB, latencia <16 ms.

**Modo de trabajo**:
- Dúo: tú network engineer (con Python sólido, aprendiendo Rust) + yo (arquitectura, código, revisión, tutoría).
- Sin fechas, iterativo, bloques cerrables con valor.
- Aprendizaje de Rust en paralelo, 8-15 h/semana.

**Próximos pasos inmediatos** (Bloque 0):
1. Decidir el nombre del proyecto.
2. Setup de entorno: instalar Rust, Node, VS Code.
3. Generar repo + esqueleto Tauri+Svelte.
4. Compilar y arrancar el "hello world" deployable.

A partir de ahí, atacamos el Bloque 1 (PoC terminal local) y empezamos a construir de verdad.
