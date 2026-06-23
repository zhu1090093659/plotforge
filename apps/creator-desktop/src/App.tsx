import {
  Loader2,
  Play,
  RefreshCcw,
  Save,
} from "lucide-react";
import { useEffect, useState } from "react";
import type {
  AiSafetyPolicy,
  AiUsageContentKind,
  AudioVoiceCard,
  Character,
  CharacterDraft,
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
import { AssetMaintenanceView } from "./AssetMaintenanceView";
import { LaunchpadView } from "./LaunchpadView";
import { CharactersView } from "./CharactersView";
import { StateView, type ResourceDraft } from "./StateView";
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
  EmptyPanel,
} from "./studioUi";
import {
  defaultNewProjectPath,
  useStudioWorkspace,
} from "./useStudioWorkspace";
import { LanguageToggle, StudioI18nProvider } from "./i18n";
import { errorMessage } from "./errorMessage";

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};



interface RuleDraft {
  id: string;
  action_type: string;
  resource_key: string;
  amount: number;
}

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
    setProjectData,
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

  useEffect(() => {
    setCreateProjectPath(defaultNewProjectPath(initialProjectPath));
  }, [initialProjectPath]);

  async function loadProject(path: string) {
    setFormStatus(null);
    await loadWorkspaceProject(path);
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
        message: errorMessage(source),
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
    // Character assembly (id trim, traits split) is now done in Rust via the
    // create_character_from_draft command (AGENTS.md line 51).
    await runFormAction("characters", "Character created.", async () => {
      const updated = await dataSource.createCharacterFromDraft(loadedPath, newCharacter);
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
    await runFormAction("rules", "Rule created.", async () => {
      const updated = await dataSource.createRuleFromDraft(loadedPath, newRule);
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
            runtimeName={dataSource.runtimeName}
            sourceFiles={sourceFiles}
            assetRecordCount={
              assetCatalog.items.filter((item) => item.source === "record").length
            }
            exportProfileCount={exportProfiles.length}
            playtestReport={playtestReport}
            onOpenTrace={() => openStudioSection("debugger")}
            onRunPlayableProof={() => void runPlaytest()}
          />
        );
      case "world":
        return (
          <WorldView
            worldEditDocument={worldEditDocument}
            saving={formSaving === "world"}
            formStatus={formStatus}
            worldExpansionGoal={worldExpansionGoal}
            onWorldExpansionGoalChange={setWorldExpansionGoal}
            onSave={() => void saveWorldEditDocument()}
            onGenerateWorldExpansion={() => void generateWorldExpansionFromGoal()}
            onUpdateWorldDocument={updateWorldDocument}
          />
        );
      case "story":
        return (
          <StoryView
            storyCraftEditDocument={storyCraftEditDocument}
            saving={formSaving === "story"}
            formStatus={formStatus}
            storyGenerationConcept={storyGenerationConcept}
            onStoryGenerationConceptChange={setStoryGenerationConcept}
            onSave={() => void saveStoryCraftEditDocument()}
            onGenerateStoryCraft={() => void generateStoryCraftFromConcept()}
            onUpdateStoryBible={updateStoryBible}
            onUpdateStoryCraftDocument={(patch) => {
              if (storyCraftEditDocument) {
                setStoryCraftEditDocument({ ...storyCraftEditDocument, ...patch });
              }
            }}
          />
        );
      case "characters":
        return (
          <CharactersView
            characterEditDocument={characterEditDocument}
            saving={formSaving === "characters"}
            formStatus={formStatus}
            characterGenerationConcept={characterGenerationConcept}
            onCharacterGenerationConceptChange={setCharacterGenerationConcept}
            characterGenerationRoleHint={characterGenerationRoleHint}
            onCharacterGenerationRoleHintChange={setCharacterGenerationRoleHint}
            characterDraft={newCharacter}
            onCharacterDraftChange={setNewCharacter}
            onSave={() => void saveCharacterEditDocument()}
            onGenerateCharacter={() => void generateCharacterFromConcept()}
            onCreateCharacterFromDraft={() => void createCharacterFromDraft()}
            onUpdateCharacter={updateCharacter}
          />
        );
      case "state":
        return (
          <StateView
            stateVariablesEditDocument={stateVariablesEditDocument}
            saving={formSaving === "state"}
            formStatus={formStatus}
            newResource={newResource}
            onNewResourceChange={setNewResource}
            onSave={() => void saveStateVariablesEditDocument()}
            onUpdateResource={updateResource}
            onUpdateInitialWorldResource={updateInitialWorldResource}
            onUpdateInitialSceneKey={(value) => {
              if (stateVariablesEditDocument) {
                setStateVariablesEditDocument({
                  ...stateVariablesEditDocument,
                  initial_story_state: {
                    ...stateVariablesEditDocument.initial_story_state,
                    current_scene_key: value,
                  },
                });
              }
            }}
            onUpdateInitialTurn={(value) => {
              if (stateVariablesEditDocument) {
                setStateVariablesEditDocument({
                  ...stateVariablesEditDocument,
                  initial_story_state: {
                    ...stateVariablesEditDocument.initial_story_state,
                    turn: value,
                  },
                });
              }
            }}
            onCreateResourceFromDraft={() => void createResourceFromDraft()}
          />
        );
      case "rules":
        return (
          <RulesView
            rulesEditDocument={rulesEditDocument}
            resourceKeys={
              stateVariablesEditDocument?.resources.map((r) => r.key) ?? []
            }
            saving={formSaving === "rules"}
            formStatus={formStatus}
            ruleDraft={newRule}
            onRuleDraftChange={setNewRule}
            onSave={() => void saveRulesEditDocument()}
            onCreateRuleFromDraft={() => void createRuleFromDraft()}
            onUpdateRule={updateRule}
          />
        );
      case "assets":
        return (
          <ArtifactReviewView
            projectSummary={projectSummary}
            loadedPath={loadedPath}
            sourceFiles={sourceFiles}
            assetCatalog={assetCatalog}
            playtestReport={playtestReport}
            playtesting={playtesting}
            playtestError={playtestError}
            exportReport={exportReport}
            onOpenTrace={() => openStudioSection("debugger")}
            onRunPlayableProof={() => void runPlaytest()}
          >
            <AssetMaintenanceView
              assetCatalog={assetCatalog}
              sourceFiles={sourceFiles}
              visualBible={visualBible}
              audioBible={audioBible}
              saving={formSaving === "assets"}
              formStatus={formStatus}
              onUpdateVisualStyleCard={updateVisualStyleCard}
              onUpdateAudioVoiceCard={updateAudioVoiceCard}
              onSaveVisualBible={() => void saveVisualBible()}
              onSaveAudioBible={() => void saveAudioBible()}
            />
          </ArtifactReviewView>
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
          <TraceDebugView
            report={playtestReport}
            error={playtestError}
            selectedExportProfile={selectedExportProfile}
            exportReport={exportReport}
            aiSafetyPolicy={aiSafetyPolicy}
            loadedPath={loadedPath}
            projectId={projectData?.game.id ?? null}
          />
        );
      case "export-kit":
        return (
          <ExportView
            exportDir={exportDir}
            setExportDir={setExportDir}
            archivePath={archivePath}
            setArchivePath={setArchivePath}
            exportProfiles={exportProfiles}
            selectedExportProfileId={selectedExportProfileId}
            selectedExportProfile={selectedExportProfile}
            staticExportSelected={staticExportSelected}
            exportReport={exportReport}
            exporting={exporting}
            exportError={exportError}
            assetCatalog={assetCatalog}
            projectData={projectData}
            aiSafetyPolicy={aiSafetyPolicy}
            formSaving={formSaving}
            formStatus={formStatus}
            runStaticZipExport={runStaticZipExport}
            selectExportProfile={selectExportProfile}
            updateAiSafetyPolicy={updateAiSafetyPolicy}
            saveAiSafetyPolicy={saveAiSafetyPolicy}
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
        playtestInput={playtestInput}
        setPlaytestInput={setPlaytestInput}
        playtesting={playtesting}
        playtestReport={playtestReport}
        playtestError={playtestError}
        exportProfiles={exportProfiles}
        exportReport={exportReport}
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
        onCreateProject={(path, request, force) => void handleCreateProject(path, request, force)}
        dataSource={dataSource}
      />
    );
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
                Backend Boundary
              </p>
              <h4 className="mt-1 text-sm font-semibold text-canvas-50">
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
              value={playtestReport?.trace.id ?? "not run"}
            />
            <PreviewEvidenceLine
              label="Export"
              value={exportReport?.archive_path ?? "not exported"}
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
              "ACP workers, approval queues, provider calls, and publishing automation are not implemented.",
            ].map((boundary) => (
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
    </StudioI18nProvider>
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
