import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LaunchpadView } from "./LaunchpadView";
import { demoProjectData } from "./demoStudioData";
import { summarizeProject } from "./projectSummary";
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
    metrics: [
      { labelKey: "metrics.scenes", value: "2", tone: "border-sage/50 text-sage" },
    ],
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
    onOpenSection: vi.fn(),
    onCreateProject: vi.fn(),
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
  it("renders the project overview region with title and metrics", () => {
    renderLaunchpad();

    expect(
      screen.getByRole("region", { name: /Project overview/i }),
    ).toBeTruthy();
    expect(screen.getByText("Starter Project")).toBeTruthy();
    expect(screen.getByText("/tmp/starter-project")).toBeTruthy();
    expect(screen.getByText("Project loaded")).toBeTruthy();
    // Metric renders.
    expect(screen.getByText("2")).toBeTruthy();
  });

  it("shows the New Project form fields by default (first tab)", () => {
    renderLaunchpad();

    expect(screen.getByLabelText("New project path")).toBeTruthy();
    expect(screen.getByLabelText("Visual style")).toBeTruthy();
    expect(screen.getByLabelText("Concept")).toBeTruthy();
    expect(screen.getByLabelText("Initial scene request")).toBeTruthy();
    expect(screen.getByRole("checkbox", { name: "Voice enabled" })).toBeTruthy();
    expect(screen.getByRole("checkbox", { name: "Overwrite existing path" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: /New Project/i })).toBeTruthy();
  });

  it("exposes the New Project and Project Health tabs", () => {
    renderLaunchpad();

    expect(screen.getByRole("tab", { name: /New Project/i })).toBeTruthy();
    expect(screen.getByRole("tab", { name: /Project Health/i })).toBeTruthy();
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

  it("shows real boundary checks from checkReport, not hardcoded pass", () => {
    const checkReport: ProjectCheckReport = {
      title: "My Project",
      entry_scene: "opening-scene",
      scene_count: 3,
      rule_count: 5,
      character_count: 2,
    };
    renderLaunchpad({ checkReport });

    fireEvent.click(screen.getByRole("tab", { name: /Project Health/ }));

    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(screen.getByText("My Project")).toBeTruthy();
    expect(screen.getAllByText("opening-scene").length).toBeGreaterThan(0);
    expect(screen.getByText("3")).toBeTruthy(); // scene_count
    expect(screen.getByText("5")).toBeTruthy(); // rule_count
  });

  it("shows the no-project boundary-check state when no project is loaded", () => {
    renderLaunchpad({ checkReport: null, projectData: null, loadedPath: "" });

    fireEvent.click(screen.getByRole("tab", { name: /Project Health/ }));

    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(screen.getByText("Open a project to run boundary checks.")).toBeTruthy();
  });

  it("shows a pending boundary-check state when a project is loaded but no checkReport is available", () => {
    renderLaunchpad({ checkReport: null });

    fireEvent.click(screen.getByRole("tab", { name: /Project Health/ }));

    expect(screen.getByText("Boundary Checks")).toBeTruthy();
    expect(
      screen.getByText(
        "Boundary checks pending — run check or reload the project.",
      ),
    ).toBeTruthy();
    expect(screen.queryByText("Open a project to run boundary checks.")).toBeNull();
  });

  it("calls onOpenSection when the Open Play / Open Export buttons are clicked", () => {
    const onOpenSection = vi.fn();
    renderLaunchpad({ onOpenSection });

    fireEvent.click(screen.getByRole("button", { name: /^Play$/i }));
    expect(onOpenSection).toHaveBeenCalledWith("play");

    fireEvent.click(screen.getByRole("button", { name: /^Export$/i }));
    expect(onOpenSection).toHaveBeenCalledWith("export-kit");
  });
});
