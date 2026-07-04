import { useState, type ReactNode } from "react";
import type {
  Condition,
  Effect,
  Rule,
  RuleDraft,
  RulesEditDocument,
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

export interface RulesViewProps {
  /** Loaded rules edit document (null if not loaded). */
  rulesEditDocument: RulesEditDocument | null;
  /** Available resource keys for the Add Rule dropdown. */
  resourceKeys: string[];
  /** True while the rules form is saving. */
  saving: boolean;
  /** Cross-section form status for displaying save/error feedback. */
  formStatus: FormStatus | null;
  /** Current manual-creation draft state. */
  ruleDraft: RuleDraft;
  onRuleDraftChange(draft: RuleDraft): void;
  /** Save existing rules. */
  onSave(): void;
  /** Create a new rule from the manual draft via the Rust bridge. */
  onCreateRuleFromDraft(): void;
  /** Inline update for an existing rule. */
  onUpdateRule(index: number, patch: Partial<Rule>): void;
}

// ---------------------------------------------------------------------------
// RulesView
// ---------------------------------------------------------------------------

export function RulesView({
  rulesEditDocument,
  resourceKeys,
  saving,
  formStatus,
  ruleDraft,
  onRuleDraftChange,
  onSave,
  onCreateRuleFromDraft,
  onUpdateRule,
}: RulesViewProps) {
  const { t } = useStudioI18n();
  return (
    <Reveal as="section" className={`${studioUiClassNames.panel} grid`}>
      <ViewHeader
        title={t("rules.title")}
        subtitle={t("common.rulesCount", {
          count: rulesEditDocument?.rules.length ?? 0,
        })}
        actions={
          <SaveButton
            saving={saving}
            onSave={onSave}
            label={t("rules.save")}
            ariaLabel={t("rules.save")}
          />
        }
      />

      <SectionStatusMessage section="rules" formStatus={formStatus} />

      {rulesEditDocument ? (
        <div className="mt-3 grid gap-3">
          <RuleCardGrid
            rules={rulesEditDocument.rules}
            onUpdateRule={onUpdateRule}
          />

          <Collapsible
            label={t("rules.addRule")}
            defaultOpen={false}
            className="rounded-md border border-ink/10 bg-canvas-50 p-4"
          >
            <ManualRuleForm
              draft={ruleDraft}
              resourceKeys={resourceKeys}
              saving={saving}
              onDraftChange={onRuleDraftChange}
              onSubmit={onCreateRuleFromDraft}
            />
          </Collapsible>
        </div>
      ) : (
        <EmptyState>{t("rules.notLoaded")}</EmptyState>
      )}
    </Reveal>
  );
}

// ---------------------------------------------------------------------------
// RuleCardGrid — paginated rule list (original index preserved for updates).
// ---------------------------------------------------------------------------

function RuleCardGrid({
  rules,
  onUpdateRule,
}: {
  rules: Rule[];
  onUpdateRule(index: number, patch: Partial<Rule>): void;
}) {
  const { t } = useStudioI18n();
  const indexed = rules.map((rule, index) => ({ rule, index }));
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
    filter: ({ rule }, q) =>
      [rule.id, rule.action_type].filter(Boolean).some((field) =>
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
          searchAriaLabel={t("pagination.aria.searchRules")}
          searchPlaceholder={t("pagination.searchRules")}
        />
      }
    >
      {visible.map(({ rule, index }) => (
        <RuleCard
          key={`${rule.id}:${index}`}
          rule={rule}
          index={index}
          onUpdate={(patch) => onUpdateRule(index, patch)}
        />
      ))}
    </PaginatedCardGrid>
  );
}

// ---------------------------------------------------------------------------
// RuleCard — collapsed summary with expandable detail edit
// ---------------------------------------------------------------------------

function RuleCard({
  rule,
  index,
  onUpdate,
}: {
  rule: Rule;
  index: number;
  onUpdate(patch: Partial<Rule>): void;
}) {
  const { t } = useStudioI18n();
  return (
    <article className="rounded-md border border-ink/10 bg-canvas-50 p-4 transition ease-expo hover:-translate-y-0.5 hover:shadow-panel-lift">
      <Collapsible
        label={rule.id || t("rules.ruleFallback", { index: index + 1 })}
        id={`rule-${rule.id}`}
        defaultOpen={false}
        badge={rule.action_type || undefined}
      >
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <TextInput
            label={t("rules.ruleId")}
            ariaLabel={t("rules.aria.ruleIdN", { index: index + 1 })}
            value={rule.id}
            onChange={(value) => onUpdate({ id: value })}
          />
          <TextInput
            label={t("rules.actionType")}
            ariaLabel={t("rules.aria.ruleActionTypeN", { index: index + 1 })}
            value={rule.action_type}
            onChange={(value) => onUpdate({ action_type: value })}
          />
        </div>

        <div className="mt-3">
          <p className="text-xs font-medium uppercase text-ink/55">
            {t("rules.conditions")}
          </p>
          {rule.conditions.length === 0 ? (
            <p className="mt-1 text-sm text-ink/40 italic">
              {t("common.noneParen")}
            </p>
          ) : (
            <ul className="mt-1 grid gap-1">
              {rule.conditions.map((cond, i) => (
                <li
                  key={i}
                  className="rounded-md bg-canvas-50 px-3 py-1.5 text-sm text-ink/70"
                >
                  <ConditionLabel condition={cond} />
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="mt-3">
          <p className="text-xs font-medium uppercase text-ink/55">
            {t("rules.effects")}
          </p>
          {rule.effects.length === 0 ? (
            <p className="mt-1 text-sm text-ink/40 italic">
              {t("common.noneParen")}
            </p>
          ) : (
            <ul className="mt-1 grid gap-1">
              {rule.effects.map((effect, i) => (
                <li
                  key={i}
                  className="rounded-md bg-canvas-50 px-3 py-1.5 text-sm text-ink/70"
                >
                  <EffectLabel effect={effect} />
                </li>
              ))}
            </ul>
          )}
        </div>
      </Collapsible>
    </article>
  );
}

// ---------------------------------------------------------------------------
// Human-readable Condition label
// ---------------------------------------------------------------------------

function ConditionLabel({ condition }: { condition: Condition }): ReactNode {
  const { t } = useStudioI18n();
  switch (condition.kind) {
    case "resource_at_least":
      return (
        <span>
          <span className="font-medium">{condition.key}</span>
          {" ≥ "}
          <span className="font-mono">{condition.value}</span>
        </span>
      );
    case "resource_at_most":
      return (
        <span>
          <span className="font-medium">{condition.key}</span>
          {" ≤ "}
          <span className="font-mono">{condition.value}</span>
        </span>
      );
    case "flag_equals":
      return (
        <span>
          <span className="font-medium">{condition.key}</span>
          {" = "}
          <span className="font-mono">
            {condition.value ? t("common.true") : t("common.false")}
          </span>
        </span>
      );
    default:
      return (
        <span className="font-mono text-xs">{JSON.stringify(condition)}</span>
      );
  }
}

// ---------------------------------------------------------------------------
// Human-readable Effect label
// ---------------------------------------------------------------------------

function EffectLabel({ effect }: { effect: Effect }): ReactNode {
  const { t } = useStudioI18n();
  switch (effect.kind) {
    case "add_resource":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">
            {t("common.add")}
          </span>{" "}
          <span className="font-medium">{effect.key}</span>{" "}
          <span className="font-mono">
            {effect.amount > 0 ? `+${effect.amount}` : String(effect.amount)}
          </span>
        </span>
      );
    case "set_resource":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">
            {t("common.set")}
          </span>{" "}
          <span className="font-medium">{effect.key}</span>
          {" = "}
          <span className="font-mono">{effect.value}</span>
        </span>
      );
    case "set_flag":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">
            {t("common.flag")}
          </span>{" "}
          <span className="font-medium">{effect.key}</span>
          {" = "}
          <span className="font-mono">
            {effect.value ? t("common.true") : t("common.false")}
          </span>
        </span>
      );
    case "trigger_event":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">
            {t("common.event")}
          </span>{" "}
          <span className="font-medium">{effect.event}</span>
        </span>
      );
    default:
      return <span className="font-mono text-xs">{JSON.stringify(effect)}</span>;
  }
}

// ---------------------------------------------------------------------------
// ManualRuleForm
// ---------------------------------------------------------------------------

function ManualRuleForm({
  draft,
  resourceKeys,
  saving,
  onDraftChange,
  onSubmit,
}: {
  draft: RuleDraft;
  resourceKeys: string[];
  saving: boolean;
  onDraftChange(draft: RuleDraft): void;
  onSubmit(): void;
}) {
  const { t } = useStudioI18n();
  return (
    <div className="mt-3 grid gap-3">
      <p className="text-xs text-ink/55">{t("rules.manualNote")}</p>
      <div className="grid gap-3 lg:grid-cols-4">
        <TextInput
          label={t("rules.ruleId")}
          ariaLabel={t("rules.aria.newRuleId")}
          placeholder={t("rules.placeholder.ruleId")}
          value={draft.id}
          onChange={(value) => onDraftChange({ ...draft, id: value })}
        />
        <TextInput
          label={t("rules.actionType")}
          ariaLabel={t("rules.aria.newRuleActionType")}
          placeholder={t("rules.placeholder.actionType")}
          value={draft.action_type}
          onChange={(value) => onDraftChange({ ...draft, action_type: value })}
        />
        <label className="grid gap-1">
          <span className="text-xs font-medium uppercase text-ink/55">
            {t("rules.resource")}
          </span>
          <select
            aria-label={t("rules.aria.newRuleResource")}
            value={draft.resource_key}
            onChange={(event) =>
              onDraftChange({ ...draft, resource_key: event.target.value })
            }
            className={studioUiClassNames.input}
          >
            <option value="">{t("rules.selectResource")}</option>
            {resourceKeys.map((key) => (
              <option key={key} value={key}>
                {key}
              </option>
            ))}
          </select>
        </label>
        <NumberInput
          label={t("rules.amount")}
          ariaLabel={t("rules.aria.newRuleAmount")}
          value={draft.amount}
          onChange={(value) => onDraftChange({ ...draft, amount: value })}
        />
      </div>
      <div className="flex justify-end">
        <button
          type="button"
          onClick={onSubmit}
          disabled={saving}
          className={studioUiClassNames.secondaryButton}
        >
          {t("rules.create")}
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to RulesView)
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
}) {
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
