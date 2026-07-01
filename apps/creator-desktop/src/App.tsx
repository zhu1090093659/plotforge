import { Loader2, Play, RefreshCcw } from "lucide-react";
import { useEffect, useState } from "react";
import type {
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectTemplateId,
} from "../../../contracts/plotforge";
import { AgentMeshView } from "./AgentMeshView";
import { ArtifactReviewView } from "./ArtifactReviewView";
import { AssetMaintenanceView } from "./AssetMaintenanceView";
import { LaunchpadView } from "./LaunchpadView";
import { CharactersView } from "./CharactersView";
import { StateView } from "./StateView";
import { CommandCenterView } from "./CommandCenterView";
import { DirectorModeView } from "./DirectorModeView";
import { ExportView } from "./ExportView";
import { RulesView } from "./RulesView";
import { TraceDebugView } from "./TraceDebugView";
import { WorldView } from "./WorldView";
import { StoryView } from "./StoryView";
import {
  createDefaultStudioDataSource,
  defaultProjectPath,
  type StudioDataSource,
} from "./studioDataSource";
import {
  agentNativeWorkflows,
  defaultSectionForWorkflow,
  getAgentNativeWorkflow,
  getStudioSection,
  isSectionInWorkflow,
  screenReferencesForWorkflow,
  workflowForSection,
  type AgentNativeWorkflowId,
  type StudioSectionId,
} from "./studioModel";
import {
  StudioButton,
  StudioPanel,
  StudioShell,
  StudioStatusChip,
  studioUiClassNames,
} from "./studioUi";
import {
  defaultNewProjectPath,
  useStudioWorkspace,
} from "./useStudioWorkspace";
import { LanguageToggle, StudioI18nProvider } from "./i18n";
import { errorMessage } from "./errorMessage";

export interface AppProps {
  dataSource?: StudioDataSource;
  initialProjectPath?: string;
}

export function App({
  dataSource = createDefaultStudioDataSource(),
  initialProjectPath = defaultProjectPath(),
}: AppProps) {
  const [activeWorkflow, setActiveWorkflow] =
    useState<AgentNativeWorkflowId>("command");
  const [activeSection, setActiveSection] =
    useState<StudioSectionId>("launchpad");

  const {
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
    loadProject: loadWorkspaceProject,
    refreshProjectOverview,
    selectSourceFile,
    saveSelectedFile,
    editing,
    playtest,
    export: exportWorkspace,
  } = useStudioWorkspace({ dataSource, initialProjectPath });

  // Create project state (Launchpad-specific, kept in App)
  const [createProjectPath, setCreateProjectPath] = useState(
    defaultNewProjectPath(initialProjectPath),
  );
  const [createTemplate, setCreateTemplate] =
    useState<ProjectTemplateId>("historical_crisis");
  const [createConcept, setCreateConcept] = useState("");
  const [createVisualStyle, setCreateVisualStyle] = useState("");
  const [createVoiceEnabled, setCreateVoiceEnabled] = useState(false);
  const [createInitialSceneRequest, setCreateInitialSceneRequest] =
    useState("");
  const [createForce, setCreateForce] = useState(false);
  const [createReport, setCreateReport] =
    useState<ProjectCreationReport | null>(null);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);

  const activeSectionMeta = getStudioSection(activeSection);
  const activeWorkflowMeta = getAgentNativeWorkflow(activeWorkflow);
  const activeScreenReferences = screenReferencesForWorkflow(activeWorkflow);
  const activeWorkflowSections = activeWorkflowMeta.sectionIds.map(
    (sectionId) => getStudioSection(sectionId),
  );

  useEffect(() => {
    setCreateProjectPath(defaultNewProjectPath(initialProjectPath));
  }, [initialProjectPath]);

  // Routing helpers -------------------------------------------------------

  async function loadProject(path: string) {
    await loadWorkspaceProject(path);
  }

  async function runPlaytest() {
    const succeeded = await playtest.runPlaytest(loadedPath);
    if (succeeded) {
      setActiveWorkflow("proof");
      setActiveSection("debugger");
    } else {
      setActiveWorkflow("game");
      setActiveSection("playtest");
    }
  }

  async function handleCreateProject(
    path: string,
    request: ProjectCreationRequest,
    force: boolean,
  ) {
    if (
      !path ||
      !request.concept ||
      !request.visual_style ||
      !request.initial_scene_request
    ) {
      setCreateError(
        "Project path, concept, visual style, and initial scene are required.",
      );
      return;
    }

    setCreating(true);
    setCreateError(null);
    try {
      const report = await dataSource.createProject(path, request, force);
      setCreateReport(report);
      setCreateProjectPath(report.project_path);
      await loadProject(report.project_path);
    } catch (source) {
      setCreateError(errorMessage(source));
    } finally {
      setCreating(false);
    }
  }

  function openWorkflow(workflowId: AgentNativeWorkflowId) {
    setActiveWorkflow(workflowId);
    setActiveSection(defaultSectionForWorkflow(workflowId));
  }

  function openStudioSection(section: StudioSectionId) {
    const workflowId = isSectionInWorkflow(section, activeWorkflow)
      ? activeWorkflow
      : workflowForSection(section).id;
    setActiveWorkflow(workflowId);
    setActiveSection(section);
  }

  function openExportProfile(profileId: string) {
    exportWorkspace.selectExportProfile(profileId);
    openStudioSection("export-kit");
  }

  // View rendering --------------------------------------------------------

  function renderActiveSection() {
    switch (activeSection) {
      case "agent-mesh":
        return (
          <AgentMeshView
            projectSummary={projectSummary}
            loadedPath={loadedPath}
            runtimeName={dataSource.runtimeName}
            sourceFiles={sourceFiles}
            assetRecordCount={
              assetCatalog.items.filter((item) => item.source === "record")
                .length
            }
            exportProfileCount={exportWorkspace.exportProfiles.length}
            playtestReport={playtest.playtestReport}
            onOpenTrace={() => openStudioSection("debugger")}
            onRunPlayableProof={() => void runPlaytest()}
          />
        );
      case "world":
        return (
          <WorldView
            worldEditDocument={editing.worldEditDocument}
            saving={editing.formSaving === "world"}
            formStatus={editing.formStatus}
            worldExpansionGoal={editing.worldExpansionGoal}
            onWorldExpansionGoalChange={editing.setWorldExpansionGoal}
            onSave={() => void editing.saveWorldEditDocument(loadedPath)}
            onGenerateWorldExpansion={() =>
              void editing.generateWorldExpansionFromGoal(loadedPath)
            }
            onUpdateWorldDocument={editing.updateWorldDocument}
          />
        );
      case "story":
        return (
          <StoryView
            storyCraftEditDocument={editing.storyCraftEditDocument}
            saving={editing.formSaving === "story"}
            formStatus={editing.formStatus}
            storyGenerationConcept={editing.storyGenerationConcept}
            onStoryGenerationConceptChange={editing.setStoryGenerationConcept}
            onSave={() => void editing.saveStoryCraftEditDocument(loadedPath)}
            onGenerateStoryCraft={() =>
              void editing.generateStoryCraftFromConcept(loadedPath)
            }
            onUpdateStoryBible={editing.updateStoryBible}
            onUpdateStoryCraftDocument={editing.updateStoryCraftDocument}
          />
        );
      case "characters":
        return (
          <CharactersView
            characterEditDocument={editing.characterEditDocument}
            saving={editing.formSaving === "characters"}
            formStatus={editing.formStatus}
            characterGenerationConcept={editing.characterGenerationConcept}
            onCharacterGenerationConceptChange={
              editing.setCharacterGenerationConcept
            }
            characterGenerationRoleHint={editing.characterGenerationRoleHint}
            onCharacterGenerationRoleHintChange={
              editing.setCharacterGenerationRoleHint
            }
            characterDraft={editing.newCharacter}
            onCharacterDraftChange={editing.setNewCharacter}
            onSave={() => void editing.saveCharacterEditDocument(loadedPath)}
            onGenerateCharacter={() =>
              void editing.generateCharacterFromConcept(loadedPath)
            }
            onCreateCharacterFromDraft={() =>
              void editing.createCharacterFromDraft(loadedPath)
            }
            onUpdateCharacter={editing.updateCharacter}
          />
        );
      case "state":
        return (
          <StateView
            stateVariablesEditDocument={editing.stateVariablesEditDocument}
            saving={editing.formSaving === "state"}
            formStatus={editing.formStatus}
            newResource={editing.newResource}
            onNewResourceChange={editing.setNewResource}
            onSave={() =>
              void editing.saveStateVariablesEditDocument(loadedPath)
            }
            onUpdateResource={editing.updateResource}
            onUpdateInitialWorldResource={editing.updateInitialWorldResource}
            onUpdateInitialSceneKey={editing.updateInitialSceneKey}
            onUpdateInitialTurn={editing.updateInitialTurn}
            onCreateResourceFromDraft={() =>
              void editing.createResourceFromDraft(loadedPath)
            }
          />
        );
      case "rules":
        return (
          <RulesView
            rulesEditDocument={editing.rulesEditDocument}
            resourceKeys={
              editing.stateVariablesEditDocument?.resources.map(
                (r) => r.key,
              ) ?? []
            }
            saving={editing.formSaving === "rules"}
            formStatus={editing.formStatus}
            ruleDraft={editing.newRule}
            onRuleDraftChange={editing.setNewRule}
            onSave={() => void editing.saveRulesEditDocument(loadedPath)}
            onCreateRuleFromDraft={() =>
              void editing.createRuleFromDraft(loadedPath)
            }
            onUpdateRule={editing.updateRule}
          />
        );
      case "assets":
        return (
          <ArtifactReviewView
            projectSummary={projectSummary}
            loadedPath={loadedPath}
            sourceFiles={sourceFiles}
            assetCatalog={assetCatalog}
            playtestReport={playtest.playtestReport}
            playtesting={playtest.playtesting}
            playtestError={playtest.playtestError}
            exportReport={exportWorkspace.exportReport}
            onOpenTrace={() => openStudioSection("debugger")}
            onRunPlayableProof={() => void runPlaytest()}
          >
            <AssetMaintenanceView
              assetCatalog={assetCatalog}
              sourceFiles={sourceFiles}
              visualBible={editing.visualBible}
              audioBible={editing.audioBible}
              saving={editing.formSaving === "assets"}
              formStatus={editing.formStatus}
              onUpdateVisualStyleCard={editing.updateVisualStyleCard}
              onUpdateAudioVoiceCard={editing.updateAudioVoiceCard}
              onSaveVisualBible={() =>
                void editing.saveVisualBible(loadedPath)
              }
              onSaveAudioBible={() =>
                void editing.saveAudioBible(loadedPath)
              }
            />
          </ArtifactReviewView>
        );
      case "playtest":
        return (
          <DirectorModeView
            projectData={projectData}
            loadedPath={loadedPath}
            input={playtest.playtestInput}
            running={playtest.playtesting}
            report={playtest.playtestReport}
            error={playtest.playtestError}
            saveId={playtest.playtestSaveId}
            restoreId={playtest.playtestRestoreId}
            restoreLatest={playtest.playtestRestoreLatest}
            onInputChange={playtest.setPlaytestInput}
            onSaveIdChange={playtest.setPlaytestSaveId}
            onRestoreIdChange={playtest.setPlaytestRestoreId}
            onRestoreLatestChange={playtest.setPlaytestRestoreLatest}
            onRun={() => void runPlaytest()}
            onOpenStory={() => openStudioSection("story")}
            onOpenTrace={() => openStudioSection("debugger")}
          />
        );
      case "debugger":
        return (
          <TraceDebugView
            report={playtest.playtestReport}
            error={playtest.playtestError}
            selectedExportProfile={exportWorkspace.selectedExportProfile}
            exportReport={exportWorkspace.exportReport}
            aiSafetyPolicy={editing.aiSafetyPolicy}
            loadedPath={loadedPath}
            projectId={projectData?.game.id ?? null}
          />
        );
      case "export-kit":
        return (
          <ExportView
            exportDir={exportWorkspace.exportDir}
            setExportDir={exportWorkspace.setExportDir}
            archivePath={exportWorkspace.archivePath}
            setArchivePath={exportWorkspace.setArchivePath}
            exportProfiles={exportWorkspace.exportProfiles}
            selectedExportProfileId={exportWorkspace.selectedExportProfileId}
            selectedExportProfile={exportWorkspace.selectedExportProfile}
            staticExportSelected={exportWorkspace.staticExportSelected}
            exportReport={exportWorkspace.exportReport}
            exporting={exportWorkspace.exporting}
            exportError={exportWorkspace.exportError}
            assetCatalog={assetCatalog}
            projectData={projectData}
            aiSafetyPolicy={editing.aiSafetyPolicy}
            formSaving={editing.formSaving}
            formStatus={editing.formStatus}
            runStaticZipExport={async () => {
              await exportWorkspace.runStaticZipExport(loadedPath);
            }}
            selectExportProfile={exportWorkspace.selectExportProfile}
            updateAiSafetyPolicy={editing.updateAiSafetyPolicy}
            saveAiSafetyPolicy={async () => {
              await editing.saveAiSafetyPolicy(loadedPath);
            }}
          />
        );
      case "launchpad":
      default:
        return renderLaunchpad();
    }
  }

  function renderLaunchpad() {
    return (
      <LaunchpadView
        projectSummary={projectSummary}
        projectData={projectData}
        loadedPath={loadedPath}
        checkReport={checkReport}
        metrics={metrics}
        sourceFiles={sourceFiles}
        selectedFile={selectedFile}
        editorContent={editorContent}
        setEditorContent={setEditorContent}
        dirty={dirty}
        saving={saving}
        error={error}
        playtestInput={playtest.playtestInput}
        setPlaytestInput={playtest.setPlaytestInput}
        playtesting={playtest.playtesting}
        playtestReport={playtest.playtestReport}
        playtestError={playtest.playtestError}
        exportProfiles={exportWorkspace.exportProfiles}
        exportReport={exportWorkspace.exportReport}
        assetCatalog={assetCatalog}
        createProjectPath={createProjectPath}
        setCreateProjectPath={setCreateProjectPath}
        createTemplate={createTemplate}
        setCreateTemplate={setCreateTemplate}
        createConcept={createConcept}
        setCreateConcept={setCreateConcept}
        createVisualStyle={createVisualStyle}
        setCreateVisualStyle={setCreateVisualStyle}
        createVoiceEnabled={createVoiceEnabled}
        setCreateVoiceEnabled={setCreateVoiceEnabled}
        createInitialSceneRequest={createInitialSceneRequest}
        setCreateInitialSceneRequest={setCreateInitialSceneRequest}
        createForce={createForce}
        setCreateForce={setCreateForce}
        createReport={createReport}
        creating={creating}
        createError={createError}
        onRunPlayableProof={() => void runPlaytest()}
        onOpenSection={openStudioSection}
        onOpenExportProfile={openExportProfile}
        onSelectSourceFile={(file) => void selectSourceFile(file)}
        onSaveSelectedFile={() => void saveSelectedFile()}
        onCreateProject={(path, request, force) =>
          void handleCreateProject(path, request, force)
        }
        dataSource={dataSource}
      />
    );
  }

  function renderEvidencePanel() {
    const healthTone =
      error || playtest.playtestError || exportWorkspace.exportError
        ? "danger"
        : "health";
    const healthLabel =
      error || playtest.playtestError || exportWorkspace.exportError
        ? "Error visible"
        : "Trace visible";

    return (
      <div className="grid gap-4">
        <div>
          <p className="text-xs font-semibold uppercase tracking-tightish text-amber-400">
            Evidence Panel
          </p>
          <h3 className="font-display mt-1 text-lg font-semibold tracking-display text-canvas-50">
            {activeWorkflowMeta.label}
          </h3>
          <p className="mt-1 text-sm leading-6 text-canvas-200/65">
            {activeWorkflowMeta.description}
          </p>
        </div>

        <StudioPanel>
          <div className="flex flex-wrap gap-2">
            <StudioStatusChip tone={healthTone}>{healthLabel}</StudioStatusChip>
            <StudioStatusChip tone="accent">{dataSource.runtimeName}</StudioStatusChip>
            <StudioStatusChip tone="agent">
              {activeSectionMeta.status}
            </StudioStatusChip>
          </div>
          <div className="mt-4 grid gap-3 text-sm">
            <EvidenceLine label="Project" value={projectSummary?.title ?? "none"} />
            <EvidenceLine label="Loaded path" value={loadedPath} />
            <EvidenceLine label="Source files" value={String(sourceFiles.length)} />
            <EvidenceLine
              label="Asset records"
              value={String(
                assetCatalog.items.filter((item) => item.source === "record")
                  .length,
              )}
            />
          </div>
        </StudioPanel>

        <div className="rounded-lg border border-canvas-200/12 bg-graphite-850 px-3 py-3">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase tracking-tightish text-canvas-200/55">
                Backend Boundary
              </p>
              <h4 className="font-display mt-1 text-sm font-semibold tracking-display text-canvas-50">
                Real Studio command surface
              </h4>
            </div>
            <StudioStatusChip tone="health">{dataSource.runtimeName}</StudioStatusChip>
          </div>

          <div className="mt-3 grid gap-2 text-sm">
            <PreviewEvidenceLine
              label="Project source"
              value="folder files"
            />
            <PreviewEvidenceLine
              label="Runtime"
              value={playtest.playtestReport?.trace.id ?? "not run"}
            />
            <PreviewEvidenceLine
              label="Export"
              value={exportWorkspace.exportReport?.archive_path ?? "not exported"}
            />
            <PreviewEvidenceLine
              label="External agents"
              value="not implemented"
            />
          </div>

          <div className="mt-3 grid gap-2">
            {[
              "Browser mode uses the HTTP dev bridge backed by plotforge-studio.",
              "Tauri mode uses the same command names through IPC.",
              "pi-Agent runtime is local and schema-backed; external agent execution, hidden network calls, and publishing automation remain not implemented.",
            ].map((boundary) => (
              <p
                key={boundary}
                className="rounded-md border border-canvas-200/12 bg-canvas-50/5 px-3 py-2 text-xs leading-5 text-canvas-200/65"
              >
                {boundary}
              </p>
            ))}
          </div>
        </div>

        <div className="rounded-lg border border-canvas-200/12 bg-graphite-850 px-3 py-3">
          <p className="text-xs font-semibold uppercase tracking-tightish text-canvas-200/55">
            Reference Screens
          </p>
          <div className="mt-3 grid gap-2">
            {activeScreenReferences.map((reference) => (
              <div
                key={reference.id}
                title={reference.title}
                className="rounded-md border border-canvas-200/12 bg-canvas-50/5 px-3 py-2"
              >
                <p className="font-display truncate text-sm font-semibold tracking-tightish text-canvas-50">
                  {reference.fileName}
                </p>
                <p className="mt-1 truncate text-xs text-canvas-200/45">
                  {reference.id}
                </p>
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  function renderCommandDock() {
    return (
      <>
        <div className="min-w-0">
          <p className="text-xs font-semibold uppercase tracking-tightish text-canvas-200/50">
            Command Dock
          </p>
          <p className="font-display truncate text-sm font-semibold tracking-tightish text-canvas-50">
            {projectSummary?.title ?? "No project loaded"} /{" "}
            {activeSectionMeta.label}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <StudioStatusChip tone="action">
            {dirty ? "Unsaved source" : "Workspace synced"}
          </StudioStatusChip>
          <StudioButton
            variant="primary"
            onClick={() => openStudioSection("playtest")}
          >
            <Play aria-hidden size={16} />
            Run playable proof
          </StudioButton>
        </div>
      </>
    );
  }

  return (
    <StudioI18nProvider>
      <StudioShell
        projectPath={loadedPath}
        projectLoading={loading}
        onOpenProject={() => void loadProject(projectPath)}
        workflowItems={agentNativeWorkflows.map((workflow) => ({
          id: workflow.id,
          label: workflow.label,
          sublabel: workflow.shortLabel,
          description: workflow.description,
          icon: workflow.icon,
          selected: workflow.id === activeWorkflow,
          onSelect: () => openWorkflow(workflow.id),
        }))}
        surfaceItems={activeWorkflowSections.map((section) => ({
          id: section.id,
          label: section.label,
          sublabel: section.status,
          description: section.description,
          icon: section.icon,
          selected: section.id === activeSection,
          onSelect: () => openStudioSection(section.id),
        }))}
        header={{
          eyebrow: `${activeWorkflowMeta.label} / ${dataSource.runtimeName}`,
          title: activeSectionMeta.label,
          subtitle: `${projectSummary?.title ?? "No project loaded"} - ${activeWorkflowMeta.description}`,
          badges: activeScreenReferences.map((reference) => ({
            id: reference.id,
            label: reference.title,
            title: reference.fileName,
          })),
        }}
        topActions={
          <>
            <LanguageToggle />
            <input
              aria-label="Project path"
              value={projectPath}
              onChange={(event) => setProjectPath(event.target.value)}
              className={`${studioUiClassNames.input} sm:w-72`}
            />
            <StudioButton
              title="Refresh project files"
              aria-label="Refresh project files"
              onClick={() => void loadProject(projectPath)}
            >
              {loading ? (
                <Loader2 aria-hidden size={18} className="animate-spin" />
              ) : (
                <RefreshCcw aria-hidden size={18} />
              )}
            </StudioButton>
          </>
        }
        rightPanel={renderEvidencePanel()}
        commandDock={renderCommandDock()}
      >
        {renderActiveSection()}
      </StudioShell>
    </StudioI18nProvider>
  );
}

// ---------------------------------------------------------------------------
// Inline helpers (used by renderEvidencePanel)
// ---------------------------------------------------------------------------

function EvidenceLine({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-medium uppercase tracking-tightish text-canvas-200/55">{label}</p>
      <p className="font-display mt-1 truncate text-sm font-semibold tracking-tightish text-canvas-50">{value}</p>
    </div>
  );
}

function PreviewEvidenceLine({
  label,
  value,
}: {
  label: string;
  value: string;
}) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3">
      <p className="text-xs font-medium uppercase tracking-tightish text-canvas-200/45">
        {label}
      </p>
      <p className="font-display truncate text-xs font-semibold tracking-tightish text-canvas-50">{value}</p>
    </div>
  );
}
