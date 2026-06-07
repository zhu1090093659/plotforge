# Milestones

| # | Milestone | Target Phase | Criteria | Status |
|:--|:--|:--|:--|:--|
| 1 | Full Phase 1: Core Runtime Hardening | After Phase 1 | ActionIntent, runtime planner injection, trace expansion, fixture validation, export whitelist audit all merged and CI green | Pending |
| 2 | Full Phase 2: Story Craft and Provider Contracts | After Phase 2 | Richer StoryCraft contracts, agent proposal schemas, fake text provider, and reference compliance checks merged | Pending |
| 3 | Full Phase 3: Desktop Studio Skeleton | After Phase 3 | Tauri/React Studio skeleton can open/check/play/export a project and display trace/debug state | Pending |
| 4 | Full Phase 4: Media and Job Pipeline | After Phase 4 | Asset registry, job queue, fake image provider, and referenced-asset export integration merged | Pending |
| 5 | Full Phase 5: Persistence, Debugger, and Export v2 | After Phase 5 | Save/restore, non-source SQLite cache, player/export v2, export profiles, and AI Usage Manifest base merged | Pending |
| 6 | Full Phase 6: Steam and Workshop Exploration | After Phase 6 | Offline Workshop package schema, Submission Kit draft, and compliance docs merged without Steam API upload dependency | Pending |

## Milestone Gates

- Every milestone requires `scripts/qa/full_local.sh` or the expanded equivalent for new surfaces.
- Any frontend/Tauri milestone also requires frontend typecheck/build and a local smoke path.
- Any provider/media milestone must use fake providers in CI and must not require real API keys.
- Any export/Steam milestone must include package content audit tests and no-secret/no-private-trace checks.
