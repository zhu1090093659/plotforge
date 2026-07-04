import type { ReactNode } from "react";
import type {
  ResourceDefinition,
  StateVariablesEditDocument,
} from "../../../contracts/plotforge";
import type { StudioSectionId } from "./studioModel";
import {
  Collapsible,
  EmptyState,
  Reveal,
  SaveButton,
  SectionStatusMessage,
  TextInput,
  ViewHeader,
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
    <Reveal as="section" className={`${studioUiClassNames.panel} grid`}>
      <ViewHeader
        title={t("state.title")}
        subtitle={t("common.resources", {
          count: stateVariablesEditDocument?.resources.length ?? 0,
        })}
        actions={
          <SaveButton
            saving={saving}
            onSave={onSave}
            disabled={saving}
            label={t("state.save")}
            ariaLabel={t("state.save")}
          />
        }
      />

      <SectionStatusMessage section="state" formStatus={formStatus} />

      {stateVariablesEditDocument ? (
        <div className="mt-3 grid gap-3">
          <ResourceCardGrid
            resources={stateVariablesEditDocument.resources}
            initialWorldState={stateVariablesEditDocument.initial_world_state.resources}
            onUpdateResource={onUpdateResource}
            onUpdateInitialWorldResource={onUpdateInitialWorldResource}
          />

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
        <EmptyState>{t("state.notLoaded")}</EmptyState>
      )}
    </Reveal>
  );
}

// ---------------------------------------------------------------------------
// ResourceCardGrid — paginated resource list (original index preserved).
// ---------------------------------------------------------------------------

function ResourceCardGrid({
  resources,
  initialWorldState,
  onUpdateResource,
  onUpdateInitialWorldResource,
}: {
  resources: ResourceDefinition[];
  initialWorldState: Record<string, number>;
  onUpdateResource(index: number, patch: Partial<ResourceDefinition>): void;
  onUpdateInitialWorldResource(key: string, value: number): void;
}) {
  const { t } = useStudioI18n();
  const indexed = resources.map((resource, index) => ({ resource, index }));
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
    filter: ({ resource }, q) =>
      [resource.key, resource.label].filter(Boolean).some((field) =>
        field.toLowerCase().includes(q.toLowerCase()),
      ),
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
          searchAriaLabel={t("pagination.aria.searchResources")}
          searchPlaceholder={t("pagination.searchResources")}
        />
      }
    >
      {visible.map(({ resource, index }) => (
        <ResourceCard
          key={`${resource.key}:${index}`}
          resource={resource}
          index={index}
          initialWorldValue={
            initialWorldState[resource.key] ?? resource.initial
          }
          onUpdate={(patch) => onUpdateResource(index, patch)}
          onUpdateInitialWorldValue={(value) =>
            onUpdateInitialWorldResource(resource.key, value)
          }
        />
      ))}
    </PaginatedCardGrid>
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
    <article className="transition ease-expo hover:-translate-y-0.5 hover:shadow-panel-lift rounded-md border border-ink/10 bg-canvas-50 p-4">
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
