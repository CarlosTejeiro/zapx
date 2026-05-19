# Bloque 0 — Desglose ejecutable

## Contexto

Bajada a tareas concretas del Bloque 0 de [plan_desarrollo_v2.md](plan_desarrollo_v2.md). El proyecto se llama **zapx**. El Bloque 0 se cierra cuando existe un "hello world" desplegable — una ventana que abre y muestra `zapx v0.0.1`, construida y arrancada con el dev loop documentado, con CI verde en Windows/macOS/Linux.

Convenciones del desglose:
- **Owner**: quién dirige la tarea. *AI* significa borrador escrito por el asistente, revisado por el engineer (modelo del plan §6.2). *Eng* significa tarea manual del engineer (instalaciones, decisiones, claves).
- **Effort**: estimación de tiempo de trabajo focalizado, no calendario.
- **Depends on**: tareas que deben estar cerradas antes de empezar ésta.
- **Done when**: criterios objetivos, no descripción.

## Pre-condiciones

- Windows 11 con derechos de administrador para instalar toolchains.
- Cuenta GitHub disponible (`cto074` o nueva org `zapx` — se decide en T-01).
- Conexión estable para descargar Rust, Node, deps.
- Bloques de 1-2 h sin interrupciones; el scaffolding fragmentado en trozos de 15 min se rompe.

---

## Tareas

### T-01 — Decidir cuenta GitHub y reservar handles
- **Owner**: Eng
- **Effort**: 30 min
- **Depends on**: —
- **Done when**:
  - Decidido: GitHub personal (`cto074/zapx`) vs organización nueva (`zapx-dev/zapx`).
  - Comprobada y reservada disponibilidad de handles externos: `crates.io/zapx`, `npmjs.com/zapx` (aunque no se publique, evitar squatting), dominio (`zapx.dev` / `zapx.io` / `zapx.app` — el primero libre).
  - Decisión anotada en una línea al inicio de [plan_desarrollo_v2.md](plan_desarrollo_v2.md).

### T-02 — Crear repo GitHub y mover los docs foundational
- **Owner**: Eng
- **Effort**: 45 min
- **Depends on**: T-01
- **Done when**:
  - Repo `<owner>/zapx` creado, **público desde el día 0**.
  - `LICENSE` = Apache 2.0 (texto canónico).
  - `README.md` mínimo: "zapx — modern terminal for network engineers. Pre-alpha, no usable code yet."
  - Default branch `main`.
  - Branch protection en `main`: require PR, require CI green (cuando llegue CI), no force-push, no delete branch. *Sin* required reviewers (somos uno).
  - Initial commit incluye los cuatro docs: `plan_desarrollo_v2.md`, `style_guide.md`, `repo_structure.md`, `bloque_0.md`.

### T-03 — Instalar toolchain Rust
- **Owner**: Eng
- **Effort**: 45 min (incluye MSVC Build Tools si no estaban)
- **Depends on**: — (paralelizable con T-01/T-02)
- **Done when**:
  - `rustup` instalado, toolchain stable activo. `rustc --version` y `cargo --version` responden.
  - Componentes: `rustfmt`, `clippy`, `rust-analyzer`.
  - Targets registrados: `x86_64-pc-windows-msvc` (default), `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin` (para `cargo check` cross-platform local).
  - `cargo install`: `cargo-nextest`, `cargo-deny`, `cargo-edit`, `cargo-watch`, `cargo-udeps` (nightly opcional).
  - **Microsoft C++ Build Tools** instalados (necesarios para `rusqlite bundled` y `portable-pty`). Esta es la trampa silenciosa de Windows.
  - WiX Toolset v3 instalado (para el MSI en T-12; se puede aplazar a T-12 si urge).

### T-04 — Instalar toolchain JS
- **Owner**: Eng
- **Effort**: 20 min
- **Depends on**: —
- **Done when**:
  - Node.js LTS instalado.
  - `pnpm` activado via Corepack: `corepack enable && corepack prepare pnpm@latest --activate`.
  - `node --version`, `pnpm --version` verifican.

### T-05 — VS Code y extensiones
- **Owner**: Eng
- **Effort**: 15 min
- **Depends on**: —
- **Done when**:
  - VS Code instalado.
  - Extensiones: `rust-analyzer`, `Even Better TOML`, `CodeLLDB`, `Tauri`, `Svelte for VS Code`, `Tailwind CSS IntelliSense`, `ESLint`, `Prettier`, `GitHub Pull Requests`.
  - (No tocar `settings.json` aún; se hace en T-06 como parte del workspace).

### T-06 — Clonar repo y crear estructura raíz
- **Owner**: AI + Eng (pair)
- **Effort**: 1 h
- **Depends on**: T-02, T-03
- **Done when**:
  - Repo clonado en `~/zapx` (o equivalente Windows).
  - Ficheros de configuración top-level creados según [repo_structure.md](repo_structure.md): `Cargo.toml` (workspace, members vacío), `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `.editorconfig`, `.gitignore`, `.gitattributes`, `.markdownlint.yaml`, `NOTICE`, `CHANGELOG.md` (sección Unreleased), `CONTRIBUTING.md` (stub), `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1), `SECURITY.md` (stub).
  - Directorios vacíos con `.gitkeep`: `crates/`, `frontend/`, `xtask/`, `docs/adr/`, `docs/architecture/`, `docs/user-guide/`, `docs/developer-guide/`, `assets/fonts/`, `assets/themes/`, `assets/highlight-rules/`.
  - `.vscode/settings.json` workspace: rust-analyzer, Tailwind, format-on-save.
  - PR: `chore: scaffold repo structure` — auto-merge porque CI aún no existe.

### T-07 — Scaffold Tauri 2 + Svelte 5 + Tailwind
- **Owner**: AI draft, Eng review
- **Effort**: 2-3 h (debug del build chain en Windows ocupa la mitad)
- **Depends on**: T-04, T-06
- **Done when**:
  - `crates/app/` con `Cargo.toml` (dependencias Tauri 2 pinneadas a versión exacta), `tauri.conf.json` (`productName: "zapx"`, `identifier: "dev.zapx"`, `frontendDist: "../../frontend/dist"`, `devUrl: "http://localhost:1420"`), `capabilities/default.json`, `icons/` con placeholders.
  - `crates/app/src/main.rs` minimal: abre ventana, no más.
  - `frontend/` inicializado: `package.json`, `pnpm-lock.yaml`, `tsconfig.json` (strict + noUncheckedIndexedAccess), `svelte.config.js` (Svelte 5 runes), `vite.config.ts` (Tauri-aware), `tailwind.config.ts`, `postcss.config.js`, `.eslintrc.cjs`, `.prettierrc`, `index.html`.
  - `frontend/src/`: `main.ts`, `App.svelte` mostrando "zapx v0.0.1" centrado con Tailwind, `app.css` con directivas Tailwind.
  - Scripts en `frontend/package.json`: `dev`, `build`, `lint`, `type-check`, `test`.
  - `Cargo.toml` workspace members: `["crates/app"]`.
  - `pnpm install && pnpm dev` arranca; `cargo run -p app` abre la ventana.
  - PR: `feat(app,frontend): scaffold Tauri 2 + Svelte 5 + Tailwind`.

### T-08 — Crear los 8 crates core vacíos
- **Owner**: AI draft, Eng review
- **Effort**: 1 h
- **Depends on**: T-06
- **Done when**:
  - 8 crates creados como `lib`: `core-transport`, `core-terminal`, `core-session`, `core-persistence`, `core-vault`, `core-logging`, `core-highlight`, `core-config`.
  - Cada uno con: `Cargo.toml`, `README.md` (purpose / public API / non-goals — stubs), `src/lib.rs` con `#![forbid(unsafe_code)]`, `src/error.rs` con enum `Error` vacío (thiserror), `tests/.gitkeep`.
  - `core-transport/src/` además: `ssh.rs`, `telnet.rs`, `serial.rs`, `local_pty.rs` con un único `// scaffold` cada uno.
  - Crates hot-path con `benches/.gitkeep`: `core-transport`, `core-terminal`, `core-highlight`.
  - `Cargo.toml` workspace members actualizado.
  - `cargo build --workspace` pasa limpio.
  - PR: `feat(crates): scaffold 8 core crates`.

### T-09 — Crear xtask
- **Owner**: AI draft
- **Effort**: 45 min
- **Depends on**: T-06
- **Done when**:
  - `xtask/Cargo.toml` y `xtask/src/main.rs` con dispatcher de subcomandos.
  - Subcomandos stub (devuelven `unimplemented!()` con mensaje claro): `build`, `dev`, `package`, `release`, `gen-bindings`.
  - `.cargo/config.toml` con alias: `xtask = "run --package xtask --"`.
  - `cargo xtask --help` lista los subcomandos.
  - `Cargo.toml` workspace members incluye `xtask`.
  - PR: `feat(xtask): scaffold automation crate`.

### T-10 — GitHub Actions: CI matrix + housekeeping
- **Owner**: AI draft, Eng review
- **Effort**: 2 h (YAML es frágil, ajustar hasta que pase tres veces seguidas)
- **Depends on**: T-07, T-08, T-09
- **Done when**:
  - `.github/workflows/ci.yml` con matrix `ubuntu-latest` × `macos-latest` × `windows-latest`. Steps por job: checkout → setup rust (con cache) → setup node + pnpm → `cargo fmt --check` → `cargo clippy --workspace -- -D warnings` → `cargo nextest run --workspace` → `cargo deny check` → `pnpm install --frozen-lockfile` → `pnpm type-check` → `pnpm lint` → `pnpm test`.
  - `.github/workflows/security.yml` schedule semanal: `cargo deny check advisories` + `pnpm audit`.
  - `.github/dependabot.yml` (o `renovate.json`) para cargo + npm + github-actions.
  - `.github/PULL_REQUEST_TEMPLATE.md` con secciones: motivation, approach, test plan, follow-ups.
  - `.github/ISSUE_TEMPLATE/`: `bug_report.yml`, `feature_request.yml`, `config.yml` (disable blank issues, link a Discussions).
  - `.github/CODEOWNERS` (apuntando al engineer).
  - `.github/FUNDING.yml` (GitHub Sponsors + Open Collective placeholders).
  - Tres ejecuciones consecutivas verdes en los tres OS antes de mergear.
  - PR: `ci: cross-platform matrix + security workflow`.
  - **Branch protection actualizada**: required checks = los jobs de `ci.yml` (los 3 OS).

### T-11 — Drafts de README, CONTRIBUTING, SECURITY
- **Owner**: AI draft, Eng review
- **Effort**: 1.5 h
- **Depends on**: T-10 (badge de CI en README)
- **Done when**:
  - `README.md`: nombre + tagline + badge CI + estado (pre-alpha, no usable) + qué es + qué *no* es + screenshots placeholder + build instructions + licencia.
  - `CONTRIBUTING.md`: dev setup → `cargo xtask dev`, commit conventions (puntero a [style_guide.md](style_guide.md) §7), PR process, code review.
  - `SECURITY.md`: cómo reportar vulnerabilidades (email + GPG key TBD), scope (la app, no las redes que el usuario administra), no bug bounty (hobby project), expected response time.
  - PR: `docs: README/CONTRIBUTING/SECURITY initial drafts`.

### T-12 — Primer "hello world" desplegable (gate del Bloque 0)
- **Owner**: AI + Eng (pair)
- **Effort**: 1-2 h
- **Depends on**: T-07, T-10
- **Done when**:
  - `cargo xtask dev` abre la ventana mostrando `zapx v0.0.1`.
  - `cargo xtask build --release` produce el binario.
  - `cargo xtask package` produce un MSI **sin firmar** (suficiente para esta fase).
  - El MSI se instala y arranca en una máquina Windows 11 limpia (VM o equipo secundario).
  - Tag `v0.0.1` empujado al repo. Sin release de GitHub asociada — el binario aún no se distribuye.
  - PR: `feat: zapx v0.0.1 hello world`.
  - **Verificación cruzada de SLAs** (plan §5.2): medir tiempo de arranque en frío y RAM idle. Aunque no haya features, este número es la baseline contra la que vamos a comparar todo lo que añadamos.

---

## Grafo de dependencias

```
T-01 ──▶ T-02 ──┐
                ├──▶ T-06 ──┬──▶ T-07 ──┐
T-03 ───────────┘           │            │
T-04 ──▶ (input a T-07) ────┼──▶ T-08 ──┼──▶ T-10 ──▶ T-11
T-05 (independiente)        │            │
                            └──▶ T-09 ──┘            │
                                                     │
                                T-07, T-10 ──▶ T-12 ◀┘
```

Ruta crítica: **T-01 → T-02 → T-06 → T-07 → T-10 → T-12**. Aprox. 8-10 h de trabajo focalizado.

---

## Riesgos específicos del Bloque 0

| Riesgo | Mitigación |
|---|---|
| Tauri 2 + Svelte 5 + Tailwind 4 cambian rápido; ejemplos desactualizados. | Pin de versiones exactas en `Cargo.toml` y `package.json` desde T-07. No "latest" implícito. |
| MSVC Build Tools no instalados → `cargo build` falla opacamente al compilar `rusqlite` o `portable-pty`. | T-03 los instala explícitamente como sub-step, no como afterthought. |
| CI en macOS GitHub runners es lento y a veces flaky. | Mantenerlo como `required` desde el principio (ADR-009) pero documentar el patrón de re-run. Si flakea consistentemente, abrir un ADR específico. |
| Branch protection con required reviewers bloquea al solo dev. | Política inicial: required CI + require PR, sin required reviewers. La disciplina la pone el style guide §8, no el setting. |
| Renovate/Dependabot abre 50 PRs el primer día. | Agrupar updates (`groups:` en dependabot.yml) por ecosistema y por severity. |

---

## Lo que el Bloque 0 explícitamente NO hace

- Cero código de protocolo (SSH/Telnet/Serial) — Bloques 1-4.
- Cero schema SQLite — Bloque 3.
- Cero highlighting — Bloque 5.
- Cero UI real más allá del "hello world" — los bloques siguientes la van armando.
- Cero ADRs escritos. Los ADRs de §3.3 del plan se escriben *cuando se toma la decisión real* en el código, no como prosa post-hoc.
- Cero release pública. Hay tag `v0.0.1` para fijar la baseline, pero no entry en GitHub Releases.
- Cero firma del binario. EV cert se evalúa cuando lleguemos a 0.1.0 público (Bloque 8).

---

## Gate de cierre

El Bloque 0 está cerrado cuando, partiendo de una máquina limpia:

1. `git clone <repo>` funciona.
2. Las instrucciones del `CONTRIBUTING.md` llevan a `cargo xtask dev` sin pasos no documentados.
3. La ventana se abre y muestra `zapx v0.0.1`.
4. `git push` de un PR de prueba activa el CI matrix y queda verde en los tres OS.

Cumplidos los cuatro, pasamos al Bloque 1 (PoC del terminal local).
