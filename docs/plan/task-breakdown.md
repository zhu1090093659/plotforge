# Task Breakdown

## Confirmed Task Definition

Continue PlotForge from the current CLI-first MVP toward the full PRD in staged releases. The implementation must preserve the project-level rules in `AGENTS.md`: new features require tests, schema remains the contract source, runtime must not depend on UI/Tauri/provider implementations, fallbacks must be visible, and exports must not leak secrets or private traces.

## Overview

- **Tracking Mode**: `GITHUB_STANDARD`
- **Repository**: `zhu1090093659/plotforge`
- **Total Phases**: 6
- **Total Tasks**: 26
- **Estimated Total Effort**: XL
- **Decomposition Strategy**: Core-hardening-first, then product surfaces.

## S.U.P.E.R Design Constraints

- **S**: Keep each crate and app surface single-purpose. Split storage/template/cache/media/export responsibilities when they start to overlap.
- **U**: Data flows inward: UI/CLI/Tauri/providers are adapters; runtime/rule/storycraft/schema remain core.
- **P**: Contracts first. Provider outputs, action intents, traces, export packages, Steam manifests, and player data must have typed/serializable schemas before implementations.
- **E**: No hardcoded secrets, personal paths, provider keys, or environment-specific assumptions.
- **R**: Provider, media, player, export, and Steam implementations must be replaceable behind ports.

## Testing and Governance Constraints

- Every feature task must add/update relevant automated tests.
- Pure docs/planning tasks must state why tests are not applicable and run `git diff --check`.
- Changes to durable agent instructions must update root `AGENTS.md`.
- CLI/export smoke tests must use temp dirs and must not mutate committed fixtures in CI.
- Computer Use is local-only QA and never a GitHub Actions dependency.

## Phase 1: Core Runtime Hardening

**Goal**: Prepare current MVP core for real providers, desktop UI, and media without adding those outer layers yet.
**Prerequisite**: Current main at completed CLI MVP and test-system baseline.
**S.U.P.E.R Focus**: U, P, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Test Expectation | Memory Impact | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| 1.1 | Add `ActionIntent` schema and interpreter service | P0 | M | - | A | P, U | Add schema/interpreter/runtime tests for known, unknown, and no-mutation paths | Update `AGENTS.md` if action intent becomes a durable rule | Unknown player input no longer silently defaults to `raise_tax`; CLI/runtime keep existing supported demo actions |
| 1.2 | Inject scene planner into runtime instead of constructing mock directly | P0 | M | 1.1 | A | U, R | Add runtime tests using fake planners for success, fallback, and error paths | None | Runtime can run with `MockAgentPipeline` or a test planner through a port |
| 1.3 | Expand runtime trace contract for action, commit, fallback, and validation stages | P0 | M | 1.1, 1.2 | A | P, E | Add trace roundtrip, redaction, and fallback-visible tests | May update `AGENTS.md` if trace invariants change | Trace explains action intent, selected choice, rule result, planner result, and errors without leaking secrets |
| 1.4 | Strengthen fixture/source-of-truth validation | P1 | M | - | B | S, P | Add canonical generated-vs-committed fixture comparison and no generated trace tests | None | `examples/dynasty-embers` stays semantically aligned with generated demo as schema expands |
| 1.5 | Replace export secret blacklist with whitelist package audit foundation | P0 | M | 1.3 | C | P, E | Add export package content audit tests, provider/trace exclusion tests, HTTP smoke remains green | May add export memory rule if durable | Static export writes only allowed manifest/player/assets; traces and provider config are excluded by construction |

### Parallel Lanes

| Lane | Tasks | Merge Risk | Key Files |
|:--|:--|:--|:--|
| A | 1.1, 1.2, 1.3 | Medium | `plotforge-schema`, `plotforge-runtime`, `plotforge-agent`, runtime tests |
| B | 1.4 | Low | `plotforge-storage`, examples tests |
| C | 1.5 | Low-medium | `plotforge-export`, export tests, smoke script |

## Phase 2: Story Craft and Provider Contracts

**Goal**: Define contracts for richer story generation and text providers before integrating real model SDKs.
**Prerequisite**: Phase 1.
**S.U.P.E.R Focus**: S, P, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Test Expectation | Memory Impact | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| 2.1 | Enrich StoryCraft schema toward PRD fields | P0 | L | 1.3 | A | P | Add schema roundtrip and fixture migration/update tests | None | Story promises, pacing, character arcs, issue kinds, and review notes are typed |
| 2.2 | Add agent output contracts for scene plan, beat draft, and review | P0 | M | 2.1 | A | P, R | Add serde and validation tests for proposed outputs | None | Agent outputs are proposals, not committed runtime state |
| 2.3 | Add `TextModelProvider` trait and fake provider pipeline | P0 | L | 2.2 | B | P, E, R | Add fake provider success/error/timeout/schema-validation tests; no network in CI | May add provider key rule if needed | LLM path can be exercised without external keys; provider errors are trace-visible |
| 2.4 | Add reference-library compliance scaffolding | P1 | M | 2.1 | C | E, P | Add tests that user imports store metadata/summary only and reject large raw copyrighted text fixtures | Update memory if import boundary becomes durable | Reference analysis stores structure and short summaries, never default-scraped copyrighted bodies |
| 2.5 | Add JSON Schema / TypeScript contract export and version envelope | P0 | L | 2.1 | D | P, R | Add reproducible contract snapshot tests and invalid-version tests | Update `AGENTS.md` if generated contract workflow becomes durable | Rust schema remains the source of truth; frontend contracts are generated or snapshot-validated, not hand-maintained |

### Parallel Lanes

| Lane | Tasks | Merge Risk | Key Files |
|:--|:--|:--|:--|
| A | 2.1, 2.2 | Medium | `plotforge-schema`, `plotforge-storycraft`, `plotforge-agent` |
| B | 2.3 | Medium | `plotforge-agent`, runtime integration tests |
| C | 2.4 | Low-medium | storage/reference modules and tests |
| D | 2.5 | Medium | `plotforge-schema`, contract snapshots, CI |

## Phase 3: Desktop Studio Skeleton

**Goal**: Add the creator desktop shell as an adapter over stable core commands.
**Prerequisite**: Phase 1 core hardening; Phase 2 contracts can proceed in parallel but provider SDKs are not required.
**S.U.P.E.R Focus**: U, E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Test Expectation | Memory Impact | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| 3.1 | Initialize frontend workspace and `apps/creator-desktop` skeleton | P0 | L | 1.2 | A | E, R | Add TS typecheck/build scripts and CI job | Update commands in README/AGENTS if durable | Vite/React/Tailwind app builds without touching core logic |
| 3.2 | Add Tauri v2 command bridge for project open/check/play/export | P0 | L | 3.1, 1.3, 2.5 | A | U, E | Add Rust command tests and frontend API type tests | None | Tauri commands call core crates and generated contracts; no business logic in UI |
| 3.3 | Build project dashboard and source-file editor shell | P1 | L | 3.2 | B | S, U | Add component tests/typecheck and fixture load smoke | None | Users can open `dynasty-embers`, inspect files, and edit safe text surfaces |
| 3.4 | Build playtest and debugger trace views | P0 | L | 3.2, 1.3 | C | P, E | Add UI tests for trace/error/fallback display and no-key rendering | None | Runtime trace and fallback/errors are visible in Studio |
| 3.5 | Add local desktop QA runbook and optional browser smoke for Studio | P1 | M | 3.3, 3.4 | D | E | Add scripted smoke where possible; Computer Use remains local-only | Update `scripts/qa` docs if workflow changes | Studio QA has reproducible build/typecheck plus manual desktop validation steps |

### Parallel Lanes

| Lane | Tasks | Merge Risk | Key Files |
|:--|:--|:--|:--|
| A | 3.1, 3.2 | High | `apps/creator-desktop`, `Cargo.toml`, CI |
| B | 3.3 | Medium | React project/editor components |
| C | 3.4 | Medium | React playtest/debugger components |
| D | 3.5 | Low | `scripts/qa`, docs |

## Phase 4: Media and Job Pipeline

**Goal**: Add asset/media/job foundations before real image/TTS providers.
**Prerequisite**: Phase 1; provider contracts from Phase 2 recommended.
**S.U.P.E.R Focus**: S, P, E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Test Expectation | Memory Impact | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| 4.1 | Add `plotforge-media` crate with `AssetRecord` registry | P0 | L | 1.5, 2.1 | A | S, P | Add hash/ref/reachability tests | None | Assets have typed records, hashes, refs, provider metadata, and exportable paths |
| 4.2 | Add job queue core for long-running tasks | P0 | L | 1.3 | B | S, P, E | Add fake clock/cancel/retry/timeout/progress tests | May update memory with job invariants | LLM/image/TTS/export jobs share typed state and explicit failures |
| 4.3 | Add image provider port and placeholder/fake image pipeline | P0 | L | 4.1, 4.2, 2.3 | A | P, E, R | Add fake provider, fallback-visible, cache reuse tests | None | Scene image generation can run with fake/placeholder provider and trace failures |
| 4.4 | Integrate media references into runtime/export | P1 | M | 4.3 | C | U, P | Add runtime/export tests that only referenced assets ship | None | Runtime records media calls; export packages only referenced assets |

### Parallel Lanes

| Lane | Tasks | Merge Risk | Key Files |
|:--|:--|:--|:--|
| A | 4.1, 4.3 | Medium | `plotforge-media`, schema, tests |
| B | 4.2 | Medium | job crate/module, tests |
| C | 4.4 | Medium | runtime/export integration |

## Phase 5: Persistence, Debugger, and Export v2

**Goal**: Make play sessions resumable, observable, and packageable.
**Prerequisite**: Phases 1 and 4; desktop debugger from Phase 3 benefits from these contracts.
**S.U.P.E.R Focus**: P, E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Test Expectation | Memory Impact | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| 5.1 | Add save/restore runtime snapshots | P0 | L | 1.3 | A | P, E | Add save roundtrip, multi-turn restore, failed rule no-corruption tests | None | Sessions resume deterministically from saved snapshots |
| 5.2 | Add SQLite cache/index as non-source-of-truth | P1 | XL | 5.1, 4.1 | B | U, E | Add migrations, rebuild-from-folder, cache miss/source precedence tests | Update memory if cache invariant changes | Project loads from files without DB; DB can be rebuilt |
| 5.3 | Promote player web/export v2 package | P0 | L | 1.5, 4.4 | C | P, R | Add package snapshot, DOM click, mobile viewport, no-network static tests | None | Static export is zip/package-ready and uses a real player surface |
| 5.4 | Add export profiles and AI Usage Manifest base | P1 | M | 5.3, 2.3 | C | P, E | Add manifest snapshot and no-secret tests | None | Export profiles describe static/dynamic/desktop/Steam intent without leaking keys |

### Parallel Lanes

| Lane | Tasks | Merge Risk | Key Files |
|:--|:--|:--|:--|
| A | 5.1 | Medium | runtime/storage/schema |
| B | 5.2 | High | storage/cache/migrations |
| C | 5.3, 5.4 | Medium | export/player-web/schema |

## Phase 6: Steam and Workshop Exploration

**Goal**: Prepare Steam/Workshop packaging and compliance artifacts without integrating upload APIs or making launch promises.
**Prerequisite**: Phase 5 export profiles and package surface.
**S.U.P.E.R Focus**: P, E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Test Expectation | Memory Impact | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|:--|:--|
| 6.1 | Add Workshop item package schema and validator | P1 | M | 5.4 | A | P, E | Add package validation and metadata snapshot tests | None | Workshop package can be validated locally without Steam API |
| 6.2 | Add Steam Submission Kit draft generator | P2 | M | 6.1 | B | P, E | Add snapshot tests for checklist, AI disclosure draft, and content warnings | None | Kit makes no legal guarantee and does not claim one-click Steam release |
| 6.3 | Add Steam compliance QA docs and no-launch-promise guard | P2 | S | 6.2 | C | E | Docs-only plus `git diff --check`; optional static text lint | Update `AGENTS.md` if durable Steam rule emerges | Repo documents launch-claim boundaries and Steam official source links |

### Parallel Lanes

| Lane | Tasks | Merge Risk | Key Files |
|:--|:--|:--|:--|
| A | 6.1 | Low-medium | steam schema/module |
| B | 6.2 | Low | kit generator/tests |
| C | 6.3 | Low | docs/AGENTS if needed |
