import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LaunchpadView } from "./LaunchpadView";
import { demoExportProfiles, demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";
import { projectAssetCatalog } from "./assetCatalog";
import { createDefaultStudioDataSource } from "./studioDataSource";
import type { ProjectCreationRequest, ProjectTemplateId } from "../../../contracts/plotforge";
import type { ProjectCheckReport } from "./tauriBridge";
import { StudioI18nProvider } from "./i18n";

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
  }
  cleanup();
});

// Minimal props builder so tests don't need to repeat everything
function baseProps(overrides: Partial<Parameters<typeof LaunchpadView>[0]> = {}) {
  const checkReport: ProjectCheckReport = {
    title: "Starter Project",
    entry_scene: "opening-scene",
    scene_count: 2,
    rule_count: 3,
    character_count: 4,
  };

  return {
    projectSummary: summarizeProject(demoProjectData),
    projectData: demoProjectData,
    loadedPath: "/tmp/starter-project",
    checkReport,
    metrics: [],
    sourceFiles: [
      { path: "game.toml", kind: "toml" as const, bytes: 120, editable: false },
      { path: "world/world.md", kind: "markdown" as const, bytes: 80, editable: true },
    ],
    selectedFile: null,
    dirty: false,
    playtestInput: "Raise emergency taxes.",
    setPlaytestInput: vi.fn(),
    playtesting: false,
    playtestReport: null,
    playtestError: null,
    exportProfiles: demoExportProfiles,
    exportReport: null,
    assetCatalog: projectAssetCatalog(demoProjectData, demoProjectData.asset_records),
    createProjectPath: "/tmp/new-project",
    setCreateProjectPath: vi.fn(),
    createTemplate: "historical_crisis" as ProjectTemplateId,
    setCreateTemplate: vi.fn(),
    createConcept: "",
    setCreateConcept: vi.fn(),
    createVisualStyle: "",
    setCreateVisualStyle: vi.fn(),
    createVoiceEnabled: false,
    setCreateVoiceEnabled: vi.fn(),
    createInitialSceneRequest: "",
    setCreateInitialSceneRequest: vi.fn(),
    createForce: false,
    setCreateForce: vi.fn(),
    createReport: null,
    creating: false,
    createError: null,
    onRunPlayableProof: vi.fn(),
    onOpenSection: vi.fn(),
    onOpenExportProfile: vi.fn(),
    onCreateProject: vi.fn(),
    dataSource: createDefaultStudioDataSource(),
    ...overrides,
  };
}

function renderLaunchpad(overrides: Partial<Parameters<typeof LaunchpadView>[0]> = {}) {
  return render(
    <StudioI18nProvider>
      <LaunchpadView {...baseProps(overrides)} />
    </StudioI18nProvider>,
  );
}

describe("LaunchpadView", () => {
  it("renders the Command Center region and core CTAs", () => {
    renderLaunchpad();

    // Command Center region should be visible
    expect(screen.getByRole("region", { name: "Project Launchpad" })).toBeTruthy();
    // Run proof button accessible
    expect(screen.getByRole("button", { name: "Run proof" })).toBeTruthy();
  });

  it("shows the New Project form fields by default (first tab)", () => {
    renderLaunchpad();

    // The New Project tab is selected by default, so the form fields are visible.
    expect(screen.getByLabelText("New project path")).toBeTruthy();
    expect(screen.getByLabelText("Visual style")).toBeTruthy();
    expect(screen.getByLabelText("Concept")).toBeTruthy();
    expect(screen.getByLabelText("Initial scene request")).toBeTruthy();
    // The tab button should exist too.
    expect(screen.getByRole("tab", { name: /New Project/i })).toBeTruthy();
  });

  it("exposes the New Project and Project Health tabs (source moved to Source view)", () => {
    renderLaunchpad();

    expect(screen.getByRole("tab", { name: /New Project/i })).toBeTruthy();
    expect(screen.getByRole("tab", { name: /Project Health/i })).toBeTruthy();
    // Source browsing/editing moved to the dedicated Source view (Phase B).
    expect(screen.queryByRole("tab", { name: /Source Artifacts/i })).toBeNull();
    expect(screen.queryByRole("tab", { name: /Artifact Text Editor/i })).toBeNull();
  });

  it("calls onCreateProject when the create form is submitted", async () => {
    const onCreateProject = vi.fn();
    renderLaunchpad({
      onCreateProject,
      createProjectPath: "/tmp/my-game",
      createConcept: "A frozen council crisis.",
      createVisualStyle: "ink wash",
      createInitialSceneRequest: "Open with a sealed edict.",
    });

    // New Project tab is active by default; submit directly.
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));

    await waitFor(() => {
      expect(onCreateProject).toHaveBeenCalledWith(
        "/tmp/my-game",
        expect.objectContaining({
          concept: "A frozen council crisis.",
          visual_style: "ink wash",
          initial_scene_request: "Open with a sealed edict.",
        }),
        false,
      );
    });
  });

  it("calls onRunPlayableProof when the run proof button is clicked", () => {
    const onRunPlayableProof = vi.fn();
    renderLaunchpad({ onRunPlayableProof });

    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));
    expect(onRunPlayableProof).toHaveBeenCalledTimes(1);
  });

  it("shows real boundary checks from checkReport, not hardcoded pass", () => {
    const checkReport: ProjectCheckReport = {
      title: "My Project",
      entry_scene: "opening-scene",
      scene_count: 3,
      rule_count: 5,
      character_count: 2,
    };
    renderLaunchpad({ checkReport });

    // Switch to the Project Health tab to see the boundary checks.
    fireEvent.click(screen.getByRole("tab", { name: /Project Health/ }));

    // Should display real data from checkReport (not hardcoded strings)
    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(screen.getByText("My Project")).toBeTruthy();
    expect(screen.getAllByText("opening-scene").length).toBeGreaterThan(0);
    expect(screen.getByText("3")).toBeTruthy(); // scene_count
    expect(screen.getByText("5")).toBeTruthy(); // rule_count
  });

  it("shows the no-project boundary-check state when no project is loaded", () => {
    renderLaunchpad({ checkReport: null, projectData: null, loadedPath: "" });

    // Switch to the Project Health tab.
    fireEvent.click(screen.getByRole("tab", { name: /Project Health/ }));

    // No project loaded -> no fake "passed" checks; show the empty state.
    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(screen.getByText("Open a project to run boundary checks.")).toBeTruthy();
    expect(screen.queryByText("Generated contracts")).toBeNull();
    expect(screen.queryByText("Tauri bridge")).toBeNull();
  });

  it("shows a pending boundary-check state when a project is loaded but no checkReport is available", () => {
    renderLaunchpad({ checkReport: null });

    // Switch to the Project Health tab.
    fireEvent.click(screen.getByRole("tab", { name: /Project Health/ }));

    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(
      screen.getByText(
        "Boundary checks pending — run check or reload the project.",
      ),
    ).toBeTruthy();
    // The no-project message must not appear when a project is loaded.
    expect(screen.queryByText("Open a project to run boundary checks.")).toBeNull();
  });

  it("renders a playtest report result when proof has been run", () => {
    const report = demoPlayOnceReport("pay the army");
    renderLaunchpad({ playtestReport: report });

    // The recent runs area should show the trace
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
  });
});
