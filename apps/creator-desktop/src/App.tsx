import {
  CheckCircle2,
  ChevronRight,
  Download,
  FolderOpen,
  Loader2,
  PlusCircle,
  Play,
  RefreshCcw,
  Save,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type {
  AiSafetyPolicy,
  AssetRecord,
  AiUsageContentKind,
  AudioBible,
  AudioVoiceCard,
  Character,
  CharacterEditDocument,
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectData,
  ProjectTemplateId,
  ResourceDefinition,
  Rule,
  RulesEditDocument,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  VisualBible,
  VisualStyleCard,
  WorldEditDocument,
} from "../../../contracts/plotforge";
import { summarizeProject, type CreatorProjectSummary } from "./projectSummary";
import { PlaytestPanel, RuntimeTracePanel } from "./runtimeTraceView";
import {
  createDefaultStudioDataSource,
  defaultProjectPath,
  type StudioDataSource,
} from "./studioDataSource";
import {
  projectAssetCatalog,
  studioSections,
  type AssetCatalogItem,
} from "./studioModel";
import type {
  PlayOnceReport,
  ProjectCheckReport,
  SourceFileContent,
  SourceFileSummary,
  StaticExportReport,
} from "./tauriBridge";

const boundaryChecks = [
  { label: "Generated contracts", value: "plotforge.d.ts", ok: true },
  { label: "Rust core boundary", value: "UI adapter only", ok: true },
  { label: "Tauri bridge", value: "commands wired", ok: true },
];

const defaultPlaytestInput =
  "Raise emergency taxes while auditing corrupt officials.";

type StudioSectionId =
  | "dashboard"
  | "world"
  | "story"
  | "characters"
  | "state"
  | "rules"
  | "assets"
  | "playtest"
  | "debugger"
  | "export";

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};

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
}

export function App({
  dataSource = createDefaultStudioDataSource(),
  initialProjectPath = defaultProjectPath(),
}: AppProps) {
  const [activeSection, setActiveSection] =
    useState<StudioSectionId>("dashboard");
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
  const [exportReport, setExportReport] = useState<StaticExportReport | null>(
    null,
  );
  const [exporting, setExporting] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);
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
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeSectionMeta =
    studioSections.find((section) => section.id === activeSection) ??
    studioSections[0];
  const dirty = Boolean(selectedFile?.editable && editorContent !== savedContent);
  const assetCatalog = useMemo(
    () => projectAssetCatalog(projectData, assetRecords),
    [assetRecords, projectData],
  );
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
    setCreateProjectPath(defaultNewProjectPath(initialProjectPath));
    void loadProject(initialProjectPath);
  }, [initialProjectPath]);

  async function loadProject(path: string) {
    setLoading(true);
    setError(null);
    setFormStatus(null);
    try {
      const [
        project,
        report,
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
      const projectWithBible = {
        ...project,
        visual_bible: visualBibleDocument,
        audio_bible: audioBibleDocument,
      };

      setLoadedPath(path);
      setProjectPath(path);
      setProjectData(projectWithBible);
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
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setLoading(false);
    }
  }

  async function refreshProjectOverview(path: string) {
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
    const projectWithBible = {
      ...project,
      visual_bible: visualBibleDocument,
      audio_bible: audioBibleDocument,
    };
    setProjectData(projectWithBible);
    setAssetRecords(records);
    setVisualBible(visualBibleDocument);
    setAudioBible(audioBibleDocument);
    setProjectSummary(summarizeProject(projectWithBible));
    setCheckReport(report);
    setSourceFiles(files);
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
    const input = playtestInput.trim();
    if (!input) {
      setPlaytestError("Playtest input is required.");
      return;
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
      setActiveSection("debugger");
    } catch (source) {
      setPlaytestError(source instanceof Error ? source.message : String(source));
      setActiveSection("playtest");
    } finally {
      setPlaytesting(false);
    }
  }

  async function runStaticZipExport() {
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
      setExportError(source instanceof Error ? source.message : String(source));
    } finally {
      setExporting(false);
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
      setError(source instanceof Error ? source.message : String(source));
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
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setSaving(false);
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
    await runFormAction("export", "AI safety policy saved.", async () => {
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

  function renderActiveSection() {
    switch (activeSection) {
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
        return renderAssetsPanel();
      case "playtest":
        return (
          <PlaytestPanel
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
          />
        );
      case "debugger":
        return <RuntimeTracePanel report={playtestReport} error={playtestError} />;
      case "export":
        return renderExportPanel();
      case "dashboard":
      default:
        return renderDashboard();
    }
  }

  function renderDashboard() {
    return (
      <div className="grid gap-5">
        <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
          {metrics.map((metric) => (
            <article
              key={metric.label}
              className={`rounded-md border bg-white p-4 shadow-sm ${metric.tone}`}
            >
              <p className="text-sm font-medium text-ink/55">{metric.label}</p>
              <p className="mt-2 text-3xl font-semibold text-current">
                {metric.value}
              </p>
            </article>
          ))}
        </section>

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

  function renderAssetsPanel() {
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
          title="Assets"
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
    return (
      <section className={panelClassName}>
        <PanelHeader
          title="Export"
          subtitle="Static Web zip package"
          action={
            <button
              type="button"
              onClick={() => void runStaticZipExport()}
              disabled={exporting}
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
        <SectionMessage section="export" status={formStatus} />

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
                exportReport.allowed_files.length === exportReport.files_found.length
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
                saving={formSaving === "export"}
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
            <h3 className="text-lg font-semibold">Source Files</h3>
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
            <h3 className="text-lg font-semibold">Source Editor</h3>
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

  return (
    <div className="min-h-screen bg-parchment text-ink">
      <div className="grid min-h-screen grid-cols-[280px_1fr] max-lg:grid-cols-1">
        <aside className="border-r border-ink/10 bg-white/75 px-4 py-5 max-lg:border-b max-lg:border-r-0">
          <div className="flex items-center gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-md bg-ink text-white">
              PF
            </div>
            <div>
              <p className="text-sm font-semibold uppercase text-signal">
                PlotForge Studio
              </p>
              <h1 className="text-xl font-semibold">Creator Desktop</h1>
            </div>
          </div>

          <div className="mt-6 flex items-center justify-between rounded-md border border-ink/10 bg-parchment px-3 py-2">
            <div>
              <p className="text-xs font-medium uppercase text-ink/60">
                Open Project
              </p>
              <p className="max-w-44 truncate text-sm font-semibold">
                {loadedPath}
              </p>
            </div>
            <button
              type="button"
              title="Open project"
              onClick={() => void loadProject(projectPath)}
              className="grid h-9 w-9 place-items-center rounded-md border border-ink/15 bg-white text-ink transition hover:border-ink/40"
            >
              {loading ? (
                <Loader2 aria-hidden size={18} className="animate-spin" />
              ) : (
                <FolderOpen aria-hidden size={18} />
              )}
            </button>
          </div>

          <nav className="mt-5 grid gap-1">
            {studioSections.map((section) => {
              const Icon = section.icon;
              const selected = section.id === activeSection;
              return (
                <button
                  type="button"
                  key={section.id}
                  title={section.description}
                  onClick={() => setActiveSection(section.id as StudioSectionId)}
                  className={[
                    "flex min-h-12 items-center gap-3 rounded-md px-3 py-2 text-left transition",
                    selected
                      ? "bg-ink text-white"
                      : "text-ink/75 hover:bg-ink/5 hover:text-ink",
                  ].join(" ")}
                >
                  <Icon aria-hidden size={18} className="shrink-0" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">
                      {section.label}
                    </span>
                    <span
                      className={[
                        "block truncate text-xs",
                        selected ? "text-white/65" : "text-ink/45",
                      ].join(" ")}
                    >
                      {section.status}
                    </span>
                  </span>
                  {selected ? <ChevronRight aria-hidden size={16} /> : null}
                </button>
              );
            })}
          </nav>
        </aside>

        <main className="min-w-0 px-6 py-5 lg:px-8">
          <header className="flex flex-wrap items-center justify-between gap-4 border-b border-ink/10 pb-5">
            <div>
              <p className="text-sm font-medium uppercase text-ink/55">
                {dataSource.runtimeName}
              </p>
              <h2 className="mt-1 text-2xl font-semibold">
                {activeSectionMeta.label}
              </h2>
              <p className="mt-1 text-sm text-ink/55">
                {projectSummary?.title ?? "No project loaded"}
              </p>
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <input
                aria-label="Project path"
                value={projectPath}
                onChange={(event) => setProjectPath(event.target.value)}
                className="h-10 min-w-0 rounded-md border border-ink/15 bg-white px-3 text-sm text-ink outline-none transition focus:border-ink/45 sm:w-72"
              />
              <button
                type="button"
                title="Refresh project files"
                onClick={() => void loadProject(projectPath)}
                className="grid h-10 w-10 place-items-center rounded-md border border-ink/15 bg-white text-ink transition hover:border-ink/40"
              >
                {loading ? (
                  <Loader2 aria-hidden size={18} className="animate-spin" />
                ) : (
                  <RefreshCcw aria-hidden size={18} />
                )}
              </button>
              <button
                type="button"
                onClick={() => setActiveSection("playtest")}
                className="inline-flex h-10 items-center gap-2 rounded-md border border-ink/15 bg-white px-4 text-sm font-semibold text-ink transition hover:border-ink/40"
              >
                <Play aria-hidden size={16} />
                Playtest
              </button>
            </div>
          </header>

          <div className="py-5">{renderActiveSection()}</div>
        </main>
      </div>
    </div>
  );
}

const inputClassName =
  "h-10 min-w-0 rounded-md border border-ink/15 bg-parchment px-3 text-sm text-ink outline-none transition focus:border-ink/45";
const textareaClassName =
  "w-full resize-y rounded-md border border-ink/15 bg-parchment px-3 py-2 text-sm leading-6 text-ink outline-none transition focus:border-ink/45";
const panelClassName =
  "rounded-md border border-ink/10 bg-white p-5 shadow-sm";
const secondaryButtonClassName =
  "inline-flex h-9 items-center gap-2 rounded-md border border-ink/15 bg-white px-3 text-sm font-semibold text-ink transition hover:border-ink/40 disabled:cursor-not-allowed disabled:text-ink/30";

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

function defaultStaticExportDir(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}/exports/static`;
}

function defaultStaticArchivePath(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}/exports/static.zip`;
}

function defaultNewProjectPath(projectPath: string) {
  return `${trimTrailingSlashes(projectPath)}-new`;
}

function trimTrailingSlashes(path: string) {
  return path.replace(/\/+$/, "") || ".";
}
