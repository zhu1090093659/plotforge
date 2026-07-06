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
   - Project title area starts as `No project loaded`
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
   - Unavailable interfaces such as external agent execution (non-pi-Agent), approval queues, hidden network provider calls, and publishing automation must remain explicitly `not implemented`. The internal pi-Agent runtime is wired (local, schema-backed).
6. Open `Export Package` and verify:
   - `Local export package only`
   - `Package Readiness`
   - `Evidence & Boundaries`
   - `Export zip` is present for `static-web`
   - Draft profiles such as `steam-workshop` remain metadata-only and do not run the static export command.
7. Inspect browser console output. Any uncaught error, failed Vite asset, or blank page fails the smoke.
8. Repeat at a narrow mobile viewport, for example 390 x 844, and verify `Command Center`, `Agent Mesh`, `Playable Proof`, `Trace Debug`, and `Export Package` remain reachable without text overlap blocking the primary controls.

## MCP Server Management Smoke (Phase 6)

This smoke verifies the real MCP server management surface in Settings → MCP tab. It is local-only and must not become a CI dependency.

1. Navigate to Settings (sidebar item #12) → MCP tab.
2. Verify the real server management surface renders (not the old "coming in a future release" placeholder):
   - An `Add server` button is visible.
   - The empty state `No MCP servers registered. Add a server to begin.` is shown when no servers are registered.
3. Click `Add server`, fill the form:
   - Server id: `local-fs-smoke`
   - Label: `Smoke FS`
   - Transport: `Stdio`
   - Command: a real or stub MCP server binary (e.g. `npx -y @modelcontextprotocol/server-memory`).
   - Credential env var: `MCP_SMOKE_TOKEN` (the input must be a plain text field, never a password field).
4. Save the entry. Verify it appears in the server list with the stdio transport kind badge.
5. Click `Test connection`. Verify a `McpServerTestResult` renders (ok/failed + tools count). A failed connection (binary not found) must surface an explicit error, not a silent no-op.
6. Toggle `Enable for this project` on. Verify the toggle persists (reload the project; the toggle remains on).
7. Open the tool list (collapsible under the server row after a successful test). Verify tool names + descriptions render. Click `Invoke` on a tool; verify a redacted `McpToolCallResult` renders (no raw tool bodies or secret markers).
8. Run a turn via the Agent Chat Rail (`describe a change / run a turn`) with the MCP server enabled. Verify the turn completes and the Evidence popover carries `mcp_tool_call_hash` (redaction-safe) — no raw tool arguments or results leak into the trace.

## Evidence

Record the URL, viewport, clicked command, visible trace id, proof section, trace section, Agent Mesh backend boundary, export package boundary, and whether console errors were present. Screenshots may be kept under `artifacts/qa/` for local review, but should not be committed by default.

## Boundaries

- Starting the Vite server is not evidence; perform at least one visible browser/app interaction.
- Do not log into accounts, upload files, create API keys, or grant browser permissions for this smoke test.
- Do not use `file://`; Creator Desktop is validated through the Vite dev server or Tauri dev shell.
- Vite browser mode calls the local HTTP dev bridge backed by `plotforge-cli studio`; this is local command execution, not an in-browser fake source.
- Do not treat Browser or Computer Use smoke as a replacement for TypeScript, Vitest, build, Tauri, Rust, CLI, and export checks.
- Forbidden copy includes automatic publishing, legal/approval guarantees, official-logo ownership claims, real external agent execution claims (pi-Agent is local and allowed), hidden network/model execution claims, raw provider responses, provider credentials, and secret marker text.
