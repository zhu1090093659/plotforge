import {
  CheckCircle2,
  Download,
  Loader2,
  PlusCircle,
  Play,
  RefreshCcw,
  Save,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useState } from "react";
import type {
  AiSafetyPolicy,
  AssetRecord,
  AiUsageContentKind,
  AudioVoiceCard,
  Character,
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectTemplateId,
  ResourceDefinition,
  Rule,
  StoryCraftEditDocument,
  VisualStyleCard,
  WorldEditDocument,
} from "../../../contracts/plotforge";
import { AgentMeshView } from "./AgentMeshView";
import { ArtifactReviewView } from "./ArtifactReviewView";
import { CommandCenterView } from "./CommandCenterView";
import { DirectorModeView } from "./DirectorModeView";
import {
  defaultLocalPreviewState,
  localPreviewSummary,
  type LocalPreviewState,
} from "./localPreviewModel";
import { RuntimeTracePanel } from "./runtimeTraceView";
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
  type AssetCatalogItem,
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

const boundaryChecks = [
  { label: "Generated contracts", value: "plotforge.d.ts", ok: true },
  { label: "Rust core boundary", value: "UI adapter only", ok: true },
  { label: "Tauri bridge", value: "commands wired", ok: true },
];

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};

type EvidenceStatus = "pass" | "pending" | "review";

type CharacterDraft = Omit<Character, "traits"> & { traits_text: string };

interface ResourceDraft {
  key: string;
  label: string;
  initial: number;
  min: number;
  max: number;
}

interface RuleDraft {
  id: string;
  action_type: string;
  resource_key: string;
  amount: number;
}

export interface AppProps {
  dataSource?: StudioDataSource;
  initialProjectPath?: string;
  localPreviewState?: LocalPreviewState;
}

export function App({
  dataSource = createDefaultStudioDataSource(),
  initialProjectPath = defaultProjectPath(),
  localPreviewState = defaultLocalPreviewState,
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
    setProjectData,
    visualBible,
    setVisualBible,
    audioBible,
    setAudioBible,
    projectSummary,
    sourceFiles,
    selectedFile,
    editorContent,
    setEditorContent,
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
    loadProject: loadWorkspaceProject,
    refreshProjectOverview,
    selectSourceFile,
    saveSelectedFile,
    runPlaytest: runWorkspacePlaytest,
    runStaticZipExport,
    selectExportProfile,
  } = useStudioWorkspace({ dataSource, initialProjectPath });
  const [newCharacter, setNewCharacter] = useState<CharacterDraft>(
    emptyCharacterDraft(),
  );
  const [newResource, setNewResource] = useState<ResourceDraft>(
    emptyResourceDraft(),
  );
  const [newRule, setNewRule] = useState<RuleDraft>(emptyRuleDraft());
  const [worldExpansionGoal, setWorldExpansionGoal] = useState(
    "Expand canon, factions, and forbidden facts for the next playable arc.",
  );
  const [storyGenerationConcept, setStoryGenerationConcept] = useState(
    "Generate a three-thread pressure arc from the current World Bible.",
  );
  const [characterGenerationConcept, setCharacterGenerationConcept] = useState(
    "Design a pressure-bearing character with visible story costs.",
  );
  const [characterGenerationRoleHint, setCharacterGenerationRoleHint] =
    useState("Generated Envoy");
  const [formStatus, setFormStatus] = useState<FormStatus | null>(null);
  const [formSaving, setFormSaving] = useState<StudioSectionId | null>(null);
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
  const activeWorkflowSections = activeWorkflowMeta.sectionIds.map((sectionId) =>
    getStudioSection(sectionId),
  );
  const previewSummary = localPreviewSummary(localPreviewState);

  useEffect(() => {
    setCreateProjectPath(defaultNewProjectPath(initialProjectPath));
  }, [initialProjectPath]);

  async function loadProject(path: string) {
    setFormStatus(null);
    await loadWorkspaceProject(path);
  }

  async function createProjectFromWizard() {
    const path = createProjectPath.trim();
    const request: ProjectCreationRequest = {
      template: createTemplate,
      concept: createConcept.trim(),
      visual_style: createVisualStyle.trim(),
      voice_enabled: createVoiceEnabled,
      initial_scene_request: createInitialSceneRequest.trim(),
    };

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
      const report = await dataSource.createProject(path, request, createForce);
      setCreateReport(report);
      setCreateProjectPath(report.project_path);
      await loadProject(report.project_path);
    } catch (source) {
      setCreateError(source instanceof Error ? source.message : String(source));
    } finally {
      setCreating(false);
    }
  }

  async function runPlaytest() {
    const succeeded = await runWorkspacePlaytest();
    if (succeeded) {
      setActiveWorkflow("proof");
      setActiveSection("debugger");
    } else {
      setActiveWorkflow("game");
      setActiveSection("playtest");
    }
  }

  async function runFormAction(
    section: StudioSectionId,
    successMessage: string,
    action: () => Promise<FormStatus | void>,
  ) {
    setFormSaving(section);
    setFormStatus(null);
    try {
      const status = await action();
      setFormStatus(
        status ?? {
          section,
          tone: "success",
          message: successMessage,
        },
      );
    } catch (source) {
      setFormStatus({
        section,
        tone: "error",
        message: source instanceof Error ? source.message : String(source),
      });
    } finally {
      setFormSaving(null);
    }
  }

  async function saveWorldEditDocument() {
    if (!worldEditDocument) {
      return;
    }
    await runFormAction("world", "World Bible saved.", async () => {
      const updated = await dataSource.updateWorldEditDocument(
        loadedPath,
        worldEditDocument,
      );
      setWorldEditDocument(updated);
    });
  }

  async function generateWorldExpansionFromGoal() {
    const goal = worldExpansionGoal.trim();
    if (!goal) {
      setFormStatus({
        section: "world",
        tone: "error",
        message: "World expansion goal is required.",
      });
      return;
    }

    await runFormAction("world", "World generation applied.", async () => {
      const report = await dataSource.generateWorldExpansion(loadedPath, goal);
      setWorldEditDocument(report.document);
      await refreshProjectOverview(loadedPath);
      return {
        section: "world",
        tone: report.evidence.fallback_used ? "error" : "success",
        message: `World generation ${report.evidence.status}${report.evidence.fallback_used ? " with visible fallback" : ""}.`,
      };
    });
  }

  async function saveStoryCraftEditDocument() {
    if (!storyCraftEditDocument) {
      return;
    }
    await runFormAction("story", "Story Craft saved.", async () => {
      const updated = await dataSource.updateStoryCraftEditDocument(
        loadedPath,
        storyCraftEditDocument,
      );
      setStoryCraftEditDocument(updated);
      await refreshProjectOverview(loadedPath);
    });
  }

  async function generateStoryCraftFromConcept() {
    const concept = storyGenerationConcept.trim();
    if (!concept) {
      setFormStatus({
        section: "story",
        tone: "error",
        message: "Story generation concept is required.",
      });
      return;
    }

    await runFormAction("story", "Story Craft generation applied.", async () => {
      const report = await dataSource.generateStoryCraft(loadedPath, concept);
      setStoryCraftEditDocument(report.document);
      await refreshProjectOverview(loadedPath);
      return {
        section: "story",
        tone: report.evidence.fallback_used ? "error" : "success",
        message: `Story Craft generation ${report.evidence.status}${report.evidence.fallback_used ? " with visible fallback" : ""}.`,
      };
    });
  }

  async function saveCharacterEditDocument() {
    if (!characterEditDocument) {
      return;
    }
    await runFormAction("characters", "Characters saved.", async () => {
      const updated = await dataSource.updateCharacterEditDocument(
        loadedPath,
        characterEditDocument,
      );
      setCharacterEditDocument(updated);
      await refreshProjectOverview(loadedPath);
    });
  }

  async function createCharacterFromDraft() {
    const character: Character = {
      id: newCharacter.id.trim(),
      name: newCharacter.name.trim(),
      role: newCharacter.role.trim(),
      traits: linesToList(newCharacter.traits_text),
      visual_card: newCharacter.visual_card.trim(),
      voice_card: newCharacter.voice_card.trim(),
      portrait_request: newCharacter.portrait_request ?? null,
    };

    await runFormAction("characters", "Character created.", async () => {
      const updated = await dataSource.createCharacter(loadedPath, character);
      setCharacterEditDocument(updated);
      setNewCharacter(emptyCharacterDraft());
      await refreshProjectOverview(loadedPath);
    });
  }

  async function generateCharacterFromConcept() {
    const concept = characterGenerationConcept.trim();
    const roleHint = characterGenerationRoleHint.trim();
    if (!concept || !roleHint) {
      setFormStatus({
        section: "characters",
        tone: "error",
        message: "Character generation concept and role hint are required.",
      });
      return;
    }

    await runFormAction("characters", "Character generation applied.", async () => {
      const report = await dataSource.generateCharacter(
        loadedPath,
        concept,
        roleHint,
      );
      setCharacterEditDocument((current) => ({
        characters: [
          ...(current?.characters.filter(
            (character) => character.id !== report.character.id,
          ) ?? []),
          report.character,
        ],
      }));
      await refreshProjectOverview(loadedPath);
      return {
        section: "characters",
        tone: report.evidence.fallback_used ? "error" : "success",
        message: `Character generation ${report.evidence.status}${report.evidence.fallback_used ? " with visible fallback" : ""}.`,
      };
    });
  }

  async function saveStateVariablesEditDocument() {
    if (!stateVariablesEditDocument) {
      return;
    }
    await runFormAction("state", "State variables saved.", async () => {
      const updated = await dataSource.updateStateVariablesEditDocument(
        loadedPath,
        stateVariablesEditDocument,
      );
      setStateVariablesEditDocument(updated);
      await refreshProjectOverview(loadedPath);
    });
  }

  async function createResourceFromDraft() {
    const resource: ResourceDefinition = {
      key: newResource.key.trim(),
      label: newResource.label.trim(),
      initial: newResource.initial,
      min: newResource.min,
      max: newResource.max,
    };

    await runFormAction("state", "Resource created.", async () => {
      const updated = await dataSource.createResource(loadedPath, resource);
      setStateVariablesEditDocument(updated);
      setNewResource(emptyResourceDraft());
      await refreshProjectOverview(loadedPath);
    });
  }

  async function saveRulesEditDocument() {
    if (!rulesEditDocument) {
      return;
    }
    await runFormAction("rules", "Rules saved.", async () => {
      const updated = await dataSource.updateRulesEditDocument(
        loadedPath,
        rulesEditDocument,
      );
      setRulesEditDocument(updated);
      await refreshProjectOverview(loadedPath);
    });
  }

  async function createRuleFromDraft() {
    const rule: Rule = {
      id: newRule.id.trim(),
      action_type: newRule.action_type.trim(),
      conditions: [],
      effects: [
        {
          kind: "add_resource",
          key:
            newRule.resource_key ||
            stateVariablesEditDocument?.resources[0]?.key ||
            "",
          amount: newRule.amount,
        },
      ],
    };

    await runFormAction("rules", "Rule created.", async () => {
      const updated = await dataSource.createRule(loadedPath, rule);
      setRulesEditDocument(updated);
      setNewRule(emptyRuleDraft());
      await refreshProjectOverview(loadedPath);
    });
  }

  async function saveAiSafetyPolicy() {
    if (!aiSafetyPolicy) {
      return;
    }
    await runFormAction("export-kit", "AI safety policy saved.", async () => {
      const updated = await dataSource.updateAiSafetyPolicy(
        loadedPath,
        aiSafetyPolicy,
      );
      setAiSafetyPolicy(updated);
      setProjectData((current) =>
        current ? { ...current, ai_safety_policy: updated } : current,
      );
    });
  }

  async function saveVisualBible() {
    if (!visualBible) {
      return;
    }
    await runFormAction("assets", "Visual Bible saved.", async () => {
      const updated = await dataSource.updateVisualBible(loadedPath, visualBible);
      setVisualBible(updated);
      setProjectData((current) =>
        current ? { ...current, visual_bible: updated } : current,
      );
    });
  }

  async function saveAudioBible() {
    if (!audioBible) {
      return;
    }
    await runFormAction("assets", "Audio Bible saved.", async () => {
      const updated = await dataSource.updateAudioBible(loadedPath, audioBible);
      setAudioBible(updated);
      setProjectData((current) =>
        current ? { ...current, audio_bible: updated } : current,
      );
    });
  }

  function updateWorldDocument(patch: Partial<WorldEditDocument>) {
    if (!worldEditDocument) {
      return;
    }
    setWorldEditDocument({ ...worldEditDocument, ...patch });
  }

  function updateStoryBible(
    patch: Partial<StoryCraftEditDocument["story_craft"]["bible"]>,
  ) {
    if (!storyCraftEditDocument) {
      return;
    }
    setStoryCraftEditDocument({
      ...storyCraftEditDocument,
      story_craft: {
        ...storyCraftEditDocument.story_craft,
        bible: {
          ...storyCraftEditDocument.story_craft.bible,
          ...patch,
        },
      },
    });
  }

  function updateCharacter(index: number, patch: Partial<Character>) {
    if (!characterEditDocument) {
      return;
    }
    setCharacterEditDocument({
      characters: characterEditDocument.characters.map((character, current) =>
        current === index ? { ...character, ...patch } : character,
      ),
    });
  }

  function updateResource(index: number, patch: Partial<ResourceDefinition>) {
    if (!stateVariablesEditDocument) {
      return;
    }
    setStateVariablesEditDocument({
      ...stateVariablesEditDocument,
      resources: stateVariablesEditDocument.resources.map((resource, current) =>
        current === index ? { ...resource, ...patch } : resource,
      ),
    });
  }

  function updateInitialWorldResource(key: string, value: number) {
    if (!stateVariablesEditDocument) {
      return;
    }
    setStateVariablesEditDocument({
      ...stateVariablesEditDocument,
      initial_world_state: {
        ...stateVariablesEditDocument.initial_world_state,
        resources: {
          ...stateVariablesEditDocument.initial_world_state.resources,
          [key]: value,
        },
      },
    });
  }

  function updateRule(index: number, patch: Partial<Rule>) {
    if (!rulesEditDocument) {
      return;
    }
    setRulesEditDocument({
      rules: rulesEditDocument.rules.map((rule, current) =>
        current === index ? { ...rule, ...patch } : rule,
      ),
    });
  }

  function updateAiSafetyPolicy(patch: Partial<AiSafetyPolicy>) {
    if (!aiSafetyPolicy) {
      return;
    }
    setAiSafetyPolicy({ ...aiSafetyPolicy, ...patch });
  }

  function updateVisualStyleCard(
    index: number,
    patch: Partial<VisualStyleCard>,
  ) {
    if (!visualBible) {
      return;
    }
    setVisualBible({
      style_cards: visualBible.style_cards.map((card, current) =>
        current === index ? { ...card, ...patch } : card,
      ),
    });
  }

  function updateAudioVoiceCard(index: number, patch: Partial<AudioVoiceCard>) {
    if (!audioBible) {
      return;
    }
    setAudioBible({
      voice_cards: audioBible.voice_cards.map((card, current) =>
        current === index ? { ...card, ...patch } : card,
      ),
    });
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
    selectExportProfile(profileId);
    openStudioSection("export-kit");
  }

  function renderActiveSection() {
    switch (activeSection) {
      case "agent-mesh":
        return (
          <AgentMeshView
            projectSummary={projectSummary}
            loadedPath={loadedPath}
            localPreviewState={localPreviewState}
            playtestReport={playtestReport}
            onOpenTrace={() => openStudioSection("debugger")}
            onRunPlayableProof={() => void runPlaytest()}
          />
        );
      case "world":
        return renderWorldPanel();
      case "story":
        return renderStoryPanel();
      case "characters":
        return renderCharactersPanel();
      case "state":
        return renderStatePanel();
      case "rules":
        return renderRulesPanel();
      case "assets":
        return (
          <ArtifactReviewView
            projectSummary={projectSummary}
            loadedPath={loadedPath}
            assetCatalog={assetCatalog}
            localPreviewState={localPreviewState}
            playtestReport={playtestReport}
            playtesting={playtesting}
            playtestError={playtestError}
            onOpenTrace={() => openStudioSection("debugger")}
            onRunPlayableProof={() => void runPlaytest()}
            assetMaintenance={renderAssetMaintenancePanel()}
          />
        );
      case "playtest":
        return (
          <DirectorModeView
            projectData={projectData}
            loadedPath={loadedPath}
            input={playtestInput}
            running={playtesting}
            report={playtestReport}
            error={playtestError}
            saveId={playtestSaveId}
            restoreId={playtestRestoreId}
            restoreLatest={playtestRestoreLatest}
            localPreviewState={localPreviewState}
            onInputChange={setPlaytestInput}
            onSaveIdChange={setPlaytestSaveId}
            onRestoreIdChange={setPlaytestRestoreId}
            onRestoreLatestChange={setPlaytestRestoreLatest}
            onRun={() => void runPlaytest()}
            onOpenStory={() => openStudioSection("story")}
            onOpenTrace={() => openStudioSection("debugger")}
          />
        );
      case "debugger":
        return (
          <RuntimeTracePanel
            report={playtestReport}
            error={playtestError}
            selectedExportProfile={selectedExportProfile}
            exportReport={exportReport}
            aiSafetyPolicy={aiSafetyPolicy}
            localPreviewState={localPreviewState}
            loadedPath={loadedPath}
            projectId={projectData?.game.id ?? null}
          />
        );
      case "export-kit":
        return renderExportPanel();
      case "launchpad":
      default:
        return renderLaunchpad();
    }
  }

  function renderLaunchpad() {
    return (
      <div className="grid gap-5">
        <CommandCenterView
          projectSummary={projectSummary}
          projectData={projectData}
          loadedPath={loadedPath}
          metrics={metrics}
          sourceFiles={sourceFiles}
          selectedFile={selectedFile}
          playtestInput={playtestInput}
          playtesting={playtesting}
          playtestReport={playtestReport}
          playtestError={playtestError}
          exportProfiles={exportProfiles}
          localPreviewState={localPreviewState}
          dirty={dirty}
          onIntentChange={setPlaytestInput}
          onRunPlayableProof={() => void runPlaytest()}
          onOpenSection={openStudioSection}
          onOpenExportProfile={openExportProfile}
        />

        <section className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
          {renderProjectCreationPanel()}
          {renderCreationReportPanel()}
        </section>

        <section className="grid gap-5 xl:grid-cols-[1.2fr_0.8fr]">
          {renderSourceFileList()}
          {renderBoundaryChecks()}
        </section>

        {renderSourceEditor()}
      </div>
    );
  }

  function renderProjectCreationPanel() {
    return (
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void createProjectFromWizard();
        }}
        className="rounded-md border border-ink/10 bg-white p-5 shadow-sm"
      >
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <h3 className="text-lg font-semibold">New Project</h3>
            <p className="mt-1 truncate text-sm text-ink/55">
              Folder-backed project scaffold
            </p>
          </div>
          <button
            type="submit"
            disabled={creating}
            className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
          >
            {creating ? (
              <Loader2 aria-hidden size={16} className="animate-spin" />
            ) : (
              <PlusCircle aria-hidden size={16} />
            )}
            Create project
          </button>
        </div>

        <div className="mt-4 grid gap-3 lg:grid-cols-2">
          <TextInput
            label="Project path"
            ariaLabel="New project path"
            value={createProjectPath}
            onChange={setCreateProjectPath}
            className="lg:col-span-2"
          />
          <label className="grid gap-1">
            <FieldLabel>Template</FieldLabel>
            <select
              aria-label="Template"
              value={createTemplate}
              onChange={(event) =>
                setCreateTemplate(event.target.value as ProjectTemplateId)
              }
              className={inputClassName}
            >
              <option value="historical_crisis">Historical Crisis</option>
              <option value="dynasty_embers">Dynasty Embers</option>
            </select>
          </label>
          <TextInput
            label="Visual style"
            ariaLabel="Visual style"
            value={createVisualStyle}
            onChange={setCreateVisualStyle}
          />
          <TextareaInput
            label="Concept"
            ariaLabel="Concept"
            value={createConcept}
            onChange={setCreateConcept}
            className="lg:col-span-2"
            minHeight="min-h-20"
          />
          <TextareaInput
            label="Initial scene"
            ariaLabel="Initial scene request"
            value={createInitialSceneRequest}
            onChange={setCreateInitialSceneRequest}
            className="lg:col-span-2"
            minHeight="min-h-20"
          />
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-4 text-sm font-medium text-ink/70">
          <CheckboxInput
            label="Voice enabled"
            checked={createVoiceEnabled}
            onChange={setCreateVoiceEnabled}
          />
          <CheckboxInput
            label="Overwrite existing path"
            checked={createForce}
            onChange={setCreateForce}
          />
        </div>

        {createError ? (
          <Message tone="error" className="mt-4">
            {createError}
          </Message>
        ) : null}
      </form>
    );
  }

  function renderCreationReportPanel() {
    return (
      <section className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
        <h3 className="text-lg font-semibold">Creation Report</h3>
        {createReport ? (
          <div className="mt-4 grid gap-3 text-sm">
            <div className="rounded-md border border-jade/25 bg-jade/10 px-3 py-2">
              <p className="text-xs font-medium uppercase text-jade">Project</p>
              <p className="mt-1 truncate font-semibold text-ink">
                {createReport.project.game.title}
              </p>
              <p className="mt-1 truncate text-ink/60">
                {createReport.project_path}
              </p>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <MetricBox label="Files" value={createReport.files_created.length} />
              <MetricBox label="Template" value={createReport.template} />
            </div>
          </div>
        ) : (
          <p className="mt-4 text-sm text-ink/55">
            No project created in this session.
          </p>
        )}
      </section>
    );
  }

  function renderWorldPanel() {
    return (
      <section className={panelClassName}>
        <PanelHeader
          title="World Bible"
          subtitle={`${worldEditDocument?.forbidden_facts.length ?? 0} forbidden facts`}
          action={
            <SaveButton
              label="Save World Bible"
              saving={formSaving === "world"}
              onClick={() => void saveWorldEditDocument()}
            />
          }
        />
        <SectionMessage section="world" status={formStatus} />
        {worldEditDocument ? (
          <div className="mt-4 grid gap-4 xl:grid-cols-2">
            <TextareaInput
              label="World Bible"
              ariaLabel="World bible markdown"
              value={worldEditDocument.world_bible_markdown}
              onChange={(value) =>
                updateWorldDocument({ world_bible_markdown: value })
              }
              minHeight="min-h-80"
            />
            <div className="grid gap-4">
              <TextareaInput
                label="Canon"
                ariaLabel="Canon markdown"
                value={worldEditDocument.canon_markdown}
                onChange={(value) =>
                  updateWorldDocument({ canon_markdown: value })
                }
                minHeight="min-h-36"
              />
              <TextareaInput
                label="Forbidden facts"
                ariaLabel="Forbidden facts"
                value={listToLines(worldEditDocument.forbidden_facts)}
                onChange={(value) =>
                  updateWorldDocument({ forbidden_facts: linesToList(value) })
                }
                minHeight="min-h-36"
              />
            </div>
            <div className="xl:col-span-2 rounded-md border border-ink/10 bg-white p-4">
              <div className="flex flex-wrap items-end gap-3">
                <TextInput
                  label="AI expansion goal"
                  ariaLabel="World generation goal"
                  value={worldExpansionGoal}
                  onChange={setWorldExpansionGoal}
                  className="min-w-0 flex-1"
                />
                <button
                  type="button"
                  onClick={() => void generateWorldExpansionFromGoal()}
                  disabled={formSaving === "world"}
                  className={secondaryButtonClassName}
                >
                  Generate World Expansion
                </button>
              </div>
            </div>
          </div>
        ) : (
          <EmptyPanel label="World edit document not loaded." />
        )}
      </section>
    );
  }

  function renderStoryPanel() {
    const bible = storyCraftEditDocument?.story_craft.bible;

    return (
      <section className={panelClassName}>
        <PanelHeader
          title="Story Craft"
          subtitle={`${storyCraftEditDocument?.story_craft.plot_threads.length ?? 0} plot threads`}
          action={
            <SaveButton
              label="Save Story Craft"
              saving={formSaving === "story"}
              onClick={() => void saveStoryCraftEditDocument()}
            />
          }
        />
        <SectionMessage section="story" status={formStatus} />
        {storyCraftEditDocument && bible ? (
          <div className="mt-4 grid gap-4">
            <div className="rounded-md border border-ink/10 bg-white p-4">
              <div className="flex flex-wrap items-end gap-3">
                <TextareaInput
                  label="AI story concept"
                  ariaLabel="Story generation concept"
                  value={storyGenerationConcept}
                  onChange={setStoryGenerationConcept}
                  className="min-w-0 flex-1"
                  minHeight="min-h-20"
                />
                <button
                  type="button"
                  onClick={() => void generateStoryCraftFromConcept()}
                  disabled={formSaving === "story"}
                  className={secondaryButtonClassName}
                >
                  Generate StoryCraft
                </button>
              </div>
            </div>
            <div className="grid gap-4 xl:grid-cols-2">
              <TextareaInput
                label="Story Bible"
                ariaLabel="Story bible markdown"
                value={storyCraftEditDocument.story_bible_markdown}
                onChange={(value) =>
                  setStoryCraftEditDocument({
                    ...storyCraftEditDocument,
                    story_bible_markdown: value,
                  })
                }
                minHeight="min-h-52"
              />
              <TextareaInput
                label="Style Guide"
                ariaLabel="Style guide markdown"
                value={storyCraftEditDocument.style_guide_markdown}
                onChange={(value) =>
                  setStoryCraftEditDocument({
                    ...storyCraftEditDocument,
                    style_guide_markdown: value,
                  })
                }
                minHeight="min-h-52"
              />
            </div>
            <div className="grid gap-4 xl:grid-cols-2">
              <TextInput
                label="Genre promise"
                ariaLabel="Genre promise"
                value={bible.genre_promise}
                onChange={(value) => updateStoryBible({ genre_promise: value })}
              />
              <TextInput
                label="Central question"
                ariaLabel="Central question"
                value={bible.central_question}
                onChange={(value) =>
                  updateStoryBible({ central_question: value })
                }
              />
              <TextareaInput
                label="Target emotions"
                ariaLabel="Target emotions"
                value={listToLines(bible.target_emotions)}
                onChange={(value) =>
                  updateStoryBible({ target_emotions: linesToList(value) })
                }
                minHeight="min-h-32"
              />
              <TextareaInput
                label="Core foreshadowing"
                ariaLabel="Core foreshadowing"
                value={listToLines(bible.core_foreshadowing)}
                onChange={(value) =>
                  updateStoryBible({ core_foreshadowing: linesToList(value) })
                }
                minHeight="min-h-32"
              />
            </div>
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {storyCraftEditDocument.story_craft.plot_threads.map((thread) => (
                <article
                  key={thread.id}
                  className="rounded-md border border-ink/10 bg-parchment px-3 py-3"
                >
                  <p className="text-sm font-semibold">{thread.title}</p>
                  <p className="mt-1 text-xs font-medium uppercase text-ink/45">
                    {thread.status}
                  </p>
                  <p className="mt-2 text-sm text-ink/65">{thread.promise}</p>
                </article>
              ))}
            </div>
          </div>
        ) : (
          <EmptyPanel label="Story Craft edit document not loaded." />
        )}
      </section>
    );
  }

  function renderCharactersPanel() {
    return (
      <section className={panelClassName}>
        <PanelHeader
          title="Characters"
          subtitle={`${characterEditDocument?.characters.length ?? 0} records`}
          action={
            <SaveButton
              label="Save Characters"
              saving={formSaving === "characters"}
              onClick={() => void saveCharacterEditDocument()}
            />
          }
        />
        <SectionMessage section="characters" status={formStatus} />
        {characterEditDocument ? (
          <div className="mt-4 grid gap-4">
            <div className="grid gap-4 xl:grid-cols-2">
              {characterEditDocument.characters.map((character, index) => (
                <article
                  key={`${character.id}:${index}`}
                  className="rounded-md border border-ink/10 bg-parchment p-4"
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    <TextInput
                      label="Character id"
                      ariaLabel={`Character id ${index + 1}`}
                      value={character.id}
                      onChange={(value) => updateCharacter(index, { id: value })}
                    />
                    <TextInput
                      label="Name"
                      ariaLabel={`Character name ${index + 1}`}
                      value={character.name}
                      onChange={(value) =>
                        updateCharacter(index, { name: value })
                      }
                    />
                    <TextInput
                      label="Role"
                      ariaLabel={`Character role ${index + 1}`}
                      value={character.role}
                      onChange={(value) =>
                        updateCharacter(index, { role: value })
                      }
                    />
                    <TextareaInput
                      label="Traits"
                      ariaLabel={`Character traits ${index + 1}`}
                      value={listToLines(character.traits)}
                      onChange={(value) =>
                        updateCharacter(index, { traits: linesToList(value) })
                      }
                      minHeight="min-h-24"
                    />
                    <TextareaInput
                      label="Visual card"
                      ariaLabel={`Visual card ${index + 1}`}
                      value={character.visual_card}
                      onChange={(value) =>
                        updateCharacter(index, { visual_card: value })
                      }
                      minHeight="min-h-24"
                    />
                    <TextareaInput
                      label="Voice card"
                      ariaLabel={`Voice card ${index + 1}`}
                      value={character.voice_card}
                      onChange={(value) =>
                        updateCharacter(index, { voice_card: value })
                      }
                      minHeight="min-h-24"
                    />
                    {character.portrait_request ? (
                      <div className="sm:col-span-2 rounded-md border border-ink/10 bg-white px-3 py-2 text-sm">
                        <p className="text-xs font-medium uppercase text-ink/55">
                          Portrait request
                        </p>
                        <p className="mt-1 text-ink/70">
                          {character.portrait_request.prompt_summary}
                        </p>
                        <code className="mt-2 block truncate text-xs text-ink/55">
                          {character.portrait_request.prompt_hash}
                        </code>
                      </div>
                    ) : null}
                  </div>
                </article>
              ))}
            </div>
            <div className="rounded-md border border-ink/10 bg-white p-4">
              <div className="flex flex-wrap items-end gap-3">
                <TextareaInput
                  label="AI character concept"
                  ariaLabel="Character generation concept"
                  value={characterGenerationConcept}
                  onChange={setCharacterGenerationConcept}
                  className="min-w-0 flex-[2]"
                  minHeight="min-h-20"
                />
                <TextInput
                  label="Role hint"
                  ariaLabel="Character generation role hint"
                  value={characterGenerationRoleHint}
                  onChange={setCharacterGenerationRoleHint}
                  className="min-w-56"
                />
                <button
                  type="button"
                  onClick={() => void generateCharacterFromConcept()}
                  disabled={formSaving === "characters"}
                  className={secondaryButtonClassName}
                >
                  Generate Character
                </button>
              </div>
            </div>
            <div className="rounded-md border border-ink/10 bg-white p-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h4 className="text-sm font-semibold uppercase text-ink/55">
                  New Character
                </h4>
                <button
                  type="button"
                  onClick={() => void createCharacterFromDraft()}
                  disabled={formSaving === "characters"}
                  className={secondaryButtonClassName}
                >
                  Create Character
                </button>
              </div>
              <div className="mt-3 grid gap-3 lg:grid-cols-2">
                <TextInput
                  label="Character id"
                  ariaLabel="New character id"
                  value={newCharacter.id}
                  onChange={(value) =>
                    setNewCharacter({ ...newCharacter, id: value })
                  }
                />
                <TextInput
                  label="Name"
                  ariaLabel="New character name"
                  value={newCharacter.name}
                  onChange={(value) =>
                    setNewCharacter({ ...newCharacter, name: value })
                  }
                />
                <TextInput
                  label="Role"
                  ariaLabel="New character role"
                  value={newCharacter.role}
                  onChange={(value) =>
                    setNewCharacter({ ...newCharacter, role: value })
                  }
                />
                <TextareaInput
                  label="Traits"
                  ariaLabel="New character traits"
                  value={newCharacter.traits_text}
                  onChange={(value) =>
                    setNewCharacter({ ...newCharacter, traits_text: value })
                  }
                  minHeight="min-h-24"
                />
                <TextareaInput
                  label="Visual card"
                  ariaLabel="New visual card"
                  value={newCharacter.visual_card}
                  onChange={(value) =>
                    setNewCharacter({ ...newCharacter, visual_card: value })
                  }
                  minHeight="min-h-24"
                />
                <TextareaInput
                  label="Voice card"
                  ariaLabel="New voice card"
                  value={newCharacter.voice_card}
                  onChange={(value) =>
                    setNewCharacter({ ...newCharacter, voice_card: value })
                  }
                  minHeight="min-h-24"
                />
              </div>
            </div>
          </div>
        ) : (
          <EmptyPanel label="Character edit document not loaded." />
        )}
      </section>
    );
  }

  function renderStatePanel() {
    return (
      <section className={panelClassName}>
        <PanelHeader
          title="State"
          subtitle={`${stateVariablesEditDocument?.resources.length ?? 0} resources`}
          action={
            <SaveButton
              label="Save State"
              saving={formSaving === "state"}
              onClick={() => void saveStateVariablesEditDocument()}
            />
          }
        />
        <SectionMessage section="state" status={formStatus} />
        {stateVariablesEditDocument ? (
          <div className="mt-4 grid gap-4">
            <div className="grid gap-4 xl:grid-cols-2">
              {stateVariablesEditDocument.resources.map((resource, index) => (
                <article
                  key={`${resource.key}:${index}`}
                  className="rounded-md border border-ink/10 bg-parchment p-4"
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    <TextInput
                      label="Resource key"
                      ariaLabel={`Resource key ${index + 1}`}
                      value={resource.key}
                      onChange={(value) => updateResource(index, { key: value })}
                    />
                    <TextInput
                      label="Label"
                      ariaLabel={`Resource label ${index + 1}`}
                      value={resource.label}
                      onChange={(value) =>
                        updateResource(index, { label: value })
                      }
                    />
                    <NumberInput
                      label="Initial"
                      ariaLabel={`Resource initial ${index + 1}`}
                      value={resource.initial}
                      onChange={(value) =>
                        updateResource(index, { initial: value })
                      }
                    />
                    <NumberInput
                      label="Initial world value"
                      ariaLabel={`Initial world value ${index + 1}`}
                      value={
                        stateVariablesEditDocument.initial_world_state.resources[
                          resource.key
                        ] ?? resource.initial
                      }
                      onChange={(value) =>
                        updateInitialWorldResource(resource.key, value)
                      }
                    />
                    <NumberInput
                      label="Min"
                      ariaLabel={`Resource min ${index + 1}`}
                      value={resource.min}
                      onChange={(value) => updateResource(index, { min: value })}
                    />
                    <NumberInput
                      label="Max"
                      ariaLabel={`Resource max ${index + 1}`}
                      value={resource.max}
                      onChange={(value) => updateResource(index, { max: value })}
                    />
                  </div>
                </article>
              ))}
            </div>
            <div className="grid gap-4 xl:grid-cols-[1fr_1fr]">
              <div className="rounded-md border border-ink/10 bg-white p-4">
                <h4 className="text-sm font-semibold uppercase text-ink/55">
                  Initial Story State
                </h4>
                <div className="mt-3 grid gap-3 sm:grid-cols-2">
                  <TextInput
                    label="Current scene"
                    ariaLabel="Initial current scene"
                    value={
                      stateVariablesEditDocument.initial_story_state
                        .current_scene_key
                    }
                    onChange={(value) =>
                      setStateVariablesEditDocument({
                        ...stateVariablesEditDocument,
                        initial_story_state: {
                          ...stateVariablesEditDocument.initial_story_state,
                          current_scene_key: value,
                        },
                      })
                    }
                  />
                  <NumberInput
                    label="Turn"
                    ariaLabel="Initial turn"
                    value={stateVariablesEditDocument.initial_story_state.turn}
                    onChange={(value) =>
                      setStateVariablesEditDocument({
                        ...stateVariablesEditDocument,
                        initial_story_state: {
                          ...stateVariablesEditDocument.initial_story_state,
                          turn: value,
                        },
                      })
                    }
                  />
                </div>
              </div>
              <div className="rounded-md border border-ink/10 bg-white p-4">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <h4 className="text-sm font-semibold uppercase text-ink/55">
                    New Resource
                  </h4>
                  <button
                    type="button"
                    onClick={() => void createResourceFromDraft()}
                    disabled={formSaving === "state"}
                    className={secondaryButtonClassName}
                  >
                    Create Resource
                  </button>
                </div>
                <div className="mt-3 grid gap-3 sm:grid-cols-2">
                  <TextInput
                    label="Resource key"
                    ariaLabel="New resource key"
                    value={newResource.key}
                    onChange={(value) =>
                      setNewResource({ ...newResource, key: value })
                    }
                  />
                  <TextInput
                    label="Label"
                    ariaLabel="New resource label"
                    value={newResource.label}
                    onChange={(value) =>
                      setNewResource({ ...newResource, label: value })
                    }
                  />
                  <NumberInput
                    label="Initial"
                    ariaLabel="New resource initial"
                    value={newResource.initial}
                    onChange={(value) =>
                      setNewResource({ ...newResource, initial: value })
                    }
                  />
                  <NumberInput
                    label="Min"
                    ariaLabel="New resource min"
                    value={newResource.min}
                    onChange={(value) =>
                      setNewResource({ ...newResource, min: value })
                    }
                  />
                  <NumberInput
                    label="Max"
                    ariaLabel="New resource max"
                    value={newResource.max}
                    onChange={(value) =>
                      setNewResource({ ...newResource, max: value })
                    }
                  />
                </div>
              </div>
            </div>
          </div>
        ) : (
          <EmptyPanel label="State edit document not loaded." />
        )}
      </section>
    );
  }

  function renderRulesPanel() {
    const resourceKeys =
      stateVariablesEditDocument?.resources.map((resource) => resource.key) ?? [];

    return (
      <section className={panelClassName}>
        <PanelHeader
          title="Rules"
          subtitle={`${rulesEditDocument?.rules.length ?? 0} rules`}
          action={
            <SaveButton
              label="Save Rules"
              saving={formSaving === "rules"}
              onClick={() => void saveRulesEditDocument()}
            />
          }
        />
        <SectionMessage section="rules" status={formStatus} />
        {rulesEditDocument ? (
          <div className="mt-4 grid gap-4">
            <div className="grid gap-4 xl:grid-cols-2">
              {rulesEditDocument.rules.map((rule, index) => (
                <article
                  key={`${rule.id}:${index}`}
                  className="rounded-md border border-ink/10 bg-parchment p-4"
                >
                  <div className="grid gap-3 sm:grid-cols-2">
                    <TextInput
                      label="Rule id"
                      ariaLabel={`Rule id ${index + 1}`}
                      value={rule.id}
                      onChange={(value) => updateRule(index, { id: value })}
                    />
                    <TextInput
                      label="Action type"
                      ariaLabel={`Rule action type ${index + 1}`}
                      value={rule.action_type}
                      onChange={(value) =>
                        updateRule(index, { action_type: value })
                      }
                    />
                  </div>
                  <div className="mt-3 grid gap-2 text-sm">
                    <p className="text-xs font-medium uppercase text-ink/55">
                      Conditions
                    </p>
                    <code className="break-words rounded-md bg-white px-3 py-2 text-xs text-ink/70">
                      {JSON.stringify(rule.conditions)}
                    </code>
                    <p className="text-xs font-medium uppercase text-ink/55">
                      Effects
                    </p>
                    <code className="break-words rounded-md bg-white px-3 py-2 text-xs text-ink/70">
                      {JSON.stringify(rule.effects)}
                    </code>
                  </div>
                </article>
              ))}
            </div>
            <div className="rounded-md border border-ink/10 bg-white p-4">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h4 className="text-sm font-semibold uppercase text-ink/55">
                  New Rule
                </h4>
                <button
                  type="button"
                  onClick={() => void createRuleFromDraft()}
                  disabled={formSaving === "rules"}
                  className={secondaryButtonClassName}
                >
                  Create Rule
                </button>
              </div>
              <div className="mt-3 grid gap-3 lg:grid-cols-4">
                <TextInput
                  label="Rule id"
                  ariaLabel="New rule id"
                  value={newRule.id}
                  onChange={(value) => setNewRule({ ...newRule, id: value })}
                />
                <TextInput
                  label="Action type"
                  ariaLabel="New rule action type"
                  value={newRule.action_type}
                  onChange={(value) =>
                    setNewRule({ ...newRule, action_type: value })
                  }
                />
                <label className="grid gap-1">
                  <FieldLabel>Resource</FieldLabel>
                  <select
                    aria-label="New rule resource"
                    value={newRule.resource_key}
                    onChange={(event) =>
                      setNewRule({
                        ...newRule,
                        resource_key: event.target.value,
                      })
                    }
                    className={inputClassName}
                  >
                    <option value="">Select resource</option>
                    {resourceKeys.map((key) => (
                      <option key={key} value={key}>
                        {key}
                      </option>
                    ))}
                  </select>
                </label>
                <NumberInput
                  label="Amount"
                  ariaLabel="New rule amount"
                  value={newRule.amount}
                  onChange={(value) => setNewRule({ ...newRule, amount: value })}
                />
              </div>
            </div>
          </div>
        ) : (
          <EmptyPanel label="Rules edit document not loaded." />
        )}
      </section>
    );
  }

  function renderAssetMaintenancePanel() {
    const recordCount = assetCatalog.items.filter(
      (item) => item.source === "record",
    ).length;
    const referenceCount = assetCatalog.items.reduce(
      (total, item) =>
        item.source === "record" ? total + item.record.references.length : total,
      0,
    );
    return (
      <section className={panelClassName}>
        <PanelHeader
          title="Asset Maintenance"
          subtitle={
            assetCatalog.source === "records"
              ? `${recordCount} asset records`
              : `${assetCatalog.items.length} scene background fallbacks`
          }
        />
        <SectionMessage section="assets" status={formStatus} />
        <div className="mt-4 grid gap-4 xl:grid-cols-[0.75fr_1.25fr]">
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
            <MetricBox label="Source files" value={sourceFiles.length} />
            <MetricBox label="Asset records" value={recordCount} />
            <MetricBox label="References" value={referenceCount} />
            <MetricBox
              label="Visual cards"
              value={visualBible?.style_cards.length ?? 0}
            />
            <MetricBox
              label="Audio cards"
              value={audioBible?.voice_cards.length ?? 0}
            />
          </div>
          <div className="grid gap-3 md:grid-cols-2">
            {assetCatalog.items.length > 0 ? (
              assetCatalog.items.map((item) => (
                <AssetCatalogCard key={assetCatalogItemKey(item)} item={item} />
              ))
            ) : (
              <EmptyPanel label="No asset records or scene background paths found." />
            )}
          </div>
        </div>
        <div className="mt-5 grid gap-4 lg:grid-cols-2">
          {renderVisualBibleEditor()}
          {renderAudioBibleEditor()}
        </div>
      </section>
    );
  }

  function renderVisualBibleEditor() {
    return (
      <section className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h4 className="text-sm font-semibold">Visual Bible</h4>
            <p className="mt-1 text-xs font-medium uppercase text-ink/45">
              {visualBible?.style_cards.length ?? 0} style cards
            </p>
          </div>
          <SaveButton
            label="Save Visual Bible"
            saving={formSaving === "assets"}
            onClick={() => void saveVisualBible()}
          />
        </div>
        <div className="mt-3 grid gap-3">
          {visualBible && visualBible.style_cards.length > 0 ? (
            visualBible.style_cards.map((card, index) => (
              <article
                key={`${card.id}:${index}`}
                className="rounded-md border border-ink/10 bg-white px-3 py-3"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0">
                    <h5 className="truncate text-sm font-semibold">
                      {card.title}
                    </h5>
                    <code className="mt-1 block truncate text-xs text-ink/45">
                      {card.id}
                    </code>
                  </div>
                  <span className="rounded-md border border-ink/10 bg-parchment px-2 py-1 text-xs text-ink/60">
                    style
                  </span>
                </div>
                <div className="mt-3 grid gap-3">
                  <TextareaInput
                    label="Prompt"
                    ariaLabel={`Visual style prompt ${index + 1}`}
                    value={card.prompt}
                    onChange={(value) =>
                      updateVisualStyleCard(index, { prompt: value })
                    }
                    minHeight="min-h-28"
                  />
                  <div className="grid gap-3 sm:grid-cols-3">
                    <TextareaInput
                      label="Palette"
                      ariaLabel={`Visual style palette ${index + 1}`}
                      value={listToLines(card.palette)}
                      onChange={(value) =>
                        updateVisualStyleCard(index, {
                          palette: linesToList(value),
                        })
                      }
                      minHeight="min-h-24"
                    />
                    <TextareaInput
                      label="Tags"
                      ariaLabel={`Visual style tags ${index + 1}`}
                      value={listToLines(card.tags)}
                      onChange={(value) =>
                        updateVisualStyleCard(index, {
                          tags: linesToList(value),
                        })
                      }
                      minHeight="min-h-24"
                    />
                    <TextareaInput
                      label="Reference assets"
                      ariaLabel={`Visual style reference asset ids ${index + 1}`}
                      value={listToLines(card.reference_asset_ids)}
                      onChange={(value) =>
                        updateVisualStyleCard(index, {
                          reference_asset_ids: linesToList(value),
                        })
                      }
                      minHeight="min-h-24"
                    />
                  </div>
                </div>
              </article>
            ))
          ) : (
            <p className="text-sm text-ink/55">
              No Visual Bible style cards in project data.
            </p>
          )}
        </div>
      </section>
    );
  }

  function renderAudioBibleEditor() {
    return (
      <section className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h4 className="text-sm font-semibold">Audio Bible</h4>
            <p className="mt-1 text-xs font-medium uppercase text-ink/45">
              {audioBible?.voice_cards.length ?? 0} voice cards
            </p>
          </div>
          <SaveButton
            label="Save Audio Bible"
            saving={formSaving === "assets"}
            onClick={() => void saveAudioBible()}
          />
        </div>
        <div className="mt-3 grid gap-3">
          {audioBible && audioBible.voice_cards.length > 0 ? (
            audioBible.voice_cards.map((card, index) => (
              <article
                key={`${card.id}:${index}`}
                className="rounded-md border border-ink/10 bg-white px-3 py-3"
              >
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0">
                    <h5 className="truncate text-sm font-semibold">
                      {card.title}
                    </h5>
                    <code className="mt-1 block truncate text-xs text-ink/45">
                      {card.id}
                    </code>
                  </div>
                  <span className="rounded-md border border-ink/10 bg-parchment px-2 py-1 text-xs text-ink/60">
                    voice
                  </span>
                </div>
                <div className="mt-3 grid gap-3">
                  <TextInput
                    label="Voice"
                    ariaLabel={`Audio voice ${index + 1}`}
                    value={card.voice}
                    onChange={(value) =>
                      updateAudioVoiceCard(index, { voice: value })
                    }
                  />
                  <TextareaInput
                    label="Delivery"
                    ariaLabel={`Audio delivery ${index + 1}`}
                    value={card.delivery}
                    onChange={(value) =>
                      updateAudioVoiceCard(index, { delivery: value })
                    }
                    minHeight="min-h-24"
                  />
                  <TextareaInput
                    label="Sample text"
                    ariaLabel={`Audio sample text ${index + 1}`}
                    value={card.sample_text ?? ""}
                    onChange={(value) =>
                      updateAudioVoiceCard(index, {
                        sample_text: optionalText(value),
                      })
                    }
                    minHeight="min-h-24"
                  />
                  <div className="grid gap-3 sm:grid-cols-2">
                    <TextareaInput
                      label="Tags"
                      ariaLabel={`Audio tags ${index + 1}`}
                      value={listToLines(card.tags)}
                      onChange={(value) =>
                        updateAudioVoiceCard(index, {
                          tags: linesToList(value),
                        })
                      }
                      minHeight="min-h-24"
                    />
                    <TextareaInput
                      label="Reference assets"
                      ariaLabel={`Audio reference asset ids ${index + 1}`}
                      value={listToLines(card.reference_asset_ids)}
                      onChange={(value) =>
                        updateAudioVoiceCard(index, {
                          reference_asset_ids: linesToList(value),
                        })
                      }
                      minHeight="min-h-24"
                    />
                  </div>
                </div>
              </article>
            ))
          ) : (
            <p className="text-sm text-ink/55">
              No Audio Bible voice cards in project data.
            </p>
          )}
        </div>
      </section>
    );
  }

  function renderExportPanel() {
    const exportDisabled = exporting || !staticExportSelected;
    const exportAuditMatched = exportReport
      ? sameStringSet(exportReport.allowed_files, exportReport.files_found)
      : false;
    const packagePaths = exportReport
      ? [
          ...exportReport.allowed_files,
          ...exportReport.files_found,
          ...exportReport.archived_files,
        ]
      : [];
    const hasAbsolutePackagePath = packagePaths.some(isAbsoluteMachinePath);
    const packageItems = [
      {
        label: "Player files",
        detail: "HTML, CSS, JS, fonts",
        status: exportReport ? "Ready" : staticExportSelected ? "Pending" : "Draft",
        files: exportReport?.files_found.length ?? 0,
      },
      {
        label: "ExportManifest",
        detail: "export-manifest.json",
        status: exportReport ? "Ready" : "Pending",
        files: 1,
      },
      {
        label: "Reachable assets",
        detail: "Images, audio, fonts",
        status: exportReport && assetCatalog.items.length > 0 ? "Ready" : "Pending",
        files: assetCatalog.items.length,
      },
      {
        label: "Story and rules data",
        detail: "Scenes, rules, contracts",
        status: exportReport && projectData ? "Ready" : "Pending",
        files: projectData ? projectData.scenes.length + projectData.rules.length : 0,
      },
      {
        label: "AI usage disclosure",
        detail: aiSafetyPolicy?.policy_source_path ?? "ai-usage manifest",
        status: exportReport && aiSafetyPolicy ? "Ready" : "Pending",
        files: aiSafetyPolicy ? 1 : 0,
      },
      {
        label: "Content warning draft",
        detail: "local creator review",
        status: exportReport && aiSafetyPolicy ? "Ready" : "Pending",
        files: aiSafetyPolicy?.content_kinds.length ?? 0,
      },
      {
        label: "Archive manifest",
        detail: exportReport?.archive_path ?? "created after export",
        status: exportReport ? "Ready" : "Pending",
        files: exportReport?.archived_files.length ?? 0,
      },
      {
        label: "Local smoke evidence",
        detail: "static export smoke not run by this command",
        status: "Pending",
        files: 0,
      },
    ];
    const evidenceChecks = [
      {
        label: "No provider configuration",
        status: selectedExportProfile
          ? selectedExportProfile.includes_provider_config
            ? "review"
            : "pass"
          : "pending",
      },
      {
        label: "No private traces",
        status: selectedExportProfile
          ? selectedExportProfile.includes_private_traces
            ? "review"
            : "pass"
          : "pending",
      },
      { label: "No raw responses", status: "pending" },
      { label: "No secret markers", status: "pending" },
      {
        label: "All referenced assets copied",
        status: exportReport ? (exportAuditMatched ? "pass" : "review") : "pending",
      },
      {
        label: "No absolute machine paths",
        status: exportReport
          ? hasAbsolutePackagePath
            ? "review"
            : "pass"
          : "pending",
      },
      { label: "HTTP smoke test passed", status: "pending" },
    ] satisfies Array<{ label: string; status: EvidenceStatus }>;
    const passedEvidenceCount = evidenceChecks.filter(
      (check) => check.status === "pass",
    ).length;
    const reviewEvidenceCount = evidenceChecks.filter(
      (check) => check.status === "review",
    ).length;
    const selectedProfileReady = evidenceChecks.every(
      (check) => check.status === "pass",
    );
    const packageHash = exportReport
      ? "pending explicit package hash"
      : "pending export";

    return (
      <section className={panelClassName}>
        <PanelHeader
          title="Export Package"
          subtitle="Local package readiness, manifest evidence, and boundary checks"
          action={
            <button
              type="button"
              onClick={() => void runStaticZipExport()}
              disabled={exportDisabled}
              className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
            >
              {exporting ? (
                <Loader2 aria-hidden size={16} className="animate-spin" />
              ) : (
                <Download aria-hidden size={16} />
              )}
              Export zip
            </button>
          }
        />
        <SectionMessage section="export-kit" status={formStatus} />

        <div className="mt-4 grid gap-4 2xl:grid-cols-[0.85fr_1.35fr_1fr]">
          <aside className="grid content-start gap-3 rounded-lg border border-ink/10 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-amber-400">
                  Build Profile
                </p>
                <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                  {selectedExportProfile?.id ?? "No profile selected"}
                </h3>
              </div>
              <StudioStatusChip tone={staticExportSelected ? "health" : "agent"}>
                {staticExportSelected ? "Executable" : "Draft"}
              </StudioStatusChip>
            </div>
            <p className="text-sm leading-6 text-canvas-200/65">
              {selectedExportProfile?.intent ??
                "Select a local package profile to inspect export readiness."}
            </p>

            <ExportEvidenceCard
              title="AI Usage Manifest"
              badge={aiSafetyPolicy ? "Included" : "Pending"}
            >
              <p className="text-sm leading-6 text-canvas-200/65">
                {aiSafetyPolicy?.moderation_policy ??
                  "AI usage evidence appears after policy load."}
              </p>
            </ExportEvidenceCard>

            <ExportEvidenceCard
              title="Content Warnings"
              badge={aiSafetyPolicy ? "Included" : "Pending"}
            >
              <div className="grid gap-2">
                {(aiSafetyPolicy?.content_kinds ?? ["text"]).map((kind) => (
                  <div
                    key={kind}
                    className="flex items-center justify-between gap-3 rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2 text-sm"
                  >
                    <span className="capitalize text-canvas-50">{kind}</span>
                    <span className="text-canvas-200/55">creator review</span>
                  </div>
                ))}
              </div>
            </ExportEvidenceCard>

            <ExportEvidenceCard title="Redaction Rules" badge="On">
              <div className="grid gap-2 text-sm text-canvas-200/70">
                {[
                  "Strip provider configuration",
                  "Remove private traces",
                  "Remove raw responses",
                  "Remove secret markers",
                ].map((rule) => (
                  <div key={rule} className="flex items-center justify-between gap-3">
                    <span>{rule}</span>
                    <span className="font-semibold text-health-400">On</span>
                  </div>
                ))}
              </div>
            </ExportEvidenceCard>

            <ExportEvidenceCard title="Asset Whitelist" badge="Local">
              <p className="text-sm leading-6 text-canvas-200/65">
                Allow only referenced assets under project asset paths.
              </p>
              <div className="mt-2 flex flex-wrap gap-2">
                {[".png", ".jpg", ".webp", ".ogg", ".mp3", ".json", ".md"].map(
                  (extension) => (
                    <span
                      key={extension}
                      className="rounded-sm border border-canvas-200/10 bg-canvas-50/5 px-2 py-1 text-xs font-semibold text-canvas-200/70"
                    >
                      {extension}
                    </span>
                  ),
                )}
              </div>
            </ExportEvidenceCard>
          </aside>

          <div className="grid content-start gap-4">
            <section className="rounded-lg border border-ink/10 bg-canvas-50 p-4 shadow-studio-panel">
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <p className="text-xs font-semibold uppercase text-health-500">
                    Package Readiness
                  </p>
                  <h3 className="mt-1 text-lg font-semibold">
                    {selectedProfileReady
                      ? "All local boundary checks passed"
                      : "Review profile boundary checks"}
                  </h3>
                </div>
                <div className="grid grid-cols-2 gap-3 text-right text-sm">
                  <ExportInfo label="Files" value={String(exportReport?.files_found.length ?? 0)} />
                  <ExportInfo
                    label="Estimated size"
                    value={`${Math.max(1, assetCatalog.items.length * 4 + 18)} MB`}
                  />
                </div>
              </div>
              <div className="mt-4 grid gap-2">
                {packageItems.map((item) => (
                  <PackageReadinessRow key={item.label} item={item} />
                ))}
              </div>
            </section>

            <div className="grid gap-4 xl:grid-cols-3">
              <section className="rounded-lg border border-ink/10 bg-canvas-50 p-4 shadow-studio-panel">
                <h3 className="text-base font-semibold">Playable Preview</h3>
                <div className="mt-3 rounded-md border border-ink/10 bg-parchment px-3 py-3">
                  <p className="text-sm font-semibold">
                    {projectSummary?.title ?? "No project loaded"}
                  </p>
                  <p className="mt-1 text-sm text-ink/60">
                    {playtestReport?.scene.title ?? "Run proof for current scene."}
                  </p>
                  <p className="mt-3 text-sm text-ink/70">
                    Trace: {playtestReport?.trace.id ?? "pending"}
                  </p>
                </div>
              </section>

              <section className="rounded-lg border border-ink/10 bg-canvas-50 p-4 shadow-studio-panel">
                <h3 className="text-base font-semibold">Dependency Map</h3>
                <div className="mt-3 grid gap-2 text-sm">
                  {["Entry", "HTML", "Script", "Data", "Asset"].map((node) => (
                    <div
                      key={node}
                      className="flex items-center justify-between rounded-md border border-ink/10 bg-parchment px-3 py-2"
                    >
                      <span>{node}</span>
                      <span className="font-semibold text-jade">resolved</span>
                    </div>
                  ))}
                </div>
              </section>

              <section className="rounded-lg border border-ink/10 bg-canvas-50 p-4 shadow-studio-panel">
                <h3 className="text-base font-semibold">Size Breakdown</h3>
                <div className="mt-3 grid gap-2 text-sm">
                  <ExportInfo label="Images" value={`${assetCatalog.items.length} records`} />
                  <ExportInfo label="Scripts" value="player bundle" />
                  <ExportInfo label="Data" value={`${sourceFiles.length} sources`} />
                  <ExportInfo
                    label="Other"
                    value={exportReport ? `${exportReport.archived_files.length} files` : "pending"}
                  />
                </div>
              </section>
            </div>
          </div>

          <aside className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel">
            <div>
              <p className="text-xs font-semibold uppercase text-health-400">
                Evidence & Boundaries
              </p>
              <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                Local export package only
              </h3>
            </div>
            <div className="grid gap-2">
              {evidenceChecks.map((check) => (
                <div
                  key={check.label}
                  className="flex items-center justify-between gap-3 rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2 text-sm"
                >
                  <span>{check.label}</span>
                  <span
                    className={[
                      "font-semibold",
                      evidenceStatusClassName(check.status),
                    ].join(" ")}
                  >
                    {evidenceStatusLabel(check.status)}
                  </span>
                </div>
              ))}
            </div>

            <ExportEvidenceCard title="Package Information" badge="Local">
              <div className="grid gap-2 text-sm">
                <ProofLikeLine label="Profile" value={selectedExportProfile?.id ?? "none"} />
                <ProofLikeLine label="Package hash" value={packageHash} />
                <ProofLikeLine
                  label="Archive"
                  value={exportReport?.archive_path ?? "not exported"}
                />
                <ProofLikeLine
                  label="Estimated size"
                  value={`${Math.max(1, assetCatalog.items.length * 4 + 18)} MB`}
                />
              </div>
            </ExportEvidenceCard>

            <ExportEvidenceCard title="Validation Summary" badge="Local">
              <div className="grid gap-2 text-sm">
                <ProofLikeLine
                  label="Checks passed"
                  value={`${passedEvidenceCount} / ${evidenceChecks.length}`}
                />
                <ProofLikeLine
                  label="Warnings"
                  value={String(
                    evidenceChecks.length - passedEvidenceCount - reviewEvidenceCount,
                  )}
                />
                <ProofLikeLine
                  label="Needs review"
                  value={String(reviewEvidenceCount + (exportError ? 1 : 0))}
                />
              </div>
            </ExportEvidenceCard>
          </aside>
        </div>

        {exportProfiles.length > 0 ? (
          <div className="mt-5 grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
            <div className="grid content-start gap-2">
              {exportProfiles.map((profile) => {
                const selected = profile.id === selectedExportProfileId;
                return (
                  <button
                    type="button"
                    key={profile.id}
                    aria-label={`Select export profile ${profile.id}`}
                    aria-pressed={selected}
                    onClick={() => selectExportProfile(profile.id)}
                    className={[
                      "rounded-md border px-3 py-3 text-left transition",
                      selected
                        ? "border-ink/45 bg-parchment"
                        : "border-ink/10 hover:border-ink/30",
                    ].join(" ")}
                  >
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div className="min-w-0">
                        <h4 className="truncate text-sm font-semibold">
                          {profile.id}
                        </h4>
                        <p className="mt-1 text-xs font-medium uppercase text-ink/45">
                          {profile.target}
                        </p>
                      </div>
                      <span
                        className={[
                          "rounded-sm px-2 py-1 text-xs font-semibold",
                          profile.target === "static_web"
                            ? "bg-jade/10 text-jade"
                            : "bg-ink/5 text-ink/55",
                        ].join(" ")}
                      >
                        {profile.target === "static_web" ? "Executable" : "Draft"}
                      </span>
                    </div>
                    <p className="mt-2 text-sm leading-5 text-ink/65">
                      {profile.intent}
                    </p>
                  </button>
                );
              })}
            </div>

            {selectedExportProfile ? (
              <article className="rounded-md border border-ink/10 bg-parchment p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <p className="text-xs font-medium uppercase text-ink/45">
                      Selected Profile
                    </p>
                    <h4 className="mt-1 text-base font-semibold">
                      {selectedExportProfile.id}
                    </h4>
                  </div>
                  <span className="rounded-sm border border-ink/10 bg-white px-2 py-1 text-xs font-semibold text-ink/60">
                    {selectedExportProfile.target}
                  </span>
                </div>

                <div className="mt-4 grid gap-2 text-sm sm:grid-cols-2">
                  <ProfileFlag
                    label="Runtime network"
                    value={
                      selectedExportProfile.requires_network_at_runtime
                        ? "required"
                        : "not required"
                    }
                    safe={!selectedExportProfile.requires_network_at_runtime}
                  />
                  <ProfileFlag
                    label="Provider config"
                    value={
                      selectedExportProfile.includes_provider_config
                        ? "included"
                        : "excluded"
                    }
                    safe={!selectedExportProfile.includes_provider_config}
                  />
                  <ProfileFlag
                    label="Private traces"
                    value={
                      selectedExportProfile.includes_private_traces
                        ? "included"
                        : "excluded"
                    }
                    safe={!selectedExportProfile.includes_private_traces}
                  />
                  <ProfileFlag
                    label="Submission ready"
                    value={
                      selectedExportProfile.platform_submission_ready
                        ? "claimed"
                        : "not claimed"
                    }
                    safe={!selectedExportProfile.platform_submission_ready}
                  />
                </div>

                <div className="mt-4">
                  <p className="text-xs font-medium uppercase text-ink/45">
                    Capabilities
                  </p>
                  <div className="mt-2 flex flex-wrap gap-2">
                    {selectedExportProfile.capabilities.map((capability) => (
                      <span
                        key={capability}
                        className="rounded-sm border border-ink/10 bg-white px-2 py-1 text-xs font-semibold text-ink/65"
                      >
                        {capability}
                      </span>
                    ))}
                  </div>
                </div>

                <div className="mt-4 grid gap-2">
                  {selectedExportProfile.notes.map((note) => (
                    <p
                      key={note}
                      className="rounded-md border border-ink/10 bg-white px-3 py-2 text-sm text-ink/65"
                    >
                      {note}
                    </p>
                  ))}
                </div>
              </article>
            ) : null}
          </div>
        ) : (
          <EmptyPanel label="No export profiles returned by the Studio adapter." />
        )}

        {selectedExportProfile && !staticExportSelected ? (
          <div className="mt-4 rounded-md border border-ink/10 bg-parchment px-3 py-2 text-sm text-ink/60">
            This profile is available as contract metadata only; no Studio export
            command is wired for this target.
          </div>
        ) : null}

        {staticExportSelected ? (
          <div className="mt-4 grid gap-3 lg:grid-cols-2">
            <TextInput
              label="Output directory"
              ariaLabel="Static export output directory"
              value={exportDir}
              onChange={setExportDir}
            />
            <TextInput
              label="Zip archive"
              ariaLabel="Static export zip archive"
              value={archivePath}
              onChange={setArchivePath}
            />
          </div>
        ) : null}

        {exportError ? (
          <Message tone="error" className="mt-4">
            {exportError}
          </Message>
        ) : null}

        {exportReport ? (
          <div className="mt-4 grid gap-3 text-sm sm:grid-cols-3">
            <MetricBox label="Archive" value={exportReport.archive_path ?? "none"} />
            <MetricBox label="Files" value={exportReport.archived_files.length} />
            <MetricBox
              label="Audit"
              value={
                sameStringSet(exportReport.allowed_files, exportReport.files_found)
                  ? "matched"
                  : "mismatch"
              }
            />
          </div>
        ) : null}

        {aiSafetyPolicy ? (
          <div className="mt-5 rounded-md border border-ink/10 bg-parchment p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h4 className="text-sm font-semibold uppercase text-ink/55">
                  AI Safety Policy
                </h4>
                <p className="mt-1 text-sm text-ink/60">
                  Export disclosure evidence
                </p>
              </div>
              <SaveButton
                label="Save AI Safety Policy"
                saving={formSaving === "export-kit"}
                onClick={() => void saveAiSafetyPolicy()}
              />
            </div>
            <div className="mt-4 grid gap-3 lg:grid-cols-2">
              <CheckboxInput
                label="Live generated content enabled"
                checked={aiSafetyPolicy.live_generated_content_enabled}
                onChange={(value) =>
                  updateAiSafetyPolicy({
                    live_generated_content_enabled: value,
                  })
                }
              />
              <CheckboxInput
                label="Human review required"
                checked={aiSafetyPolicy.human_review_required}
                onChange={(value) =>
                  updateAiSafetyPolicy({ human_review_required: value })
                }
              />
              <CheckboxInput
                label="Moderation queue enabled"
                checked={aiSafetyPolicy.moderation_queue_enabled}
                onChange={(value) =>
                  updateAiSafetyPolicy({ moderation_queue_enabled: value })
                }
              />
              <TextareaInput
                label="Content kinds"
                ariaLabel="AI safety content kinds"
                value={listToLines(aiSafetyPolicy.content_kinds)}
                onChange={(value) =>
                  updateAiSafetyPolicy({
                    content_kinds: contentKindsFromLines(value),
                  })
                }
                minHeight="min-h-24"
              />
              <TextInput
                label="Reporting path"
                ariaLabel="AI safety reporting path"
                value={aiSafetyPolicy.user_reporting_path}
                onChange={(value) =>
                  updateAiSafetyPolicy({ user_reporting_path: value })
                }
              />
              <TextareaInput
                label="Moderation policy"
                ariaLabel="AI safety moderation policy"
                value={aiSafetyPolicy.moderation_policy}
                onChange={(value) =>
                  updateAiSafetyPolicy({ moderation_policy: value })
                }
                minHeight="min-h-24"
              />
              <TextareaInput
                label="Safety guardrails"
                ariaLabel="AI safety guardrails"
                value={listToLines(aiSafetyPolicy.safety_guardrails)}
                onChange={(value) =>
                  updateAiSafetyPolicy({ safety_guardrails: linesToList(value) })
                }
                className="lg:col-span-2"
                minHeight="min-h-28"
              />
            </div>
          </div>
        ) : null}
      </section>
    );
  }

  function renderSourceFileList() {
    return (
      <section className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h3 className="text-lg font-semibold">Source Artifacts</h3>
            <p className="mt-1 text-sm text-ink/55">{sourceFiles.length} files</p>
          </div>
          <TerminalSquare aria-hidden className="text-signal" size={22} />
        </div>

        <div className="mt-4 grid max-h-80 gap-2 overflow-auto pr-1">
          {sourceFiles.map((file) => (
            <button
              type="button"
              key={file.path}
              onClick={() => void selectSourceFile(file)}
              className={[
                "flex min-h-11 items-center justify-between gap-3 rounded-md border px-3 py-2 text-left transition",
                selectedFile?.path === file.path
                  ? "border-ink/45 bg-parchment"
                  : "border-ink/10 hover:border-ink/30",
              ].join(" ")}
            >
              <code className="truncate text-sm text-ink/80">{file.path}</code>
              <span
                className={[
                  "shrink-0 rounded-sm px-2 py-1 text-xs font-medium",
                  file.editable
                    ? "bg-jade/10 text-jade"
                    : "bg-ink/5 text-ink/55",
                ].join(" ")}
              >
                {file.editable ? "editable" : file.kind}
              </span>
            </button>
          ))}
        </div>
      </section>
    );
  }

  function renderBoundaryChecks() {
    return (
      <section className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
        <h3 className="text-lg font-semibold">Boundary Checks</h3>
        <div className="mt-4 grid gap-3">
          {boundaryChecks.map((check) => (
            <div key={check.label} className="flex items-start gap-3">
              <CheckCircle2
                aria-hidden
                className="mt-0.5 shrink-0 text-jade"
                size={18}
              />
              <div className="min-w-0">
                <p className="text-sm font-semibold">{check.label}</p>
                <p className="truncate text-sm text-ink/55">{check.value}</p>
              </div>
            </div>
          ))}
        </div>
      </section>
    );
  }

  function renderSourceEditor() {
    return (
      <section className={panelClassName}>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="min-w-0">
            <h3 className="text-lg font-semibold">Artifact Text Editor</h3>
            <p className="truncate text-sm text-ink/55">
              {selectedFile?.path ?? "No source file selected"}
            </p>
          </div>
          <button
            type="button"
            disabled={!dirty || saving}
            onClick={() => void saveSelectedFile()}
            className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
          >
            {saving ? (
              <Loader2 aria-hidden size={16} className="animate-spin" />
            ) : (
              <Save aria-hidden size={16} />
            )}
            Save
          </button>
        </div>

        {error ? (
          <Message tone="error" className="mt-4">
            {error}
          </Message>
        ) : null}

        <div className="mt-4">
          <textarea
            aria-label="Source editor"
            value={editorContent}
            readOnly={!selectedFile?.editable}
            onChange={(event) => setEditorContent(event.target.value)}
            spellCheck={false}
            className="min-h-72 w-full resize-y rounded-md border border-ink/15 bg-parchment px-3 py-3 font-mono text-sm leading-6 text-ink outline-none transition focus:border-ink/45 read-only:bg-ink/5"
          />
          <div className="mt-3 flex flex-wrap items-center gap-2 text-xs font-medium uppercase text-ink/55">
            <span>{selectedFile?.kind ?? "none"}</span>
            <span>{selectedFile?.editable ? "editable" : "read only"}</span>
            {dirty ? <span className="text-brass">modified</span> : null}
          </div>
        </div>
      </section>
    );
  }

  function renderEvidencePanel() {
    const healthTone = error || playtestError || exportError ? "danger" : "health";
    const healthLabel =
      error || playtestError || exportError ? "Error visible" : "Trace visible";

    return (
      <div className="grid gap-4">
        <div>
          <p className="text-xs font-semibold uppercase text-amber-400">
            Evidence Panel
          </p>
          <h3 className="mt-1 text-lg font-semibold text-canvas-50">
            {activeWorkflowMeta.label}
          </h3>
          <p className="mt-1 text-sm leading-6 text-canvas-200/65">
            {activeWorkflowMeta.description}
          </p>
        </div>

        <StudioPanel>
          <div className="flex flex-wrap gap-2">
            <StudioStatusChip tone={healthTone}>{healthLabel}</StudioStatusChip>
            <StudioStatusChip tone="acp">{dataSource.runtimeName}</StudioStatusChip>
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

        <div className="rounded-lg border border-canvas-200/10 bg-graphite-850 px-3 py-3">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-canvas-200/55">
                Local Preview Agent State
              </p>
              <h4 className="mt-1 text-sm font-semibold text-canvas-50">
                {localPreviewState.capabilityPolicy.title}
              </h4>
            </div>
            <StudioStatusChip tone="agent">{localPreviewState.source}</StudioStatusChip>
          </div>

          <div className="mt-3 grid gap-2 text-sm">
            <PreviewEvidenceLine
              label="Workers"
              value={String(previewSummary.workerCount)}
            />
            <PreviewEvidenceLine
              label="Pending approvals"
              value={String(previewSummary.pendingApprovalCount)}
            />
            <PreviewEvidenceLine
              label="Network"
              value={previewSummary.networkEnabled ? "enabled" : "disabled"}
            />
            <PreviewEvidenceLine
              label="Project truth"
              value={
                previewSummary.isAuthoritativeProjectState
                  ? "authoritative"
                  : "not authoritative"
              }
            />
          </div>

          <div className="mt-3 grid gap-2">
            {localPreviewState.workers.map((worker) => (
              <div
                key={worker.id}
                className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2"
              >
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <p className="text-sm font-semibold text-canvas-50">
                    {worker.label}
                  </p>
                  <span className="text-xs font-semibold text-acp-400">
                    {worker.connectionState}
                  </span>
                </div>
                <p className="mt-1 text-xs leading-5 text-canvas-200/55">
                  {worker.role}
                </p>
              </div>
            ))}
          </div>

          <div className="mt-3 grid gap-2">
            {localPreviewState.approvals.map((approval) => (
              <div
                key={approval.id}
                className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2"
              >
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <p className="text-sm font-semibold text-canvas-50">
                    {approval.title}
                  </p>
                  <span className="text-xs font-semibold text-amber-400">
                    {approval.state}
                  </span>
                </div>
                <p className="mt-1 truncate text-xs text-canvas-200/55">
                  {approval.evidenceIds.join(" / ")}
                </p>
              </div>
            ))}
          </div>

          <div className="mt-3 grid gap-2">
            {localPreviewState.capabilityPolicy.boundaries.map((boundary) => (
              <p
                key={boundary}
                className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2 text-xs leading-5 text-canvas-200/65"
              >
                {boundary}
              </p>
            ))}
          </div>
        </div>

        <div className="rounded-lg border border-canvas-200/10 bg-graphite-850 px-3 py-3">
          <p className="text-xs font-semibold uppercase text-canvas-200/55">
            Reference Screens
          </p>
          <div className="mt-3 grid gap-2">
            {activeScreenReferences.map((reference) => (
              <div
                key={reference.id}
                title={reference.title}
                className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2"
              >
                <p className="truncate text-sm font-semibold text-canvas-50">
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
          <p className="text-xs font-semibold uppercase text-canvas-200/50">
            Command Dock
          </p>
          <p className="truncate text-sm font-semibold text-canvas-50">
            {projectSummary?.title ?? "No project loaded"} / {activeSectionMeta.label}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <StudioStatusChip tone="action">
            {dirty ? "Unsaved source" : "Workspace synced"}
          </StudioStatusChip>
          <StudioButton variant="primary" onClick={() => openStudioSection("playtest")}>
            <Play aria-hidden size={16} />
            Run playable proof
          </StudioButton>
        </div>
      </>
    );
  }

  return (
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
          <StudioButton onClick={() => openStudioSection("playtest")}>
            <Play aria-hidden size={16} />
            Playtest
          </StudioButton>
        </>
      }
      rightPanel={renderEvidencePanel()}
      commandDock={renderCommandDock()}
    >
      {renderActiveSection()}
    </StudioShell>
  );
}

const inputClassName = studioUiClassNames.input;
const textareaClassName = studioUiClassNames.textarea;
const panelClassName = studioUiClassNames.panel;
const secondaryButtonClassName = studioUiClassNames.secondaryButton;

function PanelHeader({
  title,
  subtitle,
  action,
}: {
  title: string;
  subtitle: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-4">
      <div className="min-w-0">
        <h3 className="text-lg font-semibold">{title}</h3>
        <p className="mt-1 truncate text-sm text-ink/55">{subtitle}</p>
      </div>
      {action}
    </div>
  );
}

function SaveButton({
  label,
  saving,
  onClick,
}: {
  label: string;
  saving: boolean;
  onClick(): void;
}) {
  return (
    <button
      type="button"
      disabled={saving}
      onClick={onClick}
      className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
    >
      {saving ? (
        <Loader2 aria-hidden size={16} className="animate-spin" />
      ) : (
        <Save aria-hidden size={16} />
      )}
      {label}
    </button>
  );
}

function TextInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <FieldLabel>{label}</FieldLabel>
      <input
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={inputClassName}
      />
    </label>
  );
}

function NumberInput({
  label,
  ariaLabel,
  value,
  onChange,
}: {
  label: string;
  ariaLabel: string;
  value: number;
  onChange(value: number): void;
}) {
  return (
    <label className="grid gap-1">
      <FieldLabel>{label}</FieldLabel>
      <input
        type="number"
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className={inputClassName}
      />
    </label>
  );
}

function TextareaInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
  minHeight = "min-h-40",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
  minHeight?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <FieldLabel>{label}</FieldLabel>
      <textarea
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={`${textareaClassName} ${minHeight}`}
      />
    </label>
  );
}

function CheckboxInput({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange(value: boolean): void;
}) {
  return (
    <label className="inline-flex items-center gap-2">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="h-4 w-4 accent-ink"
      />
      {label}
    </label>
  );
}

function FieldLabel({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-xs font-medium uppercase text-ink/55">
      {children}
    </span>
  );
}

function SectionMessage({
  section,
  status,
}: {
  section: StudioSectionId;
  status: FormStatus | null;
}) {
  if (!status || status.section !== section) {
    return null;
  }
  return (
    <Message tone={status.tone === "error" ? "error" : "success"} className="mt-4">
      {status.message}
    </Message>
  );
}

function Message({
  tone,
  className = "",
  children,
}: {
  tone: "success" | "error";
  className?: string;
  children: React.ReactNode;
}) {
  const toneClass =
    tone === "success"
      ? "border-jade/30 bg-jade/10 text-jade"
      : "border-signal/30 bg-signal/10 text-signal";
  return (
    <div className={`rounded-md border px-3 py-2 text-sm ${toneClass} ${className}`}>
      {children}
    </div>
  );
}

function MetricBox({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-ink/10 bg-parchment px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/55">{label}</p>
      <p className="mt-1 truncate font-semibold">{value}</p>
    </div>
  );
}

function ProfileFlag({
  label,
  value,
  safe,
}: {
  label: string;
  value: string;
  safe: boolean;
}) {
  return (
    <div className="rounded-md border border-ink/10 bg-white px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p
        className={[
          "mt-1 text-sm font-semibold",
          safe ? "text-jade" : "text-signal",
        ].join(" ")}
      >
        {value}
      </p>
    </div>
  );
}

function ExportEvidenceCard({
  title,
  badge,
  children,
}: {
  title: string;
  badge: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="text-sm font-semibold text-canvas-50">{title}</h4>
        <span className="rounded-sm border border-health-400/30 bg-health-500/15 px-2 py-1 text-xs font-semibold text-health-400">
          {badge}
        </span>
      </div>
      <div className="mt-3">{children}</div>
    </section>
  );
}

function PackageReadinessRow({
  item,
}: {
  item: {
    label: string;
    detail: string;
    status: string;
    files: number;
  };
}) {
  const ready = item.status === "Ready" || item.status === "Passed";
  return (
    <div className="grid gap-3 rounded-md border border-ink/10 bg-parchment px-3 py-2 text-sm md:grid-cols-[minmax(0,1fr)_120px_minmax(160px,0.7fr)_80px]">
      <div className="min-w-0">
        <p className="truncate font-semibold text-ink">{item.label}</p>
        <p className="mt-1 truncate text-xs text-ink/50">{item.detail}</p>
      </div>
      <span
        className={[
          "w-fit rounded-sm border px-2 py-1 text-xs font-semibold",
          ready
            ? "border-jade/30 bg-jade/10 text-jade"
            : "border-brass/30 bg-brass/10 text-brass",
        ].join(" ")}
      >
        {item.status}
      </span>
      <div className="h-2 self-center rounded-full bg-ink/10">
        <div
          className={[
            "h-2 rounded-full",
            ready ? "bg-jade" : "bg-brass",
          ].join(" ")}
          style={{ width: ready ? "100%" : "45%" }}
        />
      </div>
      <p className="text-right text-xs font-semibold text-ink/60">
        {item.files} files
      </p>
    </div>
  );
}

function ExportInfo({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}

function ProofLikeLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3">
      <p className="text-xs font-medium uppercase text-canvas-200/45">{label}</p>
      <p className="truncate text-xs font-semibold text-canvas-50">{value}</p>
    </div>
  );
}

function evidenceStatusLabel(status: EvidenceStatus) {
  switch (status) {
    case "pass":
      return "Pass";
    case "review":
      return "Review";
    case "pending":
      return "Pending";
  }
}

function evidenceStatusClassName(status: EvidenceStatus) {
  switch (status) {
    case "pass":
      return "text-health-400";
    case "review":
      return "text-signal";
    case "pending":
      return "text-amber-400";
  }
}

function sameStringSet(left: string[], right: string[]) {
  if (left.length !== right.length) {
    return false;
  }

  const rightValues = new Set(right);
  return left.every((value) => rightValues.has(value));
}

function isAbsoluteMachinePath(path: string) {
  return path.startsWith("/") || path.startsWith("~") || /^[A-Za-z]:[\\/]/.test(path);
}

function EvidenceLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{value}</p>
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
      <p className="text-xs font-medium uppercase text-canvas-200/45">{label}</p>
      <p className="truncate text-xs font-semibold text-canvas-50">{value}</p>
    </div>
  );
}

function AssetCatalogCard({ item }: { item: AssetCatalogItem }) {
  if (item.source === "scene-background-fallback") {
    return (
      <article className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
        <p className="text-xs font-medium uppercase text-ink/45">
          Scene background fallback
        </p>
        <code className="mt-2 block truncate text-sm text-ink/80">
          {item.path}
        </code>
      </article>
    );
  }

  const { record } = item;
  return (
    <article className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase text-ink/45">
            {record.kind} / {record.source}
          </p>
          <h4 className="mt-1 truncate text-sm font-semibold">{record.id}</h4>
        </div>
        {record.provider_metadata?.fallback_used ? (
          <span className="rounded-md border border-signal/30 bg-signal/10 px-2 py-1 text-xs font-semibold text-signal">
            Fallback
          </span>
        ) : null}
      </div>
      <div className="mt-3 grid gap-2 text-sm text-ink/70">
        <AssetField label="Project path" value={record.project_path} code />
        <AssetField label="Export path" value={record.export_path} code />
        <AssetField label="Content hash" value={record.content_hash} code />
        <AssetField label="Hash algorithm" value={record.hash_algorithm} />
        <AssetField label="Bytes" value={String(record.byte_length)} />
        <AssetField label="Provider" value={providerLabel(record)} />
        <AssetField
          label="Request id"
          value={record.provider_metadata?.request_id ?? "none"}
          code={Boolean(record.provider_metadata?.request_id)}
        />
        <AssetField
          label="Prompt hash"
          value={record.provider_metadata?.prompt_hash ?? "none"}
          code={Boolean(record.provider_metadata?.prompt_hash)}
        />
        <AssetField
          label="References"
          value={referenceLabel(record)}
          code={record.references.length > 0}
        />
      </div>
    </article>
  );
}

function AssetField({
  label,
  value,
  code = false,
}: {
  label: string;
  value: string;
  code?: boolean;
}) {
  return (
    <div className="min-w-0">
      <p className="text-[11px] font-medium uppercase text-ink/45">{label}</p>
      {code ? (
        <code className="block truncate text-xs text-ink/75">{value}</code>
      ) : (
        <p className="truncate text-xs text-ink/75">{value}</p>
      )}
    </div>
  );
}

function EmptyPanel({ label }: { label: string }) {
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-parchment px-3 py-2 text-sm text-ink/55">
      {label}
    </div>
  );
}

function assetCatalogItemKey(item: AssetCatalogItem): string {
  return item.source === "record" ? item.record.id : item.path;
}

function providerLabel(record: AssetRecord): string {
  const metadata = record.provider_metadata;
  if (!metadata) {
    return "none";
  }

  return [metadata.provider, metadata.model].filter(Boolean).join(" / ");
}

function referenceLabel(record: AssetRecord): string {
  if (record.references.length === 0) {
    return "none";
  }

  return record.references
    .map(
      (reference) =>
        `${reference.reference_kind}:${reference.reference_id}:${reference.slot}`,
    )
    .join(", ");
}

function emptyCharacterDraft(): CharacterDraft {
  return {
    id: "",
    name: "",
    role: "",
    traits_text: "",
    visual_card: "",
    voice_card: "",
  };
}

function emptyResourceDraft(): ResourceDraft {
  return {
    key: "",
    label: "",
    initial: 0,
    min: 0,
    max: 100,
  };
}

function emptyRuleDraft(): RuleDraft {
  return {
    id: "",
    action_type: "",
    resource_key: "",
    amount: 0,
  };
}

function listToLines(values: string[]) {
  return values.join("\n");
}

function linesToList(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function contentKindsFromLines(value: string): AiUsageContentKind[] {
  const allowed: AiUsageContentKind[] = ["text", "image", "audio", "voice", "data"];
  const selected = linesToList(value).filter((kind): kind is AiUsageContentKind =>
    allowed.includes(kind as AiUsageContentKind),
  );
  return selected.length > 0 ? selected : ["text"];
}
