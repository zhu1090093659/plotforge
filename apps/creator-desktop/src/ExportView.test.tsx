import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportView, type ExportViewProps } from "./ExportView";
import { demoExportProfiles } from "./demoStudioData";
import type { StaticExportReport } from "./tauriBridge";
import { StudioI18nProvider } from "./i18n";

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
  }
  cleanup();
});

function makeProps(overrides: Partial<ExportViewProps> = {}): ExportViewProps {
  return {
    exportDir: "/tmp/starter-project/export",
    setExportDir: vi.fn(),
    archivePath: "/tmp/starter-project/export.zip",
    setArchivePath: vi.fn(),
    exportProfiles: demoExportProfiles,
    selectedExportProfileId: null,
    selectedExportProfile: null,
    staticExportSelected: false,
    exportReport: null,
    exporting: false,
    exportError: null,
    assetCatalog: { items: [], source: "records" as const },
    projectData: null,
    aiSafetyPolicy: null,
    formSaving: null,
    formStatus: null,
    runStaticZipExport: vi.fn().mockResolvedValue(undefined),
    selectExportProfile: vi.fn(),
    updateAiSafetyPolicy: vi.fn(),
    saveAiSafetyPolicy: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

function renderView(props: Partial<ExportViewProps> = {}) {
  return render(
    <StudioI18nProvider>
      <ExportView {...makeProps(props)} />
    </StudioI18nProvider>,
  );
}

describe("ExportView", () => {
  it("renders the export panel header and export button", () => {
    renderView();
    expect(screen.getByText("Export Package")).toBeTruthy();
    expect(
      screen.getByText("Local package readiness, manifest evidence, and boundary checks"),
    ).toBeTruthy();
    expect(screen.getByRole("button", { name: "Export zip" })).toBeTruthy();
  });

  it("disables export button when no static profile is selected", () => {
    renderView({ staticExportSelected: false });
    expect(
      screen.getByRole("button", { name: "Export zip" }).hasAttribute("disabled"),
    ).toBe(true);
  });

  it("enables export button when a static_web profile is selected", () => {
    const staticProfile = demoExportProfiles.find((p) => p.target === "static_web")!;
    renderView({
      selectedExportProfileId: staticProfile.id,
      selectedExportProfile: staticProfile,
      staticExportSelected: true,
    });
    expect(
      screen.getByRole("button", { name: "Export zip" }).hasAttribute("disabled"),
    ).toBe(false);
  });

  it("shows Build Profile section with selected profile id", () => {
    const staticProfile = demoExportProfiles.find((p) => p.target === "static_web")!;
    renderView({
      selectedExportProfileId: staticProfile.id,
      selectedExportProfile: staticProfile,
      staticExportSelected: true,
    });
    expect(screen.getAllByText("static-web").length).toBeGreaterThan(0);
  });

  it("lists all export profiles for selection", () => {
    renderView();
    for (const profile of demoExportProfiles) {
      expect(
        screen.getByRole("button", { name: `Select export profile ${profile.id}` }),
      ).toBeTruthy();
    }
  });

  it("fires selectExportProfile when a profile button is clicked", () => {
    const selectExportProfile = vi.fn();
    renderView({ selectExportProfile });
    fireEvent.click(
      screen.getByRole("button", { name: "Select export profile static-web" }),
    );
    expect(selectExportProfile).toHaveBeenCalledWith("static-web");
  });

  it("shows profile detail after a profile is selected", () => {
    const profile = demoExportProfiles.find((p) => p.id === "steam-submission-kit")!;
    renderView({
      selectedExportProfileId: profile.id,
      selectedExportProfile: profile,
      staticExportSelected: false,
    });
    expect(screen.getAllByText("steam-submission-kit").length).toBeGreaterThan(0);
    expect(
      screen.getByText("This profile does not call Steamworks APIs or promise approval."),
    ).toBeTruthy();
  });

  it("shows non-executable notice for non-static_web profile", () => {
    const profile = demoExportProfiles.find((p) => p.id === "steam-workshop")!;
    renderView({
      selectedExportProfileId: profile.id,
      selectedExportProfile: profile,
      staticExportSelected: false,
    });
    expect(
      screen.getByText(
        /This profile is available as contract metadata only; no Studio export command is wired/,
      ),
    ).toBeTruthy();
  });

  it("always shows output directory and zip archive inputs", () => {
    renderView({ staticExportSelected: false });
    expect(
      screen.getByLabelText("Static export output directory"),
    ).toBeTruthy();
    expect(
      screen.getByLabelText("Static export zip archive"),
    ).toBeTruthy();
  });

  it("shows export metrics when export report is available", () => {
    const exportReport: StaticExportReport = {
      output_dir: "/tmp/starter-project/export",
      files_found: ["index.html", "player.js"],
      allowed_files: ["index.html", "player.js"],
      archived_files: ["index.html", "player.js"],
      files_written: ["index.html", "player.js"],
      archive_path: "/tmp/starter-project/export.zip",
    };
    renderView({ exportReport });
    expect(
      screen.getAllByText("/tmp/starter-project/export.zip").length,
    ).toBeGreaterThan(0);
    expect(screen.getByText("matched")).toBeTruthy();
  });

  it("shows export error when present", () => {
    renderView({ exportError: "Export failed: permission denied" });
    expect(screen.getByText("Export failed: permission denied")).toBeTruthy();
  });

  it("shows packageItems list collapsed by default and expands on click", () => {
    renderView();
    // Package Contents toggle should be present
    const toggleButton = screen.getByRole("button", {
      name: /Package Contents/,
    });
    expect(toggleButton).toBeTruthy();
    expect(toggleButton.getAttribute("aria-expanded")).toBe("false");

    fireEvent.click(toggleButton);
    expect(toggleButton.getAttribute("aria-expanded")).toBe("true");
    // Items should now be visible
    expect(screen.getByText("Player files")).toBeTruthy();
    expect(screen.getByText("ExportManifest")).toBeTruthy();
  });

  it("shows Technical Details section collapsed with permanently-pending checks hidden by default", () => {
    renderView();
    const toggleButton = screen.getByRole("button", {
      name: /Technical Details/,
    });
    expect(toggleButton).toBeTruthy();
    expect(toggleButton.getAttribute("aria-expanded")).toBe("false");
    // Permanently-pending checks should not be in the DOM yet
    expect(screen.queryByText("HTTP smoke test passed")).toBeNull();

    fireEvent.click(toggleButton);
    expect(screen.getByText("HTTP smoke test passed")).toBeTruthy();
    expect(screen.getByText("No raw responses")).toBeTruthy();
    expect(screen.getByText("No secret markers")).toBeTruthy();
  });

  it("shows actionable evidence checks always visible", () => {
    renderView();
    expect(screen.getByText("No provider configuration")).toBeTruthy();
    expect(screen.getByText("No private traces")).toBeTruthy();
    expect(screen.getByText("All referenced assets copied")).toBeTruthy();
    expect(screen.getByText("No absolute machine paths")).toBeTruthy();
  });

  it("shows AI Safety Policy editor when aiSafetyPolicy is provided", () => {
    const aiSafetyPolicy = {
      policy_source_path: "ai-usage.toml",
      live_generated_content_enabled: true,
      human_review_required: false,
      moderation_queue_enabled: false,
      content_kinds: ["text" as const],
      user_reporting_path: "/report",
      moderation_policy: "Content is reviewed locally.",
      safety_guardrails: ["no violent content"],
      evidence_ids: [],
      notices: [],
    };
    renderView({ aiSafetyPolicy });
    expect(screen.getByText("AI Safety Policy")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Save AI Safety Policy" }),
    ).toBeTruthy();
    expect(
      screen.getByLabelText("AI safety content kinds"),
    ).toBeTruthy();
    expect(screen.getByLabelText("AI safety moderation policy")).toBeTruthy();
  });

  it("calls runStaticZipExport when export button is clicked", () => {
    const runStaticZipExport = vi.fn().mockResolvedValue(undefined);
    const staticProfile = demoExportProfiles.find((p) => p.target === "static_web")!;
    renderView({
      selectedExportProfile: staticProfile,
      staticExportSelected: true,
      runStaticZipExport,
    });
    fireEvent.click(screen.getByRole("button", { name: "Export zip" }));
    expect(runStaticZipExport).toHaveBeenCalledOnce();
  });
});
