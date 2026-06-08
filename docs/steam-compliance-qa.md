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

- Any wording that claims a creator can bypass normal Steamworks submission, fee, questionnaire, review, or release controls.
- Any wording that promises approval, compliance, endorsement, accepted status, or final release status.
- Any wording that presents generated drafts as legal determinations, platform authorization, or a substitute for Steamworks forms and review.
- Any wording that makes PlotForge responsible for a creator's Steam Direct or Steamworks account workflow.
- Any generated or checked-in text containing concrete Steam app, depot, build, item, or upload state identifiers.

## PRD Completion Audit Rows

The final PRD completion audit for Steam/Workshop/Submission Kit scope should include these evidence rows:

| Evidence row | Source to inspect | Passing boundary |
|---|---|---|
| Workshop package validation | `plotforge-workshop` tests and generated package schema | Local/offline metadata and file-hash validation only; no Steam API, SDK, upload, fee, or account dependency. |
| Submission Kit drafts | Generated checklist, AI disclosure, content warning, and packaging notes | Draft support material only; creator must recheck Steamworks requirements and own all external submission decisions. |
| AI usage disclosure | `ai-usage.json` plus generated AI disclosure draft | Redaction-safe capability and usage evidence; no raw provider responses, secrets, or legal determinations. |
| Content warning and moderation notes | Generated content warning draft and package metadata | Describes project evidence and review prompts; does not promise platform acceptance or moderation outcomes. |
| Leak boundary | Export package contents and generated Markdown | No provider credentials, private traces, Steam app IDs, depot/build IDs, Workshop item IDs, or upload/release state. |
| Active docs boundary | `README.md`, this document, and `docs/steam-submission-kit.md` | No claims of platform endorsement, release status, creator-account delegation, or store-submission automation. |
| QA command evidence | `python3 scripts/qa/no_launch_promise_lint.py` and relevant package/doc tests | The active-doc lint passes and any changed generated text has a deterministic test or documented next-best check. |

## Required Evidence

Every Steam/Workshop change must leave reviewable local evidence:

- Source docs or generated output changed.
- Local lint/check command output.
- No provider credentials, raw provider responses, private traces, Steam app IDs, depot/build IDs, Workshop item IDs, or upload state in generated files.
- Tests or snapshots for changed generated text when applicable.
