import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ArtifactReviewView } from "./ArtifactReviewView";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";
import { projectAssetCatalog } from "./studioModel";

afterEach(cleanup);

describe("ArtifactReviewView", () => {
  it("renders real source, asset, runtime, and export evidence without fake build bundles", () => {
    const openTrace = vi.fn();
    const runProof = vi.fn();

    render(
      <ArtifactReviewView
        projectSummary={summarizeProject(demoProjectData)}
        loadedPath="/tmp/dynasty-embers"
        sourceFiles={[
          { path: "game.toml", kind: "toml", bytes: 120, editable: false },
          { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
        ]}
        assetCatalog={projectAssetCatalog(demoProjectData, demoProjectData.asset_records)}
        playtestReport={demoPlayOnceReport("raise emergency taxes")}
        playtesting={false}
        playtestError={null}
        exportReport={{
          output_dir: "/tmp/export",
          archive_path: "/tmp/export.zip",
          files_written: ["/tmp/export/index.html", "/tmp/export/game.json"],
          archived_files: ["index.html", "game.json"],
          allowed_files: ["index.html", "game.json"],
          files_found: ["index.html", "game.json"],
        }}
        assetMaintenance={<div>Asset Maintenance Slot</div>}
        onOpenTrace={openTrace}
        onRunPlayableProof={runProof}
      />,
    );

    expect(screen.getByRole("region", { name: "Artifact Review Workspace" }))
      .toBeTruthy();
    expect(screen.getByLabelText("Live Build Room")).toBeTruthy();
    expect(screen.getByText("No build run interface")).toBeTruthy();
    expect(screen.getByText("Current Source Artifacts")).toBeTruthy();
    expect(screen.getAllByText("world/world.md").length).toBeGreaterThan(0);
    expect(screen.getByText("Runtime Impact")).toBeTruthy();
    expect(screen.getByText("Export Evidence")).toBeTruthy();
    expect(screen.getByLabelText("Validation Evidence")).toBeTruthy();
    expect(screen.getByText("Project source")).toBeTruthy();
    expect(screen.getByText("Asset registry")).toBeTruthy();
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    expect(screen.getByText("/tmp/export.zip")).toBeTruthy();
    expect(screen.getByText("Asset Maintenance Slot")).toBeTruthy();
    expect(screen.queryByText("Court Crisis Pressure Bundle")).toBeNull();
    expect(screen.queryByText("Approve Bundle")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|Steam upload automation|real ACP execution/i,
    );

    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));
    fireEvent.click(screen.getByRole("button", { name: "View trace" }));

    expect(runProof).toHaveBeenCalledTimes(1);
    expect(openTrace).toHaveBeenCalledTimes(1);
  });
});
