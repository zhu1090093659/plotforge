import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import {
  demoPlayOnceReport,
  demoProjectData,
  demoReproducibilityMetadata,
} from "./demoStudioData";
import type { StudioDataSource } from "./studioDataSource";
import type {
  AssetRecord,
  ProjectCreationRequest,
} from "../../../contracts/plotforge";
import type {
  PlayOnceReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

afterEach(cleanup);

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
        content: 'title = "Dynasty Embers"\n',
      },
      "world/world.md": {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n\nThe dynasty is under pressure.\n",
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    expect(screen.getAllByText("world/world.md").length).toBeGreaterThan(0);
    expect(screen.getByDisplayValue(/The dynasty is under pressure/)).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Source editor"), {
      target: { value: "# World Bible\n\nThe court has changed.\n" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(writes).toEqual([
        {
          relativePath: "world/world.md",
          content: "# World Bible\n\nThe court has changed.\n",
        },
      ]);
    });
  });

  it("renders asset records and visual-audio bible cards before background fallback", async () => {
    const dataSource = appTestDataSource();

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Assets/ }));

    expect(screen.getByText("2 asset records")).toBeTruthy();
    expect(screen.getAllByText("asset-image-court-crisis-001").length)
      .toBeGreaterThan(0);
    expect(screen.getByText("image / generated")).toBeTruthy();
    expect(
      screen.getAllByText("sha256:2c60d8f6f2f16f4ff6b5a5e4f7a20c2c6a18f3c4d9d3b7319dd6127a98d8a501")
        .length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("sha256").length).toBeGreaterThan(0);
    expect(screen.getByText("plotforge-local-mock / plotforge-local-mock-image-v1")).toBeTruthy();
    expect(screen.getByText("mock-image-court-crisis-001")).toBeTruthy();
    expect(screen.getByText("sha256:demo-court-crisis-prompt")).toBeTruthy();
    expect(
      screen.getByText("scene:court-crisis-001:background_asset"),
    ).toBeTruthy();
    expect(screen.getAllByText("asset-voice-censor-001").length)
      .toBeGreaterThan(0);
    expect(screen.getAllByText("Fallback").length).toBeGreaterThan(0);
    expect(screen.getByText("Visual Bible")).toBeTruthy();
    expect(screen.getByText("Winter court ink wash")).toBeTruthy();
    expect(screen.getByText("Official portrait restraint")).toBeTruthy();
    expect(screen.getByText("Audio Bible")).toBeTruthy();
    expect(screen.getByText("Court Censor")).toBeTruthy();
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Assets/ }));

    expect(screen.getByText("1 scene background fallbacks")).toBeTruthy();
    expect(screen.getByText("Scene background fallback")).toBeTruthy();
    expect(screen.getByText("assets/generated/court-crisis-001.png")).toBeTruthy();
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Assets/ }));

    fireEvent.change(screen.getByLabelText("Visual style prompt 1"), {
      target: { value: "Ink court with harsher winter lanterns." },
    });
    fireEvent.change(screen.getByLabelText("Visual style palette 1"), {
      target: { value: "bone white\nseal red" },
    });
    fireEvent.change(screen.getByLabelText("Visual style tags 1"), {
      target: { value: "court\nwinter" },
    });
    fireEvent.change(screen.getByLabelText("Visual style reference asset ids 1"), {
      target: { value: "asset-image-court-crisis-001\nasset-style-ref-002" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save Visual Bible" }));

    await waitFor(() => {
      expect(updates).toHaveLength(1);
    });

    fireEvent.change(screen.getByLabelText("Audio voice 1"), {
      target: { value: "dry formal court voice" },
    });
    fireEvent.change(screen.getByLabelText("Audio delivery 1"), {
      target: { value: "quiet but cutting" },
    });
    fireEvent.change(screen.getByLabelText("Audio tags 1"), {
      target: { value: "court\nformal" },
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
        "visual:Ink court with harsher winter lanterns.:bone white|seal red:court|winter:asset-image-court-crisis-001|asset-style-ref-002",
        "audio:dry formal court voice:quiet but cutting:court|formal:The ledgers do not accuse by accident.:asset-voice-censor-001|asset-voice-ref-002",
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Playtest" }));

    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "continue" },
    });
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("Fallback Council");
    expect(screen.getAllByText("trace-fallback").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Fallback").length).toBeGreaterThan(0);
    expect(screen.getAllByText("provider_timeout").length).toBeGreaterThan(0);
    expect(screen.getByText("weak_hook")).toBeTruthy();
    expect(screen.getByText("Trace Evidence")).toBeTruthy();
    expect(screen.getByText("Action Intent")).toBeTruthy();
    expect(screen.getByText("Rule Result")).toBeTruthy();
    expect(screen.getByText("Planner Result")).toBeTruthy();
    expect(screen.getByText("Reproducibility")).toBeTruthy();
    expect(screen.getByText("plotforge-local-mock-prompt-v1")).toBeTruthy();
    expect(screen.getByText("plotforge-local-mock-model-v1")).toBeTruthy();
    expect(
      screen.getByText("sha256:plotforge-local-mock-provider-config-v1"),
    ).toBeTruthy();
    expect(screen.getByText("State Delta")).toBeTruthy();
    expect(screen.getByText("Media References")).toBeTruthy();
    expect(screen.getAllByText("continue-council").length).toBeGreaterThan(0);
    expect(screen.getAllByText("continue").length).toBeGreaterThan(0);
    expect(screen.getByText("background_asset")).toBeTruthy();
    expect(screen.getByText("assets/generated/court-crisis-001.png")).toBeTruthy();
    expect(
      screen.getByText("planner returned fallback scene `[REDACTED_SECRET]`"),
    ).toBeTruthy();
    expect(
      screen.getAllByText(
        "Provider pipeline used fallback scene [REDACTED_SECRET]",
      ).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText(/sk-test-secret/)).toBeNull();
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
          `/tmp/dynasty-embers/saves/${saveId ?? snapshotId}.runtime_snapshot.json`,
        );
      },
      async playOnceProjectFromLatestSnapshot(path, playerInput, saveId = null) {
        calls.push({ method: "latest", path, playerInput, saveId });
        return playOnceReportWithSnapshot(
          playerInput,
          saveId ?? "latest",
          `/tmp/dynasty-embers/saves/${saveId ?? "latest"}.runtime_snapshot.json`,
        );
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Playtest" }));

    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "pay the army" },
    });
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "save-after-army" },
    });
    fireEvent.change(screen.getByLabelText("Playtest restore id"), {
      target: { value: "save-before-army" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("/tmp/dynasty-embers/saves/save-after-army.runtime_snapshot.json");

    fireEvent.click(screen.getByRole("button", { name: "Playtest" }));
    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "raise emergency taxes" },
    });
    fireEvent.change(screen.getByLabelText("Playtest save id"), {
      target: { value: "save-after-tax" },
    });
    fireEvent.click(screen.getByLabelText("Restore latest save"));
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("/tmp/dynasty-embers/saves/save-after-tax.runtime_snapshot.json");
    expect(calls).toEqual([
      {
        method: "snapshot",
        path: "/tmp/dynasty-embers",
        playerInput: "pay the army",
        snapshotId: "save-before-army",
        saveId: "save-after-army",
      },
      {
        method: "latest",
        path: "/tmp/dynasty-embers",
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
    fireEvent.change(screen.getByLabelText("Static export output directory"), {
      target: { value: "/tmp/static-export" },
    });
    fireEvent.change(screen.getByLabelText("Static export zip archive"), {
      target: { value: "/tmp/dynasty-embers.zip" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Export zip" }));

    await waitFor(() => {
      expect(exports).toEqual([
        {
          path: "/tmp/dynasty-embers",
          outputDir: "/tmp/static-export",
          archivePath: "/tmp/dynasty-embers.zip",
        },
      ]);
    });
    expect(screen.getByText("/tmp/dynasty-embers.zip")).toBeTruthy();
    expect(screen.getByText("matched")).toBeTruthy();
  });

  it("creates a project from wizard fields and reloads the created folder", async () => {
    const createdProject = {
      ...structuredClone(demoProjectData),
      game: {
        ...demoProjectData.game,
        title: "Winter Regency",
        description: "A frozen court succession crisis.",
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("New project path"), {
      target: { value: "/tmp/winter-regency" },
    });
    fireEvent.change(screen.getByLabelText("Visual style"), {
      target: { value: "ink wash winter court" },
    });
    fireEvent.change(screen.getByLabelText("Concept"), {
      target: { value: "A frozen court succession crisis." },
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
            concept: "A frozen court succession crisis.",
            visual_style: "ink wash winter court",
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
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    for (const label of [
      "Dashboard",
      "World Bible",
      "Story Craft",
      "Characters",
      "State",
      "Rules",
      "Assets",
      "Playtest",
      "Debugger",
      "Export",
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

    fireEvent.click(screen.getByRole("button", { name: /Characters/ }));
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
      target: { value: "measured court speech" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Character" }));

    fireEvent.click(screen.getByRole("button", { name: /State/ }));
    fireEvent.change(screen.getByLabelText("New resource key"), {
      target: { value: "grain" },
    });
    fireEvent.change(screen.getByLabelText("New resource label"), {
      target: { value: "Grain" },
    });
    fireEvent.change(screen.getByLabelText("New resource initial"), {
      target: { value: "30" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create Resource" }));

    fireEvent.click(screen.getByRole("button", { name: /Rules/ }));
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
        "character:regent:cautious|clear",
        "resource:grain:30",
        "rule:spend-grain:add_resource",
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
              style: "court portrait",
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
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /World Bible/ }));
    fireEvent.change(screen.getByLabelText("World generation goal"), {
      target: { value: "Expand northern border canon." },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Generate World Expansion" }),
    );
    await screen.findByDisplayValue(/Expand northern border canon/);

    fireEvent.click(screen.getByRole("button", { name: /Story Craft/ }));
    fireEvent.change(screen.getByLabelText("Story generation concept"), {
      target: { value: "Generate three linked court pressures." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Generate StoryCraft" }));
    await screen.findByText("Generated Pressure");

    fireEvent.click(screen.getByRole("button", { name: /Characters/ }));
    fireEvent.change(screen.getByLabelText("Character generation concept"), {
      target: { value: "Design a grain envoy." },
    });
    fireEvent.change(screen.getByLabelText("Character generation role hint"), {
      target: { value: "Grain Envoy" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Generate Character" }));
    await screen.findByDisplayValue("generated-envoy");
    expect(screen.getByText("Generated portrait request")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: /Export/ }));
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
        "story-generation:Generate three linked court pressures.",
        "character-generation:Design a grain envoy.:Grain Envoy",
        "ai-policy:true:true:Creator reviews all live output before export.",
      ]);
    });
  });
});

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
      content: 'title = "Dynasty Embers"\n',
    },
    "world/world.md": {
      path: "world/world.md",
      kind: "markdown",
      editable: true,
      content: "# World Bible\n\nThe dynasty is under pressure.\n",
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
        title: "Dynasty Embers",
        entry_scene: "court-crisis-001",
        scene_count: 1,
        rule_count: 1,
        character_count: 2,
      };
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
            style: "court portrait",
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
    async playOnceProjectWithSave(_path, playerInput, saveId) {
      const report = demoPlayOnceReport(playerInput);
      return {
        ...report,
        snapshot: {
          id: saveId,
          timestamp_ms: report.trace.timestamp_ms,
          reproducibility: {
            ...demoReproducibilityMetadata,
            snapshot_id: saveId,
          },
          project_id: demoProjectData.game.id,
          project_version: demoProjectData.game.version,
          story_state: report.trace.story_state_after,
          world_state: report.trace.world_state_after,
          scenes: demoProjectData.scenes,
        },
        snapshot_path: `/tmp/dynasty-embers/saves/${saveId}.runtime_snapshot.json`,
      };
    },
    async playOnceProjectFromSnapshot(_path, playerInput, _snapshotId, saveId = null) {
      return saveId
        ? this.playOnceProjectWithSave("/tmp/dynasty-embers", playerInput, saveId)
        : demoPlayOnceReport(playerInput);
    },
    async playOnceProjectFromLatestSnapshot(_path, playerInput, saveId = null) {
      return saveId
        ? this.playOnceProjectWithSave("/tmp/dynasty-embers", playerInput, saveId)
        : demoPlayOnceReport(playerInput);
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
    trace_path: "/tmp/dynasty-embers/traces/trace-fallback.json",
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

function playOnceReportWithSnapshot(
  playerInput: string,
  snapshotId: string,
  snapshotPath: string,
): PlayOnceReport {
  const report = demoPlayOnceReport(playerInput);
  return {
    ...report,
    snapshot: {
      id: snapshotId,
      timestamp_ms: report.trace.timestamp_ms,
      reproducibility: {
        ...demoReproducibilityMetadata,
        snapshot_id: snapshotId,
      },
      project_id: demoProjectData.game.id,
      project_version: demoProjectData.game.version,
      story_state: report.trace.story_state_after,
      world_state: report.trace.world_state_after,
      scenes: demoProjectData.scenes,
    },
    snapshot_path: snapshotPath,
  };
}
