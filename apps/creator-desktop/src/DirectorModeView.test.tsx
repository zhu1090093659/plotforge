import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  demoPlayOnceReport,
  demoProjectData,
} from "./demoStudioData";
import { DirectorModeView } from "./DirectorModeView";
import { StudioI18nProvider } from "./i18n";

afterEach(cleanup);


function renderView(ui: React.ReactElement) {
  return render(ui, { wrapper: StudioI18nProvider });
}

describe("DirectorModeView", () => {
  it("renders a director-first playable scene preview and real runtime boundary", () => {
    const callbacks = directorCallbacks();

    renderView(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/starter-project"
        input="Make the council pressure sharper."
        running={false}
        report={null}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        {...callbacks}
      />,
    );

    expect(
      screen.getByRole("region", { name: "Director Mode Workspace" }),
    ).toBeTruthy();
    expect(screen.getByTestId("director-mode-layout").getAttribute("class"))
      .toContain("lg:grid-cols");
    expect(screen.getByTestId("director-mode-layout").getAttribute("class"))
      .toContain("2xl:grid-cols");
    expect(screen.getByLabelText("Activity Stream")).toBeTruthy();
    expect(screen.getByLabelText("Playable Scene Preview")).toBeTruthy();
    expect(screen.getByLabelText("Direction Bar")).toBeTruthy();
    expect(screen.getByLabelText("Decision Queue")).toBeTruthy();
    expect(screen.getByText("Creative Goal")).toBeTruthy();
    expect(screen.getByText("Tax Resistance Memorials / Civic Hall"))
      .toBeTruthy();
    expect(screen.getByText("Hear one more minister")).toBeTruthy();
    expect(screen.getByText("Raise emergency taxes")).toBeTruthy();
    expect(screen.getByText("Project source")).toBeTruthy();
    expect(screen.getByText("Runtime proof")).toBeTruthy();
    expect(screen.getByText(/No decision queue is available/)).toBeTruthy();
    expect(screen.queryByText("Codex Worker")).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|real pi-Agent execution|real external agent execution/i,
    );
  });

  it("routes suggested directions, run requests, and trace/story actions through callbacks", () => {
    const callbacks = directorCallbacks();

    renderView(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/starter-project"
        input="Make the council pressure sharper."
        running={false}
        report={null}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        {...callbacks}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "add clue" }));
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));
    fireEvent.click(screen.getByRole("button", { name: "Trace evidence" }));
    fireEvent.click(screen.getByRole("button", { name: "Refine goal" }));

    expect(callbacks.onInputChange).toHaveBeenCalledWith("add clue");
    expect(callbacks.onRun).toHaveBeenCalledTimes(1);
    expect(callbacks.onOpenTrace).toHaveBeenCalledTimes(1);
    expect(callbacks.onOpenStory).toHaveBeenCalledTimes(1);
  });

  it("adds playtest proof evidence to the decision queue after a run", () => {
    const callbacks = directorCallbacks();
    const report = demoPlayOnceReport("raise emergency taxes");

    renderView(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/starter-project"
        input="raise emergency taxes"
        running={false}
        report={report}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        {...callbacks}
      />,
    );

    expect(screen.getByText("Playtest result")).toBeTruthy();
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/3 visible state deltas/).length).toBeGreaterThan(0);
    expect(screen.getByText("100/100")).toBeTruthy();
  });

  it("renders the runtime current beat instead of the scene entry beat after a run", () => {
    const callbacks = directorCallbacks();
    const report = demoPlayOnceReport("continue");
    report.trace.story_state_after = {
      ...report.trace.story_state_after,
      current_beat_id: "opening-scene-beat-002",
    };

    renderView(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/starter-project"
        input="continue"
        running={false}
        report={report}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        {...callbacks}
      />,
    );

    expect(
      screen.getByText(
        "The war minister points at the unpaid garrison columns and waits for an order.",
      ),
    ).toBeTruthy();
    expect(
      screen.queryByText(
        "Memorials arrive before dawn, each asking for silver the treasury cannot admit is missing.",
      ),
    ).toBeNull();
  });
});

function directorCallbacks() {
  return {
    onInputChange: vi.fn(),
    onSaveIdChange: vi.fn(),
    onRestoreIdChange: vi.fn(),
    onRestoreLatestChange: vi.fn(),
    onRun: vi.fn(),
    onOpenStory: vi.fn(),
    onOpenTrace: vi.fn(),
  };
}
