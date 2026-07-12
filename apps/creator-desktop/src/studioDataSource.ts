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
  ImageProviderEntry,
  TtsProviderEntry,
  UsageSummary,
  PromptTemplate,
  RemoteModelInfo,
  ResourceDefinition,
  Rule,
  RuleDraft,
  RulesEditDocument,
  SkillIndex,
  SkillManifest,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  StoryCraftGenerationReport,
  VisualBible,
  WorldEditDocument,
  WorldGenerationReport,
} from "../../../contracts/plotforge";
import {
  createHttpStudioInvoke,
  createStudioBridge,
  studioBridge,
  type PlayOnceReport,
  type ProjectCheckReport,
  type ProviderTestResult,
  type StaticExportReport,
  type SourceFileContent,
  type SourceFileSummary,
} from "./tauriBridge";
import { open as openDialog } from "@tauri-apps/plugin-dialog";

export interface StudioDataSource {
  runtimeName: string;
  pickProjectDirectory(): Promise<string | null>;
  createProject(
    path: string,
    request: ProjectCreationRequest,
    force: boolean,
  ): Promise<ProjectCreationReport>;
  openProject(path: string): Promise<ProjectData>;
  openOrCreateProject(path: string): Promise<ProjectData>;
  checkProject(path: string): Promise<ProjectCheckReport>;
  listExportProfiles(): Promise<ExportProfile[]>;
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
  createCharacterFromDraft(
    path: string,
    draft: CharacterDraft,
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
  createRuleFromDraft(
    path: string,
    draft: RuleDraft,
  ): Promise<RulesEditDocument>;
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
  readVisualBible(path: string): Promise<VisualBible>;
  updateVisualBible(path: string, visualBible: VisualBible): Promise<VisualBible>;
  readAudioBible(path: string): Promise<AudioBible>;
  updateAudioBible(path: string, audioBible: AudioBible): Promise<AudioBible>;
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
  listAssetRecords(path: string): Promise<AssetRecord[]>;
  listSourceFiles(path: string): Promise<SourceFileSummary[]>;
  readSourceFile(path: string, relativePath: string): Promise<SourceFileContent>;
  writeSourceFile(
    path: string,
    relativePath: string,
    content: string,
  ): Promise<SourceFileContent>;
  piAgentRun(request: PiAgentRunRequest): Promise<PiAgentRunResult>;
  piAgentCapabilities(): Promise<PiAgentCapability[]>;
  gitCurrentBranch(path: string): Promise<string>;
  gitListBranches(path: string): Promise<GitBranchInfo[]>;
  gitSwitchBranch(path: string, branch: string): Promise<GitSwitchResult>;
  listAvailableModels(): Promise<ModelOption[]>;
  getAgentSessionConfig(path: string): Promise<AgentSessionConfig>;
  setAgentSessionConfig(
    path: string,
    config: AgentSessionConfig,
  ): Promise<AgentSessionConfig>;
  piAgentApplyRun(request: PiAgentApplyRequest): Promise<PiAgentApplyResult>;
  getUsageSummary(): Promise<UsageSummary>;
  getProviderCostReport(providerId: string): Promise<ProviderCostReport>;
  listProviders(): Promise<ProviderEntry[]>;
  upsertProvider(entry: ProviderEntry): Promise<ProviderEntry>;
  deleteProvider(id: string): Promise<ProviderEntry>;
  testProviderConnection(id: string): Promise<ProviderTestResult>;
  listRemoteModels(providerId: string): Promise<RemoteModelInfo[]>;
  listImageProviders(): Promise<ImageProviderEntry[]>;
  upsertImageProvider(entry: ImageProviderEntry): Promise<ImageProviderEntry>;
  deleteImageProvider(id: string): Promise<ImageProviderEntry>;
  testImageProvider(id: string): Promise<ProviderTestResult>;
  listTtsProviders(): Promise<TtsProviderEntry[]>;
  upsertTtsProvider(entry: TtsProviderEntry): Promise<TtsProviderEntry>;
  deleteTtsProvider(id: string): Promise<TtsProviderEntry>;
  testTtsProvider(id: string): Promise<ProviderTestResult>;
  listModerationProviders(): Promise<ModerationProviderEntry[]>;
  upsertModerationProvider(
    entry: ModerationProviderEntry,
  ): Promise<ModerationProviderEntry>;
  deleteModerationProvider(id: string): Promise<ModerationProviderEntry>;
  testModerationProvider(id: string): Promise<ProviderTestResult>;
  listUserPromptTemplates(): Promise<PromptTemplate[]>;
  listProjectPromptTemplates(projectPath: string): Promise<PromptTemplate[]>;
  upsertUserPromptTemplate(template: PromptTemplate): Promise<PromptTemplate>;
  upsertProjectPromptTemplate(
    projectPath: string,
    template: PromptTemplate,
  ): Promise<PromptTemplate>;
  deleteUserPromptTemplate(id: string): Promise<void>;
  deleteProjectPromptTemplate(projectPath: string, id: string): Promise<void>;
  listSkills(): Promise<SkillManifest[]>;
  refreshSkillIndex(): Promise<SkillIndex>;
  importSkill(skillId: string): Promise<SkillManifest>;
  readSkillBody(skillId: string): Promise<string>;
  enableSkillForProject(
    projectPath: string,
    skillId: string,
    enabled: boolean,
  ): Promise<AgentSessionConfig>;
  listMcpServers(): Promise<McpServerEntry[]>;
  upsertMcpServer(entry: McpServerEntry): Promise<McpServerEntry>;
  deleteMcpServer(id: string): Promise<McpServerEntry>;
  testMcpServer(id: string): Promise<McpServerTestResult>;
  listMcpTools(serverId: string): Promise<McpToolManifest[]>;
  invokeMcpTool(request: McpToolCallRequest): Promise<McpToolCallResult>;
  enableMcpServerForProject(
    projectPath: string,
    serverId: string,
    enabled: boolean,
  ): Promise<AgentSessionConfig>;
}

export function createTauriStudioDataSource(): StudioDataSource {
  return createStudioDataSource(
    "Tauri desktop",
    studioBridge,
    pickTauriProjectDirectory,
  );
}

export function createHttpStudioDataSource(): StudioDataSource {
  return createStudioDataSource(
    "HTTP dev bridge",
    createStudioBridge(createHttpStudioInvoke()),
    pickHttpProjectDirectory,
  );
}

export function createDefaultStudioDataSource(): StudioDataSource {
  return isTauriRuntime()
    ? createTauriStudioDataSource()
    : createHttpStudioDataSource();
}

export function defaultProjectPath() {
  return "";
}

function isTauriRuntime() {
  return Boolean(
    typeof window !== "undefined" &&
      (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__,
  );
}

function createStudioDataSource(
  runtimeName: string,
  bridge: ReturnType<typeof createStudioBridge>,
  pickProjectDirectory: () => Promise<string | null>,
): StudioDataSource {
  return {
    runtimeName,
    pickProjectDirectory,
    createProject: bridge.createProject,
    openProject: bridge.openProject,
    openOrCreateProject: bridge.openOrCreateProject,
    checkProject: bridge.checkProject,
    listExportProfiles: bridge.listExportProfiles,
    readWorldEditDocument: bridge.readWorldEditDocument,
    updateWorldEditDocument: bridge.updateWorldEditDocument,
    readStoryCraftEditDocument: bridge.readStoryCraftEditDocument,
    updateStoryCraftEditDocument: bridge.updateStoryCraftEditDocument,
    readCharacterEditDocument: bridge.readCharacterEditDocument,
    updateCharacterEditDocument: bridge.updateCharacterEditDocument,
    createCharacter: bridge.createCharacter,
    createCharacterFromDraft: bridge.createCharacterFromDraft,
    readStateVariablesEditDocument: bridge.readStateVariablesEditDocument,
    updateStateVariablesEditDocument: bridge.updateStateVariablesEditDocument,
    createResource: bridge.createResource,
    readRulesEditDocument: bridge.readRulesEditDocument,
    updateRulesEditDocument: bridge.updateRulesEditDocument,
    createRule: bridge.createRule,
    createRuleFromDraft: bridge.createRuleFromDraft,
    generateWorldExpansion: bridge.generateWorldExpansion,
    generateStoryCraft: bridge.generateStoryCraft,
    generateCharacter: bridge.generateCharacter,
    readAiSafetyPolicy: bridge.readAiSafetyPolicy,
    updateAiSafetyPolicy: bridge.updateAiSafetyPolicy,
    readVisualBible: bridge.readVisualBible,
    updateVisualBible: bridge.updateVisualBible,
    readAudioBible: bridge.readAudioBible,
    updateAudioBible: bridge.updateAudioBible,
    playOnceProject: bridge.playOnceProject,
    playOnceProjectWithSave: bridge.playOnceProjectWithSave,
    playOnceProjectFromSnapshot: bridge.playOnceProjectFromSnapshot,
    playOnceProjectFromLatestSnapshot: bridge.playOnceProjectFromLatestSnapshot,
    exportStaticProjectZip: bridge.exportStaticProjectZip,
    listAssetRecords: bridge.listAssetRecords,
    listSourceFiles: bridge.listSourceFiles,
    readSourceFile: bridge.readSourceFile,
    writeSourceFile: bridge.writeSourceFile,
    piAgentRun: bridge.piAgentRun,
    piAgentCapabilities: bridge.piAgentCapabilities,
    gitCurrentBranch: bridge.gitCurrentBranch,
    gitListBranches: bridge.gitListBranches,
    gitSwitchBranch: bridge.gitSwitchBranch,
    listAvailableModels: bridge.listAvailableModels,
    getAgentSessionConfig: bridge.getAgentSessionConfig,
    setAgentSessionConfig: bridge.setAgentSessionConfig,
    piAgentApplyRun: bridge.piAgentApplyRun,
    getUsageSummary: bridge.getUsageSummary,
    getProviderCostReport: bridge.getProviderCostReport,
    listProviders: bridge.listProviders,
    upsertProvider: bridge.upsertProvider,
    deleteProvider: bridge.deleteProvider,
    testProviderConnection: bridge.testProviderConnection,
    listRemoteModels: bridge.listRemoteModels,
    listImageProviders: bridge.listImageProviders,
    upsertImageProvider: bridge.upsertImageProvider,
    deleteImageProvider: bridge.deleteImageProvider,
    testImageProvider: bridge.testImageProvider,
    listTtsProviders: bridge.listTtsProviders,
    upsertTtsProvider: bridge.upsertTtsProvider,
    deleteTtsProvider: bridge.deleteTtsProvider,
    testTtsProvider: bridge.testTtsProvider,
    listModerationProviders: bridge.listModerationProviders,
    upsertModerationProvider: bridge.upsertModerationProvider,
    deleteModerationProvider: bridge.deleteModerationProvider,
    testModerationProvider: bridge.testModerationProvider,
    listUserPromptTemplates: bridge.listUserPromptTemplates,
    listProjectPromptTemplates: bridge.listProjectPromptTemplates,
    upsertUserPromptTemplate: bridge.upsertUserPromptTemplate,
    upsertProjectPromptTemplate: bridge.upsertProjectPromptTemplate,
    deleteUserPromptTemplate: bridge.deleteUserPromptTemplate,
    deleteProjectPromptTemplate: bridge.deleteProjectPromptTemplate,
    listSkills: bridge.listSkills,
    refreshSkillIndex: bridge.refreshSkillIndex,
    importSkill: bridge.importSkill,
    readSkillBody: bridge.readSkillBody,
    enableSkillForProject: bridge.enableSkillForProject,
    listMcpServers: bridge.listMcpServers,
    upsertMcpServer: bridge.upsertMcpServer,
    deleteMcpServer: bridge.deleteMcpServer,
    testMcpServer: bridge.testMcpServer,
    listMcpTools: bridge.listMcpTools,
    invokeMcpTool: bridge.invokeMcpTool,
    enableMcpServerForProject: bridge.enableMcpServerForProject,
  };
}

async function pickTauriProjectDirectory(): Promise<string | null> {
  const selected = await openDialog({
    directory: true,
    multiple: false,
    title: "Open PlotForge project folder",
  });
  return typeof selected === "string" ? selected : null;
}

async function pickHttpProjectDirectory(): Promise<string | null> {
  const response = await fetch("/__plotforge_studio/pick-directory", {
    method: "POST",
  });
  const payload = (await response.json().catch(() => null)) as unknown;
  if (!response.ok) {
    const message =
      payload && typeof payload === "object" && "message" in payload
        ? String(payload.message)
        : `Directory picker failed with HTTP ${response.status}`;
    throw new Error(message);
  }
  return typeof payload === "string" ? payload : null;
}
