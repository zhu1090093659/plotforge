# Codex Desktop Computer Use Static Export Smoke

This runbook verifies the exported PlotForge player in a real desktop browser window through Codex Desktop Computer Use. It complements CI; it is not a GitHub Actions dependency. Creator Desktop Studio smoke steps live in `scripts/qa/computer_use_creator_desktop.md`.

## Prepare the Export

Run from the repository root:

```bash
tmp="$(mktemp -d)"
cargo run -p plotforge-cli -- new project \
  --path "$tmp/starter-project" \
  --force \
  --concept "A local starter project." \
  --visual-style "clear readable test style" \
  --initial-scene "A creator opens a fresh PlotForge project."
cargo run -p plotforge-cli -- export static "$tmp/starter-project" --out dist/starter-project
python3 -m http.server 4173 --directory dist/starter-project
```

Use localhost HTTP instead of `file://` because the player loads `game.json` through `fetch("./game.json")`.

## Computer Use Steps

1. Open `http://127.0.0.1:4173` in a local browser.
2. Call `get_app_state` for the browser and inspect the accessibility tree.
3. Verify visible content:
   - Page title or heading: `Starter Project`
   - Scene title: `Opening Scene`
   - Initial beat text mentions the creator opening a fresh project
   - At least one choice button is visible
4. Click a choice by accessibility element index when possible. Use pixel coordinates only as a fallback.
5. Call `get_app_state` again and verify the beat text changes to the clicked choice's dramatic purpose.

## Evidence

Record the observed title, clicked choice, and post-click text in the final QA notes. Screenshots may be kept under `artifacts/qa/` for local review, but should not be committed by default.

## Boundaries

- Do not log into accounts, upload files, create API keys, or grant browser permissions for this smoke test.
- Do not treat Computer Use as a replacement for Rust/CLI/export CI tests.
- If the browser blocks localhost or the page is blank, fail the smoke and debug the export/server first.
