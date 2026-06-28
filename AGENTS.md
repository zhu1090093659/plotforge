# PlotForge Agent Rules

This file is the project-level memory for future agents. Keep durable engineering constraints here; do not use it as a task log.

## Project Goal

Build PlotForge as a CLI-first Rust engine with a creator desktop UI adapter. Real model providers, media providers, and Steam/Workshop integrations remain deferred until their planned tasks.

The current MVP contains:

- Rust workspace crates for schema, storage, rule evaluation, story craft review, mock agent planning, runtime, media asset registry, job queue core, static export, and CLI.
- `apps/player-web/static` as the source for the no-network static player package copied by `plotforge-export`.
- `apps/creator-desktop` as the Vite/React/Tailwind Studio frontend workspace with dashboard, source editor, playtest, runtime trace views, and a thin Tauri command bridge.
- Folder project source of truth using TOML, JSON, Markdown, and generated local assets.
- Rebuildable SQLite cache/index under `.plotforge/cache.sqlite` for project summary, source file hashes, trace metadata, and asset metadata.
- Export profiles and `ai-usage.json` as redaction-safe package disclosure surfaces; they describe capabilities and AI usage evidence but do not provide legal conclusions or platform approval promises.
- `plotforge-workshop` as a local-only Workshop package schema and validator; it validates draft package metadata, file hashes, and disclosure files without Steam API or upload dependencies.
- Steam Submission Kit draft generation in `plotforge-workshop`; it emits local checklist, AI disclosure, content warning, and packaging-note Markdown drafts without legal, approval, or upload claims.
- Steam compliance QA boundary in `docs/steam-compliance-qa.md`, enforced by `scripts/qa/no_launch_promise_lint.py` for active project-facing guidance surfaces.
- `examples/dynasty-embers` as the committed demo fixture.
- Static web export served from local files over HTTP.
- Spec-driven GitHub issues are task/progress units, but pull requests are Phase-level delivery units; do not open one PR per small task during PRD completion work.

## Source of Truth

- `plotforge-schema` is the only schema contract source of truth.
- Generated frontend contracts live in `contracts/`; regenerate them with `scripts/contracts/export_contracts.sh` after Rust schema changes and verify with `scripts/contracts/check_contracts.sh`.
- Folder project files are the source of truth for game projects; generated caches, exports, traces, and build artifacts must be rebuildable.
- Structured editing documents for World, StoryCraft, Characters, State variables, and Rules are schema-defined contracts backed by `plotforge-storage` source files; regenerate frontend contracts after changing them.
- SQLite cache/index files are never required to load canonical project state; when cache contents conflict with folder files, folder files win and the cache must be rebuilt.
- `examples/dynasty-embers` must remain a valid fixture and must stay semantically aligned with `create_demo_project`.
- Runtime traces are generated evidence, not committed fixture state.
- Runtime traces must use structured, redaction-safe fields for action intent, rule result, planner result, diagnostics, media references, fallback, and errors; do not write raw provider responses, secrets, or unredacted key markers into trace/debug output.
- Runtime traces, snapshots, and provider output envelopes must carry reproducibility metadata (`run_seed`, `prompt_version`, `model_version`, `provider_config_hash`, and trace/snapshot evidence ids where applicable) without storing raw provider responses or credentials.
- Creator Desktop agent-native UI references live in `docs/design/ui-mockups/agent-native-v1/`; use these PNGs and the folder README as the visual/product reference for the Agent Mesh, Command Center, Director Mode, proof, trace, and export package workflows.
- Real provider configuration is local-only: credentials must be injected through explicit local resolvers such as environment variables, provider config hashes must be derived only from non-secret config fields, and `providers/` or `provider_config.*` files must never become project source, contracts, traces, or export package content.
- Media asset records must use structured, redaction-safe provider metadata only; store prompt hashes/request ids when needed, never raw provider responses or secrets.
- Job records must use typed state, explicit failure objects, injected clocks for deterministic tests, and no hidden global async state.
- Image provider fallbacks must remain trace-visible and must register placeholder assets as fallback metadata, not as successful generated-cache hits.
- Spec-driven planning artifacts and progress logs are local/private operator state for this open-source repository; keep them out of Git and under ignored paths unless explicitly approved.
- For broad PRD completion phases, prefer parallel sub-agents for disjoint implementation lanes after shared schema/contracts are planned; keep final integration, validation, and Phase PR scope decisions centralized.

## Architecture Boundaries

- Keep rule evaluation in `plotforge-rule`; do not duplicate rule behavior in CLI, UI, storage, export, or tests.
- Keep runtime state transitions in `plotforge-runtime`; agents propose content and runtime/rules commit state.
- Runtime owns Scene/Beat progression: same-scene choices such as `continue` advance through `BeatNext::Beat` without planner calls, scene changes, new images, or turn increments; `change_scene` choices cross the planner/runtime boundary, set the next scene entry beat, and increment the turn.
- Missing current beats, entry beats, or same-scene beat transitions must fail explicitly; do not silently fall back to another beat when committing runtime state.
- Player/freeform input must resolve to a typed `ActionIntent`; unsupported input must not mutate runtime state or silently map to a default action.
- Keep persistence and fixture file layout in `plotforge-storage`.
- Keep structured editing read/update/create behavior in `plotforge-storage` and expose it through `plotforge-studio`/Tauri command adapters; Creator Desktop must call the generated-contract bridge instead of parsing or validating project files in TypeScript.
- Keep SQLite cache/index behavior in `plotforge-storage`; it may index project summaries, source file hashes, trace metadata, and asset metadata, but must not become a second project loader or source of truth.
- Keep static export behavior in `plotforge-export`; exported bundles must copy only reachable referenced assets from `plotforge-media` and must not include private traces, provider config, raw provider responses, unreferenced assets, or secrets.
- Export profiles and AI usage manifests must stay schema-defined, redaction-safe, and capability/descriptive only; do not put provider credentials, raw provider responses, private traces, legal conclusions, or platform approval promises into them.
- Keep Workshop package validation in `plotforge-workshop`; it must remain local/offline validation of draft package metadata and file hashes, not a Steamworks SDK wrapper, upload client, or release-readiness oracle.
- Keep Steam Submission Kit generation in `plotforge-workshop`; it may generate local draft documents from validated package evidence, but must not claim compliance, approval, publishing automation, or legal conclusions.
- Keep Steam-facing docs and generated guidance under the no-launch-promise QA boundary; do not promise automatic publishing, platform outcomes, legal conclusions, or ownership of a creator's Steamworks workflow.
- Keep static player package behavior in `apps/player-web/static`; it must consume `ExportManifest`, run without external network URLs, and must not duplicate rule/runtime state machines. The player is split into `player-core.js` plus `player-save.js`/`player-i18n.js`/`player-audio.js`/`player-types.js` modules; every module file must be listed in both `plotforge-export`'s `PLAYER_PACKAGE_FILES` and `scripts/qa/static_export_http_smoke.py`'s expected whitelist (defence-in-depth), and the smoke test must assert no network URLs across all player JS.
- Keep asset registry records, content hashes, references, provider metadata, and reachability in `plotforge-media`; as export, runtime, Studio, and future providers integrate media, consume this boundary instead of adding a second media registry implementation.
- Keep long-running task state transitions, cancel/retry/timeout/progress, and cost accounting in `plotforge-job`; provider/runtime/export adapters should not own parallel job state machines.
- Keep image provider ports and scene image pipeline orchestration in `plotforge-agent`; image providers should feed `plotforge-media` and `plotforge-job` instead of bypassing those boundaries.
- Keep CLI behavior in `plotforge-cli`; CLI should orchestrate crates instead of owning business logic. CLI output formatting (i18n chrome terms, labels, colorized summaries, report rendering) lives in `plotforge-cli/src/cli_output.rs`; `main.rs` only dispatches commands and orchestrates handlers. Color is emitted only when stdout is a TTY (`std::io::IsTerminal`), so pipes, redirects, and test harnesses see plain text. JSON output (studio commands, `serde_json`) is never colored and its field shape is a contract — never adjust JSON fields for human-readable changes.
- CLI interactive wizards (e.g. `workshop submission-kit`) use `dialoguer` to prompt step-by-step and are the default; `--batch` explicitly enables full flag-driven input for scripts/tests. Wizards must guard stdin TTY (`std::io::IsTerminal`) and bail with an explicit error directing to `--batch` when non-TTY — never hang on a blocked stdin or silently fall back. Wizards only collect parameters; business validation and generation stay in the owning crate (`plotforge-workshop`), never reimplemented in the CLI.
- Keep `apps/creator-desktop` as an adapter over generated contracts and future Tauri commands. TypeScript UI code may import types from `contracts/plotforge.d.ts`, but must not reimplement rule, runtime, storage, storycraft, or agent business logic.
- Creator Desktop UI is split into one View component per workspace region (Export, Launchpad, Characters, Rules, State, World, Story, AssetMaintenance, TraceDebug, DirectorMode, AgentMesh); `App.tsx` only orchestrates routing, layout, and form-action sinking into `useStudioWorkspace` sub-hooks (`useProjectEditing`/`usePlaytest`/`useExport`). New regions go in their own `*View.tsx`, never as inline `render*Panel` functions in `App.tsx`. Structured-document creation (e.g. `createCharacterFromDraft`, `createRuleFromDraft`) must sink to `plotforge-studio` Rust commands via the contract bridge, never be reimplemented in TypeScript.
- Agent-native Creator Desktop UI may show ACP workers, agent capabilities, approvals, and evidence as explicit local/mock workflow surfaces, but must not imply real external agent execution, hidden network model calls, Steam upload automation, legal conclusions, or platform approval unless those schema-backed integrations are explicitly added.
- Keep i18n as an adapter concern for UI chrome and command output, with full English/Chinese coverage for every user-visible chrome string. Creator Desktop text is localized through `apps/creator-desktop/src/i18n.tsx`; static player text is localized through `apps/player-web/static/player-i18n.js` (split from player-core in T5.1); CLI output uses the `OutputLanguage` selector (`--language` / `PLOTFORGE_LANGUAGE`). Do not automatically translate project-authored titles, source files, story text, manifest content, or creator-provided export metadata.
- Keep Tauri command behavior in testable Rust adapter crates such as `plotforge-studio`; `apps/creator-desktop/src-tauri` should stay a thin IPC wrapper.
- Do not add Steam/Workshop, provider SDKs, or networked model calls to MVP core crates unless the project scope is explicitly changed.

## Testing Policy

- Every new feature must include corresponding tests in the same change.
- Every bug fix should include a regression test that fails before the fix when feasible.
- Schema or serialization changes must include roundtrip/contract tests in `plotforge-schema` and affected integration tests.
- Rule, runtime, storage, export, media, job, storycraft, and agent behavior changes must include crate-level tests for the changed boundary.
- CLI command changes must update black-box tests under `crates/plotforge-cli/tests/`.
- Export/player behavior changes must update export tests and, when rendering or interaction matters, the HTTP smoke path.
- Creator desktop changes must run `npm run creator-desktop:qa`; user-visible UI adapters should add or update TypeScript/Vitest coverage for contract-backed behavior.
- Creator desktop Vitest coverage is behavior-based: assert via role, accessible name, and visible text (`getByRole`/`getByLabelText`/`getByText`), not DOM class names or internal state, so tests survive className/layout refactors and reflect what users actually perceive.
- Static player changes must run `npm run player-web:qa` and cover DOM interaction, mobile viewport behavior, and no-network package constraints.
- Fixture changes must keep `cargo run -p plotforge-cli -- check examples/dynasty-embers` passing and must not commit generated trace JSON.
- CLI/export smoke tests must use temp dirs; do not mutate checked-in fixtures in CI.
- If a meaningful test cannot be added, document the reason in the PR or final response and run the next best validation.

## Required Validation

Use the narrowest relevant checks during development, then run the full gate before merging broad changes.

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `npm run creator-desktop:qa`
- `npm run player-web:qa`
- `python3 scripts/qa/creator_desktop_build_smoke.py`
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
- GitHub Actions must keep creator desktop typecheck/test/build in a separate frontend job.
- `scripts/qa/full_local.sh` is the repeatable local quality gate.
- Creator Desktop build smoke lives at `scripts/qa/creator_desktop_build_smoke.py` and must stay wired into `npm run creator-desktop:qa`.
- Creator Desktop Browser/Computer Use smoke lives at `scripts/qa/computer_use_creator_desktop.md`; verify the Studio shell, source editor, playtest run, runtime trace id, narrative review, diagnostics, and mobile viewport reachability.
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
