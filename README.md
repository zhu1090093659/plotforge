# PlotForge

Someone has a game in their head.

Not a pitch deck. Not a market segment. A place.

A street after rain. A ship that never reaches shore. A house where the dead still answer. A kingdom, a kitchen, a battlefield, a school hallway at three in the afternoon. The creator can see it. They can feel the rules of it. They know what the player should fear, want, lose, and carry away.

Most of the time, that world stays there.

PlotForge is being built for the moment when it does not have to stay there.

It is a CLI-first Rust engine and creator studio for turning dreams, notes, scenes, rules, images, and choices into playable AI story games. The point is simple: help anyone build the game world they have been carrying around.

AI is not magic here. It is human work made dense. It is mathematics, language, hardware, research, code, failure, patience, and the judgment of scientists and engineers. PlotForge puts that work in the hands of creators. Not to replace their will. To give it shape.

A game is not only a game.

It can be a promise. A warning. A memory. A private country. It can hold the creator's dream and make that dream answer back.

## What PlotForge Is

PlotForge is an engine for playable imagination.

It keeps the project as files you can read: TOML, JSON, Markdown, and local assets. It keeps the schema in Rust. It lets the runtime and rule engine commit state. It treats traces, exports, media records, and package evidence as things that can be checked.

The current repository contains the MVP foundation:

- Rust workspace crates for schema, storage, rules, runtime, story craft review, local and real-provider agent planning, MCP, media assets, job state, export, Workshop draft validation, Studio adapters, and CLI orchestration.
- A folder-project format where source files are the truth and caches are rebuildable.
- A Vite/React/Tailwind Creator Desktop workspace with a thin Tauri bridge.
- A static no-network player package under `apps/player-web/static`.
- Local export profiles, AI usage disclosure files, package hashes, and Steam Submission Kit drafts.
- A local-only Workshop package schema and validator.
- A real in-app Settings surface: the Agent tab wires text, image, TTS, and moderation providers alongside per-project model settings, prompt templates, quota controls, and usage evidence; the MCP and Skills tabs manage their own user-global registries and per-project enablement. The right-side `AgentChatRail` drives `pi_agent_apply_run` to generate a scene proposal through the configured path and commit it at the runtime/rules boundary.

`local-pi` remains the default no-network text path; leave real media providers disabled for a fully no-network apply. Real provider clients are opt-in local configuration: PlotForge supplies first-party HTTP adapters for text, image, TTS, and moderation, while credentials remain in environment variables. Steamworks upload, hosted sharing, and paid Workshop flows are still deferred.

## The Shape Of A World

A PlotForge project is meant to be plain enough to inspect and strong enough to run.

The creator writes the world. The engine keeps it honest.

- `plotforge-schema` owns the contracts.
- `plotforge-storage` owns folder project loading and editing.
- `plotforge-rule` owns declarative rule evaluation.
- `plotforge-runtime` owns scene, beat, and state progression.
- `plotforge-agent` owns provider ports and planning/image orchestration.
- `plotforge-media` owns asset records, hashes, references, and reachability.
- `plotforge-job` owns long-running job state, failure, progress, retry, cancel, timeout, and cost accounting.
- `plotforge-export` owns static and desktop draft export packaging.
- `plotforge-workshop` owns offline Workshop draft validation and Steam Submission Kit draft generation.
- `plotforge-cli` ties the crates together without becoming the business logic.

Generated contracts live under `contracts/`. They come from Rust. They are not hand-maintained.

## Create A Starter Project

```bash
tmp="$(mktemp -d)"
cargo run -p plotforge-cli -- new project \
  --path "$tmp/starter-project" \
  --force \
  --concept "A local starter project." \
  --visual-style "clear readable test style" \
  --initial-scene "A creator opens a fresh PlotForge project."
cargo run -p plotforge-cli -- check "$tmp/starter-project"
cargo run -p plotforge-cli -- play "$tmp/starter-project" --once
cargo run -p plotforge-cli -- trace inspect "$tmp/starter-project/traces/latest.json"
cargo run -p plotforge-cli -- export static "$tmp/starter-project" --out "$tmp/export"
python3 scripts/qa/static_export_http_smoke.py --export-dir "$tmp/export"
```

Run the Creator Desktop workspace:

```bash
npm run creator-desktop:dev
npm run creator-desktop:qa
```

## Providers And Usage

Provider configuration lives in Settings → Agent and is stored in the user-global PlotForge registry, never in a game project or export. Entries name the environment variable that holds a credential; the credential value is not stored or displayed.

Text, image, TTS, and moderation providers support `max_concurrency` and `requests_per_minute`. `daily_token_budget` is available only for text providers because the other upstream responses do not provide reliable output-token usage. For non-local apply runs, the first enabled moderation provider screens the exact player input before text or MCP execution. A flagged result or provider failure stops the run explicitly; with no moderation provider configured, the input proceeds without a moderation hash. `local-pi` bypasses moderation and usage-ledger preflight for its local text turn.

The user-global usage ledger records redaction-safe call counts, token totals when reported, and cost units. Inspect it from the CLI:

```bash
cargo run -p plotforge-cli -- usage summary
cargo run -p plotforge-cli -- usage summary --json
cargo run -p plotforge-cli -- usage provider --id <provider-id>
```

## Build And Verify

The narrow checks are useful while working:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

npm run creator-desktop:typecheck
npm run creator-desktop:test
npm run creator-desktop:build
npm run player-web:qa

scripts/contracts/check_contracts.sh
python3 scripts/qa/no_launch_promise_lint.py
```

The full local gate is:

```bash
scripts/qa/full_local.sh
```

It covers Rust checks, contract drift, Creator Desktop QA, CLI smoke, static export smoke, desktop draft export smoke, and HTTP export smoke.

## Hard Lines

Dreams need tools. They also need boundaries.

- Folder project files win over generated caches.
- Runtime traces are evidence, not committed fixture state.
- Provider credentials stay local and out of project source, contracts, traces, exports, docs, and fixtures.
- Static exports copy only whitelisted player files, manifest files, and reachable referenced assets.
- Export profiles and `ai-usage.json` describe capability and evidence. They do not make legal conclusions or platform approval promises.
- Workshop and Steam Submission Kit flows are local draft support only. They do not upload by default and do not promise Steam outcomes.
- Provider and MCP network calls require explicit local configuration; tests use local mock servers and never contact real providers.

The work is to make creation feel open without making the system vague.

## Contributors

- Codex

## Why

Because there are too many worlds left in people's heads.

Because a person should be able to sit down with an idea and build a place others can enter.

Because imagination is not small. It only needs better doors.
