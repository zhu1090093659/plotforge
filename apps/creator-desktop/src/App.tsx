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
  type StudioNavItem,
} from "./studioUi";
import {
  defaultNewProjectPath,
  useStudioWorkspace,
} from "./useStudioWorkspace";
import { LanguageToggle, StudioI18nProvider, useStudioI18n } from "./i18n";
import { errorMessage } from "./errorMessage";

export interface AppProps {
  dataSource?: StudioDataSource;
  initialProjectPath?: string;
}

export function App(props: AppProps) {
  return (
    <StudioI18nProvider>
      <AppContent {...props} />
    </StudioI18nProvider>
  );
}

function AppContent({
  dataSource = createDefaultStudioDataSource(),
  initialProjectPath = defaultProjectPath(),
}: AppProps) {
  const { t } = useStudioI18n();
  const [activeWorkflow, setActiveWorkflow] =
    useState<AgentNativeWorkflowId>("command");
  const [activeSection, setActiveSection] =
    useState<StudioSectionId>("launchpad");
  const [expandedWorkflows, setExpandedWorkflows] = useState<
    Set<AgentNativeWorkflowId>
  >(() => new Set(["command"]));
  const [drawerOpen, setDrawerOpen] = useState(false);

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
      setExpandedWorkflows((prev) => {
        const next = new Set(prev);
        next.add("proof");
        return next;
      });
    } else {
      setActiveWorkflow("game");
      setActiveSection("playtest");
      setExpandedWorkflows((prev) => {
        const next = new Set(prev);
        next.add("game");
        return next;
      });
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
      setCreateError(t("app.createError"));
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

  /**
   * Currently unreachable: `StudioNavTreeNode` invokes `onToggleExpand` for
   * items with children, never `onSelect`.  All 6 workflows have children, so
   * clicking a workflow row only toggles expand/collapse.  `activeWorkflow`
   * changes exclusively via `openStudioSection` (surface clicks) and
   * `runPlaytest`.  Kept as the `onSelect` wiring for `navItems` so the type
   * contract holds and future workflows without children fall back to it.
   */
  function openWorkflow(workflowId: AgentNativeWorkflowId) {
    setActiveWorkflow(workflowId);
    setExpandedWorkflows((prev) => {
      const next = new Set(prev);
      next.add(workflowId);
      return next;
    });
  }

  function toggleWorkflowExpand(workflowId: AgentNativeWorkflowId) {
    setExpandedWorkflows((prev) => {
      const next = new Set(prev);
      if (next.has(workflowId)) {
        next.delete(workflowId);
      } else {
        next.add(workflowId);
      }
      return next;
    });
  }

  function openStudioSection(section: StudioSectionId) {
    const workflowId = isSectionInWorkflow(section, activeWorkflow)
      ? activeWorkflow
      : workflowForSection(section).id;
    setActiveWorkflow(workflowId);
    setActiveSection(section);
    setExpandedWorkflows((prev) => {
      const next = new Set(prev);
      next.add(workflowId);
      return next;
    });
    setDrawerOpen(false);
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
        ? t("common.errorVisible")
        : t("common.traceVisible");

    return (
      <div className="grid gap-4">
        <div>
          <p className="text-xs font-semibold uppercase tracking-tightish text-violet-600">
            {t("app.evidencePanel")}
          </p>
          <h3 className="font-display mt-1 text-lg font-semibold tracking-display text-ink">
            {t(activeWorkflowMeta.labelKey)}
          </h3>
          <p className="mt-1 text-sm leading-6 text-graphite-700/70">
            {t(activeWorkflowMeta.descriptionKey)}
          </p>
        </div>

        <StudioPanel>
          <div className="flex flex-wrap gap-2">
            <StudioStatusChip tone={healthTone}>{healthLabel}</StudioStatusChip>
            <StudioStatusChip tone="accent">{dataSource.runtimeName}</StudioStatusChip>
            <StudioStatusChip tone="agent">
              {t(activeSectionMeta.statusKey)}
            </StudioStatusChip>
          </div>
          <div className="mt-4 grid gap-3 text-sm">
            <EvidenceLine label={t("app.project")} value={projectSummary?.title ?? t("common.none")} />
            <EvidenceLine label={t("app.loadedPath")} value={loadedPath} />
            <EvidenceLine label={t("app.sourceFiles")} value={String(sourceFiles.length)} />
            <EvidenceLine
              label={t("app.assetRecords")}
              value={String(
                assetCatalog.items.filter((item) => item.source === "record")
                  .length,
              )}
            />
          </div>
        </StudioPanel>

        <div className="rounded-lg border border-canvas-200 bg-graphite-850 px-3 py-3">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase tracking-tightish text-graphite-700/65">
                {t("app.backendBoundary")}
              </p>
              <h4 className="font-display mt-1 text-sm font-semibold tracking-display text-ink">
                {t("app.realStudioCommandSurface")}
              </h4>
            </div>
            <StudioStatusChip tone="health">{dataSource.runtimeName}</StudioStatusChip>
          </div>

          <div className="mt-3 grid gap-2 text-sm">
            <PreviewEvidenceLine
              label={t("app.projectSource")}
              value={t("app.folderFiles")}
            />
            <PreviewEvidenceLine
              label={t("app.runtime")}
              value={playtest.playtestReport?.trace.id ?? t("common.notRun")}
            />
            <PreviewEvidenceLine
              label={t("app.export")}
              value={exportWorkspace.exportReport?.archive_path ?? t("common.notExported")}
            />
            <PreviewEvidenceLine
              label={t("app.externalAgents")}
              value={t("common.notImplemented")}
            />
          </div>

          <div className="mt-3 grid gap-2">
            {[
              t("app.boundary.browserMode"),
              t("app.boundary.tauriMode"),
              t("app.boundary.piAgent"),
            ].map((boundary) => (
              <p
                key={boundary}
                className="rounded-md border border-canvas-200 bg-canvas-100 px-3 py-2 text-xs leading-5 text-graphite-700/75"
              >
                {boundary}
              </p>
            ))}
          </div>
        </div>

        <div className="rounded-lg border border-canvas-200 bg-graphite-850 px-3 py-3">
          <p className="text-xs font-semibold uppercase tracking-tightish text-graphite-700/65">
            {t("app.referenceScreens")}
          </p>
          <div className="mt-3 grid gap-2">
            {activeScreenReferences.map((reference) => (
              <div
                key={reference.id}
                title={t(reference.titleKey)}
                className="rounded-md border border-canvas-200 bg-canvas-100 px-3 py-2"
              >
                <p className="font-display truncate text-sm font-semibold tracking-tightish text-ink">
                  {reference.fileName}
                </p>
                <p className="mt-1 truncate text-xs text-graphite-700/60">
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
          <p className="text-xs font-semibold uppercase tracking-tightish text-graphite-700/65">
            {t("app.commandDock")}
          </p>
          <p className="font-display truncate text-sm font-semibold tracking-tightish text-ink">
            {projectSummary?.title ?? t("app.noProjectLoaded")} /{" "}
            {t(activeSectionMeta.labelKey)}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <StudioStatusChip tone="action">
            {dirty ? t("app.unsavedSource") : t("app.workspaceSynced")}
          </StudioStatusChip>
          <StudioButton
            variant="primary"
            onClick={() => openStudioSection("playtest")}
          >
            <Play aria-hidden size={16} />
            {t("app.runPlayableProof")}
          </StudioButton>
        </div>
      </>
    );
  }

  const navItems: StudioNavItem[] = agentNativeWorkflows.map((workflow) => {
    const isActiveWorkflow = workflow.id === activeWorkflow;
    const sectionItems = workflow.sectionIds.map((sectionId) => {
      const section = getStudioSection(sectionId);
      return {
        id: sectionId,
        label: t(section.labelKey),
        sublabel: t(section.statusKey),
        description: t(section.descriptionKey),
        icon: section.icon,
        selected: sectionId === activeSection,
        onSelect: () => openStudioSection(sectionId),
      } satisfies StudioNavItem;
    });
    return {
      id: workflow.id,
      label: t(workflow.labelKey),
      sublabel: t(workflow.shortLabelKey),
      description: t(workflow.descriptionKey),
      icon: workflow.icon,
      selected: isActiveWorkflow,
      onSelect: () => openWorkflow(workflow.id),
      children: sectionItems,
    } satisfies StudioNavItem;
  });

  return (
    <StudioShell
      projectPath={loadedPath}
      projectLoading={loading}
      onOpenProject={() => void loadProject(projectPath)}
      navItems={navItems}
      expandedIds={expandedWorkflows}
      onToggleExpand={(id) =>
        toggleWorkflowExpand(id as AgentNativeWorkflowId)
      }
      header={{
        eyebrow: `${t(activeWorkflowMeta.labelKey)} / ${dataSource.runtimeName}`,
        title: t(activeSectionMeta.labelKey),
        subtitle: `${projectSummary?.title ?? t("app.noProjectLoaded")} - ${t(activeWorkflowMeta.descriptionKey)}`,
        badges: activeScreenReferences.map((reference) => ({
          id: reference.id,
          label: t(reference.titleKey),
          title: reference.fileName,
        })),
      }}
      topActions={
        <>
          <LanguageToggle />
          <input
            aria-label={t("app.projectPath")}
            value={projectPath}
            onChange={(event) => setProjectPath(event.target.value)}
            className={`${studioUiClassNames.input} min-w-0 flex-1 sm:w-72 lg:w-80 xl:w-96`}
          />
          <StudioButton
            title={t("app.refreshProjectFiles")}
            aria-label={t("app.refreshProjectFiles")}
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
      drawerOpen={drawerOpen}
      onToggleDrawer={() => setDrawerOpen((prev) => !prev)}
      onCloseDrawer={() => setDrawerOpen(false)}
    >
      {renderActiveSection()}
    </StudioShell>
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
      <p className="text-xs font-medium uppercase tracking-tightish text-graphite-700/65">{label}</p>
      <p className="font-display mt-1 truncate text-sm font-semibold tracking-tightish text-ink">{value}</p>
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
      <p className="text-xs font-medium uppercase tracking-tightish text-graphite-700/60">
        {label}
      </p>
      <p className="font-display truncate text-xs font-semibold tracking-tightish text-ink">{value}</p>
    </div>
  );
}
