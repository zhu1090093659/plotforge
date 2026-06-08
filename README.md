# PlotForge

PlotForge is an AI story game creation engine. This repository currently contains the CLI-first MVP slice from the PRD plus the first creator desktop workspace: Rust schema contracts, story craft review, declarative rules, mock runtime, trace persistence, media asset registry, job queue, fake image provider foundations, rebuildable SQLite cache/index, the `dynasty-embers` demo template, static web export, and a Vite/React/Tailwind Studio shell with a Tauri command bridge.

## Current Scope

- Rust workspace and CLI prototype.
- Folder project source of truth with TOML, JSON, and Markdown.
- Mock agent pipeline only; no external model calls are required.
- Static web export with prebaked scenes and placeholder PNG assets.
- No-network static player package sourced from `apps/player-web/static` and copied into exports.
- Export profiles and `ai-usage.json` package disclosure for static exports; these describe capabilities and AI usage evidence without legal or platform approval guarantees.
- Local-only Workshop item package schema and validator in `plotforge-workshop`; no Steam API, upload, or release-readiness integration is included.
- Media asset registry foundation with typed records, SHA-256 hashes, runtime references, provider metadata, and exportable paths.
- Job queue foundation with typed state, cancel/retry/timeout/progress, explicit failures, injected clock tests, and cost accounting.
- Fake image provider pipeline with scene background asset registration, job state, trace-visible placeholder fallback, and cache reuse.
- Rebuildable SQLite cache/index for project summaries, source file hashes, trace metadata, and asset metadata; folder files remain source of truth.
- Creator desktop Studio shell with Tauri-backed project load/check/source edit/playtest commands; real LLM/image providers and Steam/Workshop are deferred.

## Commands

```bash
cargo check --workspace
cargo test --workspace
cargo test -p plotforge-workshop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

npm run creator-desktop:dev
npm run creator-desktop:typecheck
npm run creator-desktop:test
npm run creator-desktop:build
npm run creator-desktop:smoke
npm run creator-desktop:tauri:check
npm run creator-desktop:tauri:dev
npm run creator-desktop:qa
npm run player-web:qa

cargo run -p plotforge-cli -- new demo --path examples/dynasty-embers --force
cargo run -p plotforge-cli -- check examples/dynasty-embers
cargo run -p plotforge-cli -- play examples/dynasty-embers --once
cargo run -p plotforge-cli -- trace inspect examples/dynasty-embers/traces/latest.json
cargo run -p plotforge-cli -- export static examples/dynasty-embers --out dist/dynasty-embers

scripts/contracts/export_contracts.sh
scripts/contracts/check_contracts.sh
scripts/qa/full_local.sh
python3 scripts/qa/creator_desktop_build_smoke.py
python3 scripts/qa/static_export_http_smoke.py --export-dir dist/dynasty-embers
```

## Architecture Rules

- `plotforge-schema` is the contract source of truth.
- Generated JSON Schema and TypeScript contracts live under `contracts/` and must be regenerated from Rust schema, not hand-maintained.
- AI or mock agents propose content; runtime and rule engine commit state.
- Folder project files are source of truth; caches must be rebuildable.
- SQLite cache/index lives under `.plotforge/cache.sqlite`; it is optional, rebuildable from folder state, and never overrides canonical project files.
- `plotforge-media` owns the media registry foundation: asset records, hashes, refs, provider metadata, path safety, and reachability.
- `plotforge-job` owns long-running job state transitions, progress, cancel/retry/timeout, failure, and cost accounting.
- `plotforge-agent` owns provider ports and image pipeline orchestration; image generation writes through `plotforge-media` and `plotforge-job`.
- `plotforge-export` packages only reachable referenced assets from the media registry.
- Export profiles and AI usage manifests are schema-defined disclosure artifacts; they must not include provider credentials, raw provider responses, private traces, legal guarantees, or platform approval promises.
- `plotforge-workshop` owns local Workshop draft package validation. It must not call Steamworks APIs, upload content, or claim release readiness.
- `apps/player-web/static` owns the exported static player surface and consumes `ExportManifest` without duplicating runtime/rule logic.
- Static exports must not include API keys, provider config, raw provider responses, or private traces.

## QA

The repeatable local gate is `scripts/qa/full_local.sh`. It includes Rust checks, contract drift checks, creator desktop typecheck/test/build/build-smoke/Tauri check, CLI smoke, and export smoke.

Local desktop interaction smoke is documented in `scripts/qa/computer_use_creator_desktop.md` for Creator Desktop and `scripts/qa/computer_use_static_export.md` for static export. Codex Desktop Browser/Computer Use verification is intentionally local-only and not part of GitHub Actions.
