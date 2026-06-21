import { invoke } from "@tauri-apps/api/core";
import type {
  AiSafetyPolicy,
  AssetRecord,
  AudioBible,
  Character,
  CharacterDraft,
  CharacterEditDocument,
  CharacterGenerationReport,
  ExportProfile,
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectData,
  ResourceDefinition,
  Rule,
  RulesEditDocument,
  RuntimeTrace,
  RuntimeSnapshot,
  Scene,
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
} as const;

export interface StudioCommandError {
  code: string;
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
