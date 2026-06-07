# Task Breakdown

## Confirmed Task Definition

Build a comprehensive PlotForge testing system covering Rust contracts, pure logic, fixture compatibility, CLI black-box behavior, static export smoke/security, CI gates, local QA scripts, and Codex Desktop Computer Use visual verification.

## Overview

- **Tracking Mode**: `GITHUB_STANDARD`
- **Repository**: `zhu1090093659/plotforge`
- **Total Phases**: 4
- **Total Tasks**: 8
- **Estimated Total Effort**: L

## S.U.P.E.R Design Constraints

- Tests must assert public contracts instead of duplicating implementation internals.
- CLI and QA scripts must use temp dirs, not mutate committed fixtures.
- Computer Use belongs to local desktop verification, not CI.
- Fallback/error paths must be explicit and asserted.

## Phase 1: Test Harness Foundation

**Goal**: Add reusable test dependencies and local QA script entry points.
**Prerequisite**: Current MVP on `main`.
**S.U.P.E.R Focus**: S, E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 1.1 | Add CLI integration test harness | P0 | M | - | A | S, E | `plotforge-cli` can be tested as a black-box binary with temp dirs |
| 1.2 | Add local all-in-one QA script | P0 | S | - | B | S, E | `scripts/qa/full_local.sh` runs static, unit, CLI, export smoke gates |

## Phase 2: Crate and Fixture Coverage

**Goal**: Expand behavior coverage for storage, runtime, rule, schema, storycraft, agent, export, and examples.
**Prerequisite**: Phase 1.
**S.U.P.E.R Focus**: P, U, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 2.1 | Add storage/runtime/fixture integration tests | P0 | M | 1.1 | A | P, E | Tests cover force=false, missing file, trace latest, committed fixture validation, and generated-vs-committed semantic parity |
| 2.2 | Add rule/storycraft/agent/schema boundary tests | P1 | M | 1.1 | B | P, R | Tests cover edge conditions, issue kinds, mock action mapping, and export/schema contracts |
| 2.3 | Add export manifest and security tests | P0 | M | 1.1 | C | P, E | Tests validate `game.json`, asset references, secret marker rejection, and HTTP smoke script |

## Phase 3: CI and Computer Use QA

**Goal**: Make tests repeatable in CI and document executable desktop visual verification.
**Prerequisite**: Phase 2.
**S.U.P.E.R Focus**: E, R.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 3.1 | Split and extend GitHub Actions testing | P0 | S | 2.1, 2.3 | A | E, R | CI runs static gates, unit tests, CLI smoke, and export HTTP smoke |
| 3.2 | Add Codex Computer Use static export runbook | P0 | S | 2.3 | B | S, E | Runbook explains localhost export, accessibility-tree verification, button click, and screenshot evidence |

## Phase 4: Verification and Delivery

**Goal**: Execute local and desktop validation, then close GitHub-tracked work.
**Prerequisite**: Phase 3.
**S.U.P.E.R Focus**: P, E.

| # | Task | Priority | Effort | Depends On | Lane | S.U.P.E.R | Acceptance Criteria |
|:--|:--|:--|:--|:--|:--|:--|:--|
| 4.1 | Run full local and Computer Use validation | P0 | M | 3.1, 3.2 | A | P, E | Local script passes; exported player is opened through local HTTP and clicked using Computer Use |

