# PlotForge

PlotForge is an AI story game creation engine. This repository currently contains the CLI-first MVP slice from the PRD plus the first creator desktop frontend workspace: Rust schema contracts, story craft review, declarative rules, mock runtime, trace persistence, the `dynasty-embers` demo template, static web export, and a Vite/React/Tailwind Studio shell.

## Current Scope

- Rust workspace and CLI prototype.
- Folder project source of truth with TOML, JSON, and Markdown.
- Mock agent pipeline only; no external model calls are required.
- Static web export with prebaked scenes and placeholder PNG assets.
- Creator desktop UI skeleton only; Tauri commands, real LLM/image providers, SQLite cache, and Steam/Workshop are deferred.

## Commands

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

npm run creator-desktop:dev
npm run creator-desktop:typecheck
npm run creator-desktop:test
npm run creator-desktop:build
npm run creator-desktop:qa

cargo run -p plotforge-cli -- new demo --path examples/dynasty-embers --force
cargo run -p plotforge-cli -- check examples/dynasty-embers
cargo run -p plotforge-cli -- play examples/dynasty-embers --once
cargo run -p plotforge-cli -- trace inspect examples/dynasty-embers/traces/latest.json
cargo run -p plotforge-cli -- export static examples/dynasty-embers --out dist/dynasty-embers

scripts/contracts/export_contracts.sh
scripts/contracts/check_contracts.sh
scripts/qa/full_local.sh
python3 scripts/qa/static_export_http_smoke.py --export-dir dist/dynasty-embers
```

## Architecture Rules

- `plotforge-schema` is the contract source of truth.
- Generated JSON Schema and TypeScript contracts live under `contracts/` and must be regenerated from Rust schema, not hand-maintained.
- AI or mock agents propose content; runtime and rule engine commit state.
- Folder project files are source of truth; caches must be rebuildable.
- Static exports must not include API keys, provider config, raw provider responses, or private traces.

## QA

The repeatable local gate is `scripts/qa/full_local.sh`. It includes Rust checks, contract drift checks, creator desktop typecheck/test/build, CLI smoke, and export smoke. Codex Desktop Computer Use verification is documented in `scripts/qa/computer_use_static_export.md`; it is intentionally local-only and not part of GitHub Actions.
