import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ArtifactReviewView } from "./ArtifactReviewView";
import { StudioI18nProvider } from "./i18n";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";
import { projectAssetCatalog } from "./assetCatalog";

afterEach(cleanup);

describe("ArtifactReviewView", () => {
  it("renders real source, asset, runtime, and export evidence without fake build bundles", () => {
    const openTrace = vi.fn();
    const runProof = vi.fn();

    render(
      <StudioI18nProvider>
        <ArtifactReviewView
          projectSummary={summarizeProject(demoProjectData)}
          loadedPath="/tmp/starter-project"
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
          onOpenTrace={openTrace}
          onRunPlayableProof={runProof}
        />
      </StudioI18nProvider>,
    );

    expect(screen.getByRole("region", { name: "Artifact Review Workspace" }))
      .toBeTruthy();
    expect(screen.getByLabelText("Live Build Room")).toBeTruthy();
    expect(screen.getByText("No build run interface")).toBeTruthy();
    expect(screen.getByText("Current Source Artifacts")).toBeTruthy();
    expect(screen.getAllByText("world/world.md").length).toBeGreaterThan(0);
    // Runtime Impact is the default evidence tab.
    expect(screen.getByRole("tab", { name: /Runtime Impact/i })).toBeTruthy();
    expect(screen.getByLabelText("Validation Evidence")).toBeTruthy();
    expect(screen.getByText("Project source")).toBeTruthy();
    expect(screen.getByText("Asset registry")).toBeTruthy();
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    // Export Evidence is behind its tab; switch to it to confirm the archive path.
    fireEvent.click(screen.getByRole("tab", { name: /Export Evidence/i }));
    expect(screen.getByText("/tmp/export.zip")).toBeTruthy();
    // AssetMaintenanceView is now rendered directly (ReactNode injection removed); slot text no longer applies.
    expect(screen.queryByText("Council Crisis Pressure Bundle")).toBeNull();
    expect(screen.queryByText("Approve Bundle")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|Steam upload automation|real pi-Agent execution|real external agent execution/i,
    );

    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));
    fireEvent.click(screen.getByRole("button", { name: "View trace" }));

    expect(runProof).toHaveBeenCalledTimes(1);
    expect(openTrace).toHaveBeenCalledTimes(1);
  });
});
