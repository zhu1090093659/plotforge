import { Loader2, Save } from "lucide-react";
import type { WorldEditDocument } from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import { CollapsibleSection, studioUiClassNames } from "./studioUi";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

type FormStatus = {
  section: StudioSectionId;
  tone: "success" | "error";
  message: string;
};

export interface WorldViewProps {
  /** Loaded world edit document (null if not loaded). */
  worldEditDocument: WorldEditDocument | null;
  /** True while the world form is saving. */
  saving: boolean;
  /** Cross-section form status for displaying save/error feedback. */
  formStatus: FormStatus | null;
  /** AI expansion goal state (lives in "Advanced" collapsible). */
  worldExpansionGoal: string;
  onWorldExpansionGoalChange(value: string): void;
  /** Save the current world edit document. */
  onSave(): void;
  /** Trigger AI world expansion from the current goal. */
  onGenerateWorldExpansion(): void;
  /** Inline update for world document fields. */
  onUpdateWorldDocument(patch: Partial<WorldEditDocument>): void;
}

// ---------------------------------------------------------------------------
// WorldView
// ---------------------------------------------------------------------------

export function WorldView({
  worldEditDocument,
  saving,
  formStatus,
  worldExpansionGoal,
  onWorldExpansionGoalChange,
  onSave,
  onGenerateWorldExpansion,
  onUpdateWorldDocument,
}: WorldViewProps) {
  return (
    <section className={studioUiClassNames.panel}>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">World Bible</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {worldEditDocument?.forbidden_facts.length ?? 0} forbidden facts
          </p>
        </div>
        <button
          type="button"
          disabled={saving}
          onClick={onSave}
          aria-label="Save World Bible"
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {saving ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Save aria-hidden size={16} />
          )}
          Save World Bible
        </button>
      </div>

      <SectionStatusMessage section="world" formStatus={formStatus} />

      {worldEditDocument ? (
        <div className="mt-3 grid gap-3">
          <div className="grid gap-3 lg:grid-cols-2">
            <TextareaInput
              label="World Bible"
              ariaLabel="World bible markdown"
              value={worldEditDocument.world_bible_markdown}
              onChange={(value) =>
                onUpdateWorldDocument({ world_bible_markdown: value })
              }
              minHeight="min-h-40"
            />
            <div className="grid gap-3">
              <TextareaInput
                label="Canon"
                ariaLabel="Canon markdown"
                value={worldEditDocument.canon_markdown}
                onChange={(value) =>
                  onUpdateWorldDocument({ canon_markdown: value })
                }
                minHeight="min-h-28"
              />
              <TextareaInput
                label="Forbidden facts"
                ariaLabel="Forbidden facts"
                value={listToLines(worldEditDocument.forbidden_facts)}
                onChange={(value) =>
                  onUpdateWorldDocument({ forbidden_facts: linesToList(value) })
                }
                minHeight="min-h-28"
              />
            </div>
          </div>

          {/* Advanced: AI expansion goal — collapsed by default */}
          <CollapsibleSection title="Advanced" defaultOpen={false}>
            <div className="flex flex-wrap items-end gap-3">
              <TextInput
                label="AI expansion goal"
                ariaLabel="World generation goal"
                value={worldExpansionGoal}
                onChange={onWorldExpansionGoalChange}
                className="min-w-0 flex-1"
              />
              <button
                type="button"
                onClick={onGenerateWorldExpansion}
                disabled={saving}
                className={studioUiClassNames.secondaryButton}
              >
                Generate World Expansion
              </button>
            </div>
          </CollapsibleSection>
        </div>
      ) : (
        <EmptyWorld />
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to WorldView)
// ---------------------------------------------------------------------------

function EmptyWorld() {
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-parchment px-4 py-6 text-center text-sm text-ink/55">
      World edit document not loaded.
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
      ? "border-jade/30 bg-jade/10 text-jade"
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
