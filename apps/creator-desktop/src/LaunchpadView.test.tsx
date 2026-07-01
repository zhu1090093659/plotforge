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
    editorContent: "",
    setEditorContent: vi.fn(),
    dirty: false,
    saving: false,
    error: null,
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
    onSelectSourceFile: vi.fn(),
    onSaveSelectedFile: vi.fn(),
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

  it("does not show the New Project form fields by default (collapsed)", () => {
    renderLaunchpad();

    // The New Project section should be collapsed
    expect(screen.queryByLabelText("New project path")).toBeNull();
    expect(screen.queryByLabelText("Visual style")).toBeNull();
    expect(screen.queryByLabelText("Concept")).toBeNull();
    expect(screen.queryByLabelText("Initial scene request")).toBeNull();
    // The toggle button should exist though
    expect(screen.getByRole("button", { name: "New Project" })).toBeTruthy();
  });

  it("expands the New Project form when the toggle button is clicked", () => {
    renderLaunchpad();

    // Initially hidden
    expect(screen.queryByLabelText("New project path")).toBeNull();

    // Click to expand
    fireEvent.click(screen.getByRole("button", { name: "New Project" }));

    // Form fields are now visible
    expect(screen.getByLabelText("New project path")).toBeTruthy();
    expect(screen.getByLabelText("Visual style")).toBeTruthy();
    expect(screen.getByLabelText("Concept")).toBeTruthy();
    expect(screen.getByLabelText("Initial scene request")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Create project" })).toBeTruthy();
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

    // Expand the form
    fireEvent.click(screen.getByRole("button", { name: "New Project" }));
    // Submit
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

    // Expand the Source Artifacts collapsible to see boundary checks
    const sourceArtifactsToggle = screen.getByRole("button", { name: /Source Artifacts/ });
    fireEvent.click(sourceArtifactsToggle);

    // Should display real data from checkReport (not hardcoded strings)
    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(screen.getByText("My Project")).toBeTruthy();
    expect(screen.getAllByText("opening-scene").length).toBeGreaterThan(0);
    expect(screen.getByText("3")).toBeTruthy(); // scene_count
    expect(screen.getByText("5")).toBeTruthy(); // rule_count
  });

  it("shows fallback hardcoded checks when no checkReport is available", () => {
    renderLaunchpad({ checkReport: null });

    // Expand the Source Artifacts section
    fireEvent.click(screen.getByRole("button", { name: /Source Artifacts/ }));

    // Should show the hardcoded fallback checks
    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(screen.getByText("Generated contracts")).toBeTruthy();
    expect(screen.getByText("plotforge.d.ts")).toBeTruthy();
    expect(screen.getByText("Tauri bridge")).toBeTruthy();
  });

  it("shows source file list when Source Artifacts is expanded", () => {
    renderLaunchpad({
      sourceFiles: [
        { path: "game.toml", kind: "toml" as const, bytes: 120, editable: false },
        { path: "world/world.md", kind: "markdown" as const, bytes: 80, editable: true },
      ],
    });

    // Initially not visible
    expect(screen.queryByText("game.toml")).toBeNull();

    // Expand
    fireEvent.click(screen.getByRole("button", { name: /Source Artifacts/ }));

    expect(screen.getByText("game.toml")).toBeTruthy();
    expect(screen.getByText("world/world.md")).toBeTruthy();
  });

  it("calls onSelectSourceFile when a source file is clicked", () => {
    const onSelectSourceFile = vi.fn();
    renderLaunchpad({ onSelectSourceFile });

    fireEvent.click(screen.getByRole("button", { name: /Source Artifacts/ }));
    fireEvent.click(screen.getByRole("button", { name: /game\.toml/ }));

    expect(onSelectSourceFile).toHaveBeenCalledWith(
      expect.objectContaining({ path: "game.toml" }),
    );
  });

  it("shows the source editor when a file is selected", () => {
    renderLaunchpad({
      selectedFile: {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n",
      },
      editorContent: "# World Bible content",
    });

    // Source editor section should be visible (defaultOpen=true when file selected)
    const editor = screen.getByLabelText("Source editor");
    expect(editor).toBeTruthy();
    expect((editor as HTMLTextAreaElement).value).toBe("# World Bible content");
  });

  it("calls onSaveSelectedFile when Save is clicked with dirty state", () => {
    const onSaveSelectedFile = vi.fn();
    renderLaunchpad({
      onSaveSelectedFile,
      selectedFile: {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n",
      },
      editorContent: "# Modified content\n",
      dirty: true,
    });

    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSaveSelectedFile).toHaveBeenCalledTimes(1);
  });

  it("renders a playtest report result when proof has been run", () => {
    const report = demoPlayOnceReport("pay the army");
    renderLaunchpad({ playtestReport: report });

    // The recent runs area should show the trace
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
  });
});
