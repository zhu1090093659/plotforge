import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import {
  demoAiSafetyPolicy,
  demoExportProfiles,
  demoPlayOnceReport,
} from "./demoStudioData";
import { TraceDebugView, NarrativeReviewPanel } from "./TraceDebugView";
import { StudioI18nProvider } from "./i18n";

afterEach(cleanup);

function renderView(ui: React.ReactElement) {
  return render(ui, { wrapper: StudioI18nProvider });
}

describe("TraceDebugView", () => {
  it("renders Playable Proof region without leaking raw secrets or platform promises", () => {
    const report = demoPlayOnceReport("sk-test-secret should not render");

    renderView(
      <TraceDebugView
        report={report}
        error={null}
        selectedExportProfile={demoExportProfiles[0]}
        exportReport={{
          output_dir: "/tmp/export",
          archive_path: "/tmp/export.zip",
          files_written: ["/tmp/export/index.html", "/tmp/export/game.json"],
          archived_files: ["index.html", "game.json"],
          allowed_files: ["index.html", "game.json"],
          files_found: ["index.html", "game.json"],
        }}
        aiSafetyPolicy={demoAiSafetyPolicy}
      />,
    );

    expect(
      screen.getByRole("region", { name: "Playable Proof" }),
    ).toBeTruthy();
    expect(
      screen.getByRole("complementary", { name: "Proof Evidence Panel" }),
    ).toBeTruthy();
    expect(screen.getByText("Package Evidence Summary")).toBeTruthy();
    expect(screen.getByText("AI Usage Disclosure")).toBeTruthy();
    expect(screen.getByText("Content Warning Draft")).toBeTruthy();
    expect(screen.getByText("Local Static Web Package")).toBeTruthy();
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    expect(screen.getByText("Run seed")).toBeTruthy();
    expect(screen.getAllByText("7").length).toBeGreaterThan(0);
    expect(
      screen.getAllByText("plotforge-local-mock-prompt-v1").length,
    ).toBeGreaterThan(0);
    // Expand Technical Details to verify reproducibility metadata
    fireEvent.click(screen.getByText("Technical Details"));
    expect(screen.getByText("plotforge-local-mock-model-v1")).toBeTruthy();
    expect(
      screen.getAllByText("sha256:plotforge-local-mock-provider-config-v1")
        .length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText("local ready").length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("matched").length).toBeGreaterThan(0);
    expect(screen.queryByText(/sk-test-secret/)).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|Steam upload automation|one-click Steam launch|raw provider response|secret marker/i,
    );
  });

  it("renders Run Result Summary open by default with state delta and run evidence", () => {
    const report = demoPlayOnceReport("continue");

    renderView(
      <TraceDebugView
        report={report}
        error={null}
      />,
    );

    // Run Result Summary is default open — its content is visible
    expect(screen.getByText("Run Result Summary")).toBeTruthy();
    expect(screen.getByText("State Delta")).toBeTruthy();
    expect(screen.getByText("Run Evidence")).toBeTruthy();
    expect(screen.getByText("Selected choice")).toBeTruthy();
    expect(screen.getByText("raise-tax")).toBeTruthy();
  });

  it("renders proof from the trace current beat after same-scene progression", () => {
    const report = demoPlayOnceReport("continue");
    report.trace.story_state_after = {
      ...report.trace.story_state_after,
      current_beat_id: "opening-scene-beat-002",
    };

    renderView(<TraceDebugView report={report} error={null} />);

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

  it("shows Technical Details heading but keeps content collapsed by default", () => {
    const report = demoPlayOnceReport("test");

    renderView(<TraceDebugView report={report} error={null} />);

    // Technical Details heading is present
    expect(screen.getByText("Technical Details")).toBeTruthy();

    // Trace Debug sub-content is NOT visible when collapsed
    expect(screen.queryByText("Causality Graph")).toBeNull();
    expect(screen.queryByText("Trace Evidence")).toBeNull();
    expect(screen.queryByText("Narrative Review")).toBeNull();
    expect(screen.queryByText("Diagnostics")).toBeNull();
  });

  it("shows placeholder when no trace is available", () => {
    renderView(<TraceDebugView report={null} error={null} />);

    expect(
      screen.getByText("Run a playtest turn to create proof"),
    ).toBeTruthy();
    expect(
      screen.getByText("Run a playtest turn to inspect trace evidence."),
    ).toBeTruthy();
  });

  it("shows error banner when error is present", () => {
    renderView(
      <TraceDebugView report={null} error="Something went wrong" />,
    );

    expect(screen.getByText("Something went wrong")).toBeTruthy();
  });
});

describe("NarrativeReviewPanel", () => {
  it("renders all 6 narrative scores independently", () => {
    const report = demoPlayOnceReport("test");
    const review = report.trace.narrative_review!;

    renderView(<NarrativeReviewPanel review={review} />);

    expect(screen.getByText("Narrative Review")).toBeTruthy();
    expect(screen.getByText("Hook")).toBeTruthy();
    expect(screen.getByText("Pacing")).toBeTruthy();
    expect(screen.getByText("Character")).toBeTruthy();
    expect(screen.getByText("Payoff")).toBeTruthy();
    expect(screen.getByText("Choice")).toBeTruthy();
    expect(screen.getByText("AI slop")).toBeTruthy();
  });
});
