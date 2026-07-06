import { Command, Loader2, RefreshCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { AgentChatRail } from "./AgentChatRail";
import { AssetMaintenanceView } from "./AssetMaintenanceView";
import { LaunchpadView } from "./LaunchpadView";
import { CharactersView } from "./CharactersView";
import { StateView } from "./StateView";
import { PlayView } from "./PlayView";
import { ExportView } from "./ExportView";
import { RulesView } from "./RulesView";
import { SettingsView } from "./SettingsView";
import { SourceView } from "./SourceView";
import { StudioCommandPalette, type PaletteAction } from "./StudioCommandPalette";
import { TraceDebugView } from "./TraceDebugView";
import { WorldView } from "./WorldView";
import { StoryView } from "./StoryView";
import {
  createDefaultStudioDataSource,
  defaultProjectPath,
  type StudioDataSource,
} from "./studioDataSource";
import {
  getStudioSection,
  studioSections,
  type StudioSectionId,
} from "./studioModel";
import {
  Collapsible,
  StudioButton,
  StudioShell,
  StudioStatusChip,
  type StudioNavItem,
} from "./studioUi";
import { useStudioWorkspace } from "./useStudioWorkspace";
import { LanguageToggle, StudioI18nProvider, useStudioI18n } from "./i18n";
import { useStudioRail } from "./useStudioRail";

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
  const [activeSection, setActiveSection] =
    useState<StudioSectionId>("home");
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const rail = useStudioRail();

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
    piAgentCapabilities,
    loadProject: loadWorkspaceProject,
    refreshProjectOverview,
    selectSourceFile,
    saveSelectedFile,
    editing,
    playtest,
    export: exportWorkspace,
    agent,
    gitInfo,
    agentConfig,
  } = useStudioWorkspace({ dataSource, initialProjectPath });

  const activeSectionMeta = getStudioSection(activeSection);

  // Global ⌘K / Ctrl-K to open the command palette.
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((prev) => !prev);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Routing helpers -------------------------------------------------------

  async function loadProject(path: string) {
    await loadWorkspaceProject(path);
  }

  async function runPlaytest() {
    // R7: the command palette's "Run Proof" now drives the same
    // `pi_agent_apply_run` path as the `AgentChatRail` (via
    // `agent.submit()`), so both "run a turn" entry points commit through
    // the pi-Agent and produce a chat turn. The old `playtest.runPlaytest`
    // (which drove `play_once_project*`) is intentionally not used here so
    // the two entry points do not diverge. AGENTS.md still names the rail
    // as the single "describe a change / run a turn" entry point; the
    // palette action is a keyboard shortcut onto the same path.
    const result = await agent.submit();
    if (result.succeeded) {
      setActiveSection("trace");
    }
    return result;
  }

  // "Click option = submit": submit the clicked choice label directly as
  // the turn intent through `submitWith` (which sets the shared input and
  // runs with the argument in one closure — `setPlaytestInput` + `submit()`
  // would read stale state within the same tick). The turn appears in the
  // right-side chat rail and the user is moved to the Trace view.
  async function handleChooseChoice(choiceLabel: string) {
    const result = await agent.submitWith(choiceLabel);
    if (result?.succeeded) {
      setActiveSection("trace");
    }
  }

  function openStudioSection(section: StudioSectionId) {
    setActiveSection(section);
    setDrawerOpen(false);
  }

  function openExportProfile(profileId: string) {
    exportWorkspace.selectExportProfile(profileId);
    openStudioSection("export-kit");
  }

  // View rendering --------------------------------------------------------

  function renderActiveSection() {
    switch (activeSection) {
      case "home":
        // Home now renders inside StudioShell (A1): the sidebar, agent rail,
        // command-palette entry, and collapse toggles are all available on
        // the home page — previously home was a full-screen branch outside
        // the shell, which made the sidebar toggle unreachable from home.
        return (
          <LaunchpadView
            loadedPath={loadedPath}
            projectDirName={deriveProjectDirName(loadedPath)}
            currentBranch={gitInfo.currentBranch}
            branches={gitInfo.branches}
            switchingBranch={gitInfo.switchingBranch}
            switchError={gitInfo.switchError}
            onSwitchBranch={(branch) => void gitInfo.switchBranch(branch)}
            availableModels={agentConfig.availableModels}
            agentConfig={agentConfig.agentConfig}
            onAgentConfigChange={agentConfig.setAgentConfig}
            configSaveError={agentConfig.saveError}
            input={agent.input}
            onInputChange={agent.setInput}
            onSubmit={async () => {
              const result = await agent.submit();
              if (result.succeeded) {
                setActiveSection("trace");
              }
            }}
            running={agent.running}
            canSubmit={agent.canSubmit}
          />
        );
      case "play":
        return (
          <PlayView
            projectData={projectData}
            loadedPath={loadedPath}
            report={playtest.playtestReport}
            running={playtest.playtesting}
            onChooseChoice={(label) => void handleChooseChoice(label)}
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
        );
      case "trace":
        return (
          <TraceDebugView
            report={playtest.playtestReport}
            error={playtest.playtestError}
            selectedExportProfile={exportWorkspace.selectedExportProfile}
            exportReport={exportWorkspace.exportReport}
            aiSafetyPolicy={editing.aiSafetyPolicy}
            loadedPath={loadedPath}
            projectId={projectData?.game.id ?? null}
            saveId={playtest.playtestSaveId}
            restoreId={playtest.playtestRestoreId}
            restoreLatest={playtest.playtestRestoreLatest}
            onSaveIdChange={playtest.setPlaytestSaveId}
            onRestoreIdChange={playtest.setPlaytestRestoreId}
            onRestoreLatestChange={playtest.setPlaytestRestoreLatest}
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
      case "source-files":
      default:
        return (
          <SourceView
            sourceFiles={sourceFiles}
            selectedFile={selectedFile}
            editorContent={editorContent}
            setEditorContent={setEditorContent}
            dirty={dirty}
            saving={saving}
            error={error}
            onSelectSourceFile={(file) => void selectSourceFile(file)}
            onSaveSelectedFile={() => void saveSelectedFile()}
          />
        );
      case "settings":
        return (
          <SettingsView
            dataSource={dataSource}
            loadedPath={loadedPath}
            agentConfig={agentConfig.agentConfig}
            onAgentConfigChange={agentConfig.setAgentConfig}
            configSaveError={agentConfig.saveError}
          />
        );
    }
  }

  function renderAgentRail() {
    return (
      <AgentChatRail
        turns={agent.turns}
        input={agent.input}
        onInputChange={agent.setInput}
        running={agent.running}
        canSubmit={agent.canSubmit}
        onSubmit={async () => {
          const result = await agent.submit();
          if (result.succeeded) {
            setActiveSection("trace");
          }
        }}
        onOpenTrace={() => openStudioSection("trace")}
        evidence={renderEvidencePopover()}
      />
    );
  }

  // The no-fake honesty surface (boundary evidence). Shown on demand via
  // the AgentChatRail "Evidence" popover instead of always-painted.
  function renderEvidencePopover() {
    const healthTone =
      error || playtest.playtestError || exportWorkspace.exportError
        ? "danger"
        : "health";
    const healthLabel =
      error || playtest.playtestError || exportWorkspace.exportError
        ? t("common.errorVisible")
        : t("common.traceVisible");

    return (
      <div className="grid gap-3">
        <div className="rounded-lg border border-canvas-200 bg-canvas-50 p-3">
        <div className="flex flex-wrap gap-2">
          <StudioStatusChip tone={healthTone}>{healthLabel}</StudioStatusChip>
          <StudioStatusChip tone="agent">
            {t(activeSectionMeta.statusKey)}
          </StudioStatusChip>
        </div>
          <div className="mt-3 grid gap-2 text-sm">
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
        </div>

        <Collapsible
          label={t("app.boundariesLabel")}
          defaultOpen={false}
        >
          <div className="grid gap-2 text-sm text-ink/70">
            <p>{t("app.boundariesLocalOnly")}</p>
            <p>{t("app.boundariesNoExternalAgent")}</p>
            <p>{t("app.boundariesNoUpload")}</p>
            <p>{t("app.boundariesFolderWins")}</p>
          </div>
        </Collapsible>
      </div>
    );
  }

  const navItems: StudioNavItem[] = studioSections.map((section) => ({
    id: section.id,
    label: t(section.labelKey),
    sublabel: t(section.descriptionKey),
    description: t(section.descriptionKey),
    icon: section.icon,
    selected: section.id === activeSection,
    onSelect: () => openStudioSection(section.id),
  }));

  const paletteActions: PaletteAction[] = [
    ...studioSections.map((section) => ({
      id: `nav-${section.id}`,
      label: t(section.labelKey),
      run: () => openStudioSection(section.id),
    })),
    {
      id: "run-proof",
      label: t("palette.runProof"),
      run: () => void runPlaytest(),
    },
    {
      id: "toggle-rail",
      label: t("palette.toggleRail"),
      run: rail.toggleRail,
    },
    {
      id: "refresh",
      label: t("palette.refresh"),
      run: () => void loadProject(projectPath),
    },
  ];

  return (
    <>
    <StudioShell
      projectPath={loadedPath}
      projectLoading={loading}
      onOpenProject={() => void loadProject(projectPath)}
      navItems={navItems}
      expandedIds={new Set()}
      onToggleExpand={() => {}}
      header={{
        title: t(activeSectionMeta.labelKey),
        subtitle: `${projectSummary?.title ?? t("app.noProjectLoaded")} - ${t(activeSectionMeta.descriptionKey)}`,
      }}
      topActions={
        <>
          <LanguageToggle />
          <StudioButton
            title={t("palette.kbdHint")}
            aria-label={t("palette.kbdHint")}
            onClick={() => setPaletteOpen(true)}
          >
            <Command aria-hidden size={18} />
          </StudioButton>
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
      rightPanel={renderAgentRail()}
      railCollapsed={rail.railCollapsed}
      onToggleRail={rail.toggleRail}
      sidebarCollapsed={rail.sidebarCollapsed}
      onToggleSidebar={rail.toggleSidebar}
      drawerOpen={drawerOpen}
      onToggleDrawer={() => setDrawerOpen((prev) => !prev)}
      onCloseDrawer={() => setDrawerOpen(false)}
    >
      {renderActiveSection()}
    </StudioShell>
    <StudioCommandPalette
      open={paletteOpen}
      onClose={() => setPaletteOpen(false)}
      actions={paletteActions}
      onOpenProject={(path) => {
        setProjectPath(path);
        void loadProject(path);
      }}
    />
    </>
  );
}

// ---------------------------------------------------------------------------
// Inline helpers
// ---------------------------------------------------------------------------

/** Derive the project directory basename for the home page chip. */
function deriveProjectDirName(loadedPath: string): string {
  if (!loadedPath) return "";
  const trimmed = loadedPath.replace(/\/+$/, "");
  const parts = trimmed.split("/");
  return parts[parts.length - 1] ?? "";
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
