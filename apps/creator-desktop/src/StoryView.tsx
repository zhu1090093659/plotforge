import { Loader2, Save } from "lucide-react";
import type { StoryCraftEditDocument } from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import { CollapsibleSection, studioUiClassNames } from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};

export interface StoryViewProps {
  /** Loaded story craft edit document (null if not loaded). */
  storyCraftEditDocument: StoryCraftEditDocument | null;
  /** True while the story form is saving. */
  saving: boolean;
  /** Cross-section form status for displaying save/error feedback. */
  formStatus: FormStatus | null;
  /** AI generation concept state (lives in main area for visibility). */
  storyGenerationConcept: string;
  onStoryGenerationConceptChange(value: string): void;
  /** Save the current story craft edit document. */
  onSave(): void;
  /** Trigger AI story craft generation from the current concept. */
  onGenerateStoryCraft(): void;
  /** Inline update for story bible fields. */
  onUpdateStoryBible(
    patch: Partial<StoryCraftEditDocument["story_craft"]["bible"]>,
  ): void;
  /** Inline update of top-level story craft document fields. */
  onUpdateStoryCraftDocument(patch: Partial<StoryCraftEditDocument>): void;
}

// ---------------------------------------------------------------------------
// StoryView
// ---------------------------------------------------------------------------

export function StoryView({
  storyCraftEditDocument,
  saving,
  formStatus,
  storyGenerationConcept,
  onStoryGenerationConceptChange,
  onSave,
  onGenerateStoryCraft,
  onUpdateStoryBible,
  onUpdateStoryCraftDocument,
}: StoryViewProps) {
  const { t } = useStudioI18n();
  const bible = storyCraftEditDocument?.story_craft.bible;

  return (
    <section className={studioUiClassNames.panel}>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">{t("story.title")}</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {t("common.plotThreads", {
              count: storyCraftEditDocument?.story_craft.plot_threads.length ?? 0,
            })}
          </p>
        </div>
        <button
          type="button"
          disabled={saving}
          onClick={onSave}
          aria-label={t("story.aria.save")}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {saving ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Save aria-hidden size={16} />
          )}
          {t("story.save")}
        </button>
      </div>

      <SectionStatusMessage section="story" formStatus={formStatus} />

      {storyCraftEditDocument && bible ? (
        <div className="mt-3 grid gap-3">
          <div className="rounded-md border border-ink/10 bg-canvas-50 p-4">
            <div className="flex flex-wrap items-end gap-3">
              <TextareaInput
                label={t("story.aiStoryConcept")}
                ariaLabel={t("story.aria.storyGenerationConcept")}
                value={storyGenerationConcept}
                onChange={onStoryGenerationConceptChange}
                className="min-w-0 flex-1"
                minHeight="min-h-20"
              />
              <button
                type="button"
                onClick={onGenerateStoryCraft}
                disabled={saving}
                className={studioUiClassNames.secondaryButton}
              >
                {t("story.generate")}
              </button>
            </div>
          </div>

          <div className="grid gap-3 lg:grid-cols-2">
            <TextareaInput
              label={t("story.storyBible")}
              ariaLabel={t("story.aria.storyBibleMarkdown")}
              value={storyCraftEditDocument.story_bible_markdown}
              onChange={(value) =>
                onUpdateStoryCraftDocument({ story_bible_markdown: value })
              }
                minHeight="min-h-24"
            />
            <TextareaInput
              label={t("story.styleGuide")}
              ariaLabel={t("story.aria.styleGuideMarkdown")}
              value={storyCraftEditDocument.style_guide_markdown}
              onChange={(value) =>
                onUpdateStoryCraftDocument({ style_guide_markdown: value })
              }
                minHeight="min-h-24"
            />
          </div>

          <div className="grid gap-3 lg:grid-cols-2">
            <TextInput
              label={t("story.genrePromise")}
              ariaLabel={t("story.aria.genrePromise")}
              value={bible.genre_promise}
              onChange={(value) => onUpdateStoryBible({ genre_promise: value })}
            />
            <TextInput
              label={t("story.centralQuestion")}
              ariaLabel={t("story.aria.centralQuestion")}
              value={bible.central_question}
              onChange={(value) =>
                onUpdateStoryBible({ central_question: value })
              }
            />
            <TextareaInput
              label={t("story.targetEmotions")}
              ariaLabel={t("story.aria.targetEmotions")}
              value={listToLines(bible.target_emotions)}
              onChange={(value) =>
                onUpdateStoryBible({ target_emotions: linesToList(value) })
              }
              minHeight="min-h-20"
            />
            <TextareaInput
              label={t("story.coreForeshadowing")}
              ariaLabel={t("story.aria.coreForeshadowing")}
              value={listToLines(bible.core_foreshadowing)}
              onChange={(value) =>
                onUpdateStoryBible({ core_foreshadowing: linesToList(value) })
              }
              minHeight="min-h-20"
            />
          </div>

          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {storyCraftEditDocument.story_craft.plot_threads.map((thread) => (
              <article
                key={thread.id}
                className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3"
              >
                <p className="text-sm font-semibold">{thread.title}</p>
                <p className="mt-1 text-xs font-medium uppercase text-ink/45">
                  {thread.status}
                </p>
                <p className="mt-2 text-sm text-ink/65">{thread.promise}</p>
              </article>
            ))}
          </div>

          <CollapsibleSection title={t("story.advanced")} defaultOpen={false}>
            <div className="grid gap-3 text-sm text-ink/70">
              <p className="text-xs leading-5">{t("story.sourceEditingNote")}</p>
            </div>
          </CollapsibleSection>
        </div>
      ) : (
        <EmptyStory />
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to StoryView)
// ---------------------------------------------------------------------------

function EmptyStory() {
  const { t } = useStudioI18n();
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-canvas-50 px-4 py-6 text-center text-sm text-ink/55">
      {t("story.notLoaded")}
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
}) {
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

function listToLines(values: string[]) {
  return values.join("\n");
}

function linesToList(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}
