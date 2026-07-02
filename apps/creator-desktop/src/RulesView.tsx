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
  studioUiClassNames,
} from "./studioUi";

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
  return (
    <section className={studioUiClassNames.panel}>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">Rules</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {rulesEditDocument?.rules.length ?? 0} rules
          </p>
        </div>
        <button
          type="button"
          disabled={saving}
          onClick={onSave}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {saving ? (
            <span
              aria-hidden
              className="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
            />
          ) : null}
          Save Rules
        </button>
      </div>

      {/* Section status message */}
      <SectionStatusMessage section="rules" formStatus={formStatus} />

      {rulesEditDocument ? (
        <div className="mt-3 grid gap-3">
          {/* Existing rule cards (collapsible) */}
          <div className="grid gap-3 lg:grid-cols-2">
            {rulesEditDocument.rules.map((rule, index) => (
              <RuleCard
                key={`${rule.id}:${index}`}
                rule={rule}
                index={index}
                onUpdate={(patch) => onUpdateRule(index, patch)}
              />
            ))}
          </div>

          {/* Unified "Add Rule" collapsible */}
          <Collapsible
            label="Add Rule"
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
        <EmptyRules />
      )}
    </section>
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
  return (
    <article className="rounded-md border border-ink/10 bg-canvas-50 p-4">
      <Collapsible
        label={rule.id || `Rule ${index + 1}`}
        id={`rule-${rule.id}`}
        defaultOpen={false}
        badge={rule.action_type || undefined}
      >
        <div className="mt-3 grid gap-3 sm:grid-cols-2">
          <TextInput
            label="Rule id"
            ariaLabel={`Rule id ${index + 1}`}
            value={rule.id}
            onChange={(value) => onUpdate({ id: value })}
          />
          <TextInput
            label="Action type"
            ariaLabel={`Rule action type ${index + 1}`}
            value={rule.action_type}
            onChange={(value) => onUpdate({ action_type: value })}
          />
        </div>

        {/* Conditions */}
        <div className="mt-3">
          <p className="text-xs font-medium uppercase text-ink/55">Conditions</p>
          {rule.conditions.length === 0 ? (
            <p className="mt-1 text-sm text-ink/40 italic">(none)</p>
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

        {/* Effects */}
        <div className="mt-3">
          <p className="text-xs font-medium uppercase text-ink/55">Effects</p>
          {rule.effects.length === 0 ? (
            <p className="mt-1 text-sm text-ink/40 italic">(none)</p>
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
          <span className="font-mono">{condition.value ? "true" : "false"}</span>
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
  switch (effect.kind) {
    case "add_resource":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">add</span>{" "}
          <span className="font-medium">{effect.key}</span>{" "}
          <span className="font-mono">
            {effect.amount > 0 ? `+${effect.amount}` : String(effect.amount)}
          </span>
        </span>
      );
    case "set_resource":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">set</span>{" "}
          <span className="font-medium">{effect.key}</span>
          {" = "}
          <span className="font-mono">{effect.value}</span>
        </span>
      );
    case "set_flag":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">flag</span>{" "}
          <span className="font-medium">{effect.key}</span>
          {" = "}
          <span className="font-mono">{effect.value ? "true" : "false"}</span>
        </span>
      );
    case "trigger_event":
      return (
        <span>
          <span className="text-xs font-semibold uppercase text-ink/50">
            event
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
  return (
    <div className="mt-3 grid gap-3">
      <p className="text-xs text-ink/55">
        Note: only add_resource effect is supported when creating rules
        manually. Edit the rules TOML file directly for complex conditions and
        effects.
      </p>
      <div className="grid gap-3 lg:grid-cols-4">
        <TextInput
          label="Rule id"
          ariaLabel="New rule id"
          placeholder="e.g. harvest-grain"
          value={draft.id}
          onChange={(value) => onDraftChange({ ...draft, id: value })}
        />
        <TextInput
          label="Action type"
          ariaLabel="New rule action type"
          placeholder="e.g. harvest"
          value={draft.action_type}
          onChange={(value) => onDraftChange({ ...draft, action_type: value })}
        />
        <label className="grid gap-1">
          <span className="text-xs font-medium uppercase text-ink/55">
            Resource
          </span>
          <select
            aria-label="New rule resource"
            value={draft.resource_key}
            onChange={(event) =>
              onDraftChange({ ...draft, resource_key: event.target.value })
            }
            className={studioUiClassNames.input}
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
          Create Rule
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Shared small components (local to RulesView)
// ---------------------------------------------------------------------------

function EmptyRules() {
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-canvas-50 px-4 py-6 text-center text-sm text-ink/55">
      Rule edit document not loaded.
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
