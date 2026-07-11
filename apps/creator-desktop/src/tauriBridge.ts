import { invoke } from "@tauri-apps/api/core";
import type {
  AgentSessionConfig,
  AiSafetyPolicy,
  AssetRecord,
  AudioBible,
  Character,
  CharacterDraft,
  CharacterEditDocument,
  CharacterGenerationReport,
  ExportProfile,
  GitBranchInfo,
  GitSwitchResult,
  McpServerEntry,
  McpServerTestResult,
  McpToolCallRequest,
  McpToolCallResult,
  McpToolManifest,
  ModelOption,
  ModerationProviderEntry,
  PiAgentApplyRequest,
  PiAgentApplyResult,
  PiAgentCapability,
  PiAgentRunRequest,
  PiAgentRunResult,
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectData,
  ProviderCostReport,
  ProviderEntry,
  ProviderKind,
  ImageProviderEntry,
  TtsProviderEntry,
  UsageSummary,
  PromptTemplate,
  RemoteModelInfo,
  ResourceDefinition,
  Rule,
  RuleDraft,
  RulesEditDocument,
  RuntimeTrace,
  RuntimeSnapshot,
  Scene,
  SkillIndex,
  SkillManifest,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  StoryCraftGenerationReport,
  VisualBible,
  WorldEditDocument,
  WorldGenerationReport,
} from "../../../contracts/plotforge";

export const studioCommandNames = {
  createProject: "create_project",
  openProject: "open_project",
  checkProject: "check_project",
  listExportProfiles: "list_export_profiles",
  readWorldEditDocument: "read_world_edit_document",
  updateWorldEditDocument: "update_world_edit_document",
  readStoryCraftEditDocument: "read_story_craft_edit_document",
  updateStoryCraftEditDocument: "update_story_craft_edit_document",
  readCharacterEditDocument: "read_character_edit_document",
  updateCharacterEditDocument: "update_character_edit_document",
  createCharacter: "create_character",
  createCharacterFromDraft: "create_character_from_draft",
  readStateVariablesEditDocument: "read_state_variables_edit_document",
  updateStateVariablesEditDocument: "update_state_variables_edit_document",
  createResource: "create_resource",
  readRulesEditDocument: "read_rules_edit_document",
  updateRulesEditDocument: "update_rules_edit_document",
  createRule: "create_rule",
  createRuleFromDraft: "create_rule_from_draft",
  generateWorldExpansion: "generate_world_expansion",
  generateStoryCraft: "generate_story_craft",
  generateCharacter: "generate_character",
  readAiSafetyPolicy: "read_ai_safety_policy",
  updateAiSafetyPolicy: "update_ai_safety_policy",
  readVisualBible: "read_visual_bible",
  updateVisualBible: "update_visual_bible",
  readAudioBible: "read_audio_bible",
  updateAudioBible: "update_audio_bible",
  playOnceProject: "play_once_project",
  playOnceProjectWithSave: "play_once_project_with_save",
  playOnceProjectFromSnapshot: "play_once_project_from_snapshot",
  playOnceProjectFromLatestSnapshot: "play_once_project_from_latest_snapshot",
  exportStaticProject: "export_static_project",
  exportStaticProjectZip: "export_static_project_zip",
  listAssetRecords: "list_asset_records",
  listSourceFiles: "list_source_files",
  readSourceFile: "read_source_file",
  writeSourceFile: "write_source_file",
  piAgentRun: "pi_agent_run",
  piAgentCapabilities: "pi_agent_capabilities",
  gitCurrentBranch: "git_current_branch",
  gitListBranches: "git_list_branches",
  gitSwitchBranch: "git_switch_branch",
  listAvailableModels: "list_available_models",
  getAgentSessionConfig: "get_agent_session_config",
  setAgentSessionConfig: "set_agent_session_config",
  piAgentApplyRun: "pi_agent_apply_run",
  getUsageSummary: "get_usage_summary",
  getProviderCostReport: "get_provider_cost_report",
  listProviders: "list_providers",
  upsertProvider: "upsert_provider",
  deleteProvider: "delete_provider",
  testProviderConnection: "test_provider_connection",
  listRemoteModels: "list_remote_models",
  listImageProviders: "list_image_providers",
  upsertImageProvider: "upsert_image_provider",
  deleteImageProvider: "delete_image_provider",
  testImageProvider: "test_image_provider",
  listTtsProviders: "list_tts_providers",
  upsertTtsProvider: "upsert_tts_provider",
  deleteTtsProvider: "delete_tts_provider",
  testTtsProvider: "test_tts_provider",
  listModerationProviders: "list_moderation_providers",
  upsertModerationProvider: "upsert_moderation_provider",
  deleteModerationProvider: "delete_moderation_provider",
  testModerationProvider: "test_moderation_provider",
  listUserPromptTemplates: "list_user_prompt_templates",
  listProjectPromptTemplates: "list_project_prompt_templates",
  upsertUserPromptTemplate: "upsert_user_prompt_template",
  upsertProjectPromptTemplate: "upsert_project_prompt_template",
  deleteUserPromptTemplate: "delete_user_prompt_template",
  deleteProjectPromptTemplate: "delete_project_prompt_template",
  listSkills: "list_skills",
  refreshSkillIndex: "refresh_skill_index",
  importSkill: "import_skill",
  readSkillBody: "read_skill_body",
  enableSkillForProject: "enable_skill_for_project",
  listMcpServers: "list_mcp_servers",
  upsertMcpServer: "upsert_mcp_server",
  deleteMcpServer: "delete_mcp_server",
  testMcpServer: "test_mcp_server",
  listMcpTools: "list_mcp_tools",
  invokeMcpTool: "invoke_mcp_tool",
  enableMcpServerForProject: "enable_mcp_server_for_project",
} as const;

export interface StudioCommandError {
  code: string;
  message: string;
}

export interface ProviderTestResult {
  ok: boolean;
  message: string;
}

export interface ProjectCheckReport {
  title: string;
  entry_scene: string;
  scene_count: number;
  rule_count: number;
  character_count: number;
}

export interface PlayOnceReport {
  scene: Scene;
  trace: RuntimeTrace;
  trace_path: string;
  snapshot: RuntimeSnapshot | null;
  snapshot_path: string | null;
  delta_summary: string[];
}

export interface StaticExportReport {
  output_dir: string;
  archive_path: string | null;
  files_written: string[];
  archived_files: string[];
  allowed_files: string[];
  files_found: string[];
}

export type SourceFileKind = "toml" | "json" | "markdown" | "prompt";

export interface SourceFileSummary {
  path: string;
  kind: SourceFileKind;
  bytes: number;
  editable: boolean;
}

export interface SourceFileContent {
  path: string;
  kind: SourceFileKind;
  editable: boolean;
  content: string;
}

export type StudioInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export function createHttpStudioInvoke(
  endpoint = "/__plotforge_studio/invoke",
): StudioInvoke {
  return async <T>(command: string, args: Record<string, unknown> = {}) => {
    const response = await fetch(endpoint, {
      method: "POST",
      headers: {
        "content-type": "application/json",
      },
      body: JSON.stringify({ command, args }),
    });
    const payload = (await response.json().catch(() => null)) as unknown;
    if (!response.ok) {
      throw new Error(studioHttpErrorMessage(payload, response.status));
    }
    return payload as T;
  };
}

export function createStudioBridge(invokeCommand: StudioInvoke = invoke) {
  return {
    createProject(
      path: string,
      request: ProjectCreationRequest,
      force: boolean,
    ): Promise<ProjectCreationReport> {
      return invokeCommand<ProjectCreationReport>(
        studioCommandNames.createProject,
        {
          path,
          request,
          force,
        },
      );
    },
    openProject(path: string): Promise<ProjectData> {
      return invokeCommand<ProjectData>(studioCommandNames.openProject, { path });
    },
    checkProject(path: string): Promise<ProjectCheckReport> {
      return invokeCommand<ProjectCheckReport>(studioCommandNames.checkProject, {
        path,
      });
    },
    listExportProfiles(): Promise<ExportProfile[]> {
      return invokeCommand<ExportProfile[]>(studioCommandNames.listExportProfiles);
    },
    readWorldEditDocument(path: string): Promise<WorldEditDocument> {
      return invokeCommand<WorldEditDocument>(
        studioCommandNames.readWorldEditDocument,
        { path },
      );
    },
    updateWorldEditDocument(
      path: string,
      document: WorldEditDocument,
    ): Promise<WorldEditDocument> {
      return invokeCommand<WorldEditDocument>(
        studioCommandNames.updateWorldEditDocument,
        { path, document },
      );
    },
    readStoryCraftEditDocument(path: string): Promise<StoryCraftEditDocument> {
      return invokeCommand<StoryCraftEditDocument>(
        studioCommandNames.readStoryCraftEditDocument,
        { path },
      );
    },
    updateStoryCraftEditDocument(
      path: string,
      document: StoryCraftEditDocument,
    ): Promise<StoryCraftEditDocument> {
      return invokeCommand<StoryCraftEditDocument>(
        studioCommandNames.updateStoryCraftEditDocument,
        { path, document },
      );
    },
    readCharacterEditDocument(path: string): Promise<CharacterEditDocument> {
      return invokeCommand<CharacterEditDocument>(
        studioCommandNames.readCharacterEditDocument,
        { path },
      );
    },
    updateCharacterEditDocument(
      path: string,
      document: CharacterEditDocument,
    ): Promise<CharacterEditDocument> {
      return invokeCommand<CharacterEditDocument>(
        studioCommandNames.updateCharacterEditDocument,
        { path, document },
      );
    },
    createCharacter(
      path: string,
      character: Character,
    ): Promise<CharacterEditDocument> {
      return invokeCommand<CharacterEditDocument>(
        studioCommandNames.createCharacter,
        { path, character },
      );
    },
    createCharacterFromDraft(
      path: string,
      draft: CharacterDraft,
    ): Promise<CharacterEditDocument> {
      return invokeCommand<CharacterEditDocument>(
        studioCommandNames.createCharacterFromDraft,
        { path, draft },
      );
    },
    readStateVariablesEditDocument(
      path: string,
    ): Promise<StateVariablesEditDocument> {
      return invokeCommand<StateVariablesEditDocument>(
        studioCommandNames.readStateVariablesEditDocument,
        { path },
      );
    },
    updateStateVariablesEditDocument(
      path: string,
      document: StateVariablesEditDocument,
    ): Promise<StateVariablesEditDocument> {
      return invokeCommand<StateVariablesEditDocument>(
        studioCommandNames.updateStateVariablesEditDocument,
        { path, document },
      );
    },
    createResource(
      path: string,
      resource: ResourceDefinition,
    ): Promise<StateVariablesEditDocument> {
      return invokeCommand<StateVariablesEditDocument>(
        studioCommandNames.createResource,
        { path, resource },
      );
    },
    readRulesEditDocument(path: string): Promise<RulesEditDocument> {
      return invokeCommand<RulesEditDocument>(
        studioCommandNames.readRulesEditDocument,
        { path },
      );
    },
    updateRulesEditDocument(
      path: string,
      document: RulesEditDocument,
    ): Promise<RulesEditDocument> {
      return invokeCommand<RulesEditDocument>(
        studioCommandNames.updateRulesEditDocument,
        { path, document },
      );
    },
    createRule(path: string, rule: Rule): Promise<RulesEditDocument> {
      return invokeCommand<RulesEditDocument>(studioCommandNames.createRule, {
        path,
        rule,
      });
    },
    createRuleFromDraft(
      path: string,
      draft: RuleDraft,
    ): Promise<RulesEditDocument> {
      return invokeCommand<RulesEditDocument>(
        studioCommandNames.createRuleFromDraft,
        { path, draft },
      );
    },
    generateWorldExpansion(
      path: string,
      expansionGoal: string,
    ): Promise<WorldGenerationReport> {
      return invokeCommand<WorldGenerationReport>(
        studioCommandNames.generateWorldExpansion,
        {
          path,
          expansion_goal: expansionGoal,
        },
      );
    },
    generateStoryCraft(
      path: string,
      concept: string,
    ): Promise<StoryCraftGenerationReport> {
      return invokeCommand<StoryCraftGenerationReport>(
        studioCommandNames.generateStoryCraft,
        {
          path,
          concept,
        },
      );
    },
    generateCharacter(
      path: string,
      concept: string,
      roleHint: string,
    ): Promise<CharacterGenerationReport> {
      return invokeCommand<CharacterGenerationReport>(
        studioCommandNames.generateCharacter,
        {
          path,
          concept,
          role_hint: roleHint,
        },
      );
    },
    readAiSafetyPolicy(path: string): Promise<AiSafetyPolicy> {
      return invokeCommand<AiSafetyPolicy>(studioCommandNames.readAiSafetyPolicy, {
        path,
      });
    },
    updateAiSafetyPolicy(
      path: string,
      policy: AiSafetyPolicy,
    ): Promise<AiSafetyPolicy> {
      return invokeCommand<AiSafetyPolicy>(
        studioCommandNames.updateAiSafetyPolicy,
        {
          path,
          policy,
        },
      );
    },
    readVisualBible(path: string): Promise<VisualBible> {
      return invokeCommand<VisualBible>(studioCommandNames.readVisualBible, {
        path,
      });
    },
    updateVisualBible(
      path: string,
      visualBible: VisualBible,
    ): Promise<VisualBible> {
      return invokeCommand<VisualBible>(
        studioCommandNames.updateVisualBible,
        {
          path,
          visual_bible: visualBible,
        },
      );
    },
    readAudioBible(path: string): Promise<AudioBible> {
      return invokeCommand<AudioBible>(studioCommandNames.readAudioBible, {
        path,
      });
    },
    updateAudioBible(
      path: string,
      audioBible: AudioBible,
    ): Promise<AudioBible> {
      return invokeCommand<AudioBible>(studioCommandNames.updateAudioBible, {
        path,
        audio_bible: audioBible,
      });
    },
    playOnceProject(path: string, playerInput: string): Promise<PlayOnceReport> {
      return invokeCommand<PlayOnceReport>(studioCommandNames.playOnceProject, {
        path,
        player_input: playerInput,
      });
    },
    playOnceProjectWithSave(
      path: string,
      playerInput: string,
      saveId: string,
    ): Promise<PlayOnceReport> {
      return invokeCommand<PlayOnceReport>(
        studioCommandNames.playOnceProjectWithSave,
        {
          path,
          player_input: playerInput,
          save_id: saveId,
        },
      );
    },
    playOnceProjectFromSnapshot(
      path: string,
      playerInput: string,
      snapshotId: string,
      saveId: string | null = null,
    ): Promise<PlayOnceReport> {
      return invokeCommand<PlayOnceReport>(
        studioCommandNames.playOnceProjectFromSnapshot,
        {
          path,
          player_input: playerInput,
          snapshot_id: snapshotId,
          save_id: saveId,
        },
      );
    },
    playOnceProjectFromLatestSnapshot(
      path: string,
      playerInput: string,
      saveId: string | null = null,
    ): Promise<PlayOnceReport> {
      return invokeCommand<PlayOnceReport>(
        studioCommandNames.playOnceProjectFromLatestSnapshot,
        {
          path,
          player_input: playerInput,
          save_id: saveId,
        },
      );
    },
    exportStaticProject(
      path: string,
      outputDir: string,
    ): Promise<StaticExportReport> {
      return invokeCommand<StaticExportReport>(
        studioCommandNames.exportStaticProject,
        {
          path,
          output_dir: outputDir,
        },
      );
    },
    exportStaticProjectZip(
      path: string,
      outputDir: string,
      archivePath: string,
    ): Promise<StaticExportReport> {
      return invokeCommand<StaticExportReport>(
        studioCommandNames.exportStaticProjectZip,
        {
          path,
          output_dir: outputDir,
          archive_path: archivePath,
        },
      );
    },
    listAssetRecords(path: string): Promise<AssetRecord[]> {
      return invokeCommand<AssetRecord[]>(studioCommandNames.listAssetRecords, {
        path,
      });
    },
    listSourceFiles(path: string): Promise<SourceFileSummary[]> {
      return invokeCommand<SourceFileSummary[]>(
        studioCommandNames.listSourceFiles,
        { path },
      );
    },
    readSourceFile(
      path: string,
      relativePath: string,
    ): Promise<SourceFileContent> {
      return invokeCommand<SourceFileContent>(studioCommandNames.readSourceFile, {
        path,
        relative_path: relativePath,
      });
    },
    writeSourceFile(
      path: string,
      relativePath: string,
      content: string,
    ): Promise<SourceFileContent> {
      return invokeCommand<SourceFileContent>(
        studioCommandNames.writeSourceFile,
        {
          path,
          relative_path: relativePath,
          content,
        },
      );
    },
    piAgentRun(request: PiAgentRunRequest): Promise<PiAgentRunResult> {
      return invokeCommand<PiAgentRunResult>(studioCommandNames.piAgentRun, {
        request,
      });
    },
    piAgentCapabilities(): Promise<PiAgentCapability[]> {
      return invokeCommand<PiAgentCapability[]>(
        studioCommandNames.piAgentCapabilities,
      );
    },
    gitCurrentBranch(path: string): Promise<string> {
      return invokeCommand<string>(studioCommandNames.gitCurrentBranch, {
        path,
      });
    },
    gitListBranches(path: string): Promise<GitBranchInfo[]> {
      return invokeCommand<GitBranchInfo[]>(studioCommandNames.gitListBranches, {
        path,
      });
    },
    gitSwitchBranch(
      path: string,
      branch: string,
    ): Promise<GitSwitchResult> {
      return invokeCommand<GitSwitchResult>(studioCommandNames.gitSwitchBranch, {
        path,
        branch,
      });
    },
    listAvailableModels(): Promise<ModelOption[]> {
      return invokeCommand<ModelOption[]>(studioCommandNames.listAvailableModels);
    },
    getAgentSessionConfig(path: string): Promise<AgentSessionConfig> {
      return invokeCommand<AgentSessionConfig>(
        studioCommandNames.getAgentSessionConfig,
        { path },
      );
    },
    setAgentSessionConfig(
      path: string,
      config: AgentSessionConfig,
    ): Promise<AgentSessionConfig> {
      return invokeCommand<AgentSessionConfig>(
        studioCommandNames.setAgentSessionConfig,
        { path, config },
      );
    },
    piAgentApplyRun(request: PiAgentApplyRequest): Promise<PiAgentApplyResult> {
      return invokeCommand<PiAgentApplyResult>(
        studioCommandNames.piAgentApplyRun,
        { request },
      );
    },
    getUsageSummary(): Promise<UsageSummary> {
      return invokeCommand<UsageSummary>(studioCommandNames.getUsageSummary);
    },
    getProviderCostReport(providerId: string): Promise<ProviderCostReport> {
      return invokeCommand<ProviderCostReport>(
        studioCommandNames.getProviderCostReport,
        { provider_id: providerId },
      );
    },
    listProviders(): Promise<ProviderEntry[]> {
      return invokeCommand<ProviderEntry[]>(studioCommandNames.listProviders);
    },
    upsertProvider(entry: ProviderEntry): Promise<ProviderEntry> {
      return invokeCommand<ProviderEntry>(studioCommandNames.upsertProvider, {
        entry,
      });
    },
    deleteProvider(id: string): Promise<ProviderEntry> {
      return invokeCommand<ProviderEntry>(studioCommandNames.deleteProvider, {
        id,
      });
    },
    testProviderConnection(id: string): Promise<ProviderTestResult> {
      return invokeCommand<ProviderTestResult>(
        studioCommandNames.testProviderConnection,
        { id },
      );
    },
    listRemoteModels(providerId: string): Promise<RemoteModelInfo[]> {
      return invokeCommand<RemoteModelInfo[]>(
        studioCommandNames.listRemoteModels,
        { provider_id: providerId },
      );
    },
    listImageProviders(): Promise<ImageProviderEntry[]> {
      return invokeCommand<ImageProviderEntry[]>(
        studioCommandNames.listImageProviders,
      );
    },
    upsertImageProvider(entry: ImageProviderEntry): Promise<ImageProviderEntry> {
      return invokeCommand<ImageProviderEntry>(
        studioCommandNames.upsertImageProvider,
        { entry },
      );
    },
    deleteImageProvider(id: string): Promise<ImageProviderEntry> {
      return invokeCommand<ImageProviderEntry>(
        studioCommandNames.deleteImageProvider,
        { id },
      );
    },
    testImageProvider(id: string): Promise<ProviderTestResult> {
      return invokeCommand<ProviderTestResult>(
        studioCommandNames.testImageProvider,
        { id },
      );
    },
    listTtsProviders(): Promise<TtsProviderEntry[]> {
      return invokeCommand<TtsProviderEntry[]>(
        studioCommandNames.listTtsProviders,
      );
    },
    upsertTtsProvider(entry: TtsProviderEntry): Promise<TtsProviderEntry> {
      return invokeCommand<TtsProviderEntry>(
        studioCommandNames.upsertTtsProvider,
        { entry },
      );
    },
    deleteTtsProvider(id: string): Promise<TtsProviderEntry> {
      return invokeCommand<TtsProviderEntry>(
        studioCommandNames.deleteTtsProvider,
        { id },
      );
    },
    testTtsProvider(id: string): Promise<ProviderTestResult> {
      return invokeCommand<ProviderTestResult>(
        studioCommandNames.testTtsProvider,
        { id },
      );
    },
    listModerationProviders(): Promise<ModerationProviderEntry[]> {
      return invokeCommand<ModerationProviderEntry[]>(
        studioCommandNames.listModerationProviders,
      );
    },
    upsertModerationProvider(
      entry: ModerationProviderEntry,
    ): Promise<ModerationProviderEntry> {
      return invokeCommand<ModerationProviderEntry>(
        studioCommandNames.upsertModerationProvider,
        { entry },
      );
    },
    deleteModerationProvider(id: string): Promise<ModerationProviderEntry> {
      return invokeCommand<ModerationProviderEntry>(
        studioCommandNames.deleteModerationProvider,
        { id },
      );
    },
    testModerationProvider(id: string): Promise<ProviderTestResult> {
      return invokeCommand<ProviderTestResult>(
        studioCommandNames.testModerationProvider,
        { id },
      );
    },
    listUserPromptTemplates(): Promise<PromptTemplate[]> {
      return invokeCommand<PromptTemplate[]>(
        studioCommandNames.listUserPromptTemplates,
      );
    },
    listProjectPromptTemplates(projectPath: string): Promise<PromptTemplate[]> {
      return invokeCommand<PromptTemplate[]>(
        studioCommandNames.listProjectPromptTemplates,
        { project_path: projectPath },
      );
    },
    upsertUserPromptTemplate(template: PromptTemplate): Promise<PromptTemplate> {
      return invokeCommand<PromptTemplate>(
        studioCommandNames.upsertUserPromptTemplate,
        { template },
      );
    },
    upsertProjectPromptTemplate(
      projectPath: string,
      template: PromptTemplate,
    ): Promise<PromptTemplate> {
      return invokeCommand<PromptTemplate>(
        studioCommandNames.upsertProjectPromptTemplate,
        { project_path: projectPath, template },
      );
    },
    deleteUserPromptTemplate(id: string): Promise<void> {
      return invokeCommand<void>(studioCommandNames.deleteUserPromptTemplate, {
        id,
      });
    },
    deleteProjectPromptTemplate(
      projectPath: string,
      id: string,
    ): Promise<void> {
      return invokeCommand<void>(
        studioCommandNames.deleteProjectPromptTemplate,
        { project_path: projectPath, id },
      );
    },
    listSkills(): Promise<SkillManifest[]> {
      return invokeCommand<SkillManifest[]>(studioCommandNames.listSkills);
    },
    refreshSkillIndex(): Promise<SkillIndex> {
      return invokeCommand<SkillIndex>(studioCommandNames.refreshSkillIndex);
    },
    importSkill(skillId: string): Promise<SkillManifest> {
      return invokeCommand<SkillManifest>(studioCommandNames.importSkill, {
        skill_id: skillId,
      });
    },
    readSkillBody(skillId: string): Promise<string> {
      return invokeCommand<string>(studioCommandNames.readSkillBody, {
        skill_id: skillId,
      });
    },
    enableSkillForProject(
      projectPath: string,
      skillId: string,
      enabled: boolean,
    ): Promise<AgentSessionConfig> {
      return invokeCommand<AgentSessionConfig>(
        studioCommandNames.enableSkillForProject,
        { project_path: projectPath, skill_id: skillId, enabled },
      );
    },
    listMcpServers(): Promise<McpServerEntry[]> {
      return invokeCommand<McpServerEntry[]>(studioCommandNames.listMcpServers);
    },
    upsertMcpServer(entry: McpServerEntry): Promise<McpServerEntry> {
      return invokeCommand<McpServerEntry>(studioCommandNames.upsertMcpServer, {
        entry,
      });
    },
    deleteMcpServer(id: string): Promise<McpServerEntry> {
      return invokeCommand<McpServerEntry>(studioCommandNames.deleteMcpServer, {
        id,
      });
    },
    testMcpServer(id: string): Promise<McpServerTestResult> {
      return invokeCommand<McpServerTestResult>(
        studioCommandNames.testMcpServer,
        { id },
      );
    },
    listMcpTools(serverId: string): Promise<McpToolManifest[]> {
      return invokeCommand<McpToolManifest[]>(studioCommandNames.listMcpTools, {
        server_id: serverId,
      });
    },
    invokeMcpTool(request: McpToolCallRequest): Promise<McpToolCallResult> {
      return invokeCommand<McpToolCallResult>(studioCommandNames.invokeMcpTool, {
        request,
      });
    },
    enableMcpServerForProject(
      projectPath: string,
      serverId: string,
      enabled: boolean,
    ): Promise<AgentSessionConfig> {
      return invokeCommand<AgentSessionConfig>(
        studioCommandNames.enableMcpServerForProject,
        { project_path: projectPath, server_id: serverId, enabled },
      );
    },
  };
}

export const studioBridge = createStudioBridge();

function studioHttpErrorMessage(payload: unknown, status: number) {
  if (
    payload &&
    typeof payload === "object" &&
    "message" in payload &&
    typeof payload.message === "string"
  ) {
    return payload.message;
  }
  return `Studio HTTP command failed with status ${status}`;
}
