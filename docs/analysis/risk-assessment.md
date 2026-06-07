# Risk Assessment

## S.U.P.E.R Architecture Health Summary

Current MVP health is good for a CLI-first slice: crate boundaries, explicit tests, CI, and static export security baseline are already present. Full-version risk comes from opening several new architectural boundaries at once: desktop UI, real providers, media cache/jobs, SQLite, dynamic AI compliance, and Steam/Workshop packaging.

| Principle | Status | Findings | Priority |
|:--|:--|:--|:--|
| **S** Single Purpose | 🟡/🟢 | Crate split is healthy. `plotforge-storage` is broad, and `runtime` currently owns action interpretation, rule commit, planner selection, and trace assembly. | High |
| **U** Unidirectional Flow | 🟡 | Core flow is mostly inward, but full runtime needs `PlayerInput -> ActionIntent -> Policy -> RuleCommit -> SceneProposal -> Trace`. UI/Tauri must remain outer adapters. | High |
| **P** Ports over Implementation | 🟡 | `plotforge-schema` is the contract source, but PRD full version needs JSON Schema/TS contracts and provider/media/export request/response schemas. | High |
| **E** Environment-Agnostic | 🟡/🟢 | Current MVP has no external services. Future keychain, SQLite, provider SDKs, Tauri permissions, and Steamworks increase environment coupling. | Medium |
| **R** Replaceable Parts | 🟡 | `ScenePlanner` exists, but runtime directly uses the mock. Provider, media, export, and player surfaces need real replaceable ports. | High |

Overall: directionally healthy, but full-version expansion should begin with architecture hardening before UI/provider/media features.

## High-Severity Risks

### 1. Scope Explosion

The PRD contains Tauri Studio, React editor, providers, media, debugger, save restore, export profiles, Steam/Workshop, and compliance. Treating this as a single implementation phase would violate the project memory rule to keep MVP core clean and tested.

Mitigation: split into staged releases:

- V0.3 Core Hardening
- V0.4 Desktop Shell
- V0.5 LLM Provider
- V0.6 Media Pipeline
- V0.7 Save/Debugger/Export v2
- V0.8 Steam Exploration

### 2. Silent Fallback

PRD allows fallback for agent/image/TTS failures, but project rules prohibit silent fallback. Current mock fallback is trace-visible, but unknown player input currently defaults to `raise_tax`, and missing resource/flag reads can become implicit zero/false behavior.

Mitigation:

- Add `ActionIntent` with explicit unknown/unsupported outcome.
- Add tests proving invalid commands do not mutate state.
- Ensure all fallback paths write trace errors and debugger-visible status.

### 3. Schema Multi-Source

Full version needs Rust, React/TypeScript, JSON validators, provider outputs, export manifests, and Steam package metadata. Hand-written parallel contracts would drift.

Mitigation:

- Keep Rust schema as source of truth.
- Generate or snapshot JSON Schema/TS contracts.
- Add contract tests before building Tauri/React features on top.

### 4. Provider/Key Security

Real LLM/image/TTS providers introduce key storage, prompt injection, raw provider responses, costs, and redaction.

Mitigation:

- Define provider traits and serializable request/response structs first.
- Use fake providers for tests.
- Keep keys outside frontend, exports, traces, and fixture files.
- Require schema repair/validate and redacted trace tests.

### 5. Export Security

Current export secret scan blocks obvious markers. Full version will have provider configs, raw responses, private traces, and multiple export profiles, so blacklist scanning is not enough.

Mitigation:

- Move export packaging to whitelist manifest rules.
- Add package content audits.
- Prove traces/provider config/raw provider responses are excluded.
- Keep static export no-network by default.

### 6. File Project vs SQLite Dual Truth

The PRD correctly says folder project files are source of truth and SQLite is cache/index/trace/asset DB. Introducing SQLite too early can create conflicting project state.

Mitigation:

- Add rebuild-from-folder tests.
- Forbid SQLite from being required to load canonical project state.
- Store trace/cache/job metadata separately from source files.

### 7. Steam/Workshop Compliance

Official Steamworks docs confirm:

- Store page content should only include launch-available features, and screenshots should be gameplay.
- Generative AI content consumed by players must be described in Steam's content survey; live-generated AI requires guardrails.
- Steam Direct currently requires a per-app fee.
- Curated Workshop requires approval/workflow and ISteamUGC integration for upload tooling.

Sources:

- https://partner.steamgames.com/doc/store/review_process
- https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- https://partner.steamgames.com/doc/gettingstarted/appfee
- https://partner.steamgames.com/doc/features/workshop

Mitigation: keep Steam work as P2 exploration until local desktop/export/provider foundations are real; do not write store-facing promises before implementation.

## Risk Matrix

| Risk | Impact | Likelihood | Severity | Mitigation |
|:--|:--|:--|:--|:--|
| Implementing desktop/provider/media together | Large regressions, unclear failure source | High | High | Phase by boundaries; each phase has tests |
| Unknown input mutates state | Bad gameplay, hidden bugs | High | High | `ActionIntent` explicit unknown and no-commit tests |
| Runtime remains tied to mock agent | Blocks LLM provider | High | High | Inject planner/provider ports |
| Schema drift into TS/JSON | UI/export/provider incompatibility | High | High | Generate/snapshot contracts |
| Export leaks config/traces | Security regression | Medium | Critical | Whitelist packaging and content audit tests |
| SQLite becomes source of truth | Data corruption/drift | Medium | High | Rebuild and source-precedence tests |
| Fallback hides provider/media failure | False success | Medium | High | Trace/debug visible fallback |
| Steam claims unreleased features | Store review risk | Medium | High | Steam only after implemented features; AI manifest |
| Media job cost runaway | Provider spend and UX risk | Medium | Medium | Job budget/cancel/cache/fake provider tests |

## Governance Resolution

- Durable project memory: root `AGENTS.md`.
- Fixture-local instruction surface: `examples/dynasty-embers/AGENTS.md`.
- Spec artifacts: active run uses `docs/analysis`, `docs/plan`, `docs/progress`; archive to `docs/archives/<project>` on completion.
- GitHub tracking: `GITHUB_STANDARD`.

## Immediate Architectural Recommendations

1. Start with Core Hardening before UI or provider SDKs.
2. Add `ActionIntent` and planner injection so runtime stops owning concrete interpretation/provider selection.
3. Extend trace/schema contracts before provider/media work.
4. Keep desktop Studio as adapter over core commands.
5. Keep Steam/Workshop limited to package schema and AI manifest until the local product is real.
