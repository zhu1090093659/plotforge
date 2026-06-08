# Codex Desktop Browser and Computer Use Creator Desktop Smoke

This runbook verifies the PlotForge Creator Desktop Studio in a real local desktop session. It complements `npm run creator-desktop:qa`; it is intentionally local-only and must not become a GitHub Actions dependency.

## Prepare the Studio

Run from the repository root:

```bash
npm run creator-desktop:qa
npm run creator-desktop:dev
```

Use the printed Vite URL, normally `http://127.0.0.1:5173/`. Do not append host arguments to the root `creator-desktop:dev` wrapper. If a smoke run specifically needs a direct workspace command, use:

```bash
npm --workspace @plotforge/creator-desktop run dev -- --host 127.0.0.1
```

## Browser or Computer Use Steps

1. Open the Vite URL in the Codex in-app Browser or a local browser controlled through Computer Use.
2. Verify visible Studio shell content:
   - Sidebar label: `PLOTFORGE STUDIO`
   - Project title: `Dynasty Embers`
   - Panels: `Playtest`, `Runtime Trace`, `Source Files`, `Source Editor`
   - Source list entry: `world/world.md`
3. Click `Run turn`.
4. Verify trace and playtest output:
   - Trace id: `trace-001`
   - Status badge: `COMMITTED`
   - Review section: `Narrative Review`
   - Diagnostics section: `Diagnostics`
   - World delta includes `treasury: +12`
5. Inspect browser console output. Any uncaught error, failed Vite asset, or blank page fails the smoke.
6. Repeat at a narrow mobile viewport, for example 390 x 844, and verify `Playtest` and `Runtime Trace` remain reachable after `Run turn`.

## Evidence

Record the URL, viewport, clicked command, visible trace id, review section, diagnostics section, and whether console errors were present. Screenshots may be kept under `artifacts/qa/` for local review, but should not be committed by default.

## Boundaries

- Starting the Vite server is not evidence; perform at least one visible browser/app interaction.
- Do not log into accounts, upload files, create API keys, or grant browser permissions for this smoke test.
- Do not use `file://`; Creator Desktop is validated through the Vite dev server or Tauri dev shell.
- Do not treat Browser or Computer Use smoke as a replacement for TypeScript, Vitest, build, Tauri, Rust, CLI, and export checks.
