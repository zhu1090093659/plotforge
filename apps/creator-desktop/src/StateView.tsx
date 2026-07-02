import { Loader2, Save } from "lucide-react";
import type { ReactNode } from "react";
import type {
  ResourceDefinition,
  StateVariablesEditDocument,
} from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import {
  Collapsible,
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

export interface ResourceDraft {
  key: string;
  label: string;
  /** Default initial value for the resource definition. */
  initial: number;
  min: number;
  max: number;
}

export interface StateViewProps {
  /** Loaded state variables edit document (null if not loaded). */
  stateVariablesEditDocument: StateVariablesEditDocument | null;
  /** True while the state form is saving. */
  saving: boolean;
  /** Cross-section form status for displaying save/error feedback. */
  formStatus: FormStatus | null;
  /** New resource draft for the "Add Resource" form. */
  newResource: ResourceDraft;
  onNewResourceChange(draft: ResourceDraft): void;
  /** Save the state variables document. */
  onSave(): void;
  /** Inline update for an existing resource definition. */
  onUpdateResource(index: number, patch: Partial<ResourceDefinition>): void;
  /** Update an initial world state resource value. */
  onUpdateInitialWorldResource(key: string, value: number): void;
  /** Update the initial story state current_scene_key. */
  onUpdateInitialSceneKey(value: string): void;
  /** Update the initial story state turn. */
  onUpdateInitialTurn(value: number): void;
  /** Create a new resource from the draft via the Rust bridge. */
  onCreateResourceFromDraft(): void;
}

// ---------------------------------------------------------------------------
// StateView
// ---------------------------------------------------------------------------

export function StateView({
  stateVariablesEditDocument,
  saving,
  formStatus,
  newResource,
  onNewResourceChange,
  onSave,
  onUpdateResource,
  onUpdateInitialWorldResource,
  onUpdateInitialSceneKey,
  onUpdateInitialTurn,
  onCreateResourceFromDraft,
}: StateViewProps) {
  const { t } = useStudioI18n();
  return (
    <section className={studioUiClassNames.panel}>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">{t("state.title")}</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {t("common.resources", {
              count: stateVariablesEditDocument?.resources.length ?? 0,
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
          {t("state.save")}
        </button>
      </div>

      <SectionStatusMessage section="state" formStatus={formStatus} />

      {stateVariablesEditDocument ? (
        <div className="mt-3 grid gap-3">
          <div className="grid gap-3 lg:grid-cols-2">
            {stateVariablesEditDocument.resources.map((resource, index) => (
              <ResourceCard
                key={`${resource.key}:${index}`}
                resource={resource}
                index={index}
                initialWorldValue={
                  stateVariablesEditDocument.initial_world_state.resources[
                    resource.key
                  ] ?? resource.initial
                }
                onUpdate={(patch) => onUpdateResource(index, patch)}
                onUpdateInitialWorldValue={(value) =>
                  onUpdateInitialWorldResource(resource.key, value)
                }
              />
            ))}
          </div>

          <Collapsible
            label={t("state.initialStoryState")}
            id="initial-story-state"
            defaultOpen={false}
            className="rounded-md border border-ink/10 bg-canvas-50 p-4"
          >
            <p className="mt-1 text-xs text-ink/55">
              {t("state.initialStoryStateDesc")}
            </p>
            <div className="mt-3 grid gap-3 sm:grid-cols-2">
              <TextInput
                label={t("state.currentScene")}
                ariaLabel={t("state.aria.initialCurrentScene")}
                value={
                  stateVariablesEditDocument.initial_story_state
                    .current_scene_key
                }
                onChange={onUpdateInitialSceneKey}
              />
              <NumberInput
                label={t("state.turn")}
                ariaLabel={t("state.aria.initialTurn")}
                value={stateVariablesEditDocument.initial_story_state.turn}
                onChange={onUpdateInitialTurn}
              />
            </div>
          </Collapsible>

          <Collapsible
            label={t("state.addResource")}
            id="add-resource"
            defaultOpen={false}
            className="rounded-md border border-ink/10 bg-canvas-50 p-4"
          >
            <AddResourceForm
              draft={newResource}
              saving={saving}
              onDraftChange={onNewResourceChange}
              onSubmit={onCreateResourceFromDraft}
            />
          </Collapsible>
        </div>
      ) : (
        <EmptyState />
      )}
    </section>
  );
}

// ---------------------------------------------------------------------------
// ResourceCard — collapsed summary with expandable detail edit
// ---------------------------------------------------------------------------

function ResourceCard({
  resource,
  index,
  initialWorldValue,
  onUpdate,
  onUpdateInitialWorldValue,
}: {
  resource: ResourceDefinition;
  index: number;
  /** The initial world state value for this resource. */
  initialWorldValue: number;
  onUpdate(patch: Partial<ResourceDefinition>): void;
  onUpdateInitialWorldValue(value: number): void;
}) {
  const { t } = useStudioI18n();
  const summary = t("state.resourceSummary", {
    initial: resource.initial,
    min: resource.min,
    max: resource.max,
  });
  return (
    <article className="rounded-md border border-ink/10 bg-canvas-50 p-4">
      <Collapsible
        label={resource.label || resource.key}
        id={`resource-${resource.key}`}
        defaultOpen={false}
        badge={summary}
      >
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <TextInput
            label={t("state.resourceKey")}
            ariaLabel={t("state.aria.resourceKeyN", { index: index + 1 })}
            value={resource.key}
            onChange={(value) => onUpdate({ key: value })}
          />
          <TextInput
            label={t("state.resourceLabel")}
            ariaLabel={t("state.aria.resourceLabelN", { index: index + 1 })}
            value={resource.label}
            onChange={(value) => onUpdate({ label: value })}
          />
          <NumberInput
            label={t("state.defaultInitialValue")}
            ariaLabel={t("state.aria.resourceDefaultInitialValueN", { index: index + 1 })}
            value={resource.initial}
            onChange={(value) => onUpdate({ initial: value })}
          />
          <NumberInput
            label={t("state.worldInitialValue")}
            ariaLabel={t("state.aria.resourceWorldInitialValueN", { index: index + 1 })}
            value={initialWorldValue}
            onChange={onUpdateInitialWorldValue}
          />
          <NumberInput
            label={t("state.min")}
            ariaLabel={t("state.aria.resourceMinN", { index: index + 1 })}
            value={resource.min}
            onChange={(value) => onUpdate({ min: value })}
          />
          <NumberInput
            label={t("state.max")}
            ariaLabel={t("state.aria.resourceMaxN", { index: index + 1 })}
            value={resource.max}
            onChange={(value) => onUpdate({ max: value })}
          />
        </div>
      </Collapsible>
    </article>
  );
}

// ---------------------------------------------------------------------------
// AddResourceForm
// ---------------------------------------------------------------------------

function AddResourceForm({
  draft,
  saving,
  onDraftChange,
  onSubmit,
}: {
  draft: ResourceDraft;
  saving: boolean;
  onDraftChange(draft: ResourceDraft): void;
  onSubmit(): void;
}) {
  const { t } = useStudioI18n();
  return (
    <div className="mt-3 grid gap-3 sm:grid-cols-2">
      <TextInput
        label={t("state.resourceKey")}
        ariaLabel={t("state.aria.newResourceKey")}
        placeholder={t("state.placeholder.resourceKey")}
        value={draft.key}
        onChange={(value) => onDraftChange({ ...draft, key: value })}
      />
      <TextInput
        label={t("state.resourceLabel")}
        ariaLabel={t("state.aria.newResourceLabel")}
        placeholder={t("state.placeholder.resourceLabel")}
        value={draft.label}
        onChange={(value) => onDraftChange({ ...draft, label: value })}
      />
      <NumberInput
        label={t("state.defaultInitialValue")}
        ariaLabel={t("state.aria.newResourceDefaultInitialValue")}
        value={draft.initial}
        onChange={(value) => onDraftChange({ ...draft, initial: value })}
      />
      <NumberInput
        label={t("state.min")}
        ariaLabel={t("state.aria.newResourceMin")}
        value={draft.min}
        onChange={(value) => onDraftChange({ ...draft, min: value })}
      />
      <NumberInput
        label={t("state.max")}
        ariaLabel={t("state.aria.newResourceMax")}
        value={draft.max}
        onChange={(value) => onDraftChange({ ...draft, max: value })}
      />
      <div className="sm:col-span-2 flex justify-end">
        <button
          type="button"
          onClick={onSubmit}
          disabled={saving}
          className={studioUiClassNames.secondaryButton}
        >
          {t("state.create")}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to StateView)
// ---------------------------------------------------------------------------

function EmptyState() {
  const { t } = useStudioI18n();
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-canvas-50 px-4 py-6 text-center text-sm text-ink/55">
      {t("state.notLoaded")}
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
}): ReactNode {
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
}): ReactNode {
  return (
    <label className="grid gap-1">
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <input
        type="number"
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(Number(event.target.value))}
        className={studioUiClassNames.input}
      />
    </label>
  );
}
