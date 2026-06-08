# Steam Submission Kit Drafts

PlotForge can generate local Steam Submission Kit draft documents from a validated Workshop package. The generator lives in `plotforge-workshop` and uses the local `workshop-item.json` plus `ai-usage.json` evidence; it does not call Steamworks APIs, upload builds, submit store pages, pay fees, or determine approval.

Generated files:

- `steam-store-copy-draft.md`
- `steam-submission-checklist.md`
- `steam-ai-disclosure-draft.md`
- `steam-content-warnings.md`
- `steam-asset-references.md`
- `steam-direct-checklist.md`
- `steam-content-safety-checklist.md`
- `steam-packaging-notes.md`

The drafts are support material for a responsible developer. Before any external submission, recheck current Steamworks requirements and edit the drafts against the actual build, store page, screenshots, capsule assets, AI policy, moderation policy, and content warnings. The project QA boundary for Steam-facing text is documented in [Steam Compliance QA Boundary](steam-compliance-qa.md).

Official references to recheck:

- https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- https://partner.steamgames.com/doc/gettingstarted/appfee
- https://partner.steamgames.com/doc/store/review_process
- https://partner.steamgames.com/doc/features/workshop

Boundary rules:

- Do not put provider credentials, raw provider responses, private traces, Steam app IDs, or upload state into generated drafts.
- Do not describe the kit as a compliance determination, platform approval, or publishing automation.
- Keep generated content deterministic and snapshot-tested when changing store copy, checklist, AI disclosure, content warning, asset reference, Steam Direct, content safety, or packaging-note text.

## Local Evidence Workflow

The Submission Kit should stay on this local workflow:

1. Validate the Workshop draft package metadata and file hashes.
2. Read redaction-safe package evidence from `workshop-item.json`, `ai-usage.json`, content warnings, and packaging metadata.
3. Emit deterministic Markdown drafts for the creator to review.
4. Require the creator to reconcile the drafts against current Steamworks forms, policies, build contents, store assets, and moderation plans outside PlotForge.

The kit may prepare evidence and review prompts. It must not claim that a package is accepted by Steam, that a creator has completed platform requirements, or that PlotForge has performed the creator's account workflow.

## Audit Checklist

Use these rows when auditing PRD completion for the Steam/Workshop/Submission Kit scope:

| Area | Evidence | Boundary to verify |
|---|---|---|
| Workshop item package | `workshop-item.json`, package file hashes, validator output | Local/offline validation only; no Steam API, SDK, account, fee, or upload dependency. |
| AI disclosure draft | `ai-usage.json` and `steam-ai-disclosure-draft.md` | Describes generated-content evidence and guardrail prompts without provider secrets, raw responses, or legal determinations. |
| Content warning draft | `steam-content-warnings.md` | Summarizes project evidence for creator review without platform-result claims. |
| Packaging notes | `steam-packaging-notes.md` | Documents package contents, hashes, and next review tasks without external submission state. |
| Checklist | `steam-submission-checklist.md` | Points creators to current Steamworks review, fee, questionnaire, and asset checks without replacing those workflows. |
| Leak scan | Export/package output and generated drafts | No Steam app IDs, depot/build IDs, Workshop item IDs, upload state, private traces, credentials, or raw provider output. |
| Active-doc QA | `python3 scripts/qa/no_launch_promise_lint.py` | Active Steam-facing docs stay inside the no-launch-promise boundary; PRD/archive history is intentionally outside the scan. |
