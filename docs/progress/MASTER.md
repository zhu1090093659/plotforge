# PlotForge Full Version Development — Progress Tracker

> **Task**: Continue PlotForge from the CLI-first MVP toward the full PRD through staged core hardening, contracts, desktop Studio, providers, media/jobs, export/debugger, and Steam exploration.
> **Started**: 2026-06-07
> **Last Updated**: 2026-06-08
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
| 2 | Story Craft and Provider Contracts | https://github.com/zhu1090093659/plotforge/milestone/10 | 0 | 5 | 5 |
| 3 | Desktop Studio Skeleton | https://github.com/zhu1090093659/plotforge/milestone/11 | 0 | 5 | 5 |
| 4 | Media and Job Pipeline | https://github.com/zhu1090093659/plotforge/milestone/12 | 0 | 4 | 4 |
| 5 | Persistence, Debugger, and Export v2 | https://github.com/zhu1090093659/plotforge/milestone/13 | 0 | 4 | 4 |
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
| T2.2 | #27 | Add agent output proposal contracts | closed by PR #53 |
| T2.3 | #28 | Add TextModelProvider trait and fake provider pipeline | closed by PR #54 |
| T2.4 | #29 | Add reference-library compliance scaffolding | closed by PR #55 |
| T2.5 | #30 | Add JSON Schema and TypeScript contract export | closed by PR #56 |
| T3.1 | #31 | Initialize creator desktop frontend workspace | closed by PR #57 |
| T3.2 | #32 | Add Tauri command bridge | closed by PR #58 |
| T3.3 | #33 | Build project dashboard and source-file editor shell | closed by PR #59 |
| T3.4 | #34 | Build playtest and debugger trace views | closed by PR #60 |
| T3.5 | #35 | Add Studio local QA runbook and smoke path | closed by PR #61 |
| T4.1 | #36 | Add plotforge-media asset registry | closed by PR #62 |
| T4.2 | #37 | Add job queue core | closed by PR #63 |
| T4.3 | #38 | Add image provider port and fake image pipeline | closed by PR #64 |
| T4.4 | #39 | Integrate media references into runtime and export | closed by PR #65 |
| T5.1 | #40 | Add save and restore runtime snapshots | closed by PR #66 |
| T5.2 | #41 | Add SQLite cache and index as non-source-of-truth | closed by PR #67 |
| T5.3 | #42 | Promote player web and export v2 package | closed by PR #68 |
| T5.4 | #43 | Add export profiles and AI Usage Manifest base | closed by PR #69 |
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
- [x] Phase 2: Story Craft and Provider Contracts (5/5 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/10)
- [x] Phase 3: Desktop Studio Skeleton (5/5 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/11)
- [x] Phase 4: Media and Job Pipeline (4/4 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/12)
- [x] Phase 5: Persistence, Debugger, and Export v2 (4/4 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/13)
- [ ] Phase 6: Steam and Workshop Exploration (0/3 tasks) — [milestone](https://github.com/zhu1090093659/plotforge/milestone/14)

## Current Status

**Active Phase**: Phase 6 — Steam and Workshop Exploration
**Active Task**: T6.1 Add Workshop item package schema and validator (Issue #44)
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

1. Implement #44: add Workshop item package schema and validator.
2. Keep Steam/Workshop work as schema/docs/package exploration only; do not add platform upload or launch promises.
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
| 2026-06-07 | Execution | Completed #27 via PR #53: added typed scene plan, beat draft, and review agent output proposals, role/payload validation, no-direct-state-commit rejection tests, and scene assembly validation. |
| 2026-06-07 | Execution | Completed #28 via PR #54: added TextModelProvider, deterministic fake provider pipeline, raw JSON proposal validation, provider error/timeout/schema/JSON fallback tests, and runtime trace-visible provider errors. |
| 2026-06-07 | Execution | Completed #29 via PR #55: added summary-only reference analysis schema, storage compliance validation, large raw-text rejection, demo reference fixture, and AGENTS reference-library rule. |
| 2026-06-07 | Execution | Completed #30 via PR #56: added generated JSON Schema and TypeScript contract snapshots, contract version envelope checks, export/check scripts, CI contract drift gate, and closed Phase 2 with drift_score 0. |
| 2026-06-07 | Execution | Completed #31 via PR #57: initialized npm workspace and `apps/creator-desktop` Vite/React/Tailwind shell, added contract-backed frontend adapter test, frontend CI job, full local QA integration, and AGENTS/README commands. |
| 2026-06-07 | Execution | Completed #32 via PR #58: added `plotforge-studio` command adapter crate, Tauri v2 `src-tauri` IPC wrapper, typed frontend invoke bridge, Tauri check in creator QA/CI, and browser DOM smoke coverage. |
| 2026-06-08 | Execution | Completed #33 via PR #59: added canonical source-file read/write commands, safe editable text-surface guardrails, project dashboard source browser/editor, browser preview data source, component tests, full local QA, and desktop Chrome smoke coverage. |
| 2026-06-08 | Execution | Completed #34 via PR #60: added data-source backed Playtest and Runtime Trace panels, visible fallback/errors/review/diagnostics, no raw key rendering coverage, full local QA, and desktop/mobile Browser smoke coverage. |
| 2026-06-08 | Execution | Completed #35 via PR #61: added Creator Desktop build smoke wired into `creator-desktop:qa`, Studio Browser/Computer Use local runbook, README/AGENTS QA updates, full local QA, and desktop/mobile Browser smoke. Phase 3 milestone closed with drift_score 0. |
| 2026-06-08 | Execution | Completed #36 via PR #62: added `plotforge-media` asset registry foundation with typed asset records, SHA-256 hashes, dedupe, refs, provider metadata, reachability/exportable paths, contract snapshots, AGENTS/README boundary updates, full local QA, and green GitHub Actions. |
| 2026-06-08 | Execution | Completed #37 via PR #63: added `plotforge-job` queue core with typed states, explicit failures, injected-clock tests, cancel/retry/timeout/progress, cost accounting, contract snapshots, AGENTS/README invariants, full local QA, and green GitHub Actions. |
| 2026-06-08 | Execution | Completed #38 via PR #64: added image provider ports, fake image provider, scene image pipeline over `plotforge-media` and `plotforge-job`, trace-visible placeholder fallback, cache reuse tests, runtime image failure trace coverage, AGENTS/README boundary updates, full local QA, and green GitHub Actions. |
| 2026-06-08 | Execution | Completed #39 via PR #65: added typed runtime media references, regenerated contracts, exported only reachable referenced `plotforge-media` assets, excluded unreferenced project assets, updated Creator Desktop fixtures and AGENTS/README rules, full local QA, green GitHub Actions, and closed Phase 4 milestone with drift_score 0. |
| 2026-06-08 | Execution | Completed #40 via PR #66: added typed `RuntimeSnapshot` schema/contracts, runtime snapshot/restore APIs with project id/version/current-scene validation, storage snapshot read/write helpers with safe ids and latest mirror, save roundtrip, deterministic multi-turn restore, failed-rule no-corruption tests, full local QA, and green GitHub Actions. |
| 2026-06-08 | Execution | Completed #41 via PR #67: added optional `.plotforge/cache.sqlite` cache/index with schema migrations, project/source/trace/asset metadata tables, rebuild-from-folder API, cache miss and source-precedence tests, `rusqlite` bundled dependency, AGENTS/README cache invariant updates, full local QA, and green GitHub Actions. |
| 2026-06-08 | Execution | Completed #42 via PR #68: promoted `apps/player-web/static` to the no-network export player package, replaced inline export HTML with package file copying, added player DOM/mobile/no-network tests, strengthened export package and HTTP smoke checks, wired player-web QA into full local QA and CI, verified with Codex in-app Browser desktop/mobile smoke, and green GitHub Actions. |
| 2026-06-08 | Execution | Completed #43 via PR #69: added schema-defined export profiles for static/dynamic/desktop/Steam intent, introduced redaction-safe `AiUsageManifest` contracts and `ai-usage.json`, bumped contract schema version to 2, extended static export manifest/smoke/no-secret tests, updated AGENTS/README disclosure boundaries, full local QA, green GitHub Actions, and closed Phase 5 milestone with drift_score 0. |
