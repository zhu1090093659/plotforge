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
  ModelOption,
  PiAgentCapability,
  PiAgentRunRequest,
  PiAgentRunResult,
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectData,
  ResourceDefinition,
  Rule,
  RuleDraft,
  RulesEditDocument,
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
}

export function createTauriStudioDataSource(): StudioDataSource {
  return createStudioDataSource("Tauri desktop", studioBridge);
}

export function createHttpStudioDataSource(): StudioDataSource {
  return createStudioDataSource(
    "HTTP dev bridge",
    createStudioBridge(createHttpStudioInvoke()),
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
): StudioDataSource {
  return {
    runtimeName,
    createProject: bridge.createProject,
    openProject: bridge.openProject,
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
  };
}
