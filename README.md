# PlotForge

PlotForge is an AI story game creation engine. This repository currently contains the CLI-first MVP slice from the PRD plus the first creator desktop workspace: Rust schema contracts, story craft review, declarative rules, mock runtime, trace persistence, media asset registry, job queue, fake image provider foundations, rebuildable SQLite cache/index, the `dynasty-embers` demo template, static web export, and a Vite/React/Tailwind Studio shell with a Tauri command bridge.

## Current Scope

- Rust workspace and CLI prototype.
- Folder project source of truth with TOML, JSON, and Markdown.
- Mock agent pipeline only; no external model calls are required.
- Static web export with prebaked scenes and placeholder PNG assets.
- Export profile listing plus a local Desktop Runtime draft package with `desktop-runtime-draft.json`, build notes, static player files, reachable assets, and package hashes.
- No-network static player package sourced from `apps/player-web/static` and copied into exports.
- Export profiles and `ai-usage.json` package disclosure for static and desktop draft exports; these describe capabilities and AI usage evidence without legal conclusions or platform approval promises.
- Local-only Workshop item package schema, validator, library import/load/remix/block/report/delete flow, publish draft writer, and gated upload port in `plotforge-workshop`; no Steam API call is made by default.
- Local Steam Submission Kit draft generation in `plotforge-workshop`; it produces store copy, checklist, AI disclosure, content warnings, asset reference, Steam Direct checklist, content safety, and packaging-note Markdown drafts from validated package evidence.
- Creator desktop Play/Creator/Developer product mode shell for the Steam route, plus thin Studio adapters over Workshop package and draft operations.
- Steam compliance QA guidance and static no-launch-promise lint for active project-facing docs.
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
python3 scripts/qa/no_launch_promise_lint.py

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
cargo run -p plotforge-cli -- export profiles
cargo run -p plotforge-cli -- export static examples/dynasty-embers --out dist/dynasty-embers
cargo run -p plotforge-cli -- export desktop examples/dynasty-embers --out dist/dynasty-embers-desktop
cargo test -p plotforge-cli --test cli_smoke cli_runs_workshop_local_flow_in_tempdir

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
- Desktop Runtime draft export is local package evidence only; it emits build notes and hashes but does not create an installer, bundle credentials, include private traces, or claim platform readiness.
- Export profiles and AI usage manifests are schema-defined disclosure artifacts; they must not include provider credentials, raw provider responses, private traces, legal conclusions, or platform approval promises.
- `plotforge-workshop` owns local Workshop draft package validation, local library operations, publish draft generation, and the gated Steamworks upload port. It must not call Steamworks APIs by default or claim external platform outcomes.
- `plotforge-workshop` also owns Steam Submission Kit draft generation. Generated drafts are local support material only; they do not decide compliance, approval, publishing, or legal status.
- Steam-facing docs and generated guidance must pass `scripts/qa/no_launch_promise_lint.py`; active docs must not promise automatic publishing, platform outcomes, legal conclusions, or ownership of a creator's Steamworks workflow.
- `apps/player-web/static` owns the exported static player surface and consumes `ExportManifest` without duplicating runtime/rule logic.
- Static exports must not include API keys, provider config, raw provider responses, or private traces.

## QA

The repeatable local gate is `scripts/qa/full_local.sh`. It includes Rust checks, contract drift checks, creator desktop typecheck/test/build/build-smoke/Tauri check, CLI smoke, static export smoke, desktop draft export smoke, and unpacked HTTP export smoke.

Local desktop interaction smoke is documented in `scripts/qa/computer_use_creator_desktop.md` for Creator Desktop and `scripts/qa/computer_use_static_export.md` for static export. Codex Desktop Browser/Computer Use verification is intentionally local-only and not part of GitHub Actions.
