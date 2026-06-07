# PlotForge Full Version Development — Progress Tracker

> **Task**: Continue PlotForge from the CLI-first MVP toward the full PRD through staged core hardening, contracts, desktop Studio, providers, media/jobs, export/debugger, and Steam exploration.
> **Started**: 2026-06-07
> **Last Updated**: 2026-06-07
> **Mode**: GITHUB_STANDARD
> **Repo**: zhu1090093659/plotforge

## GitHub Resources

- **Repository**: https://github.com/zhu1090093659/plotforge
- **All Full-Version Issues**: `gh issue list -R zhu1090093659/plotforge --label "full-version" --state all`
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
| 1 | Core Runtime Hardening | https://github.com/zhu1090093659/plotforge/milestone/9 | 0 | 5 | 5 |
| 2 | Story Craft and Provider Contracts | https://github.com/zhu1090093659/plotforge/milestone/10 | 4 | 1 | 5 |
| 3 | Desktop Studio Skeleton | https://github.com/zhu1090093659/plotforge/milestone/11 | 5 | 0 | 5 |
| 4 | Media and Job Pipeline | https://github.com/zhu1090093659/plotforge/milestone/12 | 4 | 0 | 4 |
| 5 | Persistence, Debugger, and Export v2 | https://github.com/zhu1090093659/plotforge/milestone/13 | 4 | 0 | 4 |
| 6 | Steam and Workshop Exploration | https://github.com/zhu1090093659/plotforge/milestone/14 | 3 | 0 | 3 |

## Issue Mapping

| Task ID | Issue | Title | Status |
|:--|:--|:--|:--|
| T1.1 | #21 | Add ActionIntent schema and interpreter service | closed by PR #47 |
| T1.2 | #22 | Inject scene planner into runtime | closed by PR #48 |
| T1.3 | #23 | Expand runtime trace contract | closed by PR #49 |
| T1.4 | #24 | Strengthen fixture/source-of-truth validation | closed by PR #50 |
| T1.5 | #25 | Add export whitelist package audit foundation | closed by PR #51 |
| T2.1 | #26 | Enrich StoryCraft schema toward PRD fields | closed by PR #52 |
| T2.2 | #27 | Add agent output proposal contracts | open |
| T2.3 | #28 | Add TextModelProvider trait and fake provider pipeline | open |
| T2.4 | #29 | Add reference-library compliance scaffolding | open |
| T2.5 | #30 | Add JSON Schema and TypeScript contract export | open |
| T3.1 | #31 | Initialize creator desktop frontend workspace | open |
| T3.2 | #32 | Add Tauri command bridge | open |
| T3.3 | #33 | Build project dashboard and source-file editor shell | open |
| T3.4 | #34 | Build playtest and debugger trace views | open |
| T3.5 | #35 | Add Studio local QA runbook and smoke path | open |
| T4.1 | #36 | Add plotforge-media asset registry | open |
| T4.2 | #37 | Add job queue core | open |
| T4.3 | #38 | Add image provider port and fake image pipeline | open |
| T4.4 | #39 | Integrate media references into runtime and export | open |
| T5.1 | #40 | Add save and restore runtime snapshots | open |
| T5.2 | #41 | Add SQLite cache and index as non-source-of-truth | open |
| T5.3 | #42 | Promote player web and export v2 package | open |
| T5.4 | #43 | Add export profiles and AI Usage Manifest base | open |
| T6.1 | #44 | Add Workshop item package schema and validator | open |
| T6.2 | #45 | Add Steam Submission Kit draft generator | open |
| T6.3 | #46 | Add Steam compliance QA docs and no-launch-promise guard | open |

## Quick Status Commands

```bash
gh issue list -R zhu1090093659/plotforge --label "full-version" --state all --json number,title,state,milestone
gh issue list -R zhu1090093659/plotforge --milestone "Full Phase 1: Core Runtime Hardening" --state open --json number,title
gh api 'repos/zhu1090093659/plotforge/milestones?state=all' --jq '.[] | select(.title | startswith("Full Phase")) | "\(.title): \(.open_issues) open, \(.closed_issues) closed, \(.state)"'
```

## Phase Checklist

- [x] Phase 1: Core Runtime Hardening (5/5 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/9)
- [ ] Phase 2: Story Craft and Provider Contracts (1/5 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/10)
- [ ] Phase 3: Desktop Studio Skeleton (0/5 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/11)
- [ ] Phase 4: Media and Job Pipeline (0/4 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/12)
- [ ] Phase 5: Persistence, Debugger, and Export v2 (0/4 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/13)
- [ ] Phase 6: Steam and Workshop Exploration (0/3 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/14)

## Current Status

**Active Phase**: Phase 2 — Story Craft and Provider Contracts
**Active Task**: T2.2 Add agent output proposal contracts (Issue #27)
**Blockers**: None

## Governance Status

**Shared instruction surface**: `AGENTS.md`
**Claude Code instruction surface**: unavailable
**Other platform rule surfaces**: none
**Memory surface**: `AGENTS.md` project-level memory
**Memory fallback path**: none

## Execution Telemetry

Per-task telemetry is stored as GitHub Issue comments. Adaptive drift state lives in Milestone descriptions.

## Next Steps

1. Implement #27: add typed agent output proposal contracts that cannot commit runtime state directly.
2. Keep provider contracts fake/mockable and avoid real model SDKs or network calls in Phase 2.
3. Use PRs with `closes #N` for GitHub issue closure and update this index after merges.

## Session Log

| Date | Session | Summary |
|:--|:--|:--|
| 2026-06-07 | Planning | Analyzed current MVP and PRD full-version gaps; created GitHub milestones #9-#14 and issues #21-#46. |
| 2026-06-07 | Execution | Completed #21 via PR #47: added typed ActionIntent, unsupported-input no-commit behavior, regression tests, and project memory rule. |
| 2026-06-07 | Execution | Completed #22 via PR #48: injected runtime scene planner, added explicit planner errors, and covered fake planner success, fallback-visible, and no-commit error paths. |
| 2026-06-07 | Execution | Completed #23 via PR #49: added structured redaction-safe trace fields for intent, rule, planner, diagnostics, fallback/errors, CLI inspect output, and regression tests. |
| 2026-06-07 | Execution | Completed #24 via PR #50: strengthened committed fixture validation against freshly generated source files, canonical project semantics, and no generated trace JSON. |
| 2026-06-07 | Execution | Completed #25 via PR #51: added static export whitelist auditing, safe asset path validation, provider/trace/raw response exclusion tests, and HTTP smoke package checks. Phase 1 milestone closed with drift_score 0. |
| 2026-06-07 | Execution | Completed #26 via PR #52: enriched StoryCraft schema with PRD pacing, hook/reversal, promises, character arcs, review notes, PlotThread metadata, NarrativeReview score fields, legacy defaults, and updated demo fixture. |
