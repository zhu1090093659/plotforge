import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import {
  demoExportProfiles,
  demoPlayOnceReport,
  demoProjectData,
} from "./demoStudioData";
import { StudioI18nProvider } from "./i18n";
import type { StudioDataSource } from "./studioDataSource";
import type {
  McpServerEntry,
  McpServerTestResult,
  McpToolCallRequest,
  McpToolCallResult,
  McpToolManifest,
  ProviderEntry,
  PromptTemplate,
} from "../../../contracts/plotforge";
import type {
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";
import { useStudioWorkspace } from "./useStudioWorkspace";

afterEach(cleanup);

describe("useStudioWorkspace", () => {
  it("loads project workspace state and refreshes derived project data", async () => {
    let sceneCount = 1;
    const dataSource = workspaceTestDataSource({
      async checkProject() {
        return {
          title: "Starter Project",
          entry_scene: "opening-scene",
          scene_count: sceneCount,
          rule_count: 1,
          character_count: 2,
        };
      },
    });

    render(
      <StudioI18nProvider>
        <WorkspaceProbe dataSource={dataSource} />
      </StudioI18nProvider>,
    );

    expect(await screen.findByText("title:Starter Project")).toBeTruthy();
    expect(screen.getByText("loaded:/tmp/starter-project")).toBeTruthy();
    expect(screen.getByText("selected:world/world.md")).toBeTruthy();
    expect(screen.getByText("metric-scenes:1")).toBeTruthy();

    sceneCount = 3;
    fireEvent.click(screen.getByRole("button", { name: "Refresh overview" }));

    await screen.findByText("metric-scenes:3");
  });

  it("saves selected source files and keeps dirty state visible", async () => {
    const writes: string[] = [];
    const dataSource = workspaceTestDataSource({
      async writeSourceFile(_path, relativePath, content) {
        writes.push(`${relativePath}:${content}`);
        return {
          path: relativePath,
          kind: "markdown",
          editable: true,
          content,
        };
      },
    });

    render(
      <StudioI18nProvider>
        <WorkspaceProbe dataSource={dataSource} />
      </StudioI18nProvider>,
    );

    await screen.findByText("dirty:false");
    fireEvent.change(screen.getByLabelText("Probe editor"), {
      target: { value: "# World Bible\n\nChanged.\n" },
    });
    expect(screen.getByText("dirty:true")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Save source" }));

    await waitFor(() => {
      expect(writes).toEqual(["world/world.md:# World Bible\n\nChanged.\n"]);
    });
    await screen.findByText("dirty:false");
  });

  it("surfaces load and refresh failures explicitly", async () => {
    const loadFailure = workspaceTestDataSource({
      async openProject() {
        throw new Error("cannot load project");
      },
    });

    render(
      <StudioI18nProvider>
        <WorkspaceProbe dataSource={loadFailure} />
      </StudioI18nProvider>,
    );
    expect(await screen.findByText("error:cannot load project")).toBeTruthy();

    cleanup();

    const refreshFailure = workspaceTestDataSource({
      async checkProject() {
        throw new Error("cannot refresh project");
      },
    });

    render(
      <StudioI18nProvider>
        <WorkspaceProbe dataSource={refreshFailure} />
      </StudioI18nProvider>,
    );
    await screen.findByText("error:cannot refresh project");
  });

  it("renders object-shaped failures without object string coercion", async () => {
    const objectFailure = workspaceTestDataSource({
      async openProject() {
        throw { reason: "cannot load project", stage: "openProject" };
      },
    });

    render(
      <StudioI18nProvider>
        <WorkspaceProbe dataSource={objectFailure} />
      </StudioI18nProvider>,
    );

    expect(
      await screen.findByText(
        'error:{"reason":"cannot load project","stage":"openProject"}',
      ),
    ).toBeTruthy();
    expect(screen.queryByText(/error:\[object Object\]/)).toBeNull();
  });

  it("keeps playtest and export state inside the workspace boundary", async () => {
    const dataSource = workspaceTestDataSource();

    render(
      <StudioI18nProvider>
        <WorkspaceProbe dataSource={dataSource} />
      </StudioI18nProvider>,
    );

    await screen.findByText("selected-export:static-web");

    fireEvent.change(screen.getByLabelText("Probe playtest input"), {
      target: { value: "pay the army" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run playtest" }));
    await screen.findByText("playtest:true:pay the army");

    fireEvent.click(screen.getByRole("button", { name: "Select draft export" }));
    expect(screen.getByText("selected-export:steam-submission-kit")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Run export" }));
    await screen.findByText("export-error:Selected export profile has no executable Studio command.");

    fireEvent.click(screen.getByRole("button", { name: "Select static export" }));
    fireEvent.click(screen.getByRole("button", { name: "Run export" }));
    await screen.findByText("export:/tmp/starter-project/exports/static.zip");
  });
});

function WorkspaceProbe({ dataSource }: { dataSource: StudioDataSource }) {
  const workspace = useStudioWorkspace({
    dataSource,
    initialProjectPath: "/tmp/starter-project",
  });
  const sceneMetric = workspace.metrics.find((metric) => metric.labelKey === "metrics.scenes");
  const wp = workspace.loadedPath;

  return (
    <div>
      <p>title:{workspace.projectSummary?.title ?? "none"}</p>
      <p>loaded:{workspace.loadedPath}</p>
      <p>selected:{workspace.selectedFile?.path ?? "none"}</p>
      <p>metric-scenes:{sceneMetric?.value ?? "none"}</p>
      <p>dirty:{String(workspace.dirty)}</p>
      <p>selected-export:{workspace.export.selectedExportProfileId || "none"}</p>
      <p>
        playtest:{String(Boolean(workspace.playtest.playtestReport))}:
        {workspace.playtest.playtestReport?.trace.player_input ?? "none"}
      </p>
      <p>export:{workspace.export.exportReport?.archive_path ?? "none"}</p>
      <p>export-error:{workspace.export.exportError ?? "none"}</p>
      <p>error:{workspace.error ?? "none"}</p>
      <textarea
        aria-label="Probe editor"
        value={workspace.editorContent}
        onChange={(event) => workspace.setEditorContent(event.target.value)}
      />
      <input
        aria-label="Probe playtest input"
        value={workspace.playtest.playtestInput}
        onChange={(event) => workspace.playtest.setPlaytestInput(event.target.value)}
      />
      <button
        type="button"
        onClick={() => void workspace.refreshProjectOverview()}
      >
        Refresh overview
      </button>
      <button type="button" onClick={() => void workspace.saveSelectedFile()}>
        Save source
      </button>
      <button
        type="button"
        onClick={() => void workspace.playtest.runPlaytest(wp)}
      >
        Run playtest
      </button>
      <button
        type="button"
        onClick={() => workspace.export.selectExportProfile("steam-submission-kit")}
      >
        Select draft export
      </button>
      <button
        type="button"
        onClick={() => workspace.export.selectExportProfile("static-web")}
      >
        Select static export
      </button>
      <button
        type="button"
        onClick={() => void workspace.export.runStaticZipExport(wp)}
      >
        Run export
      </button>
    </div>
  );
}

function workspaceTestDataSource(
  overrides: Partial<StudioDataSource> = {},
): StudioDataSource {
  const files: SourceFileSummary[] = [
    { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
    { path: "game.toml", kind: "toml", bytes: 120, editable: false },
  ];
  const contents: Record<string, SourceFileContent> = {
    "world/world.md": {
      path: "world/world.md",
      kind: "markdown",
      editable: true,
      content: "# World Bible\n\nThe city is under pressure.\n",
    },
    "game.toml": {
      path: "game.toml",
      kind: "toml",
      editable: false,
      content: 'title = "Starter Project"\n',
    },
  };

  const base: StudioDataSource = {
    runtimeName: "Hook test runtime",
    async createProject(path, request) {
      return {
        project_path: path,
        template: request.template,
        concept: request.concept,
        visual_style: request.visual_style,
        voice_enabled: request.voice_enabled,
        initial_scene_request: request.initial_scene_request,
        files_created: ["game.toml"],
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
      return structuredClone(demoProjectData.asset_records);
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
        files_written: [`${outputDir}/index.html`],
        archived_files: ["index.html"],
        allowed_files: ["index.html"],
        files_found: ["index.html"],
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
        { id: "local-pi", label: "Local pi-Agent (mock)", provider: "local-mock" },
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
    async piAgentApplyRun() {
      throw new Error("piAgentApplyRun not supported in test fixture");
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
