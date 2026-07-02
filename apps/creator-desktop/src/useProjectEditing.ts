import { useState, type Dispatch, type SetStateAction } from "react";
import type {
  AiSafetyPolicy,
  AudioBible,
  AudioVoiceCard,
  Character,
  CharacterDraft,
  CharacterEditDocument,
  ProjectData,
  ResourceDefinition,
  Rule,
  RulesEditDocument,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  VisualBible,
  VisualStyleCard,
  WorldEditDocument,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { errorMessage } from "./errorMessage";
import { useStudioI18n } from "./i18n";
import type { StudioSectionId } from "./studioModel";
import type { ResourceDraft } from "./StateView";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface FormStatus {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
}

export interface RuleDraft {
  id: string;
  action_type: string;
  resource_key: string;
  amount: number;
}

/** All editable documents loaded at project-open time. */
export interface EditingDocuments {
  worldEditDocument: WorldEditDocument | null;
  storyCraftEditDocument: StoryCraftEditDocument | null;
  characterEditDocument: CharacterEditDocument | null;
  stateVariablesEditDocument: StateVariablesEditDocument | null;
  rulesEditDocument: RulesEditDocument | null;
  aiSafetyPolicy: AiSafetyPolicy | null;
  visualBible: VisualBible | null;
  audioBible: AudioBible | null;
}

export interface UseProjectEditingOptions {
  dataSource: StudioDataSource;
  /** Called after a generation action to refresh the project overview. */
  onRefreshProjectOverview?: (path?: string) => Promise<void>;
  /** Called after bible/policy saves to keep projectData in sync. */
  onSetProjectData?: Dispatch<SetStateAction<ProjectData | null>>;
}

export interface EditingWorkspace {
  // Edit documents ------------------------------------------------
  worldEditDocument: WorldEditDocument | null;
  setWorldEditDocument: Dispatch<SetStateAction<WorldEditDocument | null>>;
  storyCraftEditDocument: StoryCraftEditDocument | null;
  setStoryCraftEditDocument: Dispatch<
    SetStateAction<StoryCraftEditDocument | null>
  >;
  characterEditDocument: CharacterEditDocument | null;
  setCharacterEditDocument: Dispatch<
    SetStateAction<CharacterEditDocument | null>
  >;
  stateVariablesEditDocument: StateVariablesEditDocument | null;
  setStateVariablesEditDocument: Dispatch<
    SetStateAction<StateVariablesEditDocument | null>
  >;
  rulesEditDocument: RulesEditDocument | null;
  setRulesEditDocument: Dispatch<SetStateAction<RulesEditDocument | null>>;
  aiSafetyPolicy: AiSafetyPolicy | null;
  setAiSafetyPolicy: Dispatch<SetStateAction<AiSafetyPolicy | null>>;
  visualBible: VisualBible | null;
  setVisualBible: Dispatch<SetStateAction<VisualBible | null>>;
  audioBible: AudioBible | null;
  setAudioBible: Dispatch<SetStateAction<AudioBible | null>>;

  // Draft state ----------------------------------------------------
  newCharacter: CharacterDraft;
  setNewCharacter: Dispatch<SetStateAction<CharacterDraft>>;
  newResource: ResourceDraft;
  setNewResource: Dispatch<SetStateAction<ResourceDraft>>;
  newRule: RuleDraft;
  setNewRule: Dispatch<SetStateAction<RuleDraft>>;
  worldExpansionGoal: string;
  setWorldExpansionGoal: Dispatch<SetStateAction<string>>;
  storyGenerationConcept: string;
  setStoryGenerationConcept: Dispatch<SetStateAction<string>>;
  characterGenerationConcept: string;
  setCharacterGenerationConcept: Dispatch<SetStateAction<string>>;
  characterGenerationRoleHint: string;
  setCharacterGenerationRoleHint: Dispatch<SetStateAction<string>>;

  // Form state -----------------------------------------------------
  formSaving: StudioSectionId | null;
  formStatus: FormStatus | null;

  // Form actions (all take loadedPath) -----------------------------
  saveWorldEditDocument(loadedPath: string): Promise<void>;
  generateWorldExpansionFromGoal(loadedPath: string): Promise<void>;
  saveStoryCraftEditDocument(loadedPath: string): Promise<void>;
  generateStoryCraftFromConcept(loadedPath: string): Promise<void>;
  saveCharacterEditDocument(loadedPath: string): Promise<void>;
  createCharacterFromDraft(loadedPath: string): Promise<void>;
  generateCharacterFromConcept(loadedPath: string): Promise<void>;
  saveStateVariablesEditDocument(loadedPath: string): Promise<void>;
  createResourceFromDraft(loadedPath: string): Promise<void>;
  saveRulesEditDocument(loadedPath: string): Promise<void>;
  createRuleFromDraft(loadedPath: string): Promise<void>;
  saveAiSafetyPolicy(loadedPath: string): Promise<void>;
  saveVisualBible(loadedPath: string): Promise<void>;
  saveAudioBible(loadedPath: string): Promise<void>;

  // Update helpers -------------------------------------------------
  updateWorldDocument(patch: Partial<WorldEditDocument>): void;
  updateStoryBible(
    patch: Partial<StoryCraftEditDocument["story_craft"]["bible"]>,
  ): void;
  updateStoryCraftDocument(patch: Partial<StoryCraftEditDocument>): void;
  updateCharacter(index: number, patch: Partial<Character>): void;
  updateResource(index: number, patch: Partial<ResourceDefinition>): void;
  updateInitialWorldResource(key: string, value: number): void;
  updateInitialSceneKey(value: string): void;
  updateInitialTurn(value: number): void;
  updateRule(index: number, patch: Partial<Rule>): void;
  updateAiSafetyPolicy(patch: Partial<AiSafetyPolicy>): void;
  updateVisualStyleCard(
    index: number,
    patch: Partial<VisualStyleCard>,
  ): void;
  updateAudioVoiceCard(index: number, patch: Partial<AudioVoiceCard>): void;

  // Lifecycle ------------------------------------------------------
  /** Batch-set all editing documents after project load. */
  loadEditingDocuments(docs: EditingDocuments): void;
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useProjectEditing({
  dataSource,
  onRefreshProjectOverview,
  onSetProjectData,
}: UseProjectEditingOptions): EditingWorkspace {
  const { t } = useStudioI18n();
  // Edit documents
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
  const [visualBible, setVisualBible] = useState<VisualBible | null>(null);
  const [audioBible, setAudioBible] = useState<AudioBible | null>(null);

  // Draft state
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

  // Form state
  const [formSaving, setFormSaving] = useState<StudioSectionId | null>(null);
  const [formStatus, setFormStatus] = useState<FormStatus | null>(null);

  // Generic form-action wrapper
  async function runFormAction(
    section: StudioSectionId,
    successKey: string,
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
          message: t(successKey),
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

  // Form actions ----------------------------------------------------

  async function saveWorldEditDocument(loadedPath: string) {
    if (!worldEditDocument) return;
    await runFormAction("world", "form.worldSaved", async () => {
      const updated = await dataSource.updateWorldEditDocument(
        loadedPath,
        worldEditDocument,
      );
      setWorldEditDocument(updated);
    });
  }

  async function generateWorldExpansionFromGoal(loadedPath: string) {
    const goal = worldExpansionGoal.trim();
    if (!goal) {
      setFormStatus({
        section: "world",
        tone: "error",
        message: t("form.worldGoalRequired"),
      });
      return;
    }

    await runFormAction("world", "form.worldGenerationApplied", async () => {
      const report = await dataSource.generateWorldExpansion(
        loadedPath,
        goal,
      );
      setWorldEditDocument(report.document);
      await onRefreshProjectOverview?.(loadedPath);
      return {
        section: "world",
        tone: report.evidence.fallback_used ? "error" : "success",
        message: t("form.worldGenerationStatus", {
          status: report.evidence.status,
          fallback: report.evidence.fallback_used ? t("form.withVisibleFallback") : "",
        }),
      };
    });
  }

  async function saveStoryCraftEditDocument(loadedPath: string) {
    if (!storyCraftEditDocument) return;
    await runFormAction("story", "form.storySaved", async () => {
      const updated = await dataSource.updateStoryCraftEditDocument(
        loadedPath,
        storyCraftEditDocument,
      );
      setStoryCraftEditDocument(updated);
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function generateStoryCraftFromConcept(loadedPath: string) {
    const concept = storyGenerationConcept.trim();
    if (!concept) {
      setFormStatus({
        section: "story",
        tone: "error",
        message: t("form.storyConceptRequired"),
      });
      return;
    }

    await runFormAction("story", "form.storyGenerationApplied", async () => {
      const report = await dataSource.generateStoryCraft(
        loadedPath,
        concept,
      );
      setStoryCraftEditDocument(report.document);
      await onRefreshProjectOverview?.(loadedPath);
      return {
        section: "story",
        tone: report.evidence.fallback_used ? "error" : "success",
        message: t("form.storyGenerationStatus", {
          status: report.evidence.status,
          fallback: report.evidence.fallback_used ? t("form.withVisibleFallback") : "",
        }),
      };
    });
  }

  async function saveCharacterEditDocument(loadedPath: string) {
    if (!characterEditDocument) return;
    await runFormAction("characters", "form.charactersSaved", async () => {
      const updated = await dataSource.updateCharacterEditDocument(
        loadedPath,
        characterEditDocument,
      );
      setCharacterEditDocument(updated);
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function createCharacterFromDraft(loadedPath: string) {
    await runFormAction("characters", "form.characterCreated", async () => {
      const updated = await dataSource.createCharacterFromDraft(
        loadedPath,
        newCharacter,
      );
      setCharacterEditDocument(updated);
      setNewCharacter(emptyCharacterDraft());
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function generateCharacterFromConcept(loadedPath: string) {
    const concept = characterGenerationConcept.trim();
    const roleHint = characterGenerationRoleHint.trim();
    if (!concept || !roleHint) {
      setFormStatus({
        section: "characters",
        tone: "error",
        message: t("form.characterConceptRequired"),
      });
      return;
    }

    await runFormAction(
      "characters",
      "form.characterGenerationApplied",
      async () => {
        const report = await dataSource.generateCharacter(
          loadedPath,
          concept,
          roleHint,
        );
        setCharacterEditDocument((current) => ({
          characters: [
            ...(current?.characters.filter(
              (c) => c.id !== report.character.id,
            ) ?? []),
            report.character,
          ],
        }));
        await onRefreshProjectOverview?.(loadedPath);
        return {
          section: "characters",
          tone: report.evidence.fallback_used ? "error" : "success",
          message: t("form.characterGenerationStatus", {
            status: report.evidence.status,
            fallback: report.evidence.fallback_used ? t("form.withVisibleFallback") : "",
          }),
        };
      },
    );
  }

  async function saveStateVariablesEditDocument(loadedPath: string) {
    if (!stateVariablesEditDocument) return;
    await runFormAction("state", "form.stateSaved", async () => {
      const updated = await dataSource.updateStateVariablesEditDocument(
        loadedPath,
        stateVariablesEditDocument,
      );
      setStateVariablesEditDocument(updated);
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function createResourceFromDraft(loadedPath: string) {
    const resource: ResourceDefinition = {
      key: newResource.key.trim(),
      label: newResource.label.trim(),
      initial: newResource.initial,
      min: newResource.min,
      max: newResource.max,
    };

    await runFormAction("state", "form.resourceCreated", async () => {
      const updated = await dataSource.createResource(
        loadedPath,
        resource,
      );
      setStateVariablesEditDocument(updated);
      setNewResource(emptyResourceDraft());
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function saveRulesEditDocument(loadedPath: string) {
    if (!rulesEditDocument) return;
    await runFormAction("rules", "form.rulesSaved", async () => {
      const updated = await dataSource.updateRulesEditDocument(
        loadedPath,
        rulesEditDocument,
      );
      setRulesEditDocument(updated);
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function createRuleFromDraft(loadedPath: string) {
    await runFormAction("rules", "form.ruleCreated", async () => {
      const updated = await dataSource.createRuleFromDraft(
        loadedPath,
        newRule,
      );
      setRulesEditDocument(updated);
      setNewRule(emptyRuleDraft());
      await onRefreshProjectOverview?.(loadedPath);
    });
  }

  async function saveAiSafetyPolicy(loadedPath: string) {
    if (!aiSafetyPolicy) return;
    await runFormAction(
      "export-kit",
      "form.aiSafetyPolicySaved",
      async () => {
        const updated = await dataSource.updateAiSafetyPolicy(
          loadedPath,
          aiSafetyPolicy,
        );
        setAiSafetyPolicy(updated);
        onSetProjectData?.((current) =>
          current ? { ...current, ai_safety_policy: updated } : current,
        );
      },
    );
  }

  async function saveVisualBible(loadedPath: string) {
    if (!visualBible) return;
    await runFormAction("assets", "form.visualBibleSaved", async () => {
      const updated = await dataSource.updateVisualBible(
        loadedPath,
        visualBible,
      );
      setVisualBible(updated);
      onSetProjectData?.((current) =>
        current ? { ...current, visual_bible: updated } : current,
      );
    });
  }

  async function saveAudioBible(loadedPath: string) {
    if (!audioBible) return;
    await runFormAction("assets", "form.audioBibleSaved", async () => {
      const updated = await dataSource.updateAudioBible(
        loadedPath,
        audioBible,
      );
      setAudioBible(updated);
      onSetProjectData?.((current) =>
        current ? { ...current, audio_bible: updated } : current,
      );
    });
  }

  // Update helpers --------------------------------------------------

  function updateWorldDocument(patch: Partial<WorldEditDocument>) {
    if (!worldEditDocument) return;
    setWorldEditDocument({ ...worldEditDocument, ...patch });
  }

  function updateStoryBible(
    patch: Partial<StoryCraftEditDocument["story_craft"]["bible"]>,
  ) {
    if (!storyCraftEditDocument) return;
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

  function updateStoryCraftDocument(
    patch: Partial<StoryCraftEditDocument>,
  ) {
    if (!storyCraftEditDocument) return;
    setStoryCraftEditDocument({
      ...storyCraftEditDocument,
      ...patch,
    });
  }

  function updateCharacter(index: number, patch: Partial<Character>) {
    if (!characterEditDocument) return;
    setCharacterEditDocument({
      characters: characterEditDocument.characters.map(
        (character, current) =>
          current === index ? { ...character, ...patch } : character,
      ),
    });
  }

  function updateResource(
    index: number,
    patch: Partial<ResourceDefinition>,
  ) {
    if (!stateVariablesEditDocument) return;
    setStateVariablesEditDocument({
      ...stateVariablesEditDocument,
      resources: stateVariablesEditDocument.resources.map(
        (resource, current) =>
          current === index ? { ...resource, ...patch } : resource,
      ),
    });
  }

  function updateInitialWorldResource(key: string, value: number) {
    if (!stateVariablesEditDocument) return;
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

  function updateInitialSceneKey(value: string) {
    if (!stateVariablesEditDocument) return;
    setStateVariablesEditDocument({
      ...stateVariablesEditDocument,
      initial_story_state: {
        ...stateVariablesEditDocument.initial_story_state,
        current_scene_key: value,
      },
    });
  }

  function updateInitialTurn(value: number) {
    if (!stateVariablesEditDocument) return;
    setStateVariablesEditDocument({
      ...stateVariablesEditDocument,
      initial_story_state: {
        ...stateVariablesEditDocument.initial_story_state,
        turn: value,
      },
    });
  }

  function updateRule(index: number, patch: Partial<Rule>) {
    if (!rulesEditDocument) return;
    setRulesEditDocument({
      rules: rulesEditDocument.rules.map((rule, current) =>
        current === index ? { ...rule, ...patch } : rule,
      ),
    });
  }

  function updateAiSafetyPolicy(patch: Partial<AiSafetyPolicy>) {
    if (!aiSafetyPolicy) return;
    setAiSafetyPolicy({ ...aiSafetyPolicy, ...patch });
  }

  function updateVisualStyleCard(
    index: number,
    patch: Partial<VisualStyleCard>,
  ) {
    if (!visualBible) return;
    setVisualBible({
      style_cards: visualBible.style_cards.map((card, current) =>
        current === index ? { ...card, ...patch } : card,
      ),
    });
  }

  function updateAudioVoiceCard(
    index: number,
    patch: Partial<AudioVoiceCard>,
  ) {
    if (!audioBible) return;
    setAudioBible({
      voice_cards: audioBible.voice_cards.map((card, current) =>
        current === index ? { ...card, ...patch } : card,
      ),
    });
  }

  // Lifecycle -------------------------------------------------------

  function loadEditingDocuments(docs: EditingDocuments) {
    setWorldEditDocument(docs.worldEditDocument);
    setStoryCraftEditDocument(docs.storyCraftEditDocument);
    setCharacterEditDocument(docs.characterEditDocument);
    setStateVariablesEditDocument(docs.stateVariablesEditDocument);
    setRulesEditDocument(docs.rulesEditDocument);
    setAiSafetyPolicy(docs.aiSafetyPolicy);
    setVisualBible(docs.visualBible);
    setAudioBible(docs.audioBible);
    setFormStatus(null);
    setFormSaving(null);
  }

  return {
    // Edit documents
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
    visualBible,
    setVisualBible,
    audioBible,
    setAudioBible,

    // Draft state
    newCharacter,
    setNewCharacter,
    newResource,
    setNewResource,
    newRule,
    setNewRule,
    worldExpansionGoal,
    setWorldExpansionGoal,
    storyGenerationConcept,
    setStoryGenerationConcept,
    characterGenerationConcept,
    setCharacterGenerationConcept,
    characterGenerationRoleHint,
    setCharacterGenerationRoleHint,

    // Form state
    formSaving,
    formStatus,

    // Form actions
    saveWorldEditDocument,
    generateWorldExpansionFromGoal,
    saveStoryCraftEditDocument,
    generateStoryCraftFromConcept,
    saveCharacterEditDocument,
    createCharacterFromDraft,
    generateCharacterFromConcept,
    saveStateVariablesEditDocument,
    createResourceFromDraft,
    saveRulesEditDocument,
    createRuleFromDraft,
    saveAiSafetyPolicy,
    saveVisualBible,
    saveAudioBible,

    // Update helpers
    updateWorldDocument,
    updateStoryBible,
    updateStoryCraftDocument,
    updateCharacter,
    updateResource,
    updateInitialWorldResource,
    updateInitialSceneKey,
    updateInitialTurn,
    updateRule,
    updateAiSafetyPolicy,
    updateVisualStyleCard,
    updateAudioVoiceCard,

    // Lifecycle
    loadEditingDocuments,
  };
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
