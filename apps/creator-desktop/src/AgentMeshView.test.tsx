import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentMeshView } from "./AgentMeshView";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";

afterEach(cleanup);

describe("AgentMeshView", () => {
  it("renders real Studio command capabilities and explicit unavailable agent boundaries", () => {
    const openTrace = vi.fn();
    const runProof = vi.fn();

    render(
      <AgentMeshView
        projectSummary={summarizeProject(demoProjectData)}
        loadedPath="/tmp/dynasty-embers"
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

    expect(screen.getByRole("region", { name: "Agent Mesh Workspace" }))
      .toBeTruthy();
    expect(screen.getByLabelText("Studio Backend Bridge")).toBeTruthy();
    expect(screen.getByLabelText("Command Boundary Map")).toBeTruthy();
    expect(screen.getByLabelText("Capability Matrix")).toBeTruthy();
    expect(screen.getByLabelText("Bridge Evidence")).toBeTruthy();
    expect(screen.getByText("Project open/check")).toBeTruthy();
    expect(screen.getByText("Runtime proof")).toBeTruthy();
    expect(screen.getByText("Static export zip")).toBeTruthy();
    expect(screen.getByText("ACP / external agent bridge")).toBeTruthy();
    expect(screen.getAllByText("not implemented").length).toBeGreaterThan(0);
    expect(screen.queryByText("Codex Worker")).toBeNull();
    expect(screen.queryByText("Claude Code Worker")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|real ACP execution|official approval|Steam upload automation/i,
    );

    fireEvent.click(screen.getByRole("button", { name: "Review trace" }));
    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));

    expect(openTrace).toHaveBeenCalledTimes(1);
    expect(runProof).toHaveBeenCalledTimes(1);
  });
});
