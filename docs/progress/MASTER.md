# PlotForge Test System — Progress Tracker

> **Task**: Build a comprehensive test system with Rust, CLI, export, CI, and Codex Desktop Computer Use verification.
> **Started**: 2026-06-07
> **Last Updated**: 2026-06-07
> **Mode**: GITHUB_STANDARD
> **Repo**: zhu1090093659/plotforge

## GitHub Resources

- **Repository**: https://github.com/zhu1090093659/plotforge
- **All Issues**: `gh issue list -R zhu1090093659/plotforge --label "spec-driven" --state all`
- **Project Board**: Not created. Current token has no `project` scope.

## References

- [Project Overview](../analysis/project-overview.md)
- [Module Inventory](../analysis/module-inventory.md)
- [Risk Assessment](../analysis/risk-assessment.md)
- [Task Breakdown](../plan/task-breakdown.md)
- [Dependency Graph](../plan/dependency-graph.md)
- [Milestones](../plan/milestones.md)

## Milestones

| Phase | Name | Milestone URL | Open | Closed | Total |
|:--|:--|:--|--:|--:|--:|
| 1 | Test Harness Foundation | https://github.com/zhu1090093659/plotforge/milestone/5 | 0 | 2 | 2 |
| 2 | Crate and Fixture Coverage | https://github.com/zhu1090093659/plotforge/milestone/6 | 0 | 3 | 3 |
| 3 | CI and Computer Use QA | https://github.com/zhu1090093659/plotforge/milestone/7 | 0 | 2 | 2 |
| 4 | Verification and Delivery | https://github.com/zhu1090093659/plotforge/milestone/8 | 0 | 1 | 1 |

## Issue Mapping

| Task ID | Issue | Title | Status |
|:--|:--|:--|:--|
| T1.1 | #12 | Add CLI integration test harness | closed by PR |
| T1.2 | #13 | Add local all-in-one QA script | closed by PR |
| T2.1 | #14 | Add storage/runtime/fixture integration tests | closed by PR |
| T2.2 | #15 | Add rule/storycraft/agent/schema boundary tests | closed by PR |
| T2.3 | #16 | Add export manifest and security tests | closed by PR |
| T3.1 | #17 | Split and extend GitHub Actions testing | closed by PR |
| T3.2 | #18 | Add Codex Computer Use static export runbook | closed by PR |
| T4.1 | #19 | Run full local and Computer Use validation | closed by PR |

## Quick Status Commands

```bash
gh issue list -R zhu1090093659/plotforge --label "spec-driven" --state all --json number,title,state,milestone
gh api 'repos/zhu1090093659/plotforge/milestones?state=all' --jq '.[] | "\(.title): \(.open_issues) open, \(.closed_issues) closed, \(.state)"'
```

## Phase Checklist

- [x] Phase 1: Test Harness Foundation (2/2 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/5)
- [x] Phase 2: Crate and Fixture Coverage (3/3 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/6)
- [x] Phase 3: CI and Computer Use QA (2/2 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/7)
- [x] Phase 4: Verification and Delivery (1/1 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/8)

## Current Status

**Active Phase**: Complete
**Active Task**: None
**Blockers**: None

## Next Steps

1. Keep CI green on `main`.
2. Use `scripts/qa/full_local.sh` before future feature PRs.
3. Use `scripts/qa/computer_use_static_export.md` for local desktop export smoke.

## Computer Use Validation

- **Browser**: Google Chrome
- **URL**: `http://127.0.0.1:4173`
- **Observed**: `Dynasty Embers`, `The Red Deficit Ledger`, three choice buttons.
- **Interaction**: clicked `严查军饷贪墨`.
- **Post-click text**: `Seek stolen funds while angering court factions.`

## Execution Telemetry

Per-task telemetry is stored as GitHub Issue comments. Adaptive drift state lives in Milestone descriptions.

## Session Log

| Date | Session | Summary |
|:--|:--|:--|
| 2026-06-07 | Planning | Analyzed test coverage, created test-system plan, and prepared GitHub synchronization. |
| 2026-06-07 | Execution | Added CLI, fixture, runtime, storage, export, schema, rule, storycraft, agent tests; split CI; added local QA scripts and Computer Use runbook; ran local and desktop smoke. |
