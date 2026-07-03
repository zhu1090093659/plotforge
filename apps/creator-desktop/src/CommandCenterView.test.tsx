import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CommandCenterView } from "./CommandCenterView";
import {
  demoExportProfiles,
  demoPlayOnceReport,
  demoProjectData,
} from "./demoStudioData";
import { summarizeProject } from "./projectSummary";
import { StudioI18nProvider } from "./i18n";

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
  }
  cleanup();
});

function renderView(overrides: Partial<Parameters<typeof CommandCenterView>[0]> = {}) {
  const props = {
    projectSummary: summarizeProject(demoProjectData),
    projectData: demoProjectData,
    loadedPath: "/tmp/starter-project",
    metrics: [],
    sourceFiles: [
      { path: "game.toml", kind: "toml" as const, bytes: 120, editable: false },
      { path: "world/world.md", kind: "markdown" as const, bytes: 80, editable: true },
    ],
    selectedFile: null,
    playtestInput: "",
    playtesting: false,
    playtestReport: null,
    playtestError: null,
    exportProfiles: demoExportProfiles,
    dirty: false,
    onIntentChange: vi.fn(),
    onRunPlayableProof: vi.fn(),
    onOpenSection: vi.fn(),
    onOpenExportProfile: vi.fn(),
    ...overrides,
  };
  return render(
    <StudioI18nProvider>
      <CommandCenterView {...props} />
    </StudioI18nProvider>,
  );
}

describe("CommandCenterView", () => {
  it("renders the Project Launchpad region with title and status chips", () => {
    renderView();
    expect(
      screen.getByRole("region", { name: "Project Launchpad" }),
    ).toBeTruthy();
    expect(screen.getByText("Starter Project")).toBeTruthy();
    expect(screen.getByText("Project loaded")).toBeTruthy();
    expect(screen.getByText("Workspace synced")).toBeTruthy();
  });

  it("renders the three side-panel tabs (Director, Backend, Evidence)", () => {
    renderView();
    expect(screen.getByRole("tab", { name: /Director/i })).toBeTruthy();
    expect(screen.getByRole("tab", { name: /Backend/i })).toBeTruthy();
    expect(screen.getByRole("tab", { name: /Evidence/i })).toBeTruthy();
  });

  it("shows Director Brief and Recent Runs on the default Director tab", () => {
    renderView();
    // Director is the default active tab.
    expect(screen.getByText("Director Brief")).toBeTruthy();
    expect(screen.getByText("Recent Runs")).toBeTruthy();
    expect(screen.getByText("Loaded path")).toBeTruthy();
  });

  it("shows the empty-state message in Recent Runs when no proof has run", () => {
    renderView({ playtestReport: null });
    expect(screen.getByText("No runs in this session yet.")).toBeTruthy();
    // The previous fake "trace-ready" fallback row must not appear.
    expect(screen.queryByText(/waiting for first proof run/)).toBeNull();
    expect(screen.queryByText("trace-ready")).toBeNull();
  });

  it("shows Backend Commands and Unavailable Agent Interfaces on the Backend tab", () => {
    renderView();
    fireEvent.click(screen.getByRole("tab", { name: /Backend/i }));
    expect(screen.getByText("Backend Commands")).toBeTruthy();
    expect(screen.getByText("Unavailable Agent Interfaces")).toBeTruthy();
    expect(screen.getAllByText("not implemented").length).toBeGreaterThan(0);
  });

  it("shows Evidence Snapshot with evidence cards on the Evidence tab", () => {
    renderView();
    fireEvent.click(screen.getByRole("tab", { name: /Evidence/i }));
    expect(screen.getByText("Evidence Snapshot")).toBeTruthy();
    // Evidence cards have labels Backend, Project truth, Proof, Artifacts.
    expect(screen.getAllByText("Backend").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Artifacts").length).toBeGreaterThan(0);
  });

  it("calls onRunPlayableProof when Run proof is clicked", () => {
    const onRunPlayableProof = vi.fn();
    renderView({ onRunPlayableProof });
    fireEvent.click(screen.getByRole("button", { name: "Run proof" }));
    expect(onRunPlayableProof).toHaveBeenCalledTimes(1);
  });

  it("shows playtest report trace id in Recent Runs after a proof run", () => {
    const report = demoPlayOnceReport("pay the army");
    renderView({ playtestReport: report });
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
  });

  it("renders the localized runtime-proof disclaimer under the Director Command Input", () => {
    renderView();
    expect(
      screen.getByText(
        "Sends this through the local runtime proof command; agent proposal workflows are not implemented.",
      ),
    ).toBeTruthy();
  });

  it("renders metric detail text for open-threads and review-notes branches", () => {
    const summary = summarizeProject(demoProjectData);
    renderView({
      projectSummary: summary,
      metrics: [
        {
          labelKey: "metrics.openThreads",
          value: String(summary.openThreadCount),
          tone: "border-ink/30 text-ink",
        },
        {
          labelKey: "metrics.scenes",
          value: String(summary.sceneCount),
          tone: "border-sage/50 text-sage",
        },
      ],
    });
    // openThreads branch → "{count} promises" with activePromiseCount.
    expect(screen.getByText("1 promises")).toBeTruthy();
    // other-metric branch → "{count} review notes" with unresolvedReviewCount.
    expect(screen.getByText("1 review notes")).toBeTruthy();
  });

  it("renders not-loaded detail when project summary is null", () => {
    renderView({
      projectSummary: null,
      metrics: [
        {
          labelKey: "metrics.scenes",
          value: "0",
          tone: "border-sage/50 text-sage",
        },
      ],
    });
    expect(screen.getAllByText("not loaded").length).toBeGreaterThan(0);
  });
});
