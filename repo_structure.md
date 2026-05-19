# Repository Structure

## Purpose

This document is the map of the repository. It exists so that a contributor — human or AI, present or future — can locate any concern in under sixty seconds and know where new code belongs without guessing. It is updated whenever a top-level directory is added, removed, or repurposed. It is not updated when files inside an existing directory change; the directory's stated purpose covers that.

The companion documents are `plan_desarrollo_v2.md` (what we are building and why) and `style_guide.md` (how we write the code). This document only addresses *where things live*.

## Top-level rationale

**Workspace of crates, not a single crate.** Each domain (transport, terminal emulation, persistence, vault, logging, highlighting, configuration, session orchestration) lives in its own crate under `crates/`. The boundary is enforced by Cargo: a crate that needs another crate's internal must redesign the API rather than reach across the line. `pub(crate)` is the default; `pub` is a deliberate decision.

**Frontend separated from the Tauri crate.** The Rust binary lives in `crates/app/`; the Svelte 5 + TypeScript world lives in `/frontend/`. The Cargo workspace stays pure Rust and the JS toolchain stays pure JS. Tauri's `frontendDist` points across the boundary at build time. Either side can be developed, linted, and tested without invoking the other's toolchain.

**Build and release automation as a Rust crate (`xtask/`).** Cross-platform support is a Phase-1 commitment (ADR-009). Bash scripts do not run on Windows and PowerShell scripts do not run on Linux without translation, and maintaining two copies of every script is how they diverge. `xtask` is a regular Rust crate invoked as `cargo xtask <subcommand>`; it compiles, tests, and refactors like the rest of the code.

## The tree

```
rust_terminal/
│
├── Cargo.toml                          # Workspace root; declares all members under crates/* and xtask
├── Cargo.lock                          # Committed — we ship a binary, lockfile reproducibility matters
├── rust-toolchain.toml                 # Pins the exact stable Rust version used by CI and developers
├── rustfmt.toml                        # Formatting policy; rustfmt is law (style guide §2)
├── clippy.toml                         # Project-wide clippy thresholds and lint configuration
├── deny.toml                           # cargo-deny: license allowlist, advisory bans, source allowlist
├── .editorconfig                       # Editor-agnostic indent/charset/EOL defaults
├── .gitignore                          # target/, node_modules/, dist/, local OS files, secrets
├── .gitattributes                      # Line-ending normalization, binary marks, linguist hints
├── .markdownlint.yaml                  # Markdown lint rules so docs stay readable
│
├── README.md                           # Public face: tagline, screenshots, install, build, contribute
├── LICENSE                             # Apache 2.0 canonical text
├── NOTICE                              # Apache 2.0 third-party attributions
├── CONTRIBUTING.md                     # How to propose changes, run the dev loop, file issues
├── CODE_OF_CONDUCT.md                  # Contributor Covenant
├── SECURITY.md                         # Vulnerability disclosure policy and contact
├── CHANGELOG.md                        # Keep a Changelog format; updated per release
│
├── plan_desarrollo_v2.md               # The strategic plan (what and why)
├── style_guide.md                      # The contract for how we write code (how)
├── repo_structure.md                   # This document (where)
│
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                      # fmt + clippy -D warnings + test + cargo deny on every PR, matrix Win/macOS/Linux
│   │   ├── release.yml                 # Tag-triggered: build, sign, publish artifacts to GitHub Releases
│   │   └── security.yml                # Scheduled cargo-deny advisory scan and npm audit
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.yml              # Structured bug template with repro fields
│   │   ├── feature_request.yml         # Feature request template aligned with backlog discipline
│   │   └── config.yml                  # Disables blank issues; points users to Discussions
│   ├── PULL_REQUEST_TEMPLATE.md        # Forces motivation / approach / test plan / follow-ups
│   ├── CODEOWNERS                      # Review routing
│   ├── FUNDING.yml                     # GitHub Sponsors + Open Collective links
│   └── dependabot.yml                  # Renovate alternative; controls minor/patch bump cadence
│
├── crates/                             # Cargo workspace members — Rust only
│   │
│   ├── core-transport/                 # `Transport` trait + per-protocol implementations
│   │   ├── Cargo.toml
│   │   ├── README.md                   # Purpose, public API, explicit non-goals
│   │   ├── src/
│   │   │   ├── lib.rs                  # `Transport` trait + curated re-exports
│   │   │   ├── error.rs                # TransportError enum (thiserror)
│   │   │   ├── ssh.rs                  # russh-backed SSH client
│   │   │   ├── telnet.rs               # RFC 854 + NAWS + ECHO + SGA + TTYPE, optional TLS
│   │   │   ├── serial.rs               # tokio-serial wrapper, COM/tty cross-platform
│   │   │   └── local_pty.rs            # portable-pty wrapper (ConPTY on Windows)
│   │   ├── tests/                      # Integration tests — localhost + tempdir only
│   │   └── benches/                    # Criterion benchmarks: throughput, connect latency
│   │
│   ├── core-terminal/                  # wezterm-term wrapper, grid state, resize handling
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   ├── tests/
│   │   └── benches/                    # VT parser throughput; on the input→render hot path
│   │
│   ├── core-session/                   # Session lifecycle, state machine, protocol-agnostic
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   └── tests/
│   │
│   ├── core-persistence/               # SQLite via rusqlite (bundled) + at-rest encryption
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── migrations/                 # Numbered, idempotent SQL migrations (ADR-005)
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   └── tests/
│   │
│   ├── core-vault/                     # Credential storage: keyring + DPAPI; Zeroize-aware types
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   └── tests/
│   │
│   ├── core-logging/                   # Session loggers: rotation, search, format adapters
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   └── tests/
│   │
│   ├── core-highlight/                 # Regex highlighting engine + scope resolution (ADR-006)
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   ├── tests/
│   │   └── benches/                    # Rule evaluation throughput on representative output
│   │
│   ├── core-config/                    # Settings, themes, profiles, schema versioning
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── src/lib.rs
│   │   ├── src/error.rs
│   │   └── tests/
│   │
│   └── app/                            # Tauri 2 binary; frontend lives in /frontend (not here)
│       ├── Cargo.toml
│       ├── README.md
│       ├── build.rs                    # Tauri build helper
│       ├── tauri.conf.json             # frontendDist points to ../../frontend/dist
│       ├── capabilities/               # Tauri 2 capability files (granular permissions)
│       │   ├── default.json
│       │   └── desktop.json
│       ├── icons/                      # PNG/ICO/ICNS application icons
│       ├── src/
│       │   ├── main.rs                 # Entry point: builder, runtime setup, window
│       │   ├── lib.rs                  # Library shape, exposed for integration tests
│       │   ├── error.rs                # AppError; serde-serializable for the bridge
│       │   ├── state.rs                # Shared Tauri State (DI for core crates)
│       │   ├── events.rs               # Event emitters: terminal stream → frontend
│       │   ├── menu.rs                 # Native menu + tray (Windows initially)
│       │   └── commands/               # #[tauri::command] handlers grouped by domain
│       │       ├── mod.rs
│       │       ├── sessions.rs         # open, close, list, update
│       │       ├── folders.rs          # CRUD + drag-drop reorder
│       │       ├── vault.rs            # credential refs only — never plaintext
│       │       ├── highlight.rs        # rules CRUD, regex tester
│       │       └── settings.rs        # global settings + theme selection
│       └── tests/                      # End-to-end integration through the command surface
│
├── frontend/                           # Svelte 5 + TypeScript + Tailwind + xterm.js
│   ├── package.json                    # pnpm-managed; Tauri scripts wired here
│   ├── pnpm-lock.yaml                  # Committed
│   ├── tsconfig.json                   # strict + noUncheckedIndexedAccess (style guide §3)
│   ├── svelte.config.js                # Svelte 5 + runes
│   ├── vite.config.ts                  # Tauri-aware Vite configuration
│   ├── tailwind.config.ts              # Design tokens defined once (style guide §3)
│   ├── postcss.config.js
│   ├── .eslintrc.cjs                   # Or flat eslint.config.js — single source of truth
│   ├── .prettierrc                     # Formatter for TS/Svelte/CSS
│   ├── index.html                      # Vite entry HTML
│   ├── README.md                       # Frontend dev loop and conventions
│   │
│   ├── src/
│   │   ├── main.ts                     # Bootstrap: mount App, install global stores
│   │   ├── App.svelte                  # Root component: layout shell
│   │   ├── app.css                     # Tailwind directives + base resets
│   │   │
│   │   ├── lib/                        # Reusable internal modules; imported as $lib/*
│   │   │   ├── bridge/                 # Single point of contact with Rust
│   │   │   │   ├── commands.ts         # Typed invoke wrappers per backend command
│   │   │   │   ├── events.ts           # Typed event subscribers (terminal stream, etc.)
│   │   │   │   └── types.ts            # Generated from Rust — never hand-edited
│   │   │   │
│   │   │   ├── components/             # UI building blocks (PascalCase folders + .svelte)
│   │   │   │   ├── SessionTree/        # Folder tree with drag-drop
│   │   │   │   ├── TerminalPane/       # xterm.js wrapper; the only stream subscriber
│   │   │   │   ├── TabBar/             # Tab management
│   │   │   │   ├── QuickConnect/       # Keyboard-first connection launcher
│   │   │   │   ├── Settings/           # Settings dialog and sub-panels
│   │   │   │   └── HighlightRulesEditor/  # Rules CRUD with live regex tester
│   │   │   │
│   │   │   ├── stores/                 # Runes-backed reactive state ($state, $derived)
│   │   │   ├── themes/                 # Theme metadata client-side (bundled JSON consumed here)
│   │   │   ├── i18n/                   # Translations; English seed, others via community
│   │   │   └── utils/                  # Pure helpers; no side effects
│   │   │
│   │   └── routes/                     # Composition layer — pages/views; folder per route
│   │
│   ├── public/                         # Static assets served as-is
│   │   └── favicon.svg
│   │
│   └── tests/
│       ├── unit/                       # Vitest unit tests for lib/ utilities and stores
│       └── e2e/                        # Playwright through Tauri (offline only)
│
├── xtask/                              # Build/release automation as Rust; cross-platform by default
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                     # Dispatcher: cargo xtask <subcommand>
│       ├── build.rs                    # cargo xtask build [--release]
│       ├── dev.rs                      # cargo xtask dev — orchestrates Vite + Tauri together
│       ├── package.rs                  # cargo xtask package — MSI, MSIX (later: dmg, AppImage)
│       ├── release.rs                  # cargo xtask release <version> — full release pipeline
│       └── gen_bindings.rs             # cargo xtask gen-bindings — Rust types → TS types
│
├── docs/                               # Project documentation that is not user-facing app content
│   ├── adr/                            # Architecture Decision Records
│   │   ├── 0000-template.md            # ADR template
│   │   ├── 0001-backend-rust-nativo.md
│   │   ├── 0002-frontend-webview-xterm.md
│   │   ├── 0003-comms-events-and-commands.md
│   │   ├── 0004-sqlite-with-app-encryption.md
│   │   ├── 0005-schema-versioned-migrations.md
│   │   ├── 0006-highlight-post-emulation.md
│   │   ├── 0007-no-out-of-scope-issues.md
│   │   ├── 0008-app-logs-tracing-json.md
│   │   ├── 0009-cross-platform-ci-from-day-one.md
│   │   └── 0010-forbid-unsafe-code.md
│   ├── architecture/                   # Diagrams and deep-dives (block diagrams, sequence diagrams)
│   ├── user-guide/                     # End-user documentation; later powers the public website
│   ├── developer-guide/                # Onboarding, build, debug, release runbook
│   └── backlog.md                      # ADR-007 disciplined backlog of out-of-scope ideas
│
└── assets/                             # Bundled at build time — anything the binary ships with
    ├── fonts/
    │   ├── CascadiaCode/               # Default monospace font, with license file
    │   └── JetBrainsMono/              # Secondary monospace font, with license file
    ├── themes/                         # 10 built-in theme JSON files (One Dark, Solarized, etc.)
    └── highlight-rules/                # Default rule packs: Cisco IOS, Junos, generic IP/MAC/VLAN
```

## Conventions

- File and module naming follow `style_guide.md` §11. Filenames are not free expression.
- Every crate carries its own `README.md` and its own `error.rs`. Style guide §6 and §2 explain why.
- Hot-path crates carry `benches/` (criterion). Cold-path crates do not — benchmarks that never run are noise.
- Per-crate integration tests live in `tests/`; unit tests live inside `src/` next to the code they exercise.
- Documentation that explains *why* lives in `docs/adr/`. Documentation that explains *what* lives in code as rustdoc.

## What lives outside this repository

- **User credentials, OAuth tokens, SSH keys**: stored in the OS keyring (Windows Credential Manager / macOS Keychain / Linux Secret Service) via the `keyring` crate. Never in the repo, never in SQLite, never in error messages.
- **User session logs, vault databases, user themes, user-defined highlight rules**: in the OS-specific app data directory, resolved at runtime through the `directories` crate. Never inside the repo tree.
- **Downloaded fonts, plugins, themes from a future marketplace**: same — app data directory.
- **Release binaries and installers**: produced by `release.yml`, attached to GitHub Releases, never committed.
- **Telemetry endpoints, crash report destinations**: there are none yet. If they appear, their URLs and policies live in `SECURITY.md` and `docs/architecture/`, never hardcoded.
- **Per-developer scratch work, IDE configs, local environment files**: `.gitignore` keeps them out. Personal preferences do not enter shared history.
