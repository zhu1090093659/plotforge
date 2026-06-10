import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ArtifactReviewView } from "./ArtifactReviewView";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { defaultLocalPreviewState } from "./localPreviewModel";
import { summarizeProject } from "./projectSummary";
import { projectAssetCatalog } from "./studioModel";

afterEach(cleanup);

describe("ArtifactReviewView", () => {
  it("renders live build room, artifact bundle, approval queue, validation evidence, and playable impact", () => {
    const openTrace = vi.fn();
    const runProof = vi.fn();

    render(
      <ArtifactReviewView
        projectSummary={summarizeProject(demoProjectData)}
        loadedPath="/tmp/dynasty-embers"
        assetCatalog={projectAssetCatalog(demoProjectData, demoProjectData.asset_records)}
        localPreviewState={defaultLocalPreviewState}
        playtestReport={demoPlayOnceReport("raise emergency taxes")}
        playtesting={false}
        playtestError={null}
        assetMaintenance={<div>Asset Maintenance Slot</div>}
        onOpenTrace={openTrace}
        onRunPlayableProof={runProof}
      />,
    );

    expect(screen.getByRole("region", { name: "Artifact Review Workspace" }))
      .toBeTruthy();
    expect(screen.getByLabelText("Live Build Room")).toBeTruthy();
    expect(screen.getByText("Run Timeline")).toBeTruthy();
    expect(screen.getByText("Agent Work Status")).toBeTruthy();
    expect(screen.getByText("Approval Queue")).toBeTruthy();
    expect(screen.getByText("Court Crisis Pressure Bundle")).toBeTruthy();
    expect(screen.getByText("Story Contract")).toBeTruthy();
    expect(screen.getByText("Rule Patch")).toBeTruthy();
    expect(screen.getByText("Scene Asset")).toBeTruthy();
    expect(screen.getByText("Risk Notes")).toBeTruthy();
    expect(screen.getByText("Files Changed")).toBeTruthy();
    expect(screen.getAllByText("chapters/02.md").length).toBeGreaterThan(0);
    expect(screen.getAllByText("rules/suspicion.yml").length).toBeGreaterThan(0);
    expect(screen.getAllByText("assets/generated/court-crisis-001.png").length)
      .toBeGreaterThan(0);
    expect(screen.getByText("Playable Impact")).toBeTruthy();
    expect(screen.getByLabelText("Validation Evidence")).toBeTruthy();
    expect(screen.getByText("Schema valid")).toBeTruthy();
    expect(screen.getByText("Links and references")).toBeTruthy();
    expect(screen.getByText("Determinism")).toBeTruthy();
    expect(screen.getByText("Safety boundary")).toBeTruthy();
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    expect(screen.getByText("Asset Maintenance Slot")).toBeTruthy();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|Steam upload automation|real ACP execution/i,
    );

    fireEvent.click(screen.getByRole("button", { name: "Request test" }));
    fireEvent.click(screen.getByRole("button", { name: "View trace" }));
    fireEvent.click(screen.getByRole("button", { name: "Approve Bundle" }));
    expect(
      screen.getByText(
        "Approve Bundle recorded locally; no files committed or persisted.",
      ),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Request revision" }));
    expect(
      screen.getByText(
        "Request revision recorded locally; artifact bundle remains uncommitted.",
      ),
    ).toBeTruthy();

    expect(runProof).toHaveBeenCalledTimes(1);
    expect(openTrace).toHaveBeenCalledTimes(1);
  });
});
