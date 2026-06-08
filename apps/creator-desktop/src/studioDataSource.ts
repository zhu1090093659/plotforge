import type {
  AiSafetyPolicy,
  Character,
  CharacterEditDocument,
  CharacterGenerationReport,
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectData,
  ResourceDefinition,
  Rule,
  RulesEditDocument,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  StoryCraftGenerationReport,
  WorldEditDocument,
  WorldGenerationReport,
} from "../../../contracts/plotforge";
import {
  demoAiSafetyPolicy,
  demoPlayOnceReport,
  demoProjectData,
  demoProjectPath,
  demoReproducibilityMetadata,
  demoSourceContents,
  demoSourceFiles,
} from "./demoStudioData";
import {
  studioBridge,
  type PlayOnceReport,
  type ProjectCheckReport,
  type StaticExportReport,
  type SourceFileContent,
  type SourceFileSummary,
} from "./tauriBridge";

export interface StudioDataSource {
  runtimeName: string;
  createProject(
    path: string,
    request: ProjectCreationRequest,
    force: boolean,
  ): Promise<ProjectCreationReport>;
  openProject(path: string): Promise<ProjectData>;
  checkProject(path: string): Promise<ProjectCheckReport>;
  readWorldEditDocument(path: string): Promise<WorldEditDocument>;
  updateWorldEditDocument(
    path: string,
    document: WorldEditDocument,
  ): Promise<WorldEditDocument>;
  readStoryCraftEditDocument(path: string): Promise<StoryCraftEditDocument>;
  updateStoryCraftEditDocument(
    path: string,
    document: StoryCraftEditDocument,
  ): Promise<StoryCraftEditDocument>;
  readCharacterEditDocument(path: string): Promise<CharacterEditDocument>;
  updateCharacterEditDocument(
    path: string,
    document: CharacterEditDocument,
  ): Promise<CharacterEditDocument>;
  createCharacter(
    path: string,
    character: Character,
  ): Promise<CharacterEditDocument>;
  readStateVariablesEditDocument(
    path: string,
  ): Promise<StateVariablesEditDocument>;
  updateStateVariablesEditDocument(
    path: string,
    document: StateVariablesEditDocument,
  ): Promise<StateVariablesEditDocument>;
  createResource(
    path: string,
    resource: ResourceDefinition,
  ): Promise<StateVariablesEditDocument>;
  readRulesEditDocument(path: string): Promise<RulesEditDocument>;
  updateRulesEditDocument(
    path: string,
    document: RulesEditDocument,
  ): Promise<RulesEditDocument>;
  createRule(path: string, rule: Rule): Promise<RulesEditDocument>;
  generateWorldExpansion(
    path: string,
    expansionGoal: string,
  ): Promise<WorldGenerationReport>;
  generateStoryCraft(
    path: string,
    concept: string,
  ): Promise<StoryCraftGenerationReport>;
  generateCharacter(
    path: string,
    concept: string,
    roleHint: string,
  ): Promise<CharacterGenerationReport>;
  readAiSafetyPolicy(path: string): Promise<AiSafetyPolicy>;
  updateAiSafetyPolicy(
    path: string,
    policy: AiSafetyPolicy,
  ): Promise<AiSafetyPolicy>;
  playOnceProject(path: string, playerInput: string): Promise<PlayOnceReport>;
  playOnceProjectWithSave(
    path: string,
    playerInput: string,
    saveId: string,
  ): Promise<PlayOnceReport>;
  playOnceProjectFromSnapshot(
    path: string,
    playerInput: string,
    snapshotId: string,
    saveId?: string | null,
  ): Promise<PlayOnceReport>;
  playOnceProjectFromLatestSnapshot(
    path: string,
    playerInput: string,
    saveId?: string | null,
  ): Promise<PlayOnceReport>;
  exportStaticProjectZip(
    path: string,
    outputDir: string,
    archivePath: string,
  ): Promise<StaticExportReport>;
  listSourceFiles(path: string): Promise<SourceFileSummary[]>;
  readSourceFile(path: string, relativePath: string): Promise<SourceFileContent>;
  writeSourceFile(
    path: string,
    relativePath: string,
    content: string,
  ): Promise<SourceFileContent>;
}

export function createTauriStudioDataSource(): StudioDataSource {
  return {
    runtimeName: "Tauri desktop",
    createProject: studioBridge.createProject,
    openProject: studioBridge.openProject,
    checkProject: studioBridge.checkProject,
    readWorldEditDocument: studioBridge.readWorldEditDocument,
    updateWorldEditDocument: studioBridge.updateWorldEditDocument,
    readStoryCraftEditDocument: studioBridge.readStoryCraftEditDocument,
    updateStoryCraftEditDocument: studioBridge.updateStoryCraftEditDocument,
    readCharacterEditDocument: studioBridge.readCharacterEditDocument,
    updateCharacterEditDocument: studioBridge.updateCharacterEditDocument,
    createCharacter: studioBridge.createCharacter,
    readStateVariablesEditDocument: studioBridge.readStateVariablesEditDocument,
    updateStateVariablesEditDocument: studioBridge.updateStateVariablesEditDocument,
    createResource: studioBridge.createResource,
    readRulesEditDocument: studioBridge.readRulesEditDocument,
    updateRulesEditDocument: studioBridge.updateRulesEditDocument,
    createRule: studioBridge.createRule,
    generateWorldExpansion: studioBridge.generateWorldExpansion,
    generateStoryCraft: studioBridge.generateStoryCraft,
    generateCharacter: studioBridge.generateCharacter,
    readAiSafetyPolicy: studioBridge.readAiSafetyPolicy,
    updateAiSafetyPolicy: studioBridge.updateAiSafetyPolicy,
    playOnceProject: studioBridge.playOnceProject,
    playOnceProjectWithSave: studioBridge.playOnceProjectWithSave,
    playOnceProjectFromSnapshot: studioBridge.playOnceProjectFromSnapshot,
    playOnceProjectFromLatestSnapshot: studioBridge.playOnceProjectFromLatestSnapshot,
    exportStaticProjectZip: studioBridge.exportStaticProjectZip,
    listSourceFiles: studioBridge.listSourceFiles,
    readSourceFile: studioBridge.readSourceFile,
    writeSourceFile: studioBridge.writeSourceFile,
  };
}

export function createBrowserPreviewDataSource(): StudioDataSource {
  let previewProject = structuredClone(demoProjectData);
  let worldEdit: WorldEditDocument = {
    world_bible_markdown: demoSourceContents["world/world.md"].content,
    canon_markdown:
      "# Canon Rules\n\n- The player is the final authority.\n- Every order has visible state consequences.\n",
    forbidden_facts: [],
  };
  let storyCraftEdit: StoryCraftEditDocument = {
    story_bible_markdown: demoSourceContents["story/story_bible.md"].content,
    style_guide_markdown:
      "# Style Guide\n\nTense, concrete, political, and consequence-driven.\n",
    story_craft: structuredClone(demoProjectData.story_craft),
  };
  let characterEdit: CharacterEditDocument = {
    characters: structuredClone(demoProjectData.characters),
  };
  let stateVariablesEdit: StateVariablesEditDocument = {
    resources: structuredClone(demoProjectData.resources),
    initial_world_state: structuredClone(demoProjectData.world_state),
    initial_story_state: structuredClone(demoProjectData.story_state),
  };
  let rulesEdit: RulesEditDocument = {
    rules: structuredClone(demoProjectData.rules),
  };
  let aiSafetyPolicy = structuredClone(demoAiSafetyPolicy);
  const files = new Map(
    Object.entries(demoSourceContents).map(([path, file]) => [
      path,
      { ...file },
    ]),
  );

  return {
    runtimeName: "Browser preview",
    async createProject(path, request) {
      previewProject = {
        ...structuredClone(demoProjectData),
        game: {
          ...demoProjectData.game,
          title: titleFromProjectPath(path),
          description: request.concept,
        },
        story_craft: {
          ...structuredClone(demoProjectData.story_craft),
          bible: {
            ...demoProjectData.story_craft.bible,
            genre_promise: request.concept,
            prose_style_guide: request.visual_style,
          },
        },
      };
      worldEdit = {
        world_bible_markdown: `# World Bible\n\n${request.concept}\n`,
        canon_markdown: worldEdit.canon_markdown,
        forbidden_facts: [],
      };
      storyCraftEdit = {
        story_bible_markdown: `# Story Bible\n\n${request.initial_scene_request}\n`,
        style_guide_markdown: `# Style Guide\n\n${request.visual_style}\n`,
        story_craft: structuredClone(previewProject.story_craft),
      };
      characterEdit = {
        characters: structuredClone(previewProject.characters),
      };
      stateVariablesEdit = {
        resources: structuredClone(previewProject.resources),
        initial_world_state: structuredClone(previewProject.world_state),
        initial_story_state: structuredClone(previewProject.story_state),
      };
      rulesEdit = {
        rules: structuredClone(previewProject.rules),
      };
      aiSafetyPolicy = structuredClone(demoAiSafetyPolicy);
      previewProject = {
        ...previewProject,
        ai_safety_policy: structuredClone(aiSafetyPolicy),
      };

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
          "story/story_craft.toml",
        ],
        project: structuredClone(previewProject),
      };
    },
    async openProject() {
      return structuredClone({
        ...previewProject,
        ai_safety_policy: aiSafetyPolicy,
      });
    },
    async checkProject() {
      return {
        title: previewProject.game.title,
        entry_scene: previewProject.game.entry_scene,
        scene_count: previewProject.scenes.length,
        rule_count: previewProject.rules.length,
        character_count: previewProject.characters.length,
      };
    },
    async readWorldEditDocument() {
      return structuredClone(worldEdit);
    },
    async updateWorldEditDocument(_path, document) {
      worldEdit = structuredClone(document);
      return structuredClone(worldEdit);
    },
    async readStoryCraftEditDocument() {
      return structuredClone(storyCraftEdit);
    },
    async updateStoryCraftEditDocument(_path, document) {
      storyCraftEdit = structuredClone(document);
      previewProject = {
        ...previewProject,
        story_craft: structuredClone(document.story_craft),
      };
      return structuredClone(storyCraftEdit);
    },
    async readCharacterEditDocument() {
      return structuredClone(characterEdit);
    },
    async updateCharacterEditDocument(_path, document) {
      characterEdit = structuredClone(document);
      previewProject = {
        ...previewProject,
        characters: structuredClone(document.characters),
      };
      return structuredClone(characterEdit);
    },
    async createCharacter(_path, character) {
      characterEdit = {
        characters: [...characterEdit.characters, structuredClone(character)],
      };
      previewProject = {
        ...previewProject,
        characters: structuredClone(characterEdit.characters),
      };
      return structuredClone(characterEdit);
    },
    async readStateVariablesEditDocument() {
      return structuredClone(stateVariablesEdit);
    },
    async updateStateVariablesEditDocument(_path, document) {
      stateVariablesEdit = structuredClone(document);
      previewProject = {
        ...previewProject,
        resources: structuredClone(document.resources),
        world_state: structuredClone(document.initial_world_state),
        story_state: structuredClone(document.initial_story_state),
      };
      return structuredClone(stateVariablesEdit);
    },
    async createResource(_path, resource) {
      stateVariablesEdit = {
        ...stateVariablesEdit,
        resources: [
          ...stateVariablesEdit.resources,
          structuredClone(resource),
        ],
        initial_world_state: {
          ...stateVariablesEdit.initial_world_state,
          resources: {
            ...stateVariablesEdit.initial_world_state.resources,
            [resource.key]: resource.initial,
          },
        },
      };
      previewProject = {
        ...previewProject,
        resources: structuredClone(stateVariablesEdit.resources),
        world_state: structuredClone(stateVariablesEdit.initial_world_state),
      };
      return structuredClone(stateVariablesEdit);
    },
    async readRulesEditDocument() {
      return structuredClone(rulesEdit);
    },
    async updateRulesEditDocument(_path, document) {
      rulesEdit = structuredClone(document);
      previewProject = {
        ...previewProject,
        rules: structuredClone(document.rules),
      };
      return structuredClone(rulesEdit);
    },
    async createRule(_path, rule) {
      rulesEdit = {
        rules: [...rulesEdit.rules, structuredClone(rule)],
      };
      previewProject = {
        ...previewProject,
        rules: structuredClone(rulesEdit.rules),
      };
      return structuredClone(rulesEdit);
    },
    async generateWorldExpansion(_path, expansionGoal) {
      worldEdit = {
        world_bible_markdown: `${worldEdit.world_bible_markdown.trim()}\n\n## Generated Expansion\n\n${expansionGoal}\n\nFactions, resources, and consequences are now explicit for preview review.\n`,
        canon_markdown: `${worldEdit.canon_markdown.trim()}\n- Preview-generated facts must preserve visible consequences.\n`,
        forbidden_facts: [
          ...worldEdit.forbidden_facts,
          "Preview generation cannot erase already revealed costs.",
        ],
      };
      return {
        document: structuredClone(worldEdit),
        evidence: browserPreviewGenerationEvidence(),
      };
    },
    async generateStoryCraft(_path, concept) {
      storyCraftEdit = {
        story_bible_markdown: `# Story Bible\n\n${concept}\n\nThe preview arc uses resource pressure, faction cost, and reputation as linked promises.\n`,
        style_guide_markdown: `${storyCraftEdit.style_guide_markdown.trim()}\n\n- Keep generated choices consequence-first.\n`,
        story_craft: {
          ...structuredClone(storyCraftEdit.story_craft),
          bible: {
            ...storyCraftEdit.story_craft.bible,
            genre_promise: "A pressure-driven interactive court drama.",
            central_question:
              "Can the player preserve legitimacy while every survival choice has a cost?",
          },
          emotional_arc: [
            {
              scene_key: "opening-pressure",
              target_emotion: "urgent responsibility",
              intensity: 72,
            },
            {
              scene_key: "first-reversal",
              target_emotion: "earned consequence",
              intensity: 84,
            },
            {
              scene_key: "public-reckoning",
              target_emotion: "costly agency",
              intensity: 92,
            },
          ],
          plot_threads: [
            {
              id: "resource-legitimacy",
              title: "Resource legitimacy",
              promise: "Every emergency resource move changes public trust.",
              thread_type: "political",
              status: "open",
              introduced_at: "opening-pressure",
              expected_payoff:
                "A later scene forces a choice between reserves and legitimacy.",
              related_characters: [],
              related_world_flags: ["public_trust_tested"],
              last_update: "Generated by browser preview.",
            },
            {
              id: "hidden-court-cost",
              title: "Hidden court cost",
              promise: "A useful ally demands a future compromise.",
              thread_type: "mystery",
              status: "open",
              introduced_at: "opening-pressure",
              expected_payoff:
                "The ally's price becomes visible after the first success.",
              related_characters: ["censor"],
              related_world_flags: ["ally_price_unpaid"],
              last_update: "Generated by browser preview.",
            },
            {
              id: "player-style-mirror",
              title: "Player style mirror",
              promise: "Repeated choices teach factions what the ruler values.",
              thread_type: "relationship",
              status: "open",
              introduced_at: "opening-pressure",
              expected_payoff:
                "A faction copies or punishes the player's pattern.",
              related_characters: [],
              related_world_flags: ["ruling_pattern_noticed"],
              last_update: "Generated by browser preview.",
            },
          ],
        },
      };
      previewProject = {
        ...previewProject,
        story_craft: structuredClone(storyCraftEdit.story_craft),
      };
      return {
        document: structuredClone(storyCraftEdit),
        evidence: browserPreviewGenerationEvidence(),
      };
    },
    async generateCharacter(_path, concept, roleHint) {
      const id = slugify(roleHint || "generated-character");
      const character: Character = {
        id,
        name: roleHint.trim() || "Generated Envoy",
        role: roleHint.trim() || "Pressure-bearing story catalyst",
        traits: ["observant", "cost-aware", "direct"],
        visual_card:
          "Historically grounded portrait, restrained clothing, alert posture.",
        voice_card: `Concise, tactful, and specific about consequences from ${concept}.`,
        portrait_request: {
          prompt_summary: `Portrait for ${roleHint || "generated character"}.`,
          style: "grounded historical character card",
          target_asset_slot: "portrait",
          prompt_hash: `sha256:browser-preview-${id}`,
          provider_config_hash: demoReproducibilityMetadata.provider_config_hash,
          reference_asset_ids: [],
          fallback_allowed: true,
        },
      };
      characterEdit = {
        characters: [...characterEdit.characters, structuredClone(character)],
      };
      previewProject = {
        ...previewProject,
        characters: structuredClone(characterEdit.characters),
      };
      return {
        character: structuredClone(character),
        evidence: browserPreviewGenerationEvidence(),
      };
    },
    async readAiSafetyPolicy() {
      return structuredClone(aiSafetyPolicy);
    },
    async updateAiSafetyPolicy(_path, policy) {
      aiSafetyPolicy = structuredClone({
        ...policy,
        policy_source_path: "safety/ai_safety_policy.toml",
      });
      previewProject = {
        ...previewProject,
        ai_safety_policy: structuredClone(aiSafetyPolicy),
      };
      return structuredClone(aiSafetyPolicy);
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
          project_id: previewProject.game.id,
          project_version: previewProject.game.version,
          story_state: report.trace.story_state_after,
          world_state: report.trace.world_state_after,
          scenes: previewProject.scenes,
        },
        snapshot_path: `${demoProjectPath}/saves/${saveId}.runtime_snapshot.json`,
      };
    },
    async playOnceProjectFromSnapshot(_path, playerInput, _snapshotId, saveId = null) {
      return saveId
        ? this.playOnceProjectWithSave(demoProjectPath, playerInput, saveId)
        : demoPlayOnceReport(playerInput);
    },
    async playOnceProjectFromLatestSnapshot(_path, playerInput, saveId = null) {
      return saveId
        ? this.playOnceProjectWithSave(demoProjectPath, playerInput, saveId)
        : demoPlayOnceReport(playerInput);
    },
    async exportStaticProjectZip(_path, outputDir, archivePath) {
      return {
        output_dir: outputDir,
        archive_path: archivePath,
        files_written: [
          `${outputDir}/index.html`,
          `${outputDir}/game.json`,
          `${outputDir}/ai-usage.json`,
        ],
        archived_files: [
          "ai-usage.json",
          "game.json",
          "index.html",
          "player-core.js",
          "player.js",
          "styles.css",
        ],
        allowed_files: [
          "ai-usage.json",
          "game.json",
          "index.html",
          "player-core.js",
          "player.js",
          "styles.css",
        ],
        files_found: [
          "ai-usage.json",
          "game.json",
          "index.html",
          "player-core.js",
          "player.js",
          "styles.css",
        ],
      };
    },
    async listSourceFiles() {
      return demoSourceFiles.map((file) => ({ ...file }));
    },
    async readSourceFile(_path, relativePath) {
      const file = files.get(relativePath);
      if (!file) {
        throw new Error(`unknown demo source file: ${relativePath}`);
      }
      return { ...file };
    },
    async writeSourceFile(_path, relativePath, content) {
      const file = files.get(relativePath);
      if (!file) {
        throw new Error(`unknown demo source file: ${relativePath}`);
      }
      if (!file.editable) {
        throw new Error(`${relativePath} is not editable`);
      }
      const updated = { ...file, content };
      files.set(relativePath, updated);
      return { ...updated };
    },
  };
}

export function createDefaultStudioDataSource(): StudioDataSource {
  return isTauriRuntime()
    ? createTauriStudioDataSource()
    : createBrowserPreviewDataSource();
}

export function defaultProjectPath() {
  return demoProjectPath;
}

function isTauriRuntime() {
  return Boolean(
    typeof window !== "undefined" &&
      (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__,
  );
}

function titleFromProjectPath(path: string) {
  const name = path.split("/").filter(Boolean).at(-1) ?? "plotforge-project";
  return name
    .split(/[-_\s]+/)
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join(" ");
}

function browserPreviewGenerationEvidence() {
  return {
    status: "fallback" as const,
    fallback_used: true,
    error: {
      code: "browser_preview_fake_generation",
      message: "Browser preview uses local fake generation without provider calls.",
    },
    reproducibility: demoReproducibilityMetadata,
    envelopes: [],
  };
}

function slugify(value: string) {
  return (
    value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "generated-character"
  );
}
