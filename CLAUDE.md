# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Read `AGENTS.md` first.** It is the shared, authoritative project-level instruction source for all coding agents (Codex, Cursor, Claude Code, and others), covering project goal, source-of-truth rules, architecture boundaries, testing policy, required validation, CI/QA, error handling, and change discipline. This file is a Claude Code quick-reference summary plus Claude Code-specific reminders; it must not duplicate or contradict `AGENTS.md`. Where this summary and `AGENTS.md` disagree, `AGENTS.md` wins.

## Project Summary

PlotForge is a CLI-first Rust AI story game engine with a React/Tauri creator desktop app and a static web player. Folder-based projects (TOML/JSON/Markdown) are the source of truth; SQLite is a rebuildable cache only.

## Commands

### Rust

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p plotforge-runtime           # single crate
cargo test -p plotforge-cli -- test_name  # single test
```

### Frontend (Creator Desktop)

```bash
npm run creator-desktop:dev             # Vite dev server
npm run creator-desktop:typecheck       # tsc
npm run creator-desktop:test            # Vitest
npm run creator-desktop:build           # production build
npm run creator-desktop:qa              # full QA (typecheck + test + build + tauri check + smoke)
npm run creator-desktop:tauri:dev       # Tauri dev mode
```

### Frontend (Player Web)

```bash
npm run player-web:test
npm run player-web:qa
```

### Contracts

```bash
scripts/contracts/export_contracts.sh   # regenerate TypeScript from Rust schema
scripts/contracts/check_contracts.sh    # verify contracts are up-to-date
```

### Full QA Gate

```bash
scripts/qa/full_local.sh               # must pass before merging broad changes
```

### CLI Smoke (use temp dirs, never mutate committed fixtures)

```bash
tmp="$(mktemp -d)"
cargo run -p plotforge-cli -- new project --path "$tmp/starter-project" --force --concept "A local starter project." --visual-style "clear readable test style" --initial-scene "A creator opens a fresh PlotForge project."
cargo run -p plotforge-cli -- check "$tmp/starter-project"
cargo run -p plotforge-cli -- play "$tmp/starter-project" --once
cargo run -p plotforge-cli -- trace inspect "$tmp/starter-project/traces/latest.json"
cargo run -p plotforge-cli -- export static "$tmp/starter-project" --out "$tmp/export"
python3 scripts/qa/static_export_http_smoke.py --export-dir "$tmp/export"  # player-module whitelist + no network URLs
```

### Fixture Validation

```bash
No committed default project fixture is expected.
```

## Architecture

### Crate Dependency Flow

```
plotforge-schema (contracts, source of truth)
    ↓
plotforge-storage (folder I/O + SQLite cache)
    ↓
plotforge-rule (evaluation engine)
    ↓
plotforge-runtime (scene/beat state machine)
    ↓
plotforge-agent (provider ports, scene planning)
    ↓
plotforge-media (asset registry, hashes)
plotforge-job (task state, retry, cost)
plotforge-storycraft (narrative review)
plotforge-export (static/desktop packaging)
plotforge-workshop (local Workshop validation, Steam Kit drafts)
    ↓
plotforge-studio (Tauri command adapters)
plotforge-cli (orchestration only, no business logic)
```

### Frontend ↔ Rust Bridge

1. `plotforge-schema` defines Rust structs with `#[derive(JsonSchema)]`
2. `scripts/contracts/export_contracts.sh` generates `contracts/plotforge.d.ts`
3. Creator Desktop imports types from `contracts/plotforge.d.ts`
4. In dev: custom Vite plugin (`plotforgeStudioApi`) spawns `cargo studio` commands
5. In production: Tauri IPC via `plotforge-studio` → `apps/creator-desktop/src-tauri`

### Key Boundaries

- **Schema** is single source of truth; regenerate contracts after Rust schema changes
- **Storage** owns file I/O; cache never required for state loading
- **Runtime** owns state transitions; agents propose, runtime/rules commit
- **Export** copies only whitelisted player files + referenced assets (no secrets, no traces)
- **Workshop/Steam** is local-only draft validation; no upload, no compliance claims
- **Creator Desktop** is a UI adapter; must not reimplement business logic in TypeScript
- **CLI** orchestrates crates; must not own business logic. Output formatting (i18n chrome, colorized summaries, report rendering) lives in `crates/plotforge-cli/src/cli_output.rs`; `main.rs` only dispatches and orchestrates handlers. Interactive wizards (e.g. `workshop submission-kit`) default to `dialoguer` prompts; `--batch` enables full flag-driven input for scripts/tests.

See `AGENTS.md` for the complete set of architecture boundaries, testing policy, error handling rules, and change discipline.

## Workspace Layout

- **Cargo workspace**: 12 crates under `crates/`
- **npm workspaces**: `apps/creator-desktop`, `apps/player-web`
- **Tauri app**: `apps/creator-desktop/src-tauri` (excluded from Cargo workspace)
- **Generated contracts**: `contracts/plotforge.d.ts`, `contracts/plotforge.schema.json`
- **Project fixtures**: no default project fixture is committed; use tempdir starter projects for smoke flows
- **UI design reference**: `docs/design/ui-mockups/agent-native-v1/`

## Testing Patterns

- Rust: unit tests co-located in crates, CLI black-box tests under `crates/plotforge-cli/tests/`
- TypeScript: `*.test.ts(x)` files alongside source, Vitest + jsdom
- Creator Desktop: one `*View.tsx` per workspace region with co-located behavior-based tests (assert via role/accessible-name/text, not className); `npm run creator-desktop:test` verifies a single View, `npm run creator-desktop:qa` is the full gate
- All tests must use temp directories for generated output; never mutate committed fixtures
- Every feature/fix requires tests in the same change

## CI Jobs (GitHub Actions)

Triggers on `main` and `mvp/**` branches: rust-static, rust-unit, creator-desktop, cli-smoke, export-smoke, player-web.
