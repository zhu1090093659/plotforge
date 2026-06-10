import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  demoPlayOnceReport,
  demoProjectData,
} from "./demoStudioData";
import {
  defaultLocalPreviewState,
  type LocalPreviewState,
} from "./localPreviewModel";
import { DirectorModeView } from "./DirectorModeView";

afterEach(cleanup);

describe("DirectorModeView", () => {
  it("renders a director-first playable scene preview and local decision queue", () => {
    const callbacks = directorCallbacks();

    render(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/dynasty-embers"
        input="Make the council pressure sharper."
        running={false}
        report={null}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        localPreviewState={defaultLocalPreviewState}
        {...callbacks}
      />,
    );

    expect(
      screen.getByRole("region", { name: "Director Mode Workspace" }),
    ).toBeTruthy();
    expect(screen.getByTestId("director-mode-layout").getAttribute("class"))
      .toContain("2xl:grid-cols");
    expect(screen.getByTestId("director-mode-layout").getAttribute("class"))
      .not.toContain(" xl:grid-cols");
    expect(screen.getByLabelText("Activity Stream")).toBeTruthy();
    expect(screen.getByLabelText("Playable Scene Preview")).toBeTruthy();
    expect(screen.getByLabelText("Direction Bar")).toBeTruthy();
    expect(screen.getByLabelText("Decision Queue")).toBeTruthy();
    expect(screen.getByText("Creative Goal")).toBeTruthy();
    expect(screen.getByText("Tax Resistance Memorials / Forbidden City"))
      .toBeTruthy();
    expect(screen.getByText("Hear one more minister")).toBeTruthy();
    expect(screen.getByText("Raise emergency taxes")).toBeTruthy();
    expect(screen.getByText("Review generated story thread bundle"))
      .toBeTruthy();
    expect(screen.getByText("Confirm export disclosure evidence")).toBeTruthy();
    expect(screen.getByText("Codex Worker")).toBeTruthy();
    expect(screen.getByText("Claude Code Worker")).toBeTruthy();
    expect(screen.getByText("Story Agent")).toBeTruthy();
    expect(screen.getByText("Asset Agent")).toBeTruthy();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|real ACP execution/i,
    );
  });

  it("routes suggested directions, run requests, and trace/story actions through callbacks", () => {
    const callbacks = directorCallbacks();

    render(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/dynasty-embers"
        input="Make the council pressure sharper."
        running={false}
        report={null}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        localPreviewState={defaultLocalPreviewState}
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

    render(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/dynasty-embers"
        input="raise emergency taxes"
        running={false}
        report={report}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        localPreviewState={localPreviewStateWithSingleApproval()}
        {...callbacks}
      />,
    );

    expect(screen.getByText("Playtest result")).toBeTruthy();
    expect(screen.getByText("trace-001")).toBeTruthy();
    expect(screen.getByText(/3 visible state deltas/)).toBeTruthy();
    expect(screen.getByText("100/100")).toBeTruthy();
  });

  it("renders the runtime current beat instead of the scene entry beat after a run", () => {
    const callbacks = directorCallbacks();
    const report = demoPlayOnceReport("continue");
    report.trace.story_state_after = {
      ...report.trace.story_state_after,
      current_beat_id: "court-crisis-001-beat-002",
    };

    render(
      <DirectorModeView
        projectData={demoProjectData}
        loadedPath="/tmp/dynasty-embers"
        input="continue"
        running={false}
        report={report}
        error={null}
        saveId="save-001"
        restoreId=""
        restoreLatest={false}
        localPreviewState={defaultLocalPreviewState}
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

function localPreviewStateWithSingleApproval(): LocalPreviewState {
  return {
    ...defaultLocalPreviewState,
    approvals: [
      {
        id: "approval-story-thread",
        title: "Review generated story thread bundle",
        state: "pending-local-review",
        evidenceIds: ["trace_id", "files_changed", "playtest_result"],
      },
    ],
  };
}
