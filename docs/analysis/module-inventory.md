# Module Inventory

Scoring: `5` healthy, `3` acceptable for MVP but should be improved before full-version expansion, `1-2` high-risk. S.U.P.E.R = Single Purpose, Unidirectional Flow, Ports over Implementation, Environment-Agnostic, Replaceable Parts.

## Current Modules

| Module | Responsibility | Dependencies | Tests | Complexity | S/U/P/E/R | Full-Version Risk |
|:--|:--|:--|:--|:--|:--|:--|
| `plotforge-schema` | Data contracts for game, world/story state, scene/beat/choice, rules, review, trace, export manifest | `serde` | JSON roundtrip, enum tags, manifest/project contracts | Medium | `3/5/4/5/4` | Single file is becoming broad; PRD fields, JSON Schema/TS generation, migrations missing |
| `plotforge-rule` | Declarative conditions/effects, resource clamp, flags, events, explicit unknown resource errors | `plotforge-schema`, `thiserror` | Matching, clamp, flag, set, event dedupe, unknown resource | Medium | `5/5/4/5/4` | Needs path effects, deterministic risks, ActionIntent/ValidatedDelta layering |
| `plotforge-storycraft` | Demo story craft seed and narrative review heuristics | `plotforge-schema` | Story craft seed, weak hook, fake choice, OOC, broken thread, valid scene | Medium | `3/4/3/4/3` | Demo seed and review logic are mixed; needs Planner/Doctor/Consistency/Deslop/reference modules |
| `plotforge-agent` | `ScenePlanner` port and deterministic mock pipeline | `schema`, `storycraft` | Scene validity/review, fallback, action-thread mapping | Medium | `3/3/3/4/2` | Mock is concrete and Dynasty-specific; runtime cannot yet swap real LLM provider cleanly |
| `plotforge-runtime` | Session state, action interpretation, rule commit, scene planning, trace assembly | `agent`, `rule`, `schema`, `storycraft` | Chinese action, commit delta, continue, multi-turn, missing scene, summarize delta | Medium-high | `3/4/2/4/2` | Needs ActionIntent, injected planner, save restore, beat navigation, job/media integration |
| `plotforge-storage` | Folder project creation/loading/validation, demo template, trace writing, placeholder assets | `schema`, `storycraft`, `serde`, `toml` | Force flag, missing entry scene, trace/latest, fixture semantic parity | High | `2/4/4/3/2` | Too many roles; potential dual truth between `story_craft.toml`, `emotional_arc.json`, `plot_threads.toml` |
| `plotforge-export` | Static export manifest, generated HTML, asset output, secret marker blocking | `schema`, `storage` | File output, manifest/assets, secret marker rejection, HTTP smoke | Medium | `3/4/4/4/3` | HTML is embedded in Rust, assets are placeholder writes, export security should be whitelist-based |
| `plotforge-cli` | Developer command surface and orchestration | `runtime`, `storage`, `export`, `schema` | Black-box CLI smoke, interactive rejection, fixture validity, trace hygiene | Medium | `4/5/3/5/4` | CLI output is an implicit public contract; avoid moving business logic here |
| `apps/player-web` | Placeholder for future standalone web player | None yet | None | Low | `4/4/3/5/3` | Needs real React player consuming the same `ExportManifest` contract |
| `scripts/qa` | Full local QA, export HTTP smoke, Computer Use smoke runbook | Bash, Python stdlib, Cargo | Exercised by local/manual runs | Low | `4/4/3/3/4` | HTTP smoke does not yet perform DOM click; Computer Use is intentionally local-only |
| `.github/workflows/ci.yml` | CI jobs for Rust static/unit/CLI/export | GitHub Actions, Rust toolchain, Python | Latest main green | Low | `5/4/3/4/4` | Needs frontend/Tauri/provider/media jobs as those surfaces are added |
| `examples/dynasty-embers` | Committed historical crisis demo fixture | Storage/schema/rule/storycraft contracts | CLI/check, fixture parity, no trace JSON | Medium | `3/4/4/4/3` | Only one scene; placeholder art; fixture parity should deepen as schema expands |

## Current Dependency Flow

The current dependency direction is mostly healthy:

```text
schema
  -> rule / storycraft
  -> agent
  -> runtime
  -> storage / export
  -> cli
```

The biggest full-version risk is not circular dependency today; it is concrete implementation leaking into core before ports are stable:

- `runtime` directly owns keyword action interpretation and constructs `MockAgentPipeline`.
- `storage` owns demo template, folder layout, trace, placeholder PNG, and project read/write.
- `export` owns both export rules and the HTML player implementation.

## Required New Modules / Surfaces

| New Surface | Target Role | First Tests Required | S.U.P.E.R Risk |
|:--|:--|:--|:--|
| `ActionIntent` schema/service | Input sanitizer, policy check, action interpretation, no silent default | Known/unknown intent tests, prompt injection boundary tests, CLI/runtime integration | Prevents runtime from hiding unknown input as `raise_tax` |
| Provider ports | `TextModelProvider`, `ImageProvider`, `TtsProvider`, `VisionProvider`, `ModerationProvider` | Fake provider, error/timeout, schema validation, key redaction | Must be traits/contracts before SDK adapters |
| `plotforge-media` | `AssetRecord`, hash dedupe, refs, image/TTS requests, placeholder/silent fallback trace | hash/ref tests, missing asset tests, fallback-visible tests, export reachability | Avoid provider/cache/path coupling |
| Job queue | LLM/image/TTS/export/workshop/reference task progress, timeout, retry, cancel, cost | fake clock, cancel, retry, failure state, cost accounting | Needs job schema and handler registry, not global async state |
| SQLite cache/index | Cache, index, trace, asset metadata; not source data | rebuild-from-folder, migration, cache miss, source-precedence tests | High dual-source risk |
| `apps/creator-desktop` | Tauri + React Studio shell | Tauri command tests, TS typecheck, UI smoke, key-not-in-frontend checks | UI must not duplicate core rules/runtime |
| Real `apps/player-web` | React player for static export and later dynamic modes | manifest fixture, DOM click, mobile viewport, no-network static mode | Must consume manifest; no duplicated rule engine for static mode |
| Observability/debugger | RuntimeTrace expansion, redaction, agent/media calls, debugger UI | trace roundtrip, redaction, issue display, cost/fallback assertions | Trace cannot leak keys/raw provider responses |
| Save/restore | Runtime snapshots, save files, replay/resume | save roundtrip, multi-turn restore, failed rule no corruption | Required before dynamic/desktop playtest |
| Export Kit v2 | zip/static package, whitelist manifest, desktop/BYO/Steam profile foundations | package content audit, secret scan, manifest snapshot, HTTP smoke | Export must be profile-driven and whitelist-based |
| Steam exploration | Workshop package schema, AI Usage Manifest, submission checklist | package schema, AI disclosure snapshot, no API upload in CI | Platform/compliance coupling; keep P2 adapter |

## Module-Level Priorities

1. Hardening runtime ports and action intent before real providers.
2. Splitting storage responsibilities enough to protect source-of-truth boundaries.
3. Extending schema contracts and generated/validated cross-layer formats before React/Tauri.
4. Introducing provider/media/job surfaces with fake implementations and trace visibility.
5. Building the desktop shell as an adapter over stable core commands.
