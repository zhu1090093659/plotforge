import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { PiAgentCapability } from "../../../contracts/plotforge";
import { AgentMeshView } from "./AgentMeshView";
import { StudioI18nProvider } from "./i18n";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";

afterEach(cleanup);

const demoPiAgentCapabilities: PiAgentCapability[] = [
  {
    id: "pi-agent.text-generation",
    label: "Text generation",
    status: "wired",
    source: "local-mock-text-provider",
    evidence:
      "FakeTextModelProvider produces validated agent output envelopes.",
  },
  {
    id: "pi-agent.image-generation",
    label: "Image generation",
    status: "not-implemented",
    source: "deferred",
    evidence:
      "Deferred to a later phase; no image provider is wired into the pi-Agent facade.",
  },
  {
    id: "pi-agent.steam-upload",
    label: "Steam upload",
    status: "not-implemented",
    source: "deferred",
    evidence:
      "Steam/Workshop integration remains deferred; no upload automation is implied.",
  },
];

function renderView() {
  const openTrace = vi.fn();
  const runProof = vi.fn();
  const view = render(
    <StudioI18nProvider>
      <AgentMeshView
        projectSummary={summarizeProject(demoProjectData)}
        loadedPath="/tmp/starter-project"
        runtimeName="HTTP dev bridge"
        sourceFiles={[
          { path: "game.toml", kind: "toml", bytes: 120, editable: false },
          { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
        ]}
        assetRecordCount={demoProjectData.asset_records.length}
        exportProfileCount={3}
        playtestReport={demoPlayOnceReport("raise emergency taxes")}
        piAgentCapabilities={demoPiAgentCapabilities}
        onOpenTrace={openTrace}
        onRunPlayableProof={runProof}
      />
    </StudioI18nProvider>,
  );
  return { view, openTrace, runProof };
}

describe("AgentMeshView", () => {
  it("renders real Studio command capabilities and explicit unavailable agent boundaries", () => {
    const { openTrace, runProof } = renderView();

    expect(screen.getByRole("region", { name: "Agent Mesh Workspace" }))
      .toBeTruthy();
    expect(screen.getByLabelText("Studio Backend Bridge")).toBeTruthy();
    expect(screen.getByLabelText("Command Boundary Map")).toBeTruthy();
    expect(screen.getByLabelText("Capability Matrix")).toBeTruthy();
    expect(screen.getByLabelText("Bridge Evidence")).toBeTruthy();
    expect(screen.getByText("Project open/check")).toBeTruthy();
    expect(screen.getByText("Runtime proof")).toBeTruthy();
    expect(screen.getByText("Static export zip")).toBeTruthy();
    // pi-Agent runtime capability now comes from the real
    // pi_agent_capabilities command; the local mock lists text-generation
    // as wired and image-generation/steam-upload as not-implemented.
    expect(screen.getByText("Text generation")).toBeTruthy();
    expect(screen.getAllByText("not implemented").length).toBeGreaterThan(0);
    expect(screen.queryByText("Codex Worker")).toBeNull();
    expect(screen.queryByText("Claude Code Worker")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|official approval|Steam upload automation|real pi-Agent execution|real external agent execution/i,
    );

    fireEvent.click(screen.getByRole("button", { name: "Review trace" }));
    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));

    expect(openTrace).toHaveBeenCalledTimes(1);
    expect(runProof).toHaveBeenCalledTimes(1);
  });

  it("renders the capability matrix as responsive cards without a fixed-width table", () => {
    renderView();

    // No fixed-width scrolling table: every capability is an individual card.
    const matrix = screen.getByLabelText("Capability Matrix");
    expect(matrix.querySelector("table")).toBeNull();
    // 5 static Studio-command capabilities + 3 pi-agent capabilities.
    expect(matrix.querySelectorAll("article").length).toBe(8);
    // Field labels render per card so the matrix reads on narrow screens.
    expect(screen.getAllByText("Real source").length).toBe(8);
  });

  it("keeps the Removed Fake Surfaces technical section collapsed by default", () => {
    renderView();

    const toggle = screen.getByRole("button", {
      name: /Removed Fake Surfaces/,
    });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    // Technical boundary detail is hidden until the creator opts in.
    expect(
      screen.queryByText("No mock external workers or mock connected state."),
    ).toBeNull();

    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(
      screen.getByText("No mock external workers or mock connected state."),
    ).toBeTruthy();
  });
});
