import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import {
  demoExportProfiles,
  demoPlayOnceReport,
  demoProjectData,
  demoReproducibilityMetadata,
} from "./demoStudioData";
import type { StudioDataSource } from "./studioDataSource";
import {
  mockPlayOnceProjectFromLatestSnapshot,
  mockPlayOnceProjectFromSnapshot,
  mockPlayOnceProjectWithSave,
  playOnceReportWithSnapshot,
} from "./testHelpers/studioDataSource";
import type {
  AssetRecord,
  ImageProviderEntry,
  McpServerEntry,
  McpServerTestResult,
  McpToolCallRequest,
  McpToolCallResult,
  McpToolManifest,
  ModerationProviderEntry,
  PiAgentApplyResult,
  ProviderEntry,
  PromptTemplate,
  TtsProviderEntry,
} from "../../../contracts/plotforge";
import type {
  PlayOnceReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

/** Build a `PiAgentApplyResult` from a `PlayOnceReport` so the rail's
 * `piAgentApplyRun` mock returns the same scene/trace the test asserts on. */
function applyResultFromReport(report: PlayOnceReport): PiAgentApplyResult {
  return {
    run: {
      descriptor: {
        agent_id: "local-pi",
        is_local_pi: true,
        capabilities: [],
      },
      reproducibility: demoReproducibilityMetadata,
      trace_id: "pi-agent-evidence-test",
      evidence_summary: "",
    },
    scene_key: report.scene.key,
    scene: report.scene,
    trace: report.trace,
    trace_path: report.trace_path,
    snapshot: report.snapshot,
    snapshot_path: report.snapshot_path,
    delta_summary: report.delta_summary,
  };
}

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
    window.localStorage.removeItem("plotforge:creator-desktop:rail-collapsed");
    window.localStorage.removeItem("plotforge:creator-desktop:sidebar-collapsed");
  }
  cleanup();
});

describe("App", () => {
  it("opens a selected project folder from the sidebar Open Project button", async () => {
    const pickProjectDirectory = vi.fn(async () => "/tmp/starter-project");
    const openOrCreateProject = vi.fn(async () => demoProjectData);
    render(
      <App
        dataSource={appTestDataSource({ pickProjectDirectory, openOrCreateProject })}
        initialProjectPath=""
      />,
    );

    fireEvent.click(screen.getByTitle(/Open or create project/i));

    await waitFor(() => {
      expect(pickProjectDirectory).toHaveBeenCalledTimes(1);
      expect(openOrCreateProject).toHaveBeenCalledWith("/tmp/starter-project");
    });
  });

  it("keeps a canonical project loaded when a secondary workspace read fails", async () => {
    const dataSource = appTestDataSource({
      async pickProjectDirectory() {
        return "/tmp/starter-project";
      },
      async checkProject() {
        throw new Error("cannot hydrate project overview");
      },
    });
    render(<App dataSource={dataSource} initialProjectPath="" />);

    fireEvent.click(screen.getByTitle(/Open or create project/i));

    expect((await screen.findAllByText("starter-project")).length).toBeGreaterThan(0);
    expect(
      await screen.findByText(
        /Failed to open project folder: cannot hydrate project overview/,
      ),
    ).toBeTruthy();
  });

  it("loads project data, lists source files, and saves editable text", async () => {
    const writes: Array<{ relativePath: string; content: string }> = [];
    const files: SourceFileSummary[] = [
      { path: "game.toml", kind: "toml", bytes: 120, editable: false },
      { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
    ];
    const contents: Record<string, SourceFileContent> = {
      "game.toml": {
        path: "game.toml",
        kind: "toml",
        editable: false,
        content: 'title = "Starter Project"\n',
      },
      "world/world.md": {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n\nThe city is under pressure.\n",
      },
    };
    const dataSource = appTestDataSource({
      async listSourceFiles() {
        return files;
      },
      async readSourceFile(_path, relativePath) {
        const file = contents[relativePath];
        if (!file) {
          throw new Error(`missing source fixture: ${relativePath}`);
        }
        return file;
      },
      async writeSourceFile(_path, relativePath, content) {
        const file = contents[relativePath];
        if (!file) {
          throw new Error(`missing source fixture: ${relativePath}`);
        }
        writes.push({ relativePath, content });
        const updated = { ...file, content };
        contents[relativePath] = updated;
        return updated;
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Source"));
    expect(screen.getAllByText("world/world.md").length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: /world\/world\.md/ }));
    expect(await screen.findByDisplayValue(/The city is under pressure/)).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Source editor"), {
      target: { value: "# World Bible\n\nThe council has changed.\n" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(writes).toEqual([
        {
          relativePath: "world/world.md",
          content: "# World Bible\n\nThe council has changed.\n",
        },
      ]);
    });
  });

  it("exposes the flat nav (Home, Play, World, Story, Characters, State, Rules, Assets, Trace, Export, Source, Settings)", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    expect(getNavButton("Home")).toBeTruthy();
    expect(getNavButton("Play")).toBeTruthy();
    expect(getNavButton("World Bible")).toBeTruthy();
    expect(getNavButton("Story Craft")).toBeTruthy();
    expect(getNavButton("Characters")).toBeTruthy();
    expect(getNavButton("State")).toBeTruthy();
    expect(getNavButton("Rules")).toBeTruthy();
    expect(getNavButton("Assets")).toBeTruthy();
    expect(getNavButton("Trace")).toBeTruthy();
    expect(getNavButton("Export")).toBeTruthy();
    expect(getNavButton("Source")).toBeTruthy();
    // Settings is sidebar item #12 — its label is the EN value of
    // `nav.settings.label`. Assert it is reachable as a nav button by
    // accessible name so a future regression that drops the Settings section
    // from the flat nav fails loudly.
    expect(getNavButton("Settings")).toBeTruthy();
    // Source is the active section after waitForDefaultSourceCanvas navigates there.
    expect(getNavButton("Source").getAttribute("aria-pressed")).toBe("true");
    expect(screen.queryByLabelText("Agent rail")).toBeNull();
    expect(screen.getByLabelText("Expand agent rail")).toBeTruthy();
    expect(screen.getByLabelText("Expand sidebar")).toBeTruthy();
  });

  it("renders sidebar nav without duplicate second-level buttons", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    const navTree = screen.getByRole("navigation", {
      name: "Studio navigation tree",
    });

    // Each nav label appears exactly once in the flat tree.
    expect(within(navTree).getAllByRole("button", { name: "Assets" }))
      .toHaveLength(1);
    expect(within(navTree).getAllByRole("button", { name: "World Bible" }))
      .toHaveLength(1);
    expect(within(navTree).getAllByRole("button", { name: "Story Craft" }))
      .toHaveLength(1);
  });

  it("keeps the Launchpad and Agent rail model decks synchronized", async () => {
    render(
      <App
        dataSource={appTestDataSource()}
        initialProjectPath="/tmp/starter-project"
      />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Home"));
    fireEvent.click(screen.getByLabelText("Expand agent rail"));

    const main = screen.getByRole("main");
    const rail = screen.getByLabelText("Agent rail");
    const launchpadDeck = within(main).getByRole("button", {
      name: "Model and thinking level",
    });
    const railDeck = within(rail).getByRole("button", {
      name: "Model and thinking level",
    });

    fireEvent.click(launchpadDeck);
    fireEvent.click(
      screen.getByRole("gridcell", {
        name: "Local pi-Agent (offline) — local, High",
      }),
    );
    await waitFor(() => expect(railDeck.textContent).toContain("High"));

    fireEvent.click(
      screen.getByRole("button", { name: "Close model selector" }),
    );
    fireEvent.click(railDeck);
    fireEvent.click(
      screen.getByRole("gridcell", {
        name: "Local pi-Agent (offline) — local, Low",
      }),
    );
    await waitFor(() => expect(launchpadDeck.textContent).toContain("Low"));
  });

  it("does not render injected fake agent workers or approval queues", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(screen.getByLabelText("Expand agent rail"));
    // Honesty-surface evidence lives inside the AgentChatRail "Evidence" popover
    // (default closed). Open it to assert no-fake details + creator-facing
    // boundaries.
    //
    // NOTE: the former AgentMeshView surfaced a positive "capability rendered
    // as not-implemented" assertion (pi-Agent image-generation / steam-upload).
    // When the Agent Mesh surface was deleted and navigation was flattened,
    // that capability list no longer renders anywhere in the UI, so the
    // positive capability-honesty assertion was intentionally retired with it.
    // This test now guards only the negative honesty contract (no fake
    // workers, no approval queues, no platform-promise text) plus the
    // positive local-boundaries text. Do NOT reintroduce a capability surface
    // without re-adding a "rendered as not-implemented" assertion here.
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    const popover = screen.getByRole("dialog");
    expect(within(popover).getByText("Source files")).toBeTruthy();
    // Expand the local-boundaries collapsible.
    fireEvent.click(within(popover).getByRole("button", { name: /Local boundaries/i }));
    expect(within(popover).getByText(/All runs happen locally/)).toBeTruthy();
    expect(within(popover).getByText(/No external agents are connected/)).toBeTruthy();
    // No fake worker / approval queue text anywhere.
    expect(screen.queryByText("Mock pi-Agent Worker")).toBeNull();
    expect(screen.queryByText("Approve local preview patch")).toBeNull();
    expect(screen.queryByText("local-preview-only")).toBeNull();
    expect(screen.queryByText("Mock Story Agent")).toBeNull();
  });

  it("switches Studio chrome between English and Chinese", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();

    fireEvent.change(screen.getByLabelText("Language"), {
      target: { value: "zh" },
    });

    await waitFor(() => {
      expect(getNavButton("首页")).toBeTruthy();
    });
    if (typeof window.localStorage?.getItem === "function") {
      expect(window.localStorage.getItem("plotforge:creator-desktop:locale")).toBe(
        "zh",
      );
    }

    fireEvent.click(getNavButton("追踪"));
    await waitFor(() => {
      // Trace view placeholder when no proof has run.
      expect(screen.getByText(/运行试玩回合以创建证明|Run a playtest turn to create proof/)).toBeTruthy();
    });

    fireEvent.change(screen.getByLabelText("语言"), {
      target: { value: "en" },
    });

    await waitFor(() => {
      expect(getNavButton("Trace")).toBeTruthy();
    });
    expect(screen.queryByText("追踪")).toBeNull();
  });

  it("renders asset records and visual-audio bible cards before background fallback", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Assets"));

    expect(screen.getByText("2 asset records")).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: /Asset Catalog/ }));
    expect(screen.getAllByText("asset-image-opening-scene").length)
      .toBeGreaterThan(0);
    expect(screen.getByText("image / generated")).toBeTruthy();
    expect(
      screen.getAllByText("sha256:2c60d8f6f2f16f4ff6b5a5e4f7a20c2c6a18f3c4d9d3b7319dd6127a98d8a501")
        .length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("sha256").length).toBeGreaterThan(0);
    expect(screen.getByText("plotforge-local-mock / plotforge-local-mock-image-v1")).toBeTruthy();
    expect(screen.getByText("mock-image-opening-scene")).toBeTruthy();
    expect(screen.getByText("sha256:demo-opening-scene-prompt")).toBeTruthy();
    expect(
      screen.getByText("scene:opening-scene:background_asset"),
    ).toBeTruthy();
    expect(screen.getAllByText("asset-voice-censor-001").length)
      .toBeGreaterThan(0);
    expect(screen.getAllByText("Fallback").length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("tab", { name: /Visual Bible/i }));
    expect(screen.getByText("Winter council ink wash")).toBeTruthy();
    expect(screen.getByText("Official portrait restraint")).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: /Audio Bible/ }));
    expect(screen.getByText("Civic Auditor")).toBeTruthy();
    expect(screen.getByText("Minister of War")).toBeTruthy();
    expect(screen.queryByText("Scene background fallback")).toBeNull();
  });

  it("keeps scene background fallback when no asset records are available", async () => {
    const dataSource = appTestDataSource({
      async listAssetRecords() {
        return [];
      },
      async openProject() {
        return {
          ...demoProjectData,
          asset_records: [] as AssetRecord[],
        };
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Assets"));

    expect(screen.getByText("1 scene background fallbacks")).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: /Asset Catalog/ }));
    expect(screen.getByText("Scene background fallback")).toBeTruthy();
    expect(screen.getAllByText("assets/generated/opening-scene.png").length)
      .toBeGreaterThan(0);
  });

  it("saves Visual Bible and Audio Bible structured controls through the data source", async () => {
    const updates: string[] = [];
    const dataSource = appTestDataSource({
      async updateVisualBible(_path, visualBible) {
        const card = visualBible.style_cards[0];
        updates.push(
          `visual:${card.prompt}:${card.palette.join("|")}:${card.tags.join("|")}:${card.reference_asset_ids.join("|")}`,
        );
        return visualBible;
      },
      async updateAudioBible(_path, audioBible) {
        const card = audioBible.voice_cards[0];
        updates.push(
          `audio:${card.voice}:${card.delivery}:${card.tags.join("|")}:${card.sample_text ?? "none"}:${card.reference_asset_ids.join("|")}`,
        );
        return audioBible;
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Assets"));
    fireEvent.click(screen.getByRole("button", { name: /Winter council ink wash/ }));

    fireEvent.change(screen.getByLabelText("Visual style prompt 1"), {
      target: { value: "Ink council with harsher winter lanterns." },
    });
    fireEvent.change(screen.getByLabelText("Visual style palette 1"), {
      target: { value: "bone white\nseal red" },
    });
    fireEvent.change(screen.getByLabelText("Visual style tags 1"), {
      target: { value: "council\nwinter" },
    });
    fireEvent.change(screen.getByLabelText("Visual style reference asset ids 1"), {
      target: { value: "asset-image-opening-scene\nasset-style-ref-002" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Visual Bible" }));

    await waitFor(() => {
      expect(updates).toHaveLength(1);
    });
    fireEvent.click(screen.getByRole("tab", { name: /Audio Bible/ }));
    fireEvent.click(screen.getByRole("button", { name: /Civic Auditor/ }));

    fireEvent.change(screen.getByLabelText("Audio voice 1"), {
      target: { value: "dry formal council voice" },
    });
    fireEvent.change(screen.getByLabelText("Audio delivery 1"), {
      target: { value: "quiet but cutting" },
    });
    fireEvent.change(screen.getByLabelText("Audio tags 1"), {
      target: { value: "council\nformal" },
    });
    fireEvent.change(screen.getByLabelText("Audio sample text 1"), {
      target: { value: "The ledgers do not accuse by accident." },
    });
    fireEvent.change(screen.getByLabelText("Audio reference asset ids 1"), {
      target: { value: "asset-voice-censor-001\nasset-voice-ref-002" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Audio Bible" }));

    await waitFor(() => {
      expect(updates).toEqual([
        "visual:Ink council with harsher winter lanterns.:bone white|seal red:council|winter:asset-image-opening-scene|asset-style-ref-002",
        "audio:dry formal council voice:quiet but cutting:council|formal:The ledgers do not accuse by accident.:asset-voice-censor-001|asset-voice-ref-002",
      ]);
    });
  });

  it("runs playtest via the Agent rail Send and renders fallback trace/errors/review in Trace without raw keys", async () => {
    const fallbackReport = fallbackPlayOnceReport();
    const dataSource = appTestDataSource({
      async playOnceProject() {
        return fallbackReport;
      },
      async piAgentApplyRun() {
        return {
          ...applyResultFromReport(fallbackReport),
          turn_usage: {
            total_input_tokens: 41,
            total_output_tokens: 13,
            total_spent_cost_units: 7,
          },
        };
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(screen.getByLabelText("Expand agent rail"));
    // The Agent rail is the single "describe a change / run a turn" entry.
    fireEvent.change(screen.getByLabelText("Direct the agent — describe a change…"), {
      target: { value: "continue" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    // After a run, App switches to the Trace section.
    await waitFor(() => {
      expect(getNavButton("Trace").getAttribute("aria-pressed")).toBe("true");
    });
    await waitFor(() => {
      expect(screen.getAllByText("Fallback Council").length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText("trace-fallback").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Fallback").length).toBeGreaterThan(0);
    // provider_timeout error lives inside the Run Result Summary (collapsed by
    // default); expand it to verify the error is rendered.
    fireEvent.click(screen.getByText("Run Result Summary"));
    expect(screen.getAllByText("provider_timeout").length).toBeGreaterThan(0);

    fireEvent.click(screen.getByText("Technical Details"));
    expect(screen.getByText("weak_hook")).toBeTruthy();
    expect(screen.getByText("Trace Evidence")).toBeTruthy();
    expect(screen.getByText("Action Intent")).toBeTruthy();
    expect(screen.getByText("Rule Result")).toBeTruthy();
    expect(screen.getByText("Planner Result")).toBeTruthy();
    expect(screen.getByText("Reproducibility")).toBeTruthy();
    expect(
      screen.getAllByText("plotforge-local-mock-prompt-v1").length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText("plotforge-local-mock-model-v1").length,
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText("sha256:plotforge-local-mock-provider-config-v1")
        .length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("State Delta").length).toBeGreaterThan(0);
    expect(screen.getByText("Media References")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /^Evidence$/ }));
    const evidence = screen.getByRole("dialog", { name: "Evidence" });
    expect(within(evidence).getByText("Turn input tokens")).toBeTruthy();
    expect(within(evidence).getByText("41")).toBeTruthy();
    expect(within(evidence).getByText("Turn output tokens")).toBeTruthy();
    expect(within(evidence).getByText("13")).toBeTruthy();
    expect(within(evidence).getByText("Turn cost units")).toBeTruthy();
    expect(within(evidence).getByText("7")).toBeTruthy();
    expect(screen.getAllByText("continue-council").length).toBeGreaterThan(0);
    expect(screen.getAllByText("continue").length).toBeGreaterThan(0);
    expect(screen.getByText("background_asset")).toBeTruthy();
    expect(screen.getAllByText("assets/generated/opening-scene.png").length)
      .toBeGreaterThan(0);
    expect(
      screen.getByText(
        "planner returned fallback scene `[REDACTED_SECRET]`",
      ),
    ).toBeTruthy();
    expect(
      screen.getAllByText(
        "Provider pipeline used fallback scene [REDACTED_SECRET]",
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText(/sk-test-secret/)).toBeNull();
  });

  it("runs a playtest turn by clicking a choice button on the Play view and submits the choice label as the turn intent", async () => {
    const seenInputs: string[] = [];
    const report = demoPlayOnceReport("raise emergency taxes");
    const dataSource = appTestDataSource({
      async playOnceProject(_path, playerInput) {
        seenInputs.push(playerInput);
        return report;
      },
      async piAgentApplyRun(request) {
        seenInputs.push(request.player_input);
        return applyResultFromReport(report);
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Play"));
    // The Play view renders choice buttons; clicking one submits it as the
    // next turn's intent through the Agent conversation pipeline.
    const choiceButtons = screen
      .getAllByRole("button")
      .filter((btn) => /raise emergency taxes|hear one more minister/i.test(btn.textContent ?? ""));
    expect(choiceButtons.length).toBeGreaterThan(0);
    // The button textContent includes a numeric badge ("1Hear one more
    // minister"); the submitted intent must be the choice label only, so
    // strip the leading digits before clicking.
    const pickedLabel = (choiceButtons[0].textContent ?? "").replace(/^\d+/, "");
    fireEvent.click(choiceButtons[0]);

    await waitFor(() => {
      expect(getNavButton("Trace").getAttribute("aria-pressed")).toBe("true");
    });
    // The clicked choice label must be the playerInput sent to the data source,
    // AND exactly one playOnce call fires. This double-guards against the
    // historical bug where an earlier implementation set shared input state
    // then re-read it in a closure (submitting stale input), and against any
    // future regression that double-fires the data source on rapid clicks.
    expect(seenInputs).toEqual([pickedLabel]);
  });

  it("runs playtest with explicit and latest runtime snapshot controls", async () => {
    const calls: Array<{
      playerInput: string;
      saveId: string | null;
      restoreId: string | null;
    }> = [];
    const dataSource = appTestDataSource({
      async piAgentApplyRun(request) {
        calls.push({
          playerInput: request.player_input,
          saveId: request.save_id ?? null,
          restoreId: request.restore_id ?? null,
        });
        return applyResultFromReport(
          playOnceReportWithSnapshot(
            request.player_input,
            request.save_id ?? request.restore_id ?? "latest",
            `/tmp/starter-project/saves/${request.save_id ?? request.restore_id ?? "latest"}.runtime_snapshot.json`,
          ),
        );
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(screen.getByLabelText("Expand agent rail"));

    // First turn: explicit snapshot restore + save id.
    fireEvent.change(screen.getByLabelText("Direct the agent — describe a change…"), {
      target: { value: "pay the army" },
    });
    fireEvent.click(getNavButton("Trace"));
    fireEvent.click(screen.getByRole("button", { name: /Advanced snapshot controls/i }));
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "save-after-army" },
    });
    fireEvent.change(screen.getByLabelText("Playtest restore id"), {
      target: { value: "save-before-army" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    await waitFor(() => {
      expect(calls).toEqual([
        {
          playerInput: "pay the army",
          saveId: "save-after-army",
          restoreId: "save-before-army",
        },
      ]);
    });
    // The snapshot path renders inside Run Evidence (Run Result Summary),
    // which is collapsed by default — expand it before looking up the path.
    fireEvent.click(screen.getByText("Run Result Summary"));
    await screen.findByText("/tmp/starter-project/saves/save-after-army.runtime_snapshot.json");

    // Second turn: restore-latest + new save id.
    fireEvent.change(screen.getByLabelText("Direct the agent — describe a change…"), {
      target: { value: "raise emergency taxes" },
    });
    // Snapshot collapsible is still open from the first turn; update save id.
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "save-after-tax" },
    });
    fireEvent.click(screen.getByLabelText("Restore latest save"));
    fireEvent.click(screen.getByRole("button", { name: "Send" }));

    await waitFor(() => {
      expect(calls).toEqual([
        {
          playerInput: "pay the army",
          saveId: "save-after-army",
          restoreId: "save-before-army",
        },
        {
          playerInput: "raise emergency taxes",
          saveId: "save-after-tax",
          restoreId: null,
        },
      ]);
    });
    await screen.findByText("/tmp/starter-project/saves/save-after-tax.runtime_snapshot.json");
  });

  it("exports static zip packages through the Studio data source", async () => {
    const exports: Array<{
      path: string;
      outputDir: string;
      archivePath: string;
    }> = [];
    const dataSource = appTestDataSource({
      async exportStaticProjectZip(path, outputDir, archivePath) {
        exports.push({ path, outputDir, archivePath });
        return {
          output_dir: outputDir,
          archive_path: archivePath,
          files_written: [`${outputDir}/index.html`, `${outputDir}/game.json`],
          archived_files: ["game.json", "index.html"],
          allowed_files: ["game.json", "index.html"],
          files_found: ["game.json", "index.html"],
        };
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Export"));
    fireEvent.click(screen.getByRole("tab", { name: /^Profile/ }));
    expect(screen.getAllByText("static-web").length).toBeGreaterThan(0);
    expect(screen.getByText("byo-key-web")).toBeTruthy();
    expect(screen.getByText("self-host-backend")).toBeTruthy();
    expect(screen.getByText("desktop-runtime")).toBeTruthy();
    expect(screen.getByText("steam-workshop")).toBeTruthy();
    expect(screen.getByText("steam-submission-kit")).toBeTruthy();
    expect(screen.getByText("no_network_player")).toBeTruthy();
    expect(screen.getByText("Provider config")).toBeTruthy();
    expect(screen.getByText("Submission ready")).toBeTruthy();

    fireEvent.click(screen.getByRole("tab", { name: /^Package/ }));
    fireEvent.change(screen.getByLabelText("Static export output directory"), {
      target: { value: "/tmp/static-export" },
    });
    fireEvent.change(screen.getByLabelText("Static export zip archive"), {
      target: { value: "/tmp/starter-project.zip" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Export zip" }));

    await waitFor(() => {
      expect(exports).toEqual([
        {
          path: "/tmp/starter-project",
          outputDir: "/tmp/static-export",
          archivePath: "/tmp/starter-project.zip",
        },
      ]);
    });
    expect(
      screen.getAllByText("/tmp/starter-project.zip").length,
    ).toBeGreaterThan(0);
    expect(screen.getByText("matched")).toBeTruthy();
    expect(screen.getByText("pending explicit package hash")).toBeTruthy();
    // Actionable evidence checks + Technical Details are nested under the
    // Advanced Evidence collapsible on the Package tab.
    fireEvent.click(screen.getByRole("button", { name: /Advanced Evidence/ }));
    expectExportEvidenceStatus("All referenced assets copied", "Pass");
    fireEvent.click(screen.getByRole("button", { name: /Technical Details/ }));
    expectExportEvidenceStatus("No raw responses", "Pending");
    expectExportEvidenceStatus("No secret markers", "Pending");
    expectExportEvidenceStatus("HTTP smoke test passed", "Pending");
  });

  it("opens the static export profile from the nav tree Export button", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Export"));

    fireEvent.click(screen.getByRole("tab", { name: /^Profile/ }));
    expect(screen.getAllByText("static-web").length).toBeGreaterThan(0);
    expect(
      screen
        .getByRole("button", { name: "Select export profile static-web" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
    fireEvent.click(screen.getByRole("tab", { name: /^Package/ }));
    expect(screen.getByLabelText("Static export output directory")).toBeTruthy();
  });

  it("keeps draft export profiles visible without calling static export", async () => {
    const exports: string[] = [];
    const dataSource = appTestDataSource({
      async exportStaticProjectZip() {
        exports.push("static-zip");
        throw new Error("static export should not run for draft profiles");
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();
    fireEvent.click(getNavButton("Export"));
    fireEvent.click(screen.getByRole("tab", { name: /^Profile/ }));
    fireEvent.click(
      screen.getByRole("button", {
        name: "Select export profile steam-workshop",
      }),
    );

    expect(screen.getByText("steam_workshop_metadata")).toBeTruthy();
    expect(screen.getByText("not claimed")).toBeTruthy();
    expect(
      screen.getByText("This profile does not upload content or promise platform approval."),
    ).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: /^Package/ }));
    expect(
      screen.getByText(
        /This profile is available as contract metadata only; no Studio export command is wired/,
      ),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Export zip" }).hasAttribute("disabled"),
    ).toBe(true);
    expect(exports).toEqual([]);
  });

  it("saves structured editing forms (World/Story/Characters/State/Rules) through the data source", async () => {
    const updates: string[] = [];
    const dataSource = appTestDataSource({
      async updateWorldEditDocument(_path, document) {
        updates.push(`world:${document.forbidden_facts.join("|")}`);
        return document;
      },
      async updateStoryCraftEditDocument(_path, document) {
        updates.push(`story:${document.story_craft.bible.genre_promise}`);
        return document;
      },
      async createCharacterFromDraft(_path, draft) {
        const character = {
          id: draft.id.trim(),
          name: draft.name.trim(),
          role: draft.role.trim(),
          traits: draft.traits_text
            .split(/\r?\n/)
            .map((l) => l.trim())
            .filter(Boolean),
          visual_card: draft.visual_card.trim(),
          voice_card: draft.voice_card.trim(),
          portrait_request: null,
        };
        updates.push(`character-draft:${character.id}:${character.traits.join("|")}`);
        return { characters: [...demoProjectData.characters, character] };
      },
      async createResource(_path, resource) {
        updates.push(`resource:${resource.key}:${resource.initial}`);
        return {
          resources: [...demoProjectData.resources, resource],
          initial_world_state: {
            ...demoProjectData.world_state,
            resources: {
              ...demoProjectData.world_state.resources,
              [resource.key]: resource.initial,
            },
          },
          initial_story_state: demoProjectData.story_state,
        };
      },
      async createRuleFromDraft(_path, draft) {
        updates.push(`rule-draft:${draft.id}`);
        return {
          rules: [
            ...demoProjectData.rules,
            {
              id: draft.id,
              action_type: draft.action_type,
              conditions: [],
              effects: draft.resource_key
                ? [{ kind: "add_resource" as const, key: draft.resource_key, amount: draft.amount }]
                : [],
            },
          ],
        };
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();

    fireEvent.click(getNavButton("World Bible"));
    fireEvent.change(screen.getByLabelText("Forbidden facts"), {
      target: { value: "No secret heir" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save World Bible" }));

    fireEvent.click(getNavButton("Story Craft"));
    fireEvent.change(screen.getByLabelText("Genre promise"), {
      target: { value: "A sharper political survival story." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Story Craft" }));

    fireEvent.click(getNavButton("Characters"));
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.change(screen.getByLabelText("New character id"), {
      target: { value: "regent" },
    });
    fireEvent.change(screen.getByLabelText("New character name"), {
      target: { value: "Regent" },
    });
    fireEvent.change(screen.getByLabelText("New character role"), {
      target: { value: "Temporary authority" },
    });
    fireEvent.change(screen.getByLabelText("New character traits"), {
      target: { value: "cautious\nclear" },
    });
    fireEvent.change(screen.getByLabelText("New visual card"), {
      target: { value: "ink portrait" },
    });
    fireEvent.change(screen.getByLabelText("New voice card"), {
      target: { value: "measured council speech" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Character" }));

    fireEvent.click(getNavButton("State"));
    fireEvent.click(screen.getByRole("button", { name: "Add Resource" }));
    fireEvent.change(screen.getByLabelText("New resource key"), {
      target: { value: "grain" },
    });
    fireEvent.change(screen.getByLabelText("New resource label"), {
      target: { value: "Grain" },
    });
    fireEvent.change(screen.getByLabelText("New resource default initial value"), {
      target: { value: "30" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Resource" }));

    fireEvent.click(getNavButton("Rules"));
    fireEvent.click(screen.getByRole("button", { name: /Add Rule/ }));
    fireEvent.change(screen.getByLabelText("New rule id"), {
      target: { value: "spend-grain" },
    });
    fireEvent.change(screen.getByLabelText("New rule action type"), {
      target: { value: "spend_grain" },
    });
    fireEvent.change(screen.getByLabelText("New rule resource"), {
      target: { value: "treasury" },
    });
    fireEvent.change(screen.getByLabelText("New rule amount"), {
      target: { value: "-3" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Rule" }));

    await waitFor(() => {
      expect(updates).toEqual([
        "world:No secret heir",
        "story:A sharper political survival story.",
        "character-draft:regent:cautious|clear",
        "resource:grain:30",
        "rule-draft:spend-grain",
      ]);
    });
  });

  it("runs generation workflows and saves AI safety policy through the data source", async () => {
    const calls: string[] = [];
    const dataSource = appTestDataSource({
      async generateWorldExpansion(_path, expansionGoal) {
        calls.push(`world-generation:${expansionGoal}`);
        return {
          document: {
            world_bible_markdown: `# World Bible\n\n${expansionGoal}\n`,
            canon_markdown: "# Canon\n- Generated canon remains reviewable.\n",
            forbidden_facts: ["Generated facts cannot erase revealed costs."],
          },
          evidence: {
            status: "succeeded",
            fallback_used: false,
            error: null,
            reproducibility: demoReproducibilityMetadata,
            envelopes: [],
          },
        };
      },
      async generateStoryCraft(_path, concept) {
        calls.push(`story-generation:${concept}`);
        return {
          document: {
            story_bible_markdown: `# Story Bible\n\n${concept}\n`,
            style_guide_markdown: "# Style Guide\n\nConsequence-first choices.\n",
            story_craft: {
              ...demoProjectData.story_craft,
              plot_threads: [
                ...demoProjectData.story_craft.plot_threads,
                {
                  id: "generated-pressure",
                  title: "Generated Pressure",
                  promise: "A generated arc creates a visible cost.",
                  thread_type: "political",
                  status: "open",
                  introduced_at: "generated-scene",
                  related_characters: [],
                  related_world_flags: [],
                  last_update: "generated",
                },
              ],
            },
          },
          evidence: {
            status: "succeeded",
            fallback_used: false,
            error: null,
            reproducibility: demoReproducibilityMetadata,
            envelopes: [],
          },
        };
      },
      async generateCharacter(_path, concept, roleHint) {
        calls.push(`character-generation:${concept}:${roleHint}`);
        return {
          character: {
            id: "generated-envoy",
            name: roleHint,
            role: "Generated story catalyst",
            traits: ["observant"],
            visual_card: "generated visual card",
            voice_card: `generated voice from ${concept}`,
            portrait_request: {
              prompt_summary: "Generated portrait request",
              style: "council portrait",
              target_asset_slot: "portrait",
              prompt_hash: "sha256:test-generated-character",
              provider_config_hash:
                demoReproducibilityMetadata.provider_config_hash,
              reference_asset_ids: [],
              fallback_allowed: true,
            },
          },
          evidence: {
            status: "succeeded",
            fallback_used: false,
            error: null,
            reproducibility: demoReproducibilityMetadata,
            envelopes: [],
          },
        };
      },
      async updateAiSafetyPolicy(_path, policy) {
        calls.push(
          `ai-policy:${policy.live_generated_content_enabled}:${policy.moderation_queue_enabled}:${policy.moderation_policy}`,
        );
        return policy;
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    await waitForDefaultSourceCanvas();

    fireEvent.click(getNavButton("World Bible"));
    fireEvent.click(screen.getByRole("button", { name: /Advanced/ }));
    fireEvent.change(screen.getByLabelText("World generation goal"), {
      target: { value: "Expand northern border canon." },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Generate World Expansion" }),
    );
    await screen.findByDisplayValue(/Expand northern border canon/);

    fireEvent.click(getNavButton("Story Craft"));
    fireEvent.change(screen.getByLabelText("Story generation concept"), {
      target: { value: "Generate three linked council pressures." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Generate StoryCraft" }));
    await screen.findByText("Generated Pressure");

    fireEvent.click(getNavButton("Characters"));
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.click(screen.getByRole("button", { name: "AI Generate", pressed: false }));
    fireEvent.change(screen.getByLabelText("Character generation concept"), {
      target: { value: "Design a grain envoy." },
    });
    fireEvent.change(screen.getByLabelText("Character generation role hint"), {
      target: { value: "Grain Envoy" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Generate Character" }));
    await screen.findByRole("button", { name: /Grain Envoy/ });
    fireEvent.click(screen.getByRole("button", { name: /Grain Envoy/ }));
    expect(screen.getByText("Generated portrait request")).toBeTruthy();

    fireEvent.click(getNavButton("Export"));
    fireEvent.click(screen.getByRole("tab", { name: /^Policy$/ }));
    fireEvent.click(screen.getByLabelText("Live generated content enabled"));
    fireEvent.click(screen.getByLabelText("Moderation queue enabled"));
    fireEvent.change(screen.getByLabelText("AI safety moderation policy"), {
      target: { value: "Creator reviews all live output before export." },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Save AI Safety Policy" }),
    );

    await waitFor(() => {
      expect(calls).toEqual([
        "world-generation:Expand northern border canon.",
        "story-generation:Generate three linked council pressures.",
        "character-generation:Design a grain envoy.:Grain Envoy",
        "ai-policy:true:true:Creator reviews all live output before export.",
      ]);
    });
  });
});

function getNavButton(name: string) {
  const navTree =
    screen.queryByRole("navigation", { name: "Studio navigation tree" }) ??
    screen.getByRole("navigation", { name: "Studio 导航树" });
  return within(navTree).getByRole("button", { name });
}

async function waitForDefaultSourceCanvas() {
  // Home is the default landing section and now renders inside StudioShell
  // (A1), so the nav tree is already mounted on load. Wait for the project
  // directory chip — its presence means the project loaded — then navigate
  // to the Source section directly via the nav tree (no ⌘K palette hop
  // needed, since the shell is already present).
  expect(
    await screen.findByLabelText(/项目目录|Project directory/i),
  ).toBeTruthy();
  fireEvent.click(getNavButton("Source"));
}

function expectExportEvidenceStatus(label: string, status: string) {
  const row = screen.getByText(label).closest("div");
  expect(row).toBeTruthy();
  expect(within(row as HTMLElement).getByText(status)).toBeTruthy();
}

function appTestDataSource(
  overrides: Partial<StudioDataSource> = {},
): StudioDataSource {
  const files: SourceFileSummary[] = [
    { path: "game.toml", kind: "toml", bytes: 120, editable: false },
    { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
  ];
  const contents: Record<string, SourceFileContent> = {
    "game.toml": {
      path: "game.toml",
      kind: "toml",
      editable: false,
      content: 'title = "Starter Project"\n',
    },
    "world/world.md": {
      path: "world/world.md",
      kind: "markdown",
      editable: true,
      content: "# World Bible\n\nThe city is under pressure.\n",
    },
  };

  const base: StudioDataSource = {
    runtimeName: "Test runtime",
    async pickProjectDirectory() {
      return null;
    },
    async createProject(path, request) {
      return {
        project_path: path,
        template: request.template,
        concept: request.concept,
        visual_style: request.visual_style,
        voice_enabled: request.voice_enabled,
        initial_scene_request: request.initial_scene_request,
        files_created: ["game.toml", "world/world.md"],
        project: demoProjectData,
      };
    },
    async openProject() {
      return demoProjectData;
    },
    async openOrCreateProject() {
      return demoProjectData;
    },
    async checkProject() {
      return {
        title: "Starter Project",
        entry_scene: "opening-scene",
        scene_count: 1,
        rule_count: 1,
        character_count: 2,
      };
    },
    async listExportProfiles() {
      return structuredClone(demoExportProfiles);
    },
    async listAssetRecords() {
      return demoProjectData.asset_records;
    },
    async readWorldEditDocument() {
      return {
        world_bible_markdown: contents["world/world.md"].content,
        canon_markdown: "# Canon\n",
        forbidden_facts: [],
      };
    },
    async updateWorldEditDocument(_path, document) {
      return document;
    },
    async readStoryCraftEditDocument() {
      return {
        story_bible_markdown: "# Story Bible\n",
        style_guide_markdown: "# Style Guide\n",
        story_craft: demoProjectData.story_craft,
      };
    },
    async updateStoryCraftEditDocument(_path, document) {
      return document;
    },
    async readCharacterEditDocument() {
      return {
        characters: demoProjectData.characters,
      };
    },
    async updateCharacterEditDocument(_path, document) {
      return document;
    },
    async createCharacter(_path, character) {
      return {
        characters: [...demoProjectData.characters, character],
      };
    },
    async createCharacterFromDraft(_path, draft) {
      const character = {
        id: draft.id.trim(),
        name: draft.name.trim(),
        role: draft.role.trim(),
        traits: draft.traits_text
          .split(/\r?\n/)
          .map((l) => l.trim())
          .filter(Boolean),
        visual_card: draft.visual_card.trim(),
        voice_card: draft.voice_card.trim(),
        portrait_request: null,
      };
      return { characters: [...demoProjectData.characters, character] };
    },
    async readStateVariablesEditDocument() {
      return {
        resources: demoProjectData.resources,
        initial_world_state: demoProjectData.world_state,
        initial_story_state: demoProjectData.story_state,
      };
    },
    async updateStateVariablesEditDocument(_path, document) {
      return document;
    },
    async createResource(_path, resource) {
      return {
        resources: [...demoProjectData.resources, resource],
        initial_world_state: demoProjectData.world_state,
        initial_story_state: demoProjectData.story_state,
      };
    },
    async readRulesEditDocument() {
      return {
        rules: demoProjectData.rules,
      };
    },
    async updateRulesEditDocument(_path, document) {
      return document;
    },
    async createRule(_path, rule) {
      return {
        rules: [...demoProjectData.rules, rule],
      };
    },
    async createRuleFromDraft(_path, draft) {
      return {
        rules: [
          ...demoProjectData.rules,
          {
            id: draft.id,
            action_type: draft.action_type,
            conditions: [],
            effects: draft.resource_key
              ? [{ kind: "add_resource" as const, key: draft.resource_key, amount: draft.amount }]
              : [],
          },
        ],
      };
    },
    async generateWorldExpansion() {
      throw new Error("not used");
    },
    async generateStoryCraft() {
      throw new Error("not used");
    },
    async generateCharacter() {
      throw new Error("not used");
    },
    async readAiSafetyPolicy() {
      return demoProjectData.ai_safety_policy;
    },
    async updateAiSafetyPolicy(_path, policy) {
      return policy;
    },
    async readVisualBible() {
      return demoProjectData.visual_bible;
    },
    async updateVisualBible(_path, visualBible) {
      return visualBible;
    },
    async readAudioBible() {
      return demoProjectData.audio_bible;
    },
    async updateAudioBible(_path, audioBible) {
      return audioBible;
    },
    async playOnceProject(_path, playerInput) {
      return demoPlayOnceReport(playerInput);
    },
    async playOnceProjectWithSave(_path, playerInput) {
      return demoPlayOnceReport(playerInput);
    },
    async playOnceProjectFromSnapshot(_path, playerInput) {
      return demoPlayOnceReport(playerInput);
    },
    async playOnceProjectFromLatestSnapshot(_path, playerInput) {
      return demoPlayOnceReport(playerInput);
    },
    async exportStaticProjectZip(_path, outputDir, archivePath) {
      return {
        output_dir: outputDir,
        archive_path: archivePath,
        files_written: [`${outputDir}/index.html`, `${outputDir}/game.json`],
        archived_files: ["game.json", "index.html"],
        allowed_files: ["game.json", "index.html"],
        files_found: ["game.json", "index.html"],
      };
    },
    async listSourceFiles() {
      return files;
    },
    async readSourceFile(_path, relativePath) {
      const file = contents[relativePath];
      if (!file) {
        throw new Error(`missing source fixture: ${relativePath}`);
      }
      return file;
    },
    async writeSourceFile(_path, relativePath, content) {
      const file = contents[relativePath];
      if (!file) {
        throw new Error(`missing source fixture: ${relativePath}`);
      }
      const updated = { ...file, content };
      contents[relativePath] = updated;
      return updated;
    },
    async piAgentRun() {
      throw new Error("piAgentRun not supported in test fixture");
    },
    async piAgentCapabilities() {
      return [];
    },
    async gitCurrentBranch() {
      return "main";
    },
    async gitListBranches() {
      return [{ name: "main", is_current: true }];
    },
    async gitSwitchBranch(_path: string, branch: string) {
      return { branch };
    },
    async listAvailableModels() {
      return [
        { id: "local-pi", label: "Local pi-Agent (offline)", provider: "local" },
      ];
    },
    async getAgentSessionConfig() {
      return {
        model_id: "local-pi",
        permission_level: "ask_every_time",
        thinking_level: "medium",
        enabled_skills: [],
        enabled_mcp_servers: [],
      };
    },
    async setAgentSessionConfig(
      _path: string,
      config: { model_id: string; permission_level: string; thinking_level: string },
    ) {
      return config as never;
    },
    async piAgentApplyRun(_request) {
      // The rail now drives pi_agent_apply_run; return a PiAgentApplyResult
      // built from the same demo report shape the TurnResult renders.
      return applyResultFromReport(demoPlayOnceReport("agent turn"));
    },
    async getUsageSummary() {
      return {
        total_input_tokens: 0,
        total_output_tokens: 0,
        total_spent_cost_units: 0,
        by_provider: {},
      };
    },
    async getProviderCostReport(providerId: string) {
      return {
        provider_id: providerId,
        text_calls: 0,
        image_calls: 0,
        tts_calls: 0,
        moderation_calls: 0,
        input_tokens: 0,
        output_tokens: 0,
        spent_cost_units: 0,
      };
    },
    async listProviders() {
      return [];
    },
    async upsertProvider(_entry: ProviderEntry) {
      return _entry;
    },
    async deleteProvider(id: string) {
      return {
        id,
        kind: "openai_compatible" as const,
        label: "",
        endpoint_url: "",
        model: "",
        credential_env_var: "",
        enabled: false,
        max_concurrency: null,
        requests_per_minute: null,
        daily_token_budget: null,
      };
    },
    async testProviderConnection(_id: string) {
      return { ok: true, message: "" };
    },
    async listRemoteModels(_providerId: string) {
      return [];
    },
    async listImageProviders() {
      return [];
    },
    async upsertImageProvider(entry: ImageProviderEntry) {
      return entry;
    },
    async deleteImageProvider(id: string) {
      return {
        id,
        endpoint_url: "",
        model: "",
        credential_env_var: "",
        enabled: false,
        default_size: "1024x1024",
        default_quality: "medium",
        max_concurrency: null,
        requests_per_minute: null,
        daily_token_budget: null,
      };
    },
    async testImageProvider(_id: string) {
      return { ok: true, message: "" };
    },
    async listTtsProviders() {
      return [];
    },
    async upsertTtsProvider(entry: TtsProviderEntry) {
      return entry;
    },
    async deleteTtsProvider(id: string) {
      return {
        id,
        endpoint_url: "",
        model: "",
        credential_env_var: "",
        enabled: false,
        voice: "coral",
        format: "mp3",
        max_concurrency: null,
        requests_per_minute: null,
        daily_token_budget: null,
      };
    },
    async testTtsProvider(_id: string) {
      return { ok: true, message: "" };
    },
    async listModerationProviders() {
      return [];
    },
    async upsertModerationProvider(entry: ModerationProviderEntry) {
      return entry;
    },
    async deleteModerationProvider(id: string) {
      return {
        id,
        endpoint_url: "",
        model: "",
        credential_env_var: "",
        enabled: false,
        max_concurrency: null,
        requests_per_minute: null,
        daily_token_budget: null,
      };
    },
    async testModerationProvider(_id: string) {
      return { ok: true, message: "" };
    },
    async listUserPromptTemplates() {
      return [];
    },
    async listProjectPromptTemplates(_projectPath: string) {
      return [];
    },
    async upsertUserPromptTemplate(template: PromptTemplate) {
      return template;
    },
    async upsertProjectPromptTemplate(_projectPath: string, template: PromptTemplate) {
      return template;
    },
    async deleteUserPromptTemplate(_id: string) {
      return;
    },
    async deleteProjectPromptTemplate(_projectPath: string, _id: string) {
      return;
    },
    async listSkills() {
      return [];
    },
    async refreshSkillIndex() {
      return { version: "1", skills: [], scanned_at: "" };
    },
    async importSkill(skillId: string) {
      return {
        id: skillId,
        name: "",
        description: "",
        source: { origin: "plot_forge_user" as const, root_path: "", rel_path: "" },
        interface: null,
        body_path: "",
        scripts: [],
        references: [],
        assets: [],
      };
    },
    async readSkillBody(_skillId: string) {
      return "";
    },
    async enableSkillForProject(_projectPath: string, _skillId: string, _enabled: boolean) {
      return {
        model_id: "local-pi",
        permission_level: "ask_every_time",
        thinking_level: "medium",
        enabled_skills: [],
        enabled_mcp_servers: [],
      } as never;
    },
    async listMcpServers(): Promise<McpServerEntry[]> {
      return [];
    },
    async upsertMcpServer(entry: McpServerEntry): Promise<McpServerEntry> {
      return entry;
    },
    async deleteMcpServer(id: string): Promise<McpServerEntry> {
      return {
        id,
        kind: "stdio",
        label: "",
        transport_config: { kind: "stdio", command: "", args: [], env: {} },
        credential_env_var: "",
        enabled: false,
      };
    },
    async testMcpServer(_id: string): Promise<McpServerTestResult> {
      return { ok: true, message: "", tools_count: 0 };
    },
    async listMcpTools(_serverId: string): Promise<McpToolManifest[]> {
      return [];
    },
    async invokeMcpTool(_request: McpToolCallRequest): Promise<McpToolCallResult> {
      return { ok: true, content: [], is_error: false };
    },
    async enableMcpServerForProject(
      _projectPath: string,
      _serverId: string,
      _enabled: boolean,
    ) {
      return {
        model_id: "local-pi",
        permission_level: "ask_every_time",
        thinking_level: "medium",
        enabled_skills: [],
        enabled_mcp_servers: [],
      } as never;
    },
  };

  return { ...base, ...overrides };
}

function fallbackPlayOnceReport(): PlayOnceReport {
  const report = demoPlayOnceReport("sk-test-secret should not render");
  const review = report.trace.narrative_review;
  if (!review) {
    throw new Error("demo report should include narrative review");
  }

  return {
    ...report,
    scene: {
      ...report.scene,
      key: "fallback-001",
      title: "Fallback Council",
      hook: "A fallback council forms because the requested scene was missing.",
    },
    trace_path: "/tmp/starter-project/traces/trace-fallback.json",
    delta_summary: [],
    trace: {
      ...report.trace,
      id: "trace-fallback",
      reproducibility: {
        ...report.trace.reproducibility,
        trace_id: "trace-fallback",
      },
      player_input: "sk-test-secret should not render",
      selected_choice: "continue-council",
      action_intent: {
        status: "supported",
        choice_id: "continue-council",
        action_type: "continue",
        matched_terms: ["continue"],
        reason: null,
      },
      rule_result: {
        action_type: "continue",
        delta_empty: true,
        state_committed: true,
        error: null,
      },
      planner_result: {
        requested_action_type: "continue",
        scene_key: "fallback-001",
        fallback_used: true,
        error: {
          code: "provider_timeout",
          message: "Provider pipeline used fallback scene [REDACTED_SECRET]",
        },
      },
      diagnostics: [
        {
          stage: "interpret_action",
          status: "completed",
          message: "action `continue` matched 1 term(s)",
        },
        {
          stage: "plan_scene",
          status: "fallback",
          message: "planner returned fallback scene `[REDACTED_SECRET]`",
        },
      ],
      narrative_review: {
        ...review,
        scene_key: "fallback-001",
        score: 42,
        hook_score: 35,
        pacing_score: 45,
        character_consistency_score: 70,
        payoff_score: 30,
        choice_meaningfulness_score: 25,
        ai_slop_risk: 68,
        issues: [
          {
            kind: "weak_hook",
            severity: "warning",
            message: "Fallback scene needs a sharper hook.",
          },
        ],
      },
      errors: [
        {
          code: "provider_timeout",
          message: "Provider pipeline used fallback scene [REDACTED_SECRET]",
        },
      ],
      fallback_used: true,
    },
  };
}
