import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PlayView } from "./PlayView";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import { StudioI18nProvider } from "./i18n";

afterEach(cleanup);

function renderView(overrides: Partial<Parameters<typeof PlayView>[0]> = {}) {
  const onChooseChoice = vi.fn();
  render(
    <StudioI18nProvider>
      <PlayView
        projectData={demoProjectData}
        loadedPath="/tmp/starter-project"
        report={null}
        running={false}
        onChooseChoice={onChooseChoice}
        {...overrides}
      />
    </StudioI18nProvider>,
  );
  return { onChooseChoice };
}

describe("PlayView", () => {
  it("renders the scene preview region and the open-project placeholder when no projectData", () => {
    renderView({ projectData: null, loadedPath: "" });
    expect(screen.getByRole("region", { name: /Scene preview/i })).toBeTruthy();
    expect(
      screen.getByText(/Open a project to preview the player-facing scene/),
    ).toBeTruthy();
  });

  it("renders the beat text and choice buttons from the playtest report", () => {
    const report = demoPlayOnceReport("raise emergency taxes");
    const { onChooseChoice } = renderView({ report });

    // Beat text from demo opening scene is visible.
    expect(
      screen.getByText(
        /Memorials arrive before dawn, each asking for silver the treasury cannot admit is missing/,
      ),
    ).toBeTruthy();

    // Choice buttons are present; the label text lives inside the button
    // alongside the numeric index badge, so match on a substring.
    const choiceButtons = screen
      .getAllByRole("button")
      .filter((btn) => /Raise emergency taxes|Hear one more minister/i.test(btn.textContent ?? ""));
    expect(choiceButtons.length).toBeGreaterThan(0);

    fireEvent.click(choiceButtons[0]);
    expect(onChooseChoice).toHaveBeenCalledTimes(1);
    expect(onChooseChoice.mock.calls[0][0]).toMatch(/Raise emergency taxes|Hear one more minister/);
  });

  it("disables choice buttons while running", () => {
    const report = demoPlayOnceReport("raise emergency taxes");
    renderView({ report, running: true });

    const choiceButtons = screen
      .getAllByRole("button")
      .filter((btn) => btn.hasAttribute("disabled"));
    expect(choiceButtons.length).toBeGreaterThan(0);
  });

  it("shows the running chip when running", () => {
    renderView({ running: true });
    expect(screen.getByText(/Running/i)).toBeTruthy();
  });

  it("falls back to the entry scene/beat/choices from projectData when no report exists", () => {
    // This is the default Play state (no run yet): App passes playtestReport=null,
    // so the scene and current beat must be resolved from projectData alone.
    // Regression guard for resolveEntryScene's current_scene_key/entry_scene
    // fallback chain and story_state.current_beat_id lookup.
    renderView({ projectData: demoProjectData, report: null });

    // Entry-scene beat text from demoProjectData (no report supplied it).
    expect(
      screen.getByText(
        /Memorials arrive before dawn, each asking for silver the treasury cannot admit is missing/,
      ),
    ).toBeTruthy();
    // Entry-scene title/location chip renders from projectData.
    expect(screen.getByText(/Tax Resistance Memorials/)).toBeTruthy();
    // Choices are sourced from the projectData beat.
    const choiceButtons = screen
      .getAllByRole("button")
      .filter((btn) =>
        /Raise emergency taxes|Hear one more minister/i.test(
          btn.textContent ?? "",
        ),
      );
    expect(choiceButtons.length).toBeGreaterThan(0);
  });

  it("renders no choice buttons when the resolved beat has zero choices", () => {
    // Build a projectData whose only scene's only beat has no choices, so the
    // choices grid is empty and "Branching: 0" shows. Derive the beat from the
    // demo beat (preserving all typed fields) and just clear `choices`.
    const demoBeat = demoProjectData.scenes[0].beats[0];
    const noChoiceProjectData: typeof demoProjectData = {
      ...demoProjectData,
      scenes: [
        {
          ...demoProjectData.scenes[0],
          beats: [
            {
              ...demoBeat,
              id: "opening-scene-beat-001",
              text: "A quiet hallway with no decisions.",
              choices: [],
            },
          ],
        },
      ],
    };
    renderView({ projectData: noChoiceProjectData, report: null });

    expect(
      screen.getByText(/A quiet hallway with no decisions/),
    ).toBeTruthy();
    // Branching count renders "0 choices", and no choice buttons exist. Use
    // queryAllByRole (not getAllByRole, which throws when zero match).
    expect(screen.getByText(/0 choices/i)).toBeTruthy();
    expect(
      screen.queryAllByRole("button").filter((btn) =>
        /Raise emergency taxes|Hear one more minister/i.test(
          btn.textContent ?? "",
        ),
      ),
    ).toHaveLength(0);
  });

  it("renders the open-project placeholder when projectData is null but loadedPath is set", () => {
    // Distinct from the empty-everything placeholder case: a project was
    // targeted but never loaded. Must still show the placeholder, not crash.
    renderView({ projectData: null, loadedPath: "/tmp/never-loaded" });
    expect(
      screen.getByText(/Open a project to preview the player-facing scene/),
    ).toBeTruthy();
  });
});
