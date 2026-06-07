# PlotForge Agent Rules

This file is the project-level memory for future agents. Keep durable engineering constraints here; do not use it as a task log.

## Project Goal

Build PlotForge as a CLI-first Rust MVP before adding desktop UI, real model providers, media providers, SQLite cache, or Steam/Workshop integrations.

The current MVP contains:

- Rust workspace crates for schema, storage, rule evaluation, story craft review, mock agent planning, runtime, static export, and CLI.
- Folder project source of truth using TOML, JSON, Markdown, and generated local assets.
- `examples/dynasty-embers` as the committed demo fixture.
- Static web export served from local files over HTTP.

## Source of Truth

- `plotforge-schema` is the only schema contract source of truth.
- Generated frontend contracts live in `contracts/`; regenerate them with `scripts/contracts/export_contracts.sh` after Rust schema changes and verify with `scripts/contracts/check_contracts.sh`.
- Folder project files are the source of truth for game projects; generated caches, exports, traces, and build artifacts must be rebuildable.
- `examples/dynasty-embers` must remain a valid fixture and must stay semantically aligned with `create_demo_project`.
- Runtime traces are generated evidence, not committed fixture state.
- Runtime traces must use structured, redaction-safe fields for action intent, rule result, planner result, diagnostics, fallback, and errors; do not write raw provider responses, secrets, or unredacted key markers into trace/debug output.
- Spec-driven planning artifacts belong under `docs/archives/<initiative>/` after completion.

## Architecture Boundaries

- Keep rule evaluation in `plotforge-rule`; do not duplicate rule behavior in CLI, UI, storage, export, or tests.
- Keep runtime state transitions in `plotforge-runtime`; agents propose content and runtime/rules commit state.
- Player/freeform input must resolve to a typed `ActionIntent`; unsupported input must not mutate runtime state or silently map to a default action.
- Keep persistence and fixture file layout in `plotforge-storage`.
- Keep static export behavior in `plotforge-export`; exported bundles must not include private traces, provider config, raw provider responses, or secrets.
- Keep CLI behavior in `plotforge-cli`; CLI should orchestrate crates instead of owning business logic.
- Do not add Steam/Workshop, provider SDKs, desktop UI, or networked model calls to MVP core crates unless the project scope is explicitly changed.

## Testing Policy

- Every new feature must include corresponding tests in the same change.
- Every bug fix should include a regression test that fails before the fix when feasible.
- Schema or serialization changes must include roundtrip/contract tests in `plotforge-schema` and affected integration tests.
- Rule, runtime, storage, export, storycraft, and agent behavior changes must include crate-level tests for the changed boundary.
- CLI command changes must update black-box tests under `crates/plotforge-cli/tests/`.
- Export/player behavior changes must update export tests and, when rendering or interaction matters, the HTTP smoke path.
- Fixture changes must keep `cargo run -p plotforge-cli -- check examples/dynasty-embers` passing and must not commit generated trace JSON.
- CLI/export smoke tests must use temp dirs; do not mutate checked-in fixtures in CI.
- If a meaningful test cannot be added, document the reason in the PR or final response and run the next best validation.

## Required Validation

Use the narrowest relevant checks during development, then run the full gate before merging broad changes.

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `scripts/qa/full_local.sh`
- `scripts/contracts/check_contracts.sh`

Tempdir CLI smoke commands:

- `tmp="$(mktemp -d)"`
- `cargo run -p plotforge-cli -- new demo --path "$tmp/dynasty-embers" --force`
- `cargo run -p plotforge-cli -- check "$tmp/dynasty-embers"`
- `cargo run -p plotforge-cli -- play "$tmp/dynasty-embers" --once`
- `cargo run -p plotforge-cli -- trace inspect "$tmp/dynasty-embers/traces/latest.json"`
- `cargo run -p plotforge-cli -- export static "$tmp/dynasty-embers" --out "$tmp/export"`
- `python3 scripts/qa/static_export_http_smoke.py --export-dir "$tmp/export"`

Fixture validation commands:

- `cargo run -p plotforge-cli -- check examples/dynasty-embers`
- Only run `cargo run -p plotforge-cli -- new demo --path examples/dynasty-embers --force` when intentionally regenerating the committed fixture.

## CI and QA

- GitHub Actions must keep separate jobs for static Rust checks, unit/integration tests, CLI smoke, and export smoke.
- `scripts/qa/full_local.sh` is the repeatable local quality gate.
- Codex Desktop Computer Use smoke is local desktop QA only and must not become a GitHub Actions dependency.
- For Computer Use export smoke, serve static exports over localhost HTTP, then verify visible title, scene text, choice buttons, and post-click text changes.
- Starting a server is not enough evidence; perform at least one real browser/app interaction when Computer Use validation is requested.

## Error Handling and Security

- Do not add silent fallbacks. Any fallback must be visible in trace/debug output and covered by tests.
- Prefer explicit errors over broad catch-all handling.
- Do not hardcode secrets, API keys, tokens, provider credentials, or personal paths into source, fixtures, tests, exports, or docs.
- Static exports must reject obvious secret markers and must not copy private traces or provider configuration.
- Static export package output must be audited against an explicit whitelist of player files, manifest files, and referenced assets; stale or unexpected files in the output package are errors.
- Do not introduce hidden network calls in tests or MVP runtime paths.
- Reference-library imports must store source metadata, authorization/rights metadata, short summaries, and structure notes only; do not store large raw copyrighted bodies in fixtures or project files.

## Change Discipline

- Prefer targeted diffs that preserve crate boundaries.
- Remove obsolete logic when replacing behavior; do not layer duplicate sources of truth.
- Keep generated artifacts out of commits unless they are intentional fixtures.
- Update README and relevant docs when commands, workflow, or user-visible behavior changes.
- Keep `AGENTS.md` updated when a rule becomes durable project memory.
