# PlotForge Agent-Native UI Reference v1

This folder contains high-fidelity UI references for the next PlotForge Creator Desktop direction.

The core direction is `00-agent-mesh-core.png`: PlotForge should feel like an agent-native AI story game studio, not a traditional game editor, IDE, or admin dashboard.

## Design Principle

PlotForge is an agent-native AI story game studio where a creator directs external coding agents and PlotForge subAgents to build, test, and export playable narrative games.

The UI should emphasize:

- Director intent: the creator states what should change in the game.
- Agent mesh: PlotForge pi-Agent (local, schema-backed) and external coding agents collaborate through explicit capabilities.
- Playable proof: every agent run returns to a playable scene, state changes, validation evidence, and exportable artifacts.
- Human approval: agents propose patches; the creator approves commits.
- Trace-visible execution: runs show trace ids, seeds, prompt versions, files changed, tests, and evidence.

## Visual System

- Foundation: dark graphite desktop app with warm game-canvas surfaces.
- Accent colors: restrained amber for primary action, green for healthy state, cyan for pi-Agent connections, muted purple for secondary agents.
- Layout: thin left navigation, central workspace, right capability/evidence/approval panel, bottom command dock when needed.
- Shape: flat panels with subtle depth and 6-8px radius.
- Typography: crisp, readable hierarchy; no tiny decorative labels.
- Avoid: traditional inspector overload, generic SaaS metrics, full IDE cloning, chat-only UI, fantasy parchment overload, official product logos, automatic publishing claims, legal approval claims, decorative bokeh or gradient orbs.

## Complete Screen Set

Use `00-agent-mesh-core.png` as the master composition reference, then use the remaining screens as implementation-grade examples for each product workflow:

1. `00-agent-mesh-core.png` - Core visual direction for pi-Agent bridge, external coding agents, PlotForge subAgents, capabilities, permissions, and evidence.
2. `01-project-launchpad.png` - Project entry point with recent games, playable proof status, connected agents, project health, and quick director commands.
3. `02-command-center.png` - Natural-language command center with live game canvas, active agent run, approval queue, and evidence snapshot.
4. `03-director-mode.png` - Director-first creative workflow with intent framing, playable scene preview, decision queue, and direction suggestions.
5. `04-pi-agent-bridge-setup.png` - Capability bridge setup for the PlotForge pi-Agent runtime, external coding agents, permission policies, approval boundaries, and trace levels.
6. `05-live-build-room.png` - Live agent production run with orchestration timeline, proposed changes, validation checks, playtest output, and run status.
7. `06-artifact-review.png` - Agent-proposed artifact bundle review across story, rules, assets, risk notes, playtest evidence, and approval actions.
8. `07-playable-proof.png` - Playable result, state deltas, run evidence, artifact diff summary, package readiness, and disclosure drafts.
9. `08-trace-debug.png` - Trace timeline, causality graph, redaction-safe prompt/tool metadata, rule/state diagnostics, and reproducibility fields.
10. `09-export-package.png` - Static web export package readiness, asset whitelist, hash manifest, AI usage disclosure, content warnings, and local smoke evidence.

## Implementation Notes

- Keep `Agent Mesh` as the primary product metaphor.
- The first usable screen should not be a landing page or a traditional dashboard. It should immediately let the creator direct an agent run.
- Files, rules, story contracts, and assets are artifacts manipulated by agents; do not make manual file editing the main product mode.
- Every approval should show what changed, why it changed, how it was validated, and how to play the result.
- External agents may be named as workers, but do not use official logos or imply ownership by third-party products.
