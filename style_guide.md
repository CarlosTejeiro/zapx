# Style Guide

This document is the contract every contributor — present and future, human and AI — agrees to before opening a pull request. It is not a tutorial. It encodes the standards that protect the project's stated goals (lightweight, fast, secure, maintainable, open) at the moments when willpower is low and shortcuts are tempting.

The rules are written as obligations, not suggestions. Each rule states the *why* in one line so that future contributors can judge edge cases instead of mechanically following the letter while violating the spirit.

---

## 1. Engineering principles

Five axioms, in strict priority order. When two rules elsewhere in this document seem to conflict, the higher-ranked principle wins.

1. **Correctness over speed of delivery.** Broken code delivered fast is still broken. We have no deadline; we have a reputation to build.
2. **Security is non-negotiable for credential-handling code.** A single leaked password from a tool marketed to network engineers ends the project as a serious option.
3. **Performance is a feature, not an afterthought.** The SLAs in the development plan (≤1.5 s cold start, ≤16 ms p99 keyboard latency, ≤80 MB idle RAM) are commitments to the user, not aspirations.
4. **Readability is a force multiplier.** The code is read far more than it is written, and half of this team has no memory between sessions. Optimize for the next reader.
5. **Simplicity is the default; complexity must be earned.** Every abstraction, dependency, and indirection must justify its own existence in the diff that introduces it.

---

## 2. Rust backend

### Safety
- `#![forbid(unsafe_code)]` is required at the top of every crate, except platform-adapter crates that genuinely need FFI. Memory safety is the largest single reason we chose Rust; we do not give it away by accident.
- Every `unsafe` block carries an inline `SAFETY:` comment that names the invariant being upheld. If you cannot name the invariant, you do not yet understand the code you are writing.
- Types that hold secrets (passwords, passphrases, key material) implement `Zeroize` and zero on drop. Leaving secrets in freed memory is a credential leak on a process crash dump.

### Error handling
- `thiserror` in library crates: typed, exhaustive error enums per crate. Library consumers must be able to match on errors without parsing strings.
- `anyhow` only in the application/binary layer. Library code never returns `anyhow::Error`; that decision belongs to the caller.
- `unwrap()` and `expect()` are forbidden on production code paths except where an inline comment documents the invariant that makes the panic unreachable. Tests are exempt.
- Errors carry context, not just messages. A wrapped error must explain *what we were trying to do*, not just *what went wrong*.
- Cross-crate error conversion happens at the boundary via `From`, never by stringifying.

### Type system
- Newtype pattern for every domain identifier (`SessionId`, `FolderId`, `CredentialRef`, etc.). Raw `u32`/`String` IDs are forbidden — mixing them at a call site is a bug we refuse to make possible.
- Make illegal states unrepresentable. Use enums and sum types; do not encode mutually exclusive options as parallel boolean flags.
- Prefer borrowed parameters (`&str`, `&[T]`) over owned ones in function signatures. Allocation is a decision, not a default.
- `#[non_exhaustive]` on public enums and structs whose shape may grow. We do not freeze the API the day we ship 0.1.0.

### Async
- Tokio is the only async runtime. We do not mix runtimes; doing so is a deadlock waiting to happen.
- Blocking calls inside `async fn` are forbidden. Wrap CPU-bound or genuinely blocking I/O with `spawn_blocking`. The reactor thread is a shared resource.
- Bounded channels by default. Unbounded channels are unbounded memory leaks; if you need one, justify it in the PR.
- Every `async fn` must be cancellation-safe — the future may be dropped mid-`await` at any point and the world must remain consistent.
- Never hold a lock across an `.await`. Acquire, mutate, drop; then await. This rule has no exceptions.

### API design
- The public surface of every crate is minimal. `pub(crate)` is the default; `pub` is a deliberate choice that survives review.
- Cross-crate communication happens through traits where it is reasonable. The `Transport` trait is the model: protocol-specific code never leaks across the boundary.
- Zero circular dependencies between crates. If you find yourself wanting one, the abstraction is wrong — fix the abstraction.
- Re-exports are curated. We do not `pub use crate::internal::*;` and call it API design.

### Linting and formatting
- `rustfmt` is law. The repository's formatting is the formatter's output, full stop.
- `cargo clippy -- -D warnings` runs in CI and blocks merge. Disagreeing with a lint is allowed; ignoring it silently is not. Suppressions are inline with a justification.
- `cargo deny` enforces the license policy and security advisory checks on every PR.

### Performance
- Measure before optimizing. Criterion benchmarks exist for the terminal hot path (input → emulation → render); changes that touch that path must show their impact.
- Avoid allocations on the keyboard-input-to-render path. The user feels every microsecond.
- Stream rather than buffer. Reading a 200 MB session log into a `Vec<u8>` is a defect, not an implementation.
- Profile release builds. Debug profiles have hot spots that do not exist in release, and vice versa.

---

## 3. Frontend (Svelte 5 + TypeScript + Tailwind)

### TypeScript
- `strict: true` plus `noUncheckedIndexedAccess` are mandatory tsconfig settings. We use TypeScript so the compiler does work for us; weakening it removes the reason it exists.
- `unknown` is preferred over `any`. An `any` that survives review must carry a one-line comment explaining why no better type was available.
- Backend types come from the Tauri-generated bindings. Hand-redeclaring a Rust type in TypeScript creates two sources of truth, and only one of them is compile-checked against reality.

### Svelte 5
- Runes (`$state`, `$derived`, `$effect`) are the only reactivity primitive in new code. Legacy stores are forbidden except inside a documented migration path.
- One responsibility per component. A component that owns both `SessionTree` state and an unrelated dialog is two components in a trenchcoat.
- Props are typed. Untyped props are a runtime bug delivered to the user.
- `$effect` is reserved for side effects (DOM, subscriptions, IPC). Never use `$effect` to derive a value — `$derived` exists for that and is correctly scheduled.

### Styling
- Tailwind utility-first. Custom CSS is permitted only when Tailwind genuinely cannot express what is needed; that case is rarer than it feels.
- Design tokens (colors, spacing, fonts) are defined once in the Tailwind config. Hardcoded `#FF4422` in a component is technical debt by the time it is committed.
- Theme switching is implemented through class scoping, not duplicated rule sets. We support light/dark and per-session themes; we cannot afford to maintain three CSS trees.
- Inline styles are restricted to genuinely dynamic values (positioning, animated transforms). Static inline styles bypass the design system.

### Accessibility
- Keyboard navigation is required wherever mouse interaction exists. Network engineers live on keyboards, and accessibility is not optional in modern software.
- Icon-only buttons carry an ARIA label. Screen readers and keyboard users do not see the icon.
- Focus management for modals and dialogs is explicit. Trap focus on open, restore on close. Anything less is broken.
- Color contrast meets WCAG AA at minimum. Themes that fail the check do not ship.
- No essential functionality is gated behind hover-only interactions. Touch devices and keyboard users still exist.

### Performance
- Lazy-load heavy modules — xterm.js, settings dialogs, theme editors. Cold-start budget is 1.5 s; a 600 KB editor that nobody opens does not get to fire on launch.
- The terminal byte stream is subscribed only by the component that renders the terminal. Spreading the subscription is how you turn a 200 KB/s stream into a 200 KB/s memory leak.
- `$effect` chains that retrigger each other are bugs. If two effects fire each other, one of them is the wrong primitive.

### Tauri bridge
- Commands are typed on both sides. The Rust signature and the TS binding agree, or the build fails.
- Untrusted strings are never `JSON.stringify`'d into a command argument verbatim. Validate, then send.
- Terminal stream payloads are small and frequent (≤16 ms cadence). Batching beyond the frame budget shows up as input lag.

---

## 4. Security

- Credentials are never logged, never embedded in error messages, never stored as plaintext in SQLite. Only opaque keyring references live in the database. A grep for a password in our log files must return nothing.
- `Debug` is not derived on types that hold secrets. Implement it manually and redact the field. The default `Debug` impl will print a password to a panic message someday otherwise.
- `Zeroize` is implemented for every secret-holding type, and the type drops via `ZeroizeOnDrop` or equivalent. Memory dumps must not yield credentials.
- All input crossing a trust boundary (network, disk, IPC, user input) is validated before use. The defensive perimeter is at the boundary, not scattered throughout the code.
- `unsafe` Rust never touches untrusted data. Parsing wire formats stays in safe code. There are no exceptions to this rule.
- Cryptography crates are pinned to exact versions. We read the changelog before bumping. A silent crypto bump is how key derivation parameters change without anyone noticing.
- Dependency audits (`cargo deny`, `npm audit`) run in CI on every PR. A new advisory blocks the merge.

---

## 5. Testing

- Unit tests live next to the code they test (`#[cfg(test)] mod tests` for Rust). Integration tests live in `tests/`. Distance from the code is correlated with neglect.
- Determinism is mandatory. A flaky test is a defect with the same severity as a flaky feature, and it blocks CI until fixed or quarantined with a reason.
- Unit tests do not touch the network or the disk. Inject `Transport` and filesystem dependencies through traits and use fakes. A test that depends on the network depends on someone else's uptime.
- Integration tests may touch disk via tempdir and may bind to localhost. They may not call external services. The CI pipeline must remain runnable offline.
- Tests assert behavior through the public API. Tests that lock the implementation down to its current internals are change-resistance, not safety.
- Every bug fix lands together with a regression test that failed before the fix. Untested fixes regress.
- Snapshot tests are used for structurally complex output: terminal grid state after a sequence of escape codes, parsed device configurations, rendered logs. Eyeballing a `Vec<Cell>` in an assertion is how subtle bugs survive review.
- `cargo nextest` is the test runner. Faster cycles mean more frequent runs.

---

## 6. Documentation

- Every public item in a published crate has a rustdoc comment. `# Examples` and `# Errors` sections appear wherever they apply. The compiler runs the doctests, which is why we write them.
- Architecture Decision Records (ADRs) live in `docs/adr/`. Any decision a future contributor (including future-us) would have to reverse-engineer "why?" from the diff gets an ADR.
- Code comments explain *why*, never *what*. The identifiers, types, and structure cover the what; if they do not, fix the code, not the comment.
- Commit hashes, PR numbers, and issue links do not appear in code comments. Those references rot; git blame and the PR history are the durable record.
- Each crate has a `README.md` stating its purpose, its public API surface, and its explicit non-goals. Non-goals prevent scope creep more reliably than goals enforce focus.

---

## 7. Git workflow

- Conventional Commits are mandatory: `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`, `build:`, `ci:`. Machine-readable history is worth more than informal prose.
- One logical change per commit. If a commit cleanly splits into two, it must be split. Reviewers cannot review entanglements; bisect cannot resolve them.
- Commit messages explain *why* the change is being made. The diff already shows what changed.
- Branch names follow `feat/<slug>`, `fix/<slug>`, `chore/<slug>`. Predictable names make tooling and review faster.
- Pull requests stay small — ~400 lines of diff is the soft cap. Larger PRs require explicit justification in the description; a 2,000-line PR will not get the review it deserves.
- PR descriptions include motivation, approach summary, test plan, and known follow-ups. The PR is the durable record of the decision.
- `main` is never force-pushed. PRs squash-merge into `main`; long-lived branches may preserve history when useful for archaeology.
- `--no-verify` is forbidden. If a hook fails, the cause gets fixed, not bypassed.

---

## 8. Code review

- Authors self-review the diff before requesting review. The first pass of reviewer attention should not be spent on things the author would have caught reading their own work.
- Reviewers apply the priority order: correctness → security → performance → tests → readability. A perfect comment style on incorrect code is not a passing review.
- Reviewers suggest; authors decide. Disagreements escalate explicitly — through a follow-up comment, a sync conversation, or an ADR — and are never silently dropped.
- `Nit:` prefixes mark comments the reviewer is not blocking on. Everything else is a blocker until resolved.
- LGTM means "I read every line and would ship this myself." It is not a rubber stamp, and approving without reading is a form of negligence.

---

## 9. CI/CD

- Every PR runs the full cross-platform matrix (Windows, macOS, Linux) from day one, even though Phase 1 only ships Windows binaries. Cross-platform issues caught late are 10x harder to fix.
- The required green checks are `fmt`, `clippy -D warnings`, `test`, `cargo deny`, frontend `tsc`, frontend lint, and frontend tests. None of these are optional.
- Red CI blocks merge. There is no "I'll fix CI in a follow-up" path.
- Releases are reproducible: a tag is the only trigger that produces a published artifact. Manual builds from a developer machine are not releases.
- CI secrets come from GitHub Secrets. They are never committed, never echoed into logs, never embedded in workflow files.

---

## 10. Dependencies

- Adding a dependency requires a one-line justification in the PR description. "It was easier" is not a justification.
- Preferred profile for new dependencies: actively maintained, meaningful download volume or clearly unique value, MIT- or Apache-licensed. Anything outside that envelope is a deliberate choice that survives review.
- Major versions are pinned in `Cargo.toml`; minor and patch bumps are tracked by `Cargo.lock` and managed via Renovate or Dependabot. Surprise major bumps are a documented incident, not a routine event.
- A new direct dependency triggers a glance at its transitive tree. We do not add 40 transitive crates to get one utility function.
- Unused dependencies are removed. `cargo udeps` runs at least quarterly; lingering deps are attack surface and build time.

---

## 11. Naming

- Rust uses idiomatic conventions: `snake_case` for functions, methods, modules, and crates; `CamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants. Deviation costs reviewer attention forever.
- TypeScript uses `camelCase` for values, `PascalCase` for types and components, `SCREAMING_SNAKE_CASE` for constants. The conventions are not negotiable.
- Filenames: Rust `snake_case.rs`, Svelte components `PascalCase.svelte`, TypeScript modules `kebab-case.ts`. Filesystem grep ergonomics matter.
- Abbreviations are used only where they are industry standard (`cfg`, `id`, `db`, `url`). Invented abbreviations (`usr`, `cnctn`, `sssn`) are forbidden — they cost every reader.
- Booleans read as questions: `is_connected`, `has_credentials`, `should_retry`. Negative names (`is_not_ready`) are forbidden because they invert under boolean logic.

---

## 12. What we explicitly do not do

- We do not commit magic numbers. Named constants always — the name is the documentation.
- We do not commit `TODO` without a tracking issue or a dated removal condition. Naked TODOs become permanent residents.
- We do not commit commented-out code. Git history is the archive; the codebase is the present.
- We do not write backwards-compatibility shims before 1.0. The project has zero users we must not break; breaking changes are free and we should use that freedom.
- We do not write premature abstractions. The rule of three applies: extract on the third occurrence, not the first.
- We do not gate code behind feature flags it does not need. A flag with one possible value is dead weight.
- We do not silently swallow errors or fall back. Failures at boundaries are loud. Silent fallbacks are how production bugs masquerade as features for months.
