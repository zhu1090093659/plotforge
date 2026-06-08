# Steam Compliance QA Boundary

PlotForge treats Steam and Workshop support as local evidence preparation until a future task explicitly adds platform integration. Current code may validate local Workshop package metadata and generate Steam Submission Kit draft documents; it must not call Steamworks APIs, store Steam app IDs, upload builds, submit store pages, pay platform fees, or decide platform outcomes.

## Official Sources

Recheck these current Steamworks pages before changing Steam-facing docs, generated text, or QA rules:

- Steam Content Survey: https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- Steam Direct Fee: https://partner.steamgames.com/doc/gettingstarted/appfee
- Steam Review Process: https://partner.steamgames.com/doc/store/review_process
- Steam Workshop: https://partner.steamgames.com/doc/features/workshop

## Local QA Policy

Steam-facing work must pass local checks before review:

- `python3 scripts/qa/no_launch_promise_lint.py`
- `git diff --check`
- `scripts/qa/full_local.sh`

The lint only scans current project-facing guidance surfaces: `AGENTS.md`, `README.md`, `docs/steam-submission-kit.md`, and this document. PRD/archive files may discuss rejected wording as historical context, but active docs and generated guidance must stay conservative.

## Claim Boundary

Allowed language:

- Local draft package validation.
- Local evidence preparation.
- AI usage disclosure draft.
- Content warning draft.
- Creator-owned Steamworks review workflow.
- Official Steamworks requirements must be rechecked by the responsible developer.

Blocked language:

- Any wording that promises automatic Steam publishing.
- Any wording that promises approval, compliance, endorsement, or final release status.
- Any wording that presents generated drafts as legal advice, platform authorization, or a substitute for Steamworks forms and review.
- Any wording that says PlotForge owns the creator's Steam Direct workflow.

## Required Evidence

Every Steam/Workshop change must leave reviewable local evidence:

- Source docs or generated output changed.
- Local lint/check command output.
- No provider credentials, raw provider responses, private traces, Steam app IDs, or upload state in generated files.
- Tests or snapshots for changed generated text when applicable.
