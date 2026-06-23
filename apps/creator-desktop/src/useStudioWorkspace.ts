import { useEffect, useMemo, useState, type Dispatch, type SetStateAction } from "react";
import type {
  AiSafetyPolicy,
  AssetRecord,
  AudioBible,
  CharacterEditDocument,
  ExportProfile,
  ProjectData,
  RulesEditDocument,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  VisualBible,
  WorldEditDocument,
} from "../../../contracts/plotforge";
import { summarizeProject, type CreatorProjectSummary } from "./projectSummary";
import type { StudioDataSource } from "./studioDataSource";
import { projectAssetCatalog, type AssetCatalog } from "./assetCatalog";
import type {
  PlayOnceReport,
  ProjectCheckReport,
  SourceFileContent,
  SourceFileSummary,
  StaticExportReport,
} from "./tauriBridge";
import { errorMessage } from "./errorMessage";

export const defaultPlaytestInput =
  "Raise emergency taxes while auditing corrupt officials.";

export interface StudioMetric {
  label: string;
  value: string;
  tone: string;
}

export interface UseStudioWorkspaceOptions {
  dataSource: StudioDataSource;
  initialProjectPath: string;
}

export interface StudioWorkspace {
  projectPath: string;
  setProjectPath: Dispatch<SetStateAction<string>>;
  loadedPath: string;
  projectData: ProjectData | null;
  setProjectData: Dispatch<SetStateAction<ProjectData | null>>;
  assetRecords: AssetRecord[];
  visualBible: VisualBible | null;
  setVisualBible: Dispatch<SetStateAction<VisualBible | null>>;
  audioBible: AudioBible | null;
  setAudioBible: Dispatch<SetStateAction<AudioBible | null>>;
  projectSummary: CreatorProjectSummary | null;
  checkReport: ProjectCheckReport | null;
  sourceFiles: SourceFileSummary[];
  selectedFile: SourceFileContent | null;
  editorContent: string;
  setEditorContent: Dispatch<SetStateAction<string>>;
  savedContent: string;
  dirty: boolean;
  worldEditDocument: WorldEditDocument | null;
  setWorldEditDocument: Dispatch<SetStateAction<WorldEditDocument | null>>;
  storyCraftEditDocument: StoryCraftEditDocument | null;
  setStoryCraftEditDocument: Dispatch<SetStateAction<StoryCraftEditDocument | null>>;
  characterEditDocument: CharacterEditDocument | null;
  setCharacterEditDocument: Dispatch<SetStateAction<CharacterEditDocument | null>>;
  stateVariablesEditDocument: StateVariablesEditDocument | null;
  setStateVariablesEditDocument: Dispatch<SetStateAction<StateVariablesEditDocument | null>>;
  rulesEditDocument: RulesEditDocument | null;
  setRulesEditDocument: Dispatch<SetStateAction<RulesEditDocument | null>>;
  aiSafetyPolicy: AiSafetyPolicy | null;
  setAiSafetyPolicy: Dispatch<SetStateAction<AiSafetyPolicy | null>>;
  playtestInput: string;
  setPlaytestInput: Dispatch<SetStateAction<string>>;
  playtestSaveId: string;
  setPlaytestSaveId: Dispatch<SetStateAction<string>>;
  playtestRestoreId: string;
  setPlaytestRestoreId: Dispatch<SetStateAction<string>>;
  playtestRestoreLatest: boolean;
  setPlaytestRestoreLatest: Dispatch<SetStateAction<boolean>>;
  playtestReport: PlayOnceReport | null;
  playtesting: boolean;
  playtestError: string | null;
  exportDir: string;
  setExportDir: Dispatch<SetStateAction<string>>;
  archivePath: string;
  setArchivePath: Dispatch<SetStateAction<string>>;
  exportProfiles: ExportProfile[];
  selectedExportProfileId: string;
  selectedExportProfile: ExportProfile | null;
  staticExportSelected: boolean;
  exportReport: StaticExportReport | null;
  exporting: boolean;
  exportError: string | null;
  loading: boolean;
  saving: boolean;
  error: string | null;
  assetCatalog: AssetCatalog;
  metrics: StudioMetric[];
  loadProject(path: string): Promise<void>;
  refreshProjectOverview(path?: string): Promise<void>;
  selectSourceFile(file: SourceFileSummary): Promise<void>;
  saveSelectedFile(): Promise<void>;
  runPlaytest(): Promise<boolean>;
  runStaticZipExport(): Promise<void>;
  selectExportProfile(profileId: string): void;
}

export function useStudioWorkspace({
  dataSource,
  initialProjectPath,
}: UseStudioWorkspaceOptions): StudioWorkspace {
  const [projectPath, setProjectPath] = useState(initialProjectPath);
  const [loadedPath, setLoadedPath] = useState(initialProjectPath);
  const [projectData, setProjectData] = useState<ProjectData | null>(null);
  const [assetRecords, setAssetRecords] = useState<AssetRecord[]>([]);
  const [visualBible, setVisualBible] = useState<VisualBible | null>(null);
  const [audioBible, setAudioBible] = useState<AudioBible | null>(null);
  const [projectSummary, setProjectSummary] =
    useState<CreatorProjectSummary | null>(null);
  const [checkReport, setCheckReport] = useState<ProjectCheckReport | null>(
    null,
  );
  const [sourceFiles, setSourceFiles] = useState<SourceFileSummary[]>([]);
  const [selectedFile, setSelectedFile] = useState<SourceFileContent | null>(
    null,
  );
  const [editorContent, setEditorContent] = useState("");
  const [savedContent, setSavedContent] = useState("");
  const [worldEditDocument, setWorldEditDocument] =
    useState<WorldEditDocument | null>(null);
  const [storyCraftEditDocument, setStoryCraftEditDocument] =
    useState<StoryCraftEditDocument | null>(null);
  const [characterEditDocument, setCharacterEditDocument] =
    useState<CharacterEditDocument | null>(null);
  const [stateVariablesEditDocument, setStateVariablesEditDocument] =
    useState<StateVariablesEditDocument | null>(null);
  const [rulesEditDocument, setRulesEditDocument] =
    useState<RulesEditDocument | null>(null);
  const [aiSafetyPolicy, setAiSafetyPolicy] =
    useState<AiSafetyPolicy | null>(null);
  const [playtestInput, setPlaytestInput] = useState(defaultPlaytestInput);
  const [playtestSaveId, setPlaytestSaveId] = useState("save-001");
  const [playtestRestoreId, setPlaytestRestoreId] = useState("");
  const [playtestRestoreLatest, setPlaytestRestoreLatest] = useState(false);
  const [playtestReport, setPlaytestReport] = useState<PlayOnceReport | null>(
    null,
  );
  const [playtesting, setPlaytesting] = useState(false);
  const [playtestError, setPlaytestError] = useState<string | null>(null);
  const [exportDir, setExportDir] = useState(
    defaultStaticExportDir(initialProjectPath),
  );
  const [archivePath, setArchivePath] = useState(
    defaultStaticArchivePath(initialProjectPath),
  );
  const [exportProfiles, setExportProfiles] = useState<ExportProfile[]>([]);
  const [selectedExportProfileId, setSelectedExportProfileId] = useState("");
  const [exportReport, setExportReport] = useState<StaticExportReport | null>(
    null,
  );
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const dirty = Boolean(selectedFile?.editable && editorContent !== savedContent);
  const assetCatalog = useMemo(
    () => projectAssetCatalog(projectData, assetRecords),
    [assetRecords, projectData],
  );
  const selectedExportProfile = useMemo(
    () =>
      exportProfiles.find((profile) => profile.id === selectedExportProfileId) ??
      null,
    [exportProfiles, selectedExportProfileId],
  );
  const staticExportSelected = selectedExportProfile?.target === "static_web";
  const metrics = useMemo(
    () => [
      {
        label: "Scenes",
        value: String(checkReport?.scene_count ?? projectSummary?.sceneCount ?? 0),
        tone: "border-jade/50 text-jade",
      },
      {
        label: "Characters",
        value: String(
          checkReport?.character_count ?? projectSummary?.characterCount ?? 0,
        ),
        tone: "border-brass/50 text-brass",
      },
      {
        label: "Rules",
        value: String(checkReport?.rule_count ?? projectSummary?.ruleCount ?? 0),
        tone: "border-signal/50 text-signal",
      },
      {
        label: "Open Threads",
        value: String(projectSummary?.openThreadCount ?? 0),
        tone: "border-ink/30 text-ink",
      },
    ],
    [checkReport, projectSummary],
  );

  useEffect(() => {
    void loadProject(initialProjectPath);
  }, [initialProjectPath]);

  async function loadProject(path: string) {
    setLoading(true);
    setError(null);
    try {
      const [
        project,
        report,
        profiles,
        files,
        worldDocument,
        storyCraftDocument,
        characterDocument,
        stateVariablesDocument,
        rulesDocument,
        safetyPolicy,
        visualBibleDocument,
        audioBibleDocument,
        records,
      ] = await Promise.all([
        dataSource.openProject(path),
        dataSource.checkProject(path),
        dataSource.listExportProfiles(),
        dataSource.listSourceFiles(path),
        dataSource.readWorldEditDocument(path),
        dataSource.readStoryCraftEditDocument(path),
        dataSource.readCharacterEditDocument(path),
        dataSource.readStateVariablesEditDocument(path),
        dataSource.readRulesEditDocument(path),
        dataSource.readAiSafetyPolicy(path),
        dataSource.readVisualBible(path),
        dataSource.readAudioBible(path),
        dataSource.listAssetRecords(path),
      ]);
      const firstEditable = files.find((file) => file.editable) ?? files[0];
      const firstContent = firstEditable
        ? await dataSource.readSourceFile(path, firstEditable.path)
        : null;
      const projectWithBible = attachBibles(
        project,
        visualBibleDocument,
        audioBibleDocument,
      );

      setLoadedPath(path);
      setProjectPath(path);
      setProjectData(projectWithBible);
      setExportProfiles(profiles);
      setSelectedExportProfileId((currentId) =>
        resolveExportProfileId(profiles, currentId),
      );
      setAssetRecords(records);
      setVisualBible(visualBibleDocument);
      setAudioBible(audioBibleDocument);
      setProjectSummary(summarizeProject(projectWithBible));
      setCheckReport(report);
      setSourceFiles(files);
      setSelectedFile(firstContent);
      setEditorContent(firstContent?.content ?? "");
      setSavedContent(firstContent?.content ?? "");
      setWorldEditDocument(worldDocument);
      setStoryCraftEditDocument(storyCraftDocument);
      setCharacterEditDocument(characterDocument);
      setStateVariablesEditDocument(stateVariablesDocument);
      setRulesEditDocument(rulesDocument);
      setAiSafetyPolicy(safetyPolicy);
      setPlaytestReport(null);
      setPlaytestError(null);
      setExportDir(defaultStaticExportDir(path));
      setArchivePath(defaultStaticArchivePath(path));
      setExportReport(null);
      setExportError(null);
    } catch (source) {
      setError(errorMessage(source));
    } finally {
      setLoading(false);
    }
  }

  async function refreshProjectOverview(path = loadedPath) {
    setError(null);
    try {
      const [
        project,
        report,
        files,
        visualBibleDocument,
        audioBibleDocument,
        records,
      ] = await Promise.all([
        dataSource.openProject(path),
        dataSource.checkProject(path),
        dataSource.listSourceFiles(path),
        dataSource.readVisualBible(path),
        dataSource.readAudioBible(path),
        dataSource.listAssetRecords(path),
      ]);
      const projectWithBible = attachBibles(
        project,
        visualBibleDocument,
        audioBibleDocument,
      );
      setProjectData(projectWithBible);
      setAssetRecords(records);
      setVisualBible(visualBibleDocument);
      setAudioBible(audioBibleDocument);
      setProjectSummary(summarizeProject(projectWithBible));
      setCheckReport(report);
      setSourceFiles(files);
    } catch (source) {
      setError(errorMessage(source));
      throw source;
    }
  }

  async function selectSourceFile(file: SourceFileSummary) {
    setError(null);
    try {
      const content = await dataSource.readSourceFile(loadedPath, file.path);
      setSelectedFile(content);
      setEditorContent(content.content);
      setSavedContent(content.content);
    } catch (source) {
      setError(errorMessage(source));
    }
  }

  async function saveSelectedFile() {
    if (!selectedFile?.editable) {
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const updated = await dataSource.writeSourceFile(
        loadedPath,
        selectedFile.path,
        editorContent,
      );
      setSelectedFile(updated);
      setEditorContent(updated.content);
      setSavedContent(updated.content);
    } catch (source) {
      setError(errorMessage(source));
    } finally {
      setSaving(false);
    }
  }

  async function runPlaytest(): Promise<boolean> {
    const input = playtestInput.trim();
    if (!input) {
      setPlaytestError("Playtest input is required.");
      return false;
    }

    setPlaytesting(true);
    setPlaytestError(null);
    try {
      const saveId = playtestSaveId.trim() || null;
      const restoreId = playtestRestoreId.trim();
      const report = playtestRestoreLatest
        ? await dataSource.playOnceProjectFromLatestSnapshot(
            loadedPath,
            input,
            saveId,
          )
        : restoreId
          ? await dataSource.playOnceProjectFromSnapshot(
              loadedPath,
              input,
              restoreId,
              saveId,
            )
          : saveId
            ? await dataSource.playOnceProjectWithSave(loadedPath, input, saveId)
            : await dataSource.playOnceProject(loadedPath, input);
      setPlaytestReport(report);
      return true;
    } catch (source) {
      setPlaytestError(errorMessage(source));
      return false;
    } finally {
      setPlaytesting(false);
    }
  }

  async function runStaticZipExport() {
    if (!staticExportSelected) {
      setExportError("Selected export profile has no executable Studio command.");
      return;
    }

    const outputDir = exportDir.trim();
    const zipPath = archivePath.trim();
    if (!outputDir || !zipPath) {
      setExportError("Output directory and zip archive are required.");
      return;
    }

    setExporting(true);
    setExportError(null);
    try {
      const report = await dataSource.exportStaticProjectZip(
        loadedPath,
        outputDir,
        zipPath,
      );
      setExportReport(report);
    } catch (source) {
      setExportError(errorMessage(source));
    } finally {
      setExporting(false);
    }
  }

  function selectExportProfile(profileId: string) {
    setSelectedExportProfileId(profileId);
    setExportReport(null);
    setExportError(null);
  }

  return {
    projectPath,
    setProjectPath,
    loadedPath,
    projectData,
    setProjectData,
    assetRecords,
    visualBible,
    setVisualBible,
    audioBible,
    setAudioBible,
    projectSummary,
    checkReport,
    sourceFiles,
    selectedFile,
    editorContent,
    setEditorContent,
    savedContent,
    dirty,
    worldEditDocument,
    setWorldEditDocument,
    storyCraftEditDocument,
    setStoryCraftEditDocument,
    characterEditDocument,
    setCharacterEditDocument,
    stateVariablesEditDocument,
    setStateVariablesEditDocument,
    rulesEditDocument,
    setRulesEditDocument,
    aiSafetyPolicy,
    setAiSafetyPolicy,
    playtestInput,
    setPlaytestInput,
    playtestSaveId,
    setPlaytestSaveId,
    playtestRestoreId,
    setPlaytestRestoreId,
    playtestRestoreLatest,
    setPlaytestRestoreLatest,
    playtestReport,
    playtesting,
    playtestError,
    exportDir,
    setExportDir,
    archivePath,
    setArchivePath,
    exportProfiles,
    selectedExportProfileId,
    selectedExportProfile,
    staticExportSelected,
    exportReport,
    exporting,
    exportError,
    loading,
    saving,
    error,
    assetCatalog,
    metrics,
    loadProject,
    refreshProjectOverview,
    selectSourceFile,
    saveSelectedFile,
    runPlaytest,
    runStaticZipExport,
    selectExportProfile,
  };
}

function attachBibles(
  project: ProjectData,
  visualBible: VisualBible,
  audioBible: AudioBible,
): ProjectData {
  return {
    ...project,
    visual_bible: visualBible,
    audio_bible: audioBible,
  };
}

function resolveExportProfileId(profiles: ExportProfile[], currentId: string) {
  if (profiles.some((profile) => profile.id === currentId)) {
    return currentId;
  }

  return profiles[0]?.id ?? "";
}

export function defaultStaticExportDir(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}/exports/static`;
}

export function defaultStaticArchivePath(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}/exports/static.zip`;
}

export function defaultNewProjectPath(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}-new`;
}

function trimTrailingSlashes(path: string) {
  return path.replace(/\/+$/, "") || ".";
}
