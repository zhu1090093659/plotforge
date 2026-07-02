import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
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
  ProjectCreationRequest,
} from "../../../contracts/plotforge";
import type {
  PlayOnceReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
  }
  cleanup();
});

describe("App", () => {
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    // Source files live on the Source Artifacts tab in the Launchpad.
    fireEvent.click(screen.getByRole("tab", { name: /Source Artifacts/ }));
    expect(screen.getAllByText("world/world.md").length).toBeGreaterThan(0);
    // Open the file, then switch to the Editor tab to see the textarea.
    fireEvent.click(screen.getByRole("button", { name: /world\/world\.md/ }));
    fireEvent.click(screen.getByRole("tab", { name: /Artifact Text Editor/i }));
    expect(screen.getByDisplayValue(/The city is under pressure/)).toBeTruthy();

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

  it("switches agent-native workflows and keeps package entries local-first", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    expect(getWorkflowButton("Command Center")).toBeTruthy();
    expect(getWorkflowButton("Director Mode")).toBeTruthy();
    expect(getWorkflowButton("Agent Mesh")).toBeTruthy();
    expect(getWorkflowButton("Artifact Review")).toBeTruthy();
    expect(getWorkflowButton("Playable Proof")).toBeTruthy();
    expect(getWorkflowButton("Export Package")).toBeTruthy();
    expect(
      getWorkflowButton("Command Center").getAttribute("aria-pressed"),
    ).toBe("true");
    expect(screen.getByRole("region", { name: "Project Launchpad" })).toBeTruthy();
    expect(screen.getByText("Playable Proof Status")).toBeTruthy();
    expect(screen.getByText("Recent Runs")).toBeTruthy();
    expect(screen.getByLabelText("Director intent")).toBeTruthy();
    expect(screen.getAllByText("Command Center").length).toBeGreaterThan(0);
    const commandDock = screen.getByLabelText("Command dock");
    expect(within(commandDock).getByText("Command Dock")).toBeTruthy();
    expect(
      within(commandDock).getByRole("button", { name: "Run playable proof" }),
    ).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Director intent"), {
      target: { value: "pay the army and show the consequence" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Apply as proof run" }));
    await screen.findByText("Runtime Trace");
    expect(screen.getAllByText("trace-001").length).toBeGreaterThan(0);
    expect(screen.getAllByText("State Delta").length).toBeGreaterThan(0);

    openWorkflowDefaultSection("Director Mode", "Playtest");

    expect(screen.getAllByText("Director Mode").length).toBeGreaterThan(0);
    expect(screen.getByLabelText("Playtest input")).toBeTruthy();

    openWorkflowDefaultSection("Export Package", "Export");

    expect(screen.getAllByText("Export Package").length).toBeGreaterThan(0);
    // Profile selectors live on the Profile tab.
    fireEvent.click(screen.getByRole("tab", { name: /^Profile/ }));
    expect(screen.getAllByText("steam-submission-kit").length).toBeGreaterThan(0);
    expect(document.body.textContent ?? "").not.toMatch(
      /one-click Steam launch|automatic publishing|approval guarantee|legal guarantee|real pi-Agent execution|real external agent execution/i,
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: "Select export profile steam-submission-kit",
      }),
    );
    expect(screen.getAllByText("steam_submission_kit").length).toBeGreaterThan(0);
    expect(
      screen.getByText(
        "This profile does not call Steamworks APIs or promise approval.",
      ),
    ).toBeTruthy();
  });

  it("does not render injected fake agent workers or approval queues", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    expect(screen.getByText("Backend Boundary")).toBeTruthy();
    expect(screen.getByText("Real Studio command surface")).toBeTruthy();
    expect(screen.getAllByText("External agents").length).toBeGreaterThan(0);
    expect(screen.queryByText("Mock pi-Agent Worker")).toBeNull();
    expect(screen.queryByText("Approve local preview patch")).toBeNull();
    expect(screen.queryByText("local-preview-only")).toBeNull();
    // Agent Mesh workflow and its default surface share the label "Agent Mesh".
    // Click the workflow to expand (first match), then the surface (second match).
    fireEvent.click(screen.getAllByRole("button", { name: "Agent Mesh" })[0]);
    fireEvent.click(
      screen.getAllByRole("button", { name: "Agent Mesh" })[1],
    );
    expect(screen.getByRole("region", { name: "Agent Mesh Workspace" }))
      .toBeTruthy();
    expect(screen.getByText("Studio-backed capabilities")).toBeTruthy();
    expect(screen.getAllByText("pi-Agent runtime").length).toBeGreaterThan(0);
    expect(screen.getAllByText("not implemented").length).toBeGreaterThan(0);
    expect(screen.queryByText("Mock Story Agent")).toBeNull();
  });

  it("switches Studio chrome between English and Chinese", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    expect(screen.getAllByText("Project Launchpad").length).toBeGreaterThan(0);

    fireEvent.change(screen.getByLabelText("Language"), {
      target: { value: "zh" },
    });

    await waitFor(() => {
      expect(screen.getAllByText("项目启动台").length).toBeGreaterThan(0);
      expect(screen.getAllByText("运行可玩证明").length).toBeGreaterThan(0);
    });
    if (typeof window.localStorage?.getItem === "function") {
      expect(window.localStorage.getItem("plotforge:creator-desktop:locale")).toBe(
        "zh",
      );
    }

    fireEvent.click(getWorkflowButton("导演模式"));
    fireEvent.click(await screen.findByRole("button", { name: "试玩" }));
    fireEvent.click(screen.getByRole("button", { name: "高级快照控制" }));
    await waitFor(() => {
      expect(screen.getByText("导演指令栏")).toBeTruthy();
      expect(screen.getByText("运行一次运行时回合")).toBeTruthy();
      expect(screen.getByText(/\d+ 个选择/)).toBeTruthy();
      expect(screen.getByText("恢复最新")).toBeTruthy();
      expect(
        screen.getByText(
          "当前没有可用的决策队列。运行一个回合以生成运行时证据；Agent 审批队列尚未实现。",
        ),
      ).toBeTruthy();
    });

    fireEvent.click(getWorkflowButton("产物审查"));
    fireEvent.click(await screen.findByRole("button", { name: "资产" }));
    await waitFor(() => {
      expect(screen.getAllByText("实时构建室").length).toBeGreaterThan(0);
      expect(screen.getAllByText("未捕获").length).toBeGreaterThan(0);
      expect(screen.getByText("项目中已加载 1 个可编辑界面。")).toBeTruthy();
      expect(screen.getByText("运行可玩证明以生成追踪证据。")).toBeTruthy();
      expect(screen.getByText("当前源产物")).toBeTruthy();
      expect(screen.getByText("本会话尚未运行运行时证明。")).toBeTruthy();
    });

    fireEvent.change(screen.getByLabelText("语言"), {
      target: { value: "en" },
    });

    await waitFor(() => {
      expect(screen.getAllByText("Live Build Room").length).toBeGreaterThan(0);
      expect(screen.getAllByText("Artifact Review").length).toBeGreaterThan(0);
      expect(screen.getByText("Current Source Artifacts")).toBeTruthy();
      expect(screen.getByText("No runtime proof has been run for this session."))
        .toBeTruthy();
    });
    expect(screen.queryByText("实时构建室")).toBeNull();
  });

  it("renders asset records and visual-audio bible cards before background fallback", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Artifact Review", "Assets");

    expect(screen.getByText("2 asset records")).toBeTruthy();
    // Asset records live on the Asset Catalog tab.
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
    // Visual Bible cards live on the Visual Bible tab; switch back to it.
    fireEvent.click(screen.getByRole("tab", { name: /Visual Bible/i }));
    expect(screen.getByText("Winter council ink wash")).toBeTruthy();
    expect(screen.getByText("Official portrait restraint")).toBeTruthy();
    // Audio Bible cards live on the Audio Bible tab.
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Artifact Review", "Assets");

    expect(screen.getByText("1 scene background fallbacks")).toBeTruthy();
    // Scene background fallbacks live on the Asset Catalog tab.
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Artifact Review", "Assets");
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
    // Audio Bible cards live on the Audio Bible tab; switch to it.
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

  it("runs playtest and renders fallback trace, errors, review, and diagnostics without raw keys", async () => {
    const fallbackReport = fallbackPlayOnceReport();
    const dataSource = appTestDataSource({
      async playOnceProject() {
        return fallbackReport;
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Director Mode", "Playtest");
    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "continue" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Advanced snapshot controls" }));
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("Fallback Council");
    expect(screen.getAllByText("trace-fallback").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Fallback").length).toBeGreaterThan(0);
    expect(screen.getAllByText("provider_timeout").length).toBeGreaterThan(0);

    // Expand Technical Details to verify trace debug content
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
    expect(screen.getAllByText("continue-council").length).toBeGreaterThan(0);
    expect(screen.getAllByText("continue").length).toBeGreaterThan(0);
    expect(screen.getByText("background_asset")).toBeTruthy();
    expect(screen.getAllByText("assets/generated/opening-scene.png").length)
      .toBeGreaterThan(0);
    expect(
      screen.getByText("planner returned fallback scene `[REDACTED_SECRET]`"),
    ).toBeTruthy();
    expect(
      screen.getAllByText(
        "Provider pipeline used fallback scene [REDACTED_SECRET]",
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText(/sk-test-secret/)).toBeNull();

    // After a successful run, the workspace switches to proof/debugger.  The
    // "Playable Proof" workflow must be expanded so the "Debugger" surface is
    // visible in the sidebar tree (regression: runPlaytest must expand target).
    expect(
      within(
        screen.getByRole("navigation", { name: "Studio navigation tree" }),
      ).getByRole("button", { name: "Debugger" }),
    ).toBeTruthy();
  });

  it("runs playtest with explicit and latest runtime snapshot controls", async () => {
    const calls: Array<
      | {
          method: "snapshot";
          path: string;
          playerInput: string;
          snapshotId: string;
          saveId: string | null;
        }
      | {
          method: "latest";
          path: string;
          playerInput: string;
          saveId: string | null;
        }
    > = [];
    const dataSource = appTestDataSource({
      async playOnceProjectFromSnapshot(path, playerInput, snapshotId, saveId = null) {
        calls.push({ method: "snapshot", path, playerInput, snapshotId, saveId });
        return playOnceReportWithSnapshot(
          playerInput,
          saveId ?? snapshotId,
          `/tmp/starter-project/saves/${saveId ?? snapshotId}.runtime_snapshot.json`,
        );
      },
      async playOnceProjectFromLatestSnapshot(path, playerInput, saveId = null) {
        calls.push({ method: "latest", path, playerInput, saveId });
        return playOnceReportWithSnapshot(
          playerInput,
          saveId ?? "latest",
          `/tmp/starter-project/saves/${saveId ?? "latest"}.runtime_snapshot.json`,
        );
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Director Mode", "Playtest");

    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "pay the army" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Advanced snapshot controls" }));
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "save-after-army" },
    });
    fireEvent.change(screen.getByLabelText("Playtest restore id"), {
      target: { value: "save-before-army" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("/tmp/starter-project/saves/save-after-army.runtime_snapshot.json");
    // After a successful run, the workspace switches to the proof/debugger
    // section.  Navigate back to the Playtest surface under Director Mode
    // (still expanded) to run the next snapshot turn.  Scope to Director Mode's
    // children region because "Playtest" also appears under "Playable Proof".
    const directorRegionId = getWorkflowButton("Director Mode").getAttribute("aria-controls");
    fireEvent.click(
      within(document.getElementById(directorRegionId!)!).getByRole("button", { name: "Playtest" }),
    );
    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "raise emergency taxes" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Advanced snapshot controls" }));
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "save-after-tax" },
    });
    fireEvent.click(screen.getByLabelText("Restore latest save"));
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("/tmp/starter-project/saves/save-after-tax.runtime_snapshot.json");
    expect(calls).toEqual([
      {
        method: "snapshot",
        path: "/tmp/starter-project",
        playerInput: "pay the army",
        snapshotId: "save-before-army",
        saveId: "save-after-army",
      },
      {
        method: "latest",
        path: "/tmp/starter-project",
        playerInput: "raise emergency taxes",
        saveId: "save-after-tax",
      },
    ]);
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Export Package", "Export");
    // Profile selectors + selected-profile detail live on the Profile tab.
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

    // Output paths + export button live on the Package tab.
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
    expectExportEvidenceStatus("All referenced assets copied", "Pass");
    // Permanently-pending checks are under the "Technical Details" collapsible — expand it first
    fireEvent.click(screen.getByRole("button", { name: /Technical Details/ }));
    expectExportEvidenceStatus("No raw responses", "Pending");
    expectExportEvidenceStatus("No secret markers", "Pending");
    expectExportEvidenceStatus("HTTP smoke test passed", "Pending");
  });

  it("opens the executable static profile from the Command Center export CTA", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    const launchpad = screen.getByRole("region", { name: "Project Launchpad" });
    fireEvent.click(within(launchpad).getByRole("button", { name: "Export Package" }));

    // The export CTA switches to the Export section; the static-web profile
    // selector lives on the Profile tab.
    fireEvent.click(screen.getByRole("tab", { name: /^Profile/ }));
    expect(screen.getAllByText("static-web").length).toBeGreaterThan(0);
    expect(
      screen
        .getByRole("button", { name: "Select export profile static-web" })
        .getAttribute("aria-pressed"),
    ).toBe("true");
    // Output directory is on the Package tab.
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    openWorkflowDefaultSection("Export Package", "Export");
    // Profile selector + selected-profile detail live on the Profile tab.
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
    // The non-executable notice + Export zip button live on the Package tab.
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

  it("creates a project from wizard fields and reloads the created folder", async () => {
    const createdProject = {
      ...structuredClone(demoProjectData),
      game: {
        ...demoProjectData.game,
        title: "Winter Regency",
        description: "A frozen council succession crisis.",
      },
    };
    let currentProject = demoProjectData;
    const calls: Array<{
      path: string;
      request: ProjectCreationRequest;
      force: boolean;
    }> = [];
    const openedPaths: string[] = [];
    const dataSource = appTestDataSource({
      async createProject(path, request, force) {
        calls.push({ path, request, force });
        currentProject = createdProject;
        return {
          project_path: path,
          template: request.template,
          concept: request.concept,
          visual_style: request.visual_style,
          voice_enabled: request.voice_enabled,
          initial_scene_request: request.initial_scene_request,
          files_created: [
            "game.toml",
            "world/world.md",
            "story/story_bible.md",
          ],
          project: createdProject,
        };
      },
      async openProject(path) {
        openedPaths.push(path);
        return currentProject;
      },
      async checkProject() {
        return {
          title: currentProject.game.title,
          entry_scene: currentProject.game.entry_scene,
          scene_count: currentProject.scenes.length,
          rule_count: currentProject.rules.length,
          character_count: currentProject.characters.length,
        };
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/starter-project" />,
    );

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();

    // The New Project tab is active by default in the Launchpad; the form is visible.
    fireEvent.change(screen.getByLabelText("New project path"), {
      target: { value: "/tmp/winter-regency" },
    });
    fireEvent.change(screen.getByLabelText("Visual style"), {
      target: { value: "ink wash winter council" },
    });
    fireEvent.change(screen.getByLabelText("Concept"), {
      target: { value: "A frozen council succession crisis." },
    });
    fireEvent.change(screen.getByLabelText("Initial scene request"), {
      target: { value: "Open with a sealed imperial edict." },
    });
    fireEvent.click(screen.getByLabelText("Voice enabled"));
    fireEvent.click(screen.getByLabelText("Overwrite existing path"));
    fireEvent.click(screen.getByRole("button", { name: "Create project" }));

    await waitFor(() => {
      expect(calls).toEqual([
        {
          path: "/tmp/winter-regency",
          request: {
            template: "historical_crisis",
            concept: "A frozen council succession crisis.",
            visual_style: "ink wash winter council",
            voice_enabled: true,
            initial_scene_request: "Open with a sealed imperial edict.",
          },
          force: true,
        },
      ]);
    });
    await waitFor(() => {
      expect(screen.getAllByText("Winter Regency").length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText("/tmp/winter-regency").length).toBeGreaterThan(0);
    expect(openedPaths).toContain("/tmp/winter-regency");
  });

  it("exposes PRD navigation and saves structured editing forms through the data source", async () => {
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
      async createCharacter(_path, character) {
        updates.push(`character:${character.id}:${character.traits.join("|")}`);
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
      async createRule(_path, rule) {
        updates.push(`rule:${rule.id}:${rule.effects[0]?.kind}`);
        return {
          rules: [...demoProjectData.rules, rule],
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();
    for (const label of [
      "Command Center",
      "Director Mode",
      "Agent Mesh",
      "Artifact Review",
      "Playable Proof",
      "Export Package",
    ]) {
      expect(screen.getAllByRole("button", { name: new RegExp(label) }).length)
        .toBeGreaterThan(0);
    }

    fireEvent.click(screen.getByRole("button", { name: /World Bible/ }));
    fireEvent.change(screen.getByLabelText("Forbidden facts"), {
      target: { value: "No secret heir" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save World Bible" }));

    fireEvent.click(screen.getByRole("button", { name: /Story Craft/ }));
    fireEvent.change(screen.getByLabelText("Genre promise"), {
      target: { value: "A sharper political survival story." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Story Craft" }));

    fireEvent.click(getWorkflowButton("Director Mode"));
    fireEvent.click(getWorkflowButton("Characters"));
    // Open the "Add Character" collapsible (defaults to Manual mode).
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

    fireEvent.click(screen.getByRole("button", { name: /State/ }));
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

    fireEvent.click(screen.getByRole("button", { name: /Rules/ }));
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

    expect(await screen.findAllByText("Starter Project")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /World Bible/ }));
    // AI expansion goal is in the "Advanced" collapsible — expand it first.
    fireEvent.click(screen.getByRole("button", { name: /Advanced/ }));
    fireEvent.change(screen.getByLabelText("World generation goal"), {
      target: { value: "Expand northern border canon." },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Generate World Expansion" }),
    );
    await screen.findByDisplayValue(/Expand northern border canon/);

    fireEvent.click(screen.getByRole("button", { name: /Story Craft/ }));
    fireEvent.change(screen.getByLabelText("Story generation concept"), {
      target: { value: "Generate three linked council pressures." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Generate StoryCraft" }));
    await screen.findByText("Generated Pressure");

    fireEvent.click(getWorkflowButton("Director Mode"));
    fireEvent.click(getWorkflowButton("Characters"));
    // Open the "Add Character" collapsible and switch to AI Generate mode.
    fireEvent.click(screen.getByRole("button", { name: "Add Character" }));
    fireEvent.click(screen.getByRole("button", { name: "AI Generate", pressed: false }));
    fireEvent.change(screen.getByLabelText("Character generation concept"), {
      target: { value: "Design a grain envoy." },
    });
    fireEvent.change(screen.getByLabelText("Character generation role hint"), {
      target: { value: "Grain Envoy" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Generate Character" }));
    // After generation the new character card appears as a collapsed item.
    // The Collapsible label is the character name (roleHint passed as name).
    await screen.findByRole("button", { name: /Grain Envoy/ });
    // Expand the card to verify portrait request is visible.
    fireEvent.click(screen.getByRole("button", { name: /Grain Envoy/ }));
    expect(screen.getByText("Generated portrait request")).toBeTruthy();

    openWorkflowDefaultSection("Export Package", "Export");
    // The AI Safety Policy editor lives on the Policy tab.
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

function getWorkflowButton(name: string) {
  // The workflow row is a single button that toggles expand/collapse on click
  // (for workflows with children) or invokes onSelect (for childless items).
  // Selecting by role+name inside the nav tree targets the labeled button.
  return within(
    screen.getByRole("navigation", { name: "Studio navigation tree" }),
  ).getByRole("button", { name });
}
/**
 * Expand the given workflow by clicking its row, then click its default
 * surface section.  Used by tests that previously relied on a single click on
 * a workflow to jump to its default section; the tree navigation now requires
 * two steps (expand the workflow, then pick the section).
 *
 * The surface lookup is scoped to the clicked workflow's children region
 * (via aria-controls) so that the same section name appearing under multiple
 * expanded workflows does not cause a multiple-match error.
 */
function openWorkflowDefaultSection(workflowName: string, sectionName: string) {
  const workflowButton = getWorkflowButton(workflowName);
  fireEvent.click(workflowButton);
  const regionId = workflowButton.getAttribute("aria-controls");
  if (!regionId) {
    throw new Error(
      `Workflow "${workflowName}" has no aria-controls; cannot scope surface lookup`,
    );
  }
  const region = document.getElementById(regionId);
  if (!region) {
    throw new Error(
      `Workflow "${workflowName}" children region #${regionId} not found`,
    );
  }
  fireEvent.click(
    within(region).getByRole("button", { name: sectionName }),
  );
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
      // Simulate what Rust does: trim and split traits.
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
    async generateWorldExpansion(_path, expansionGoal) {
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
    async playOnceProjectWithSave(...args: Parameters<typeof mockPlayOnceProjectWithSave>) {
      return mockPlayOnceProjectWithSave(...args);
    },
    async playOnceProjectFromSnapshot(...args: Parameters<typeof mockPlayOnceProjectFromSnapshot>) {
      return mockPlayOnceProjectFromSnapshot(...args);
    },
    async playOnceProjectFromLatestSnapshot(...args: Parameters<typeof mockPlayOnceProjectFromLatestSnapshot>) {
      return mockPlayOnceProjectFromLatestSnapshot(...args);
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
