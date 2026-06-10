import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import {
  demoAiSafetyPolicy,
  demoExportProfiles,
  demoPlayOnceReport,
} from "./demoStudioData";
import { RuntimeTracePanel } from "./runtimeTraceView";

afterEach(cleanup);

describe("RuntimeTracePanel", () => {
  it("renders proof, trace, and export evidence without leaking raw secrets or platform promises", () => {
    const report = demoPlayOnceReport("sk-test-secret should not render");

    render(
      <RuntimeTracePanel
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

    expect(screen.getByRole("region", { name: "Playable Proof" })).toBeTruthy();
    expect(screen.getByRole("region", { name: "Trace Debug" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Proof Evidence Panel" })).toBeTruthy();
    expect(screen.getByText("Redaction-safe causality")).toBeTruthy();
    expect(screen.getByText("Package Evidence Summary")).toBeTruthy();
    expect(screen.getByText("AI Usage Disclosure")).toBeTruthy();
    expect(screen.getByText("Content Warning Draft")).toBeTruthy();
    expect(screen.getByText("Local Static Web Package")).toBeTruthy();
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    expect(screen.getByText("Run Seed")).toBeTruthy();
    expect(screen.getAllByText("7").length).toBeGreaterThan(0);
    expect(screen.getAllByText("plotforge-local-mock-prompt-v1").length)
      .toBeGreaterThan(0);
    expect(screen.getByText("plotforge-local-mock-model-v1")).toBeTruthy();
    expect(
      screen.getAllByText("sha256:plotforge-local-mock-provider-config-v1")
        .length,
    ).toBeGreaterThan(0);
    expect(screen.getByText("Provider Config Hash")).toBeTruthy();
    expect(screen.getAllByText("local ready").length).toBeGreaterThan(0);
    expect(screen.getAllByText("matched").length).toBeGreaterThan(0);
    expect(screen.queryByText(/sk-test-secret/)).toBeNull();
    expect(document.body.textContent ?? "").not.toMatch(
      /automatic publishing|approval guarantee|legal guarantee|Steam upload automation|one-click Steam launch|raw provider response|secret marker/i,
    );
  });

  it("renders proof from the trace current beat after same-scene progression", () => {
    const report = demoPlayOnceReport("continue");
    report.trace.story_state_after = {
      ...report.trace.story_state_after,
      current_beat_id: "court-crisis-001-beat-002",
    };

    render(<RuntimeTracePanel report={report} error={null} />);

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
