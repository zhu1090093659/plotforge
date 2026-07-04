import { Loader2, Save } from "lucide-react";
import { useState, type ReactNode } from "react";
import type {
  Character,
  CharacterDraft,
  CharacterEditDocument,
} from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import {
  Collapsible,
  studioUiClassNames,
} from "./studioUi";
import {
  PaginationControls,
  PaginatedCardGrid,
  usePagination,
} from "./pagination";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};

export interface CharactersViewProps {
  /** Loaded character edit document (null if not loaded). */
  characterEditDocument: CharacterEditDocument | null;
  /** True while the characters form is saving. */
  saving: boolean;
  /** Cross-section form status for displaying save/error feedback. */
  formStatus: FormStatus | null;
  /** AI generation concept state. */
  characterGenerationConcept: string;
  onCharacterGenerationConceptChange(value: string): void;
  /** AI generation role hint state. */
  characterGenerationRoleHint: string;
  onCharacterGenerationRoleHintChange(value: string): void;
  /** Current manual-creation draft state. */
  characterDraft: CharacterDraft;
  onCharacterDraftChange(draft: CharacterDraft): void;
  /** Save existing characters. */
  onSave(): void;
  /** Generate a character from concept + role hint using AI. */
  onGenerateCharacter(): void;
  /** Create a new character from the manual draft via the Rust bridge. */
  onCreateCharacterFromDraft(): void;
  /** Inline update for an existing character. */
  onUpdateCharacter(index: number, patch: Partial<Character>): void;
}

// ---------------------------------------------------------------------------
// CharactersView
// ---------------------------------------------------------------------------

export function CharactersView({
  characterEditDocument,
  saving,
  formStatus,
  characterGenerationConcept,
  onCharacterGenerationConceptChange,
  characterGenerationRoleHint,
  onCharacterGenerationRoleHintChange,
  characterDraft,
  onCharacterDraftChange,
  onSave,
  onGenerateCharacter,
  onCreateCharacterFromDraft,
  onUpdateCharacter,
}: CharactersViewProps) {
  const { t } = useStudioI18n();
  const [addMode, setAddMode] = useState<"ai" | "manual">("manual");

  return (
    <section className={studioUiClassNames.panel}>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">{t("characters.title")}</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {t("common.records", {
              count: characterEditDocument?.characters.length ?? 0,
            })}
          </p>
        </div>
        <button
          type="button"
          disabled={saving}
          onClick={onSave}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {saving ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Save aria-hidden size={16} />
          )}
          {t("characters.save")}
        </button>
      </div>

      <SectionStatusMessage section="characters" formStatus={formStatus} />

      {characterEditDocument ? (
        <div className="mt-3 grid gap-3">
          <CharacterCardGrid
            characters={characterEditDocument.characters}
            onUpdateCharacter={onUpdateCharacter}
          />

          <Collapsible
            label={t("characters.addCharacter")}
            defaultOpen={false}
            className="rounded-md border border-ink/10 bg-canvas-50 p-4"
          >
            <div className="mt-3 flex gap-2">
              <button
                type="button"
                aria-pressed={addMode === "manual"}
                onClick={() => setAddMode("manual")}
                className={[
                  "h-8 rounded-md border px-3 text-xs font-semibold transition",
                  addMode === "manual"
                    ? "border-accent-400/35 bg-accent-500/15 text-ink"
                    : "border-graphite-700/20 bg-canvas-50 text-ink/60 hover:border-graphite-700/45",
                ].join(" ")}
              >
                {t("characters.manual")}
              </button>
              <button
                type="button"
                aria-pressed={addMode === "ai"}
                onClick={() => setAddMode("ai")}
                className={[
                  "h-8 rounded-md border px-3 text-xs font-semibold transition",
                  addMode === "ai"
                    ? "border-accent-400/35 bg-accent-500/15 text-ink"
                    : "border-graphite-700/20 bg-canvas-50 text-ink/60 hover:border-graphite-700/45",
                ].join(" ")}
              >
                {t("characters.aiGenerate")}
              </button>
            </div>

            {addMode === "manual" ? (
              <ManualCharacterForm
                draft={characterDraft}
                saving={saving}
                onDraftChange={onCharacterDraftChange}
                onSubmit={onCreateCharacterFromDraft}
              />
            ) : (
              <AiCharacterForm
                concept={characterGenerationConcept}
                roleHint={characterGenerationRoleHint}
                saving={saving}
                onConceptChange={onCharacterGenerationConceptChange}
                onRoleHintChange={onCharacterGenerationRoleHintChange}
                onSubmit={onGenerateCharacter}
              />
            )}
          </Collapsible>
        </div>
      ) : (
        <EmptyCharacters />
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// CharacterCardGrid — paginated character list.
//
// Keeps the original array index stable across pages so `onUpdateCharacter`
// patches the correct character even when the visible slice is a later page.
// ---------------------------------------------------------------------------

function CharacterCardGrid({
  characters,
  onUpdateCharacter,
}: {
  characters: Character[];
  onUpdateCharacter(index: number, patch: Partial<Character>): void;
}) {
  const { t } = useStudioI18n();
  const indexed = characters.map((character, index) => ({ character, index }));
  const {
    query,
    setQuery,
    page,
    setPage,
    totalPages,
    filteredCount,
    visible,
    needsControls,
  } = usePagination(indexed, {
    filter: ({ character }, q) =>
      [character.id, character.name, character.role]
        .filter(Boolean)
        .some((field) => field.toLowerCase().includes(q.toLowerCase())),
  });
  return (
    <PaginatedCardGrid
      controls={
        <PaginationControls
          needsControls={needsControls}
          query={query}
          setQuery={setQuery}
          page={page}
          setPage={setPage}
          totalPages={totalPages}
          filteredCount={filteredCount}
          searchAriaLabel={t("pagination.aria.searchCharacters")}
          searchPlaceholder={t("pagination.searchCharacters")}
        />
      }
    >
      {visible.map(({ character, index }) => (
        <CharacterCard
          key={`${character.id}:${index}`}
          character={character}
          index={index}
          onUpdate={(patch) => onUpdateCharacter(index, patch)}
        />
      ))}
    </PaginatedCardGrid>
  );
}

// ---------------------------------------------------------------------------
// CharacterCard — collapsed summary with expandable detail edit
// ---------------------------------------------------------------------------

function CharacterCard({
  character,
  index,
  onUpdate,
}: {
  character: Character;
  index: number;
  onUpdate(patch: Partial<Character>): void;
}) {
  const { t } = useStudioI18n();
  return (
    <article className="rounded-md border border-ink/10 bg-canvas-50 p-4">
      <Collapsible
        label={character.name || character.id}
        id={`character-${character.id}`}
        defaultOpen={false}
        badge={character.role || undefined}
      >
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <TextInput
            label={t("characters.characterId")}
            ariaLabel={t("characters.aria.charIdN", { index: index + 1 })}
            value={character.id}
            onChange={(value) => onUpdate({ id: value })}
          />
          <TextInput
            label={t("characters.name")}
            ariaLabel={t("characters.aria.charNameN", { index: index + 1 })}
            value={character.name}
            onChange={(value) => onUpdate({ name: value })}
          />
          <TextInput
            label={t("characters.role")}
            ariaLabel={t("characters.aria.charRoleN", { index: index + 1 })}
            value={character.role}
            onChange={(value) => onUpdate({ role: value })}
          />
          <TextareaInput
            label={t("characters.traits")}
            ariaLabel={t("characters.aria.charTraitsN", { index: index + 1 })}
            value={character.traits.join("\n")}
            onChange={(value) =>
              onUpdate({
                traits: value
                  .split(/\r?\n/)
                  .map((l) => l.trim())
                  .filter(Boolean),
              })
            }
            minHeight="min-h-20"
          />
          <TextareaInput
            label={t("characters.visualCard")}
            ariaLabel={t("characters.aria.visualCardN", { index: index + 1 })}
            placeholder={t("characters.placeholder.visualCardExisting")}
            value={character.visual_card}
            onChange={(value) => onUpdate({ visual_card: value })}
            minHeight="min-h-20"
          />
          <TextareaInput
            label={t("characters.voiceCard")}
            ariaLabel={t("characters.aria.voiceCardN", { index: index + 1 })}
            placeholder={t("characters.placeholder.voiceCardExisting")}
            value={character.voice_card}
            onChange={(value) => onUpdate({ voice_card: value })}
            minHeight="min-h-20"
          />
          {character.portrait_request ? (
            <div className="sm:col-span-2 rounded-md border border-ink/10 bg-canvas-50 px-3 py-2 text-sm">
              <p className="text-xs font-medium uppercase text-ink/55">
                {t("characters.portraitRequest")}
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
      </Collapsible>
    </article>
  );
}

// ---------------------------------------------------------------------------
// ManualCharacterForm
// ---------------------------------------------------------------------------

function ManualCharacterForm({
  draft,
  saving,
  onDraftChange,
  onSubmit,
}: {
  draft: CharacterDraft;
  saving: boolean;
  onDraftChange(draft: CharacterDraft): void;
  onSubmit(): void;
}) {
  const { t } = useStudioI18n();
  return (
    <div className="mt-3 grid gap-3 lg:grid-cols-2">
      <TextInput
        label={t("characters.characterId")}
        ariaLabel={t("characters.aria.newCharId")}
        placeholder={t("characters.placeholder.charId")}
        value={draft.id}
        onChange={(value) => onDraftChange({ ...draft, id: value })}
      />
      <TextInput
        label={t("characters.name")}
        ariaLabel={t("characters.aria.newCharName")}
        placeholder={t("characters.placeholder.name")}
        value={draft.name}
        onChange={(value) => onDraftChange({ ...draft, name: value })}
      />
      <TextInput
        label={t("characters.role")}
        ariaLabel={t("characters.aria.newCharRole")}
        placeholder={t("characters.placeholder.role")}
        value={draft.role}
        onChange={(value) => onDraftChange({ ...draft, role: value })}
      />
      <TextareaInput
        label={t("characters.traits")}
        ariaLabel={t("characters.aria.newCharTraits")}
        placeholder={t("characters.placeholder.traits")}
        value={draft.traits_text}
        onChange={(value) => onDraftChange({ ...draft, traits_text: value })}
        minHeight="min-h-24"
      />
      <TextareaInput
        label={t("characters.visualCard")}
        ariaLabel={t("characters.aria.newVisualCard")}
        placeholder={t("characters.placeholder.visualCard")}
        value={draft.visual_card}
        onChange={(value) => onDraftChange({ ...draft, visual_card: value })}
        minHeight="min-h-24"
      />
      <TextareaInput
        label={t("characters.voiceCard")}
        ariaLabel={t("characters.aria.newVoiceCard")}
        placeholder={t("characters.placeholder.voiceCard")}
        value={draft.voice_card}
        onChange={(value) => onDraftChange({ ...draft, voice_card: value })}
        minHeight="min-h-24"
      />
      <div className="lg:col-span-2 flex justify-end">
        <button
          type="button"
          onClick={onSubmit}
          disabled={saving}
          className={studioUiClassNames.secondaryButton}
        >
          {t("characters.create")}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// AiCharacterForm
// ---------------------------------------------------------------------------

function AiCharacterForm({
  concept,
  roleHint,
  saving,
  onConceptChange,
  onRoleHintChange,
  onSubmit,
}: {
  concept: string;
  roleHint: string;
  saving: boolean;
  onConceptChange(value: string): void;
  onRoleHintChange(value: string): void;
  onSubmit(): void;
}) {
  const { t } = useStudioI18n();
  return (
    <div className="mt-3 flex flex-wrap items-end gap-3">
      <TextareaInput
        label={t("characters.aiCharacterConcept")}
        ariaLabel={t("characters.aria.characterGenerationConcept")}
        placeholder={t("characters.placeholder.concept")}
        value={concept}
        onChange={onConceptChange}
        className="min-w-0 flex-[2]"
        minHeight="min-h-20"
      />
      <TextInput
        label={t("characters.roleHint")}
        ariaLabel={t("characters.aria.characterGenerationRoleHint")}
        placeholder={t("characters.placeholder.roleHint")}
        value={roleHint}
        onChange={onRoleHintChange}
        className="min-w-56"
      />
      <button
        type="button"
        onClick={onSubmit}
        disabled={saving}
        className={studioUiClassNames.secondaryButton}
      >
        {t("characters.generate")}
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to CharactersView)
// ---------------------------------------------------------------------------

function EmptyCharacters() {
  const { t } = useStudioI18n();
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-canvas-50 px-4 py-6 text-center text-sm text-ink/55">
      {t("characters.notLoaded")}
    </div>
  );
}

function SectionStatusMessage({
  section,
  formStatus,
}: {
  section: StudioSectionId;
  formStatus: FormStatus | null;
}) {
  if (!formStatus || formStatus.section !== section) {
    return null;
  }
  const toneClass =
    formStatus.tone === "success"
      ? "border-sage/30 bg-sage/10 text-sage"
      : "border-signal/30 bg-signal/10 text-signal";
  return (
    <div className={`mt-4 rounded-md border px-3 py-2 text-sm ${toneClass}`}>
      {formStatus.message}
    </div>
  );
}

function TextInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
  placeholder,
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
  placeholder?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <input
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className={studioUiClassNames.input}
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
  minHeight = "",
  placeholder,
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
  minHeight?: string;
  placeholder?: string;
}): ReactNode {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <textarea
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        className={`${studioUiClassNames.textarea} ${minHeight}`}
      />
    </label>
  );
}
