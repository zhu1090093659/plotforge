import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentMeshView } from "./AgentMeshView";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";

afterEach(cleanup);

function renderView() {
  const openTrace = vi.fn();
  const runProof = vi.fn();
  const view = render(
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
      onOpenTrace={openTrace}
      onRunPlayableProof={runProof}
    />,
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
    expect(screen.getByText("pi-Agent runtime")).toBeTruthy();
    expect(screen.getAllByText("not implemented").length).toBeGreaterThan(0);
    expect(screen.queryByText("Codex Worker")).toBeNull();
    expect(screen.queryByText("Claude Code Worker")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|real pi-Agent execution|official approval|Steam upload automation/i,
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
    expect(matrix.querySelectorAll("article").length).toBe(6);
    // Field labels render per card so the matrix reads on narrow screens.
    expect(screen.getAllByText("Real source").length).toBe(6);
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
