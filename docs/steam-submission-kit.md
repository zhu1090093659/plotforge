# Steam Submission Kit Drafts

PlotForge can generate local Steam Submission Kit draft documents from a validated Workshop package. The generator lives in `plotforge-workshop` and uses the local `workshop-item.json` plus `ai-usage.json` evidence; it does not call Steamworks APIs, upload builds, submit store pages, pay fees, or determine approval.

Generated files:

- `steam-submission-checklist.md`
- `steam-ai-disclosure-draft.md`
- `steam-content-warnings.md`
- `steam-packaging-notes.md`

The drafts are support material for a responsible developer. Before any external submission, recheck current Steamworks requirements and edit the drafts against the actual build, store page, screenshots, capsule assets, AI policy, moderation policy, and content warnings.

Official references to recheck:

- https://partner.steamgames.com/doc/gettingstarted/contentsurvey
- https://partner.steamgames.com/doc/gettingstarted/appfee
- https://partner.steamgames.com/doc/store/review_process
- https://partner.steamgames.com/doc/features/workshop

Boundary rules:

- Do not put provider credentials, raw provider responses, private traces, Steam app IDs, or upload state into generated drafts.
- Do not describe the kit as compliance proof, platform approval, or publishing automation.
- Keep generated content deterministic and snapshot-tested when changing checklist, AI disclosure, or content warning text.
