# Risk Assessment

## S.U.P.E.R Architecture Health Summary

The repository has no implementation code yet, so current health is an initial architecture risk assessment rather than a code-debt audit. The PRD direction is coherent: Rust Core, Tauri/React UI, folder project as source of truth, SQLite as cache, provider traits, `LLM proposes, Engine commits`, and static export without API keys. The risk is MVP breadth.

| Principle | Status | Key Findings | Transformation Priority |
|:--|:--|:--|:--|
| **S** Single Purpose | 🟡 | Planned crate boundaries are strong, but `runtime`, `agent`, `storycraft`, and future desktop UI can grow into broad modules. | High |
| **U** Unidirectional Flow | 🟡 | Target flow is correct; risk is UI/Tauri/storage/provider bypassing runtime or validator boundaries. | High |
| **P** Ports over Implementation | 🟡 | Provider traits and schemas are planned, but Rust/TS/JSON contracts can drift unless schema is the single source. | High |
| **E** Environment-Agnostic | 🟡 | BYO key and local storage are planned; path, OS keychain, Tauri permission, and provider assumptions need containment. | Medium |
| **R** Replaceable Parts | 🟡 | Replaceability depends on stable schema/ports before concrete providers, SQLite, UI, or Steam integrations appear. | High |

**Overall Health**: _0/5 principles proven, 5/5 directionally healthy_ — Refactoring Needed Before Implementation Solidifies.

### S.U.P.E.R Violation Hotspots

1. **Agent pipeline as a broad coordinator**: scene planning, beat writing, review, validation, media, and provider behavior must stay separated by typed inputs/outputs.
2. **Schema multi-source risk**: `plotforge-schema` must be the contract source; TypeScript and JSON Schema should be generated or tested against it.
3. **AI commit boundary**: agents produce proposed content only; runtime/rules commit validated deltas.
4. **Silent fallback risk**: fallback can preserve demo flow, but it must write explicit trace/debug state and must not be counted as a normal success path.
5. **Folder project vs SQLite**: TOML/JSON/Markdown files are source of truth; SQLite is cache/index/trace only.
6. **Export security**: static export must exclude API keys, provider raw responses, private traces, and unreferenced/private files.

## Risk Matrix

| Risk | Impact | Likelihood | Severity | Mitigation |
|:--|:--|:--|:--|:--|
| MVP scope too broad | Core loop delayed | High | High | Build CLI/Core first; defer desktop, real providers, Steam |
| Agent output instability | Broken narrative or corrupt state | High | High | Validate proposed outputs and commit only via runtime/rules |
| Schema drift | Rust/UI/export incompatibility | High | High | Roundtrip tests and generated/checked contracts |
| Silent fallback masks failures | False confidence | High | High | Trace all fallback/error paths; tests assert fallback explicitly |
| Prompt injection | Untrusted player input contaminates system prompt | Medium | High | Input sanitizer, action interpreter, policy check, context builder |
| API key leakage | Secrets in frontend/export/trace | Medium | Critical | Environment/local key store; export secret scan |
| Copyright/reference misuse | Legal/product risk | Medium | High | MVP includes methods/templates only; user-authorized imports later |
| Static export misses assets | Unplayable package | Medium | High | Export manifest and local smoke test |
| Steam/Workshop premature scope | Schedule and compliance risk | Medium | High | Keep Steam P2 and isolate future APIs |

## High-Severity Risks

### MVP Complexity

The PRD covers project management, world/story editing, rules, runtime, agents, images, debug, save, export, and desktop UI. The maintainable first cut is:

- P0a: CLI + schema + rule + storycraft + mock runtime for `dynasty-embers`.
- P0b: trace/debug storage and explicit fallback/error states.
- P0c: static export.
- P1: desktop shell and real providers.
- P2: media provider depth, Steam, Workshop, TTS.

### AI Output and Commit Boundary

The core invariant is `LLM proposes, Engine commits`. Implementation should expose proposed scene/beat/review structs, validate them, apply declarative rules, then commit a traceable state transition.

### Fallback Visibility

PRD fallback paths should not become silent success paths. In MVP, mock generation is a normal configured mode; fallback from an error is an explicit trace entry.

### Export Security

Static web export must contain prebaked player data only. It must not include API keys, provider configs, private traces, raw provider responses, or unlicensed reference text.

## Technical Debt

There is no code debt yet. The initial debt to avoid is over-creating empty crates without enforcing contracts. Each crate added in MVP must compile, have a clear responsibility, and have at least targeted tests if it owns behavior.

## Compatibility Concerns

- Future Tauri/React code should call core commands/adapters, not reimplement rules.
- Future LLM providers must conform to Rust traits and serializable output contracts.
- Future SQLite support must be rebuildable from project files.
- Future Steam/Workshop work must remain outside MVP core.
