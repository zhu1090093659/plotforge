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
2. Verify visible agent-native Studio shell content:
   - Sidebar label: `PLOTFORGE STUDIO`
   - Project title: `Dynasty Embers`
   - Primary workflow: `Command Center`
   - Main region: `Project Launchpad`
   - Bottom region: `Command Dock`
3. Click `Run playable proof` in the command dock, or enter a director intent and click `Apply as proof run`.
4. Verify playable proof and trace output:
   - Section: `Playable Proof`
   - Section: `Trace Debug`
   - Trace id: `trace-001`
   - Evidence fields: `Run seed`, `Prompt version`, `Provider Config Hash`
   - World delta or package readiness evidence is visible.
5. Open `Agent Mesh` and verify `Studio Backend Bridge`, `Command Boundary Map`, `Capability Matrix`, and real Studio command boundary copy are visible.
   - Wired capabilities should describe folder project source, structured editing, source file read/write, runtime proof, and static export.
   - Unavailable interfaces such as ACP/external agent bridge, approval queues, provider calls, and publishing automation must remain explicitly `not implemented`.
6. Open `Export Package` and verify:
   - `Local export package only`
   - `Package Readiness`
   - `Evidence & Boundaries`
   - `Export zip` is present for `static-web`
   - Draft profiles such as `steam-workshop` remain metadata-only and do not run the static export command.
7. Inspect browser console output. Any uncaught error, failed Vite asset, or blank page fails the smoke.
8. Repeat at a narrow mobile viewport, for example 390 x 844, and verify `Command Center`, `Agent Mesh`, `Playable Proof`, `Trace Debug`, and `Export Package` remain reachable without text overlap blocking the primary controls.

## Evidence

Record the URL, viewport, clicked command, visible trace id, proof section, trace section, Agent Mesh backend boundary, export package boundary, and whether console errors were present. Screenshots may be kept under `artifacts/qa/` for local review, but should not be committed by default.

## Boundaries

- Starting the Vite server is not evidence; perform at least one visible browser/app interaction.
- Do not log into accounts, upload files, create API keys, or grant browser permissions for this smoke test.
- Do not use `file://`; Creator Desktop is validated through the Vite dev server or Tauri dev shell.
- Vite browser mode calls the local HTTP dev bridge backed by `plotforge-cli studio`; this is local command execution, not an in-browser fake source.
- Do not treat Browser or Computer Use smoke as a replacement for TypeScript, Vitest, build, Tauri, Rust, CLI, and export checks.
- Forbidden copy includes automatic publishing, legal/approval guarantees, official-logo ownership claims, real ACP execution claims, hidden network/model execution claims, raw provider responses, provider credentials, and secret marker text.
