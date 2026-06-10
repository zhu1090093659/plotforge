import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentMeshView } from "./AgentMeshView";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { defaultLocalPreviewState } from "./localPreviewModel";
import { summarizeProject } from "./projectSummary";

afterEach(cleanup);

describe("AgentMeshView", () => {
  it("renders the local/mock agent mesh, ACP setup, capability matrix, and evidence boundary", () => {
    const openTrace = vi.fn();
    const runProof = vi.fn();

    render(
      <AgentMeshView
        projectSummary={summarizeProject(demoProjectData)}
        loadedPath="/tmp/dynasty-embers"
        localPreviewState={defaultLocalPreviewState}
        playtestReport={demoPlayOnceReport("raise emergency taxes")}
        onOpenTrace={openTrace}
        onRunPlayableProof={runProof}
      />,
    );

    expect(screen.getByRole("region", { name: "Agent Mesh Workspace" }))
      .toBeTruthy();
    expect(screen.getByLabelText("ACP Bridge Setup")).toBeTruthy();
    expect(screen.getByLabelText("Mesh Map")).toBeTruthy();
    expect(screen.getByLabelText("Capability Matrix")).toBeTruthy();
    expect(screen.getByLabelText("Bridge Evidence")).toBeTruthy();
    expect(screen.getAllByText("Codex Worker").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Claude Code Worker").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Story Agent").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Rules Agent").length).toBeGreaterThan(0);
    expect(screen.getAllByText("StoryCraft.review").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Rules.patch").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Runtime.playtest").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Assets.generate").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Export.package").length).toBeGreaterThan(0);
    expect(screen.getByText("/config/provider-keys/")).toBeTruthy();
    expect(
      screen.getByText(
        "No ACP network, provider call, credential, upload, or platform publishing behavior is enabled.",
      ),
    ).toBeTruthy();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|real ACP execution|official approval|Steam upload automation/i,
    );

    fireEvent.click(screen.getByRole("button", { name: "Review trace" }));
    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));

    expect(openTrace).toHaveBeenCalledTimes(1);
    expect(runProof).toHaveBeenCalledTimes(1);
  });
});
