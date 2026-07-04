import type { WorldEditDocument } from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import {
  CollapsibleSection,
  EmptyState,
  Reveal,
  SaveButton,
  SectionStatusMessage,
  TextareaInput,
  TextInput,
  ViewHeader,
  studioUiClassNames,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

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
  const { t } = useStudioI18n();
  return (
    <Reveal as="section" className={`${studioUiClassNames.panel} grid`}>
      <ViewHeader
        title={t("world.title")}
        subtitle={t("common.forbiddenFacts", {
          count: worldEditDocument?.forbidden_facts.length ?? 0,
        })}
        actions={
          <SaveButton
            saving={saving}
            onSave={onSave}
            label={t("world.save")}
            ariaLabel={t("world.aria.save")}
          />
        }
      />

      <SectionStatusMessage section="world" formStatus={formStatus} />

      {worldEditDocument ? (
        <div className="mt-3 grid gap-3">
          <div className="grid gap-3 lg:grid-cols-2">
            <TextareaInput
              label={t("world.worldBibleMarkdown")}
              ariaLabel={t("world.aria.worldBibleMarkdown")}
              value={worldEditDocument.world_bible_markdown}
              onChange={(value) =>
                onUpdateWorldDocument({ world_bible_markdown: value })
              }
              minHeight="min-h-28"
            />
            <div className="grid gap-3">
              <TextareaInput
                label={t("world.canon")}
                ariaLabel={t("world.aria.canonMarkdown")}
                value={worldEditDocument.canon_markdown}
                onChange={(value) =>
                  onUpdateWorldDocument({ canon_markdown: value })
                }
                minHeight="min-h-20"
              />
              <TextareaInput
                label={t("world.forbiddenFacts")}
                ariaLabel={t("world.aria.forbiddenFacts")}
                value={listToLines(worldEditDocument.forbidden_facts)}
                onChange={(value) =>
                  onUpdateWorldDocument({ forbidden_facts: linesToList(value) })
                }
                minHeight="min-h-20"
              />
            </div>
          </div>

          <CollapsibleSection title={t("world.advanced")} defaultOpen={false}>
            <div className="flex flex-wrap items-end gap-3">
              <TextInput
                label={t("world.aiExpansionGoal")}
                ariaLabel={t("world.aria.worldGenerationGoal")}
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
                {t("world.generate")}
              </button>
            </div>
          </CollapsibleSection>
        </div>
      ) : (
        <EmptyState>{t("world.notLoaded")}</EmptyState>
      )}
    </Reveal>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to WorldView)
// ---------------------------------------------------------------------------

function listToLines(values: string[]) {
  return values.join("\n");
}

function linesToList(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}
