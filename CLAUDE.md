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
cargo run -p plotforge-cli -- new demo --path "$tmp/dynasty-embers" --force
cargo run -p plotforge-cli -- check "$tmp/dynasty-embers"
cargo run -p plotforge-cli -- play "$tmp/dynasty-embers" --once
cargo run -p plotforge-cli -- trace inspect "$tmp/dynasty-embers/traces/latest.json"
cargo run -p plotforge-cli -- export static "$tmp/dynasty-embers" --out "$tmp/export"
```

### Fixture Validation

```bash
cargo run -p plotforge-cli -- check examples/dynasty-embers
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
- **CLI** orchestrates crates; must not own business logic

See `AGENTS.md` for the complete set of architecture boundaries, testing policy, error handling rules, and change discipline.

## Workspace Layout

- **Cargo workspace**: 12 crates under `crates/`
- **npm workspaces**: `apps/creator-desktop`, `apps/player-web`
- **Tauri app**: `apps/creator-desktop/src-tauri` (excluded from Cargo workspace)
- **Generated contracts**: `contracts/plotforge.d.ts`, `contracts/plotforge.schema.json`
- **Demo fixture**: `examples/dynasty-embers` (read-only in tests)
- **UI design reference**: `docs/design/ui-mockups/agent-native-v1/`

## Testing Patterns

- Rust: unit tests co-located in crates, CLI black-box tests under `crates/plotforge-cli/tests/`
- TypeScript: `*.test.ts(x)` files alongside source, Vitest + jsdom
- All tests must use temp directories for generated output; never mutate committed fixtures
- Every feature/fix requires tests in the same change

## CI Jobs (GitHub Actions)

Triggers on `main` and `mvp/**` branches: rust-static, rust-unit, creator-desktop, cli-smoke, export-smoke, player-web.
