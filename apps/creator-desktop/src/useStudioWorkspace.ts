import { useEffect, useMemo, useRef, useState, type Dispatch, type SetStateAction } from "react";
import type {
  AssetRecord,
  ExportProfile,
  PiAgentCapability,
  ProjectData,
} from "../../../contracts/plotforge";
import { summarizeProject, type CreatorProjectSummary } from "./projectSummary";
import type { StudioDataSource } from "./studioDataSource";
import { projectAssetCatalog, type AssetCatalog } from "./assetCatalog";
import type {
  ProjectCheckReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";
import { errorMessage } from "./errorMessage";
import {
  useProjectEditing,
  type EditingWorkspace,
} from "./useProjectEditing";
import { usePlaytest, type PlaytestWorkspace } from "./usePlaytest";
import { useExport, type ExportWorkspace } from "./useExport";
import {
  useAgentConversation,
  type AgentConversationWorkspace,
} from "./useAgentConversation";
import { useGitInfo, type GitInfoWorkspace } from "./useGitInfo";
import { useAgentConfig, type AgentConfigWorkspace } from "./useAgentConfig";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// Exported types
// ---------------------------------------------------------------------------

export interface StudioMetric {
  labelKey: string;
  value: string;
  tone: string;
}

export interface UseStudioWorkspaceOptions {
  dataSource: StudioDataSource;
  initialProjectPath: string;
}

export interface StudioWorkspace {
  // Project-level state
  projectPath: string;
  setProjectPath: Dispatch<SetStateAction<string>>;
  loadedPath: string;
  projectData: ProjectData | null;
  projectSummary: CreatorProjectSummary | null;
  checkReport: ProjectCheckReport | null;
  sourceFiles: SourceFileSummary[];
  selectedFile: SourceFileContent | null;
  editorContent: string;
  setEditorContent: Dispatch<SetStateAction<string>>;
  savedContent: string;
  dirty: boolean;
  loading: boolean;
  saving: boolean;
  error: string | null;
  assetCatalog: AssetCatalog;
  metrics: StudioMetric[];
  piAgentCapabilities: PiAgentCapability[];

  // Project-level methods
  loadProject(path: string, createIfMissing?: boolean): Promise<void>;
  refreshProjectOverview(path?: string): Promise<void>;
  selectSourceFile(file: SourceFileSummary): Promise<void>;
  saveSelectedFile(): Promise<void>;

  // Aggregated sub-hooks (grouped)
  editing: EditingWorkspace;
  playtest: PlaytestWorkspace;
  export: ExportWorkspace;
  agent: AgentConversationWorkspace;
  gitInfo: GitInfoWorkspace;
  agentConfig: AgentConfigWorkspace;
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useStudioWorkspace({
  dataSource,
  initialProjectPath,
}: UseStudioWorkspaceOptions): StudioWorkspace {
  const { t } = useStudioI18n();
  const [projectPath, setProjectPath] = useState(initialProjectPath);
  const [loadedPath, setLoadedPath] = useState(initialProjectPath);
  const [projectData, setProjectData] = useState<ProjectData | null>(null);
  const [assetRecords, setAssetRecords] = useState<AssetRecord[]>([]);
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
  const [loading, setLoading] = useState(Boolean(initialProjectPath));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [piAgentCapabilities, setPiAgentCapabilities] = useState<
    PiAgentCapability[]
  >([]);

  const dirty = Boolean(selectedFile?.editable && editorContent !== savedContent);

  const assetCatalog = useMemo(
    () => projectAssetCatalog(projectData, assetRecords),
    [assetRecords, projectData],
  );

  const metrics = useMemo(
    () => [
      {
        labelKey: "metrics.scenes",
        value: String(
          checkReport?.scene_count ?? projectSummary?.sceneCount ?? 0,
        ),
        tone: "border-sage/50 text-sage",
      },
      {
        labelKey: "metrics.characters",
        value: String(
          checkReport?.character_count ?? projectSummary?.characterCount ?? 0,
        ),
        tone: "border-plum-500/50 text-plum-500",
      },
      {
        labelKey: "metrics.rules",
        value: String(
          checkReport?.rule_count ?? projectSummary?.ruleCount ?? 0,
        ),
        tone: "border-signal/50 text-signal",
      },
      {
        labelKey: "metrics.openThreads",
        value: String(projectSummary?.openThreadCount ?? 0),
        tone: "border-ink/30 text-ink",
      },
    ],
    [checkReport, projectSummary],
  );

  // Ref for refreshProjectOverview so useProjectEditing can call it
  // without a circular dependency.
  const refreshRef = useRef<(path?: string) => Promise<void>>(
    async () => {},
  );

  // Sub-hooks --------------------------------------------------------------

  const editing = useProjectEditing({
    dataSource,
    onRefreshProjectOverview: async (path?: string) => {
      await refreshRef.current?.(path);
    },
    onSetProjectData: setProjectData,
  });

  const playtest = usePlaytest(dataSource);

  const exportWorkspace = useExport({ dataSource, initialProjectPath });

  // Git + agent-config sub-hooks. The git hook refreshes the branch list on
  // path change; on a successful branch switch we reload the project so the
  // editor/source/trace views reflect the new work tree.
  const gitInfo = useGitInfo(dataSource, loadedPath, () => {
    void loadProject(loadedPath);
  });
  const agentConfig = useAgentConfig(dataSource, loadedPath);

  // The agent conversation now drives `pi_agent_apply_run` (not the playtest
  // play_once path), so it needs the data source + the persisted agent
  // config (for the model id) + the playtest workspace (for snapshot control
  // state and the shared input box).
  const agent = useAgentConversation(playtest, loadedPath, dataSource, agentConfig.agentConfig);

  // Core operations --------------------------------------------------------

  async function loadProject(path: string, createIfMissing = false) {
    setLoading(true);
    setError(null);
    try {
      // Opening the canonical project is the load boundary. Once this
      // succeeds the folder is a valid PlotForge project and the shell/home
      // must reflect it immediately; secondary editor/index reads may still
      // report an explicit error, but they must not roll the project back to
      // the misleading "not loaded" state.
      const project = createIfMissing
        ? await dataSource.openOrCreateProject(path)
        : await dataSource.openProject(path);
      setLoadedPath(path);
      setProjectPath(path);
      setProjectData(project);
      setAssetRecords(project.asset_records);
      setProjectSummary(summarizeProject(project));
      setCheckReport(null);
      setSourceFiles([]);
      setSelectedFile(null);
      setEditorContent("");
      setSavedContent("");

      const [
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
        piCapabilities,
      ] = await Promise.all([
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
        dataSource.piAgentCapabilities(),
      ]);
      const firstEditable =
        files.find((file) => file.editable) ?? files[0];
      const firstContent = firstEditable
        ? await dataSource.readSourceFile(path, firstEditable.path)
        : null;
      const projectWithBible = attachBibles(
        project,
        visualBibleDocument,
        audioBibleDocument,
      );

      setProjectData(projectWithBible);
      setAssetRecords(records);
      setProjectSummary(summarizeProject(projectWithBible));
      setCheckReport(report);
      setSourceFiles(files);
      setSelectedFile(firstContent);
      setEditorContent(firstContent?.content ?? "");
      setSavedContent(firstContent?.content ?? "");
      setPiAgentCapabilities(piCapabilities);

      // Sync to sub-hooks
      editing.loadEditingDocuments({
        worldEditDocument: worldDocument,
        storyCraftEditDocument: storyCraftDocument,
        characterEditDocument: characterDocument,
        stateVariablesEditDocument: stateVariablesDocument,
        rulesEditDocument: rulesDocument,
        aiSafetyPolicy: safetyPolicy,
        visualBible: visualBibleDocument,
        audioBible: audioBibleDocument,
      });
      playtest.resetPlaytest();
      exportWorkspace.resetExport(path, profiles);
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
      editing.setVisualBible(visualBibleDocument);
      editing.setAudioBible(audioBibleDocument);
      setProjectSummary(summarizeProject(projectWithBible));
      setCheckReport(report);
      setSourceFiles(files);
    } catch (source) {
      setError(errorMessage(source));
      throw source;
    }
  }

  // Wire up the ref so useProjectEditing's onRefreshProjectOverview works
  refreshRef.current = refreshProjectOverview;

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

  // Auto-load only when an explicit path is supplied by tests or deep links.
  useEffect(() => {
    if (initialProjectPath) {
      void loadProject(initialProjectPath);
    } else {
      setLoading(false);
    }
  }, [initialProjectPath]);

  // Return ----------------------------------------------------------------

  return {
    projectPath,
    setProjectPath,
    loadedPath,
    projectData,
    projectSummary,
    checkReport,
    sourceFiles,
    selectedFile,
    editorContent,
    setEditorContent,
    savedContent,
    dirty,
    loading,
    saving,
    error,
    assetCatalog,
    metrics,
    piAgentCapabilities,
    loadProject,
    refreshProjectOverview,
    selectSourceFile,
    saveSelectedFile,
    editing,
    playtest,
    export: exportWorkspace,
    agent,
    gitInfo,
    agentConfig,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function attachBibles(
  project: ProjectData,
  visualBible: import("../../../contracts/plotforge").VisualBible,
  audioBible: import("../../../contracts/plotforge").AudioBible,
): ProjectData {
  return {
    ...project,
    visual_bible: visualBible,
    audio_bible: audioBible,
  };
}
