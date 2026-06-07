# Task Breakdown

## Confirmed Task Definition

Build the first executable PlotForge MVP from `docs/plotforge_prd_final.md`: a private GitHub-backed Rust workspace with a CLI-first core loop, schema contracts, story craft review, declarative rules, mock generation, `dynasty-embers` demo, trace persistence, and static web export.

Desktop UI, real LLM/image providers, SQLite cache, and Steam/Workshop integrations are intentionally deferred until the core contracts and CLI loop are stable.

## Overview

- **Tracking Mode**: `GITHUB_STANDARD`
- **Repository**: `zhu1090093659/plotforge`
- **Total Phases**: 4
- **Total Tasks**: 10
- **Estimated Total Effort**: XL

## S.U.P.E.R Design Constraints

- **S**: Each crate owns one responsibility. Avoid a broad `core` catch-all.
- **U**: Apps and CLI call inward; schema has no project-internal dependencies.
- **P**: Cross-module data is represented by serializable Rust structs.
- **E**: Paths, seed, and provider mode are explicit arguments or config.
- **R**: Mock providers, export template, and storage adapters are replaceable behind stable contracts.

## Phase 1: Foundation Contracts

**Goal**: Establish the buildable workspace and schema source of truth.
**Prerequisite**: Private repository initialized.
**S.U.P.E.R Focus**: S, U, P.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 1.1 | Create Rust workspace and quality gates | P0 | M | - | A | S, U, E | Workspace compiles; crates exist only for MVP scope; CI runs `cargo check/test/fmt/clippy` |
| 1.2 | Implement `plotforge-schema` contracts | P0 | L | 1.1 | A | P, R | Core project/world/story/scene/rule/trace/export types derive serde traits; JSON roundtrip tests pass |

### Parallel Lanes

| Lane | Tasks | Combined Effort | Merge Risk | Key Files |
|:--|:--|:--|:--|:--|
| A | 1.1, 1.2 | L | Low | `Cargo.toml`, `crates/plotforge-schema/` |

## Phase 2: Project Data and Pure Logic

**Goal**: Make a valid PlotForge folder project, story craft review, and declarative rule evaluation work without providers.
**Prerequisite**: Phase 1 complete.
**S.U.P.E.R Focus**: S, P, E.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 2.1 | Implement project storage and `dynasty-embers` template | P0 | M | 1.2 | A | S, P, E | `plotforge new demo` creates PRD directory structure, `game.toml`, story craft files, characters, resources, rules, and AGENTS.md |
| 2.2 | Implement story craft MVP | P0 | M | 1.2 | B | S, P, R | Initial emotional arc and >=3 plot threads exist; narrative review catches weak hook/no progress/fake choice/OOC/thread break |
| 2.3 | Implement declarative rule engine | P0 | M | 1.2 | C | S, U, P | Conditions/effects work for resources and flags; deterministic tests cover add/set/min/max and action type binding |

### Parallel Lanes

| Lane | Tasks | Combined Effort | Merge Risk | Key Files |
|:--|:--|:--|:--|:--|
| A | 2.1 | M | Medium | `crates/plotforge-storage/`, `examples/dynasty-embers/` |
| B | 2.2 | M | Low | `crates/plotforge-storycraft/` |
| C | 2.3 | M | Low | `crates/plotforge-rule/` |

## Phase 3: Runtime and CLI Loop

**Goal**: Run the complete demo loop locally with explicit traces.
**Prerequisite**: Phase 2 complete.
**S.U.P.E.R Focus**: U, P, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 3.1 | Implement mock agent pipeline | P0 | M | 2.2 | A | P, E, R | Mock planner/writer/reviewer produce schema-valid scene, beat, and review outputs without external model calls |
| 3.2 | Implement runtime session, commit, save, trace | P0 | L | 2.2, 2.3, 3.1 | A | U, P, R | `continue` keeps scene image; `changeScene` creates next scene; world deltas commit via rules; trace records before/after/errors/review |
| 3.3 | Implement CLI commands | P0 | M | 2.1, 3.2 | A | S, U, E | `new demo`, `check`, `play --once`, `trace inspect`, and `export static` commands work |

### Parallel Lanes

| Lane | Tasks | Combined Effort | Merge Risk | Key Files |
|:--|:--|:--|:--|:--|
| A | 3.1, 3.2, 3.3 | L | Medium | `plotforge-agent`, `plotforge-runtime`, `plotforge-cli` |

## Phase 4: Static Export and Delivery Hardening

**Goal**: Produce a local playable static export and verify the MVP.
**Prerequisite**: Phase 3 complete.
**S.U.P.E.R Focus**: P, E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 4.1 | Implement static web export | P0 | M | 3.3 | A | P, E, R | Export writes `index.html`, manifest/data, placeholder scene assets, and excludes API keys/provider config |
| 4.2 | Add docs and repository validation | P1 | S | 4.1 | B | E, R | README, AGENTS.md, CI, and command docs reflect actual commands; validation suite passes |

### Parallel Lanes

| Lane | Tasks | Combined Effort | Merge Risk | Key Files |
|:--|:--|:--|:--|:--|
| A | 4.1 | M | Medium | `crates/plotforge-export/`, `apps/player-web/` |
| B | 4.2 | S | Medium | `README.md`, `AGENTS.md`, `.github/workflows/ci.yml` |
