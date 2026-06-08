import {
  AlertTriangle,
  Bug,
  CheckCircle2,
  Loader2,
  Play,
} from "lucide-react";
import type {
  NarrativeIssue,
  RuntimeTraceDiagnostic,
} from "../../../contracts/plotforge";
import type { PlayOnceReport } from "./tauriBridge";

interface PlaytestPanelProps {
  input: string;
  running: boolean;
  report: PlayOnceReport | null;
  error: string | null;
  saveId: string;
  restoreId: string;
  restoreLatest: boolean;
  onInputChange(input: string): void;
  onSaveIdChange(saveId: string): void;
  onRestoreIdChange(restoreId: string): void;
  onRestoreLatestChange(restoreLatest: boolean): void;
  onRun(): void;
}

interface RuntimeTracePanelProps {
  report: PlayOnceReport | null;
  error: string | null;
}

export function PlaytestPanel({
  input,
  running,
  report,
  error,
  saveId,
  restoreId,
  restoreLatest,
  onInputChange,
  onSaveIdChange,
  onRestoreIdChange,
  onRestoreLatestChange,
  onRun,
}: PlaytestPanelProps) {
  return (
    <section className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-lg font-semibold">Playtest</h3>
          <p className="mt-1 text-sm text-ink/55">
            {report?.scene.title ?? "No turn run"}
          </p>
        </div>
        <button
          type="button"
          disabled={running}
          onClick={onRun}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {running ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Play aria-hidden size={16} />
          )}
          Run turn
        </button>
      </div>

      <div className="mt-4">
        <textarea
          aria-label="Playtest input"
          value={input}
          onChange={(event) => onInputChange(event.target.value)}
          className="min-h-24 w-full resize-y rounded-md border border-ink/15 bg-parchment px-3 py-3 text-sm leading-6 text-ink outline-none transition focus:border-ink/45"
        />
      </div>

      <div className="mt-3 grid gap-3 lg:grid-cols-[1fr_1fr_auto]">
        <label className="grid min-w-0 gap-1">
          <span className="text-xs font-medium uppercase text-ink/45">
            Save ID
          </span>
          <input
            aria-label="Playtest save id"
            value={saveId}
            onChange={(event) => onSaveIdChange(event.target.value)}
            className="h-10 rounded-md border border-ink/15 bg-white px-3 text-sm text-ink outline-none transition focus:border-ink/45"
          />
        </label>
        <label className="grid min-w-0 gap-1">
          <span className="text-xs font-medium uppercase text-ink/45">
            Restore ID
          </span>
          <input
            aria-label="Playtest restore id"
            value={restoreId}
            disabled={restoreLatest}
            onChange={(event) => onRestoreIdChange(event.target.value)}
            className="h-10 rounded-md border border-ink/15 bg-white px-3 text-sm text-ink outline-none transition focus:border-ink/45 disabled:cursor-not-allowed disabled:bg-ink/5 disabled:text-ink/35"
          />
        </label>
        <label className="flex min-h-10 items-center gap-2 self-end rounded-md border border-ink/10 px-3 text-sm font-medium text-ink/70">
          <input
            type="checkbox"
            aria-label="Restore latest save"
            checked={restoreLatest}
            onChange={(event) => onRestoreLatestChange(event.target.checked)}
            className="h-4 w-4 accent-ink"
          />
          Restore latest
        </label>
      </div>

      {error ? (
        <div className="mt-4 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
          {error}
        </div>
      ) : null}

      {report ? (
        <div className="mt-4 space-y-3 border-t border-ink/10 pt-4">
          <div className="flex flex-wrap gap-2 text-xs font-semibold uppercase">
            <Badge tone={report.trace.fallback_used ? "fallback" : "completed"}>
              {report.trace.fallback_used ? "Fallback" : "Committed"}
            </Badge>
            <Badge tone={report.trace.errors.length > 0 ? "error" : "completed"}>
              {`${report.trace.errors.length} errors`}
            </Badge>
            <Badge tone="neutral">{report.trace.id}</Badge>
          </div>
          <div>
            <p className="text-sm font-semibold">{report.scene.key}</p>
            <p className="mt-1 text-sm text-ink/60">{report.scene.hook}</p>
          </div>
          <div className="grid gap-2 sm:grid-cols-3">
            <TraceField label="Selected" value={report.trace.selected_choice} />
            <TraceField
              label="Intent"
              value={report.trace.action_intent?.action_type ?? "unsupported"}
            />
            <TraceField label="Snapshot" value={report.snapshot_path} />
          </div>
          <DeltaList lines={report.delta_summary} />
        </div>
      ) : null}
    </section>
  );
}

export function RuntimeTracePanel({ report, error }: RuntimeTracePanelProps) {
  const trace = report?.trace ?? null;
  const review = trace?.narrative_review ?? null;

  return (
    <section className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div>
          <h3 className="text-lg font-semibold">Runtime Trace</h3>
          <p className="mt-1 text-sm text-ink/55">
            {trace?.id ?? "No trace selected"}
          </p>
        </div>
        {trace?.errors.length ? (
          <AlertTriangle aria-hidden className="text-signal" size={22} />
        ) : (
          <Bug aria-hidden className="text-signal" size={22} />
        )}
      </div>

      {error ? (
        <div className="mt-4 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
          {error}
        </div>
      ) : null}

      {trace ? (
        <div className="mt-4 space-y-5">
          <div className="flex flex-wrap gap-2 text-xs font-semibold uppercase">
            <Badge tone={trace.fallback_used ? "fallback" : "completed"}>
              {trace.fallback_used ? "Fallback" : "Committed"}
            </Badge>
            <Badge tone={trace.errors.length > 0 ? "error" : "completed"}>
              {`${trace.errors.length} errors`}
            </Badge>
            <Badge tone="neutral">{trace.id}</Badge>
          </div>
          {report ? (
            <div className="rounded-md border border-ink/10 bg-parchment px-3 py-2">
              <p className="text-xs font-medium uppercase text-ink/45">
                Scene
              </p>
              <p className="mt-1 text-sm font-semibold">{report.scene.title}</p>
              <p className="mt-1 text-sm text-ink/60">{report.scene.hook}</p>
            </div>
          ) : null}
          <div className="grid gap-3 sm:grid-cols-4">
            <TraceField
              label="Rule"
              value={trace.rule_result?.action_type}
              detail={
                trace.rule_result
                  ? trace.rule_result.state_committed
                    ? "committed"
                    : "not committed"
                  : undefined
              }
            />
            <TraceField
              label="Planner"
              value={trace.planner_result?.scene_key}
              detail={trace.planner_result?.fallback_used ? "fallback" : "ok"}
            />
            <TraceField
              label="Review score"
              value={review ? String(review.score) : null}
              detail={review ? `${review.issues.length} issues` : undefined}
            />
            <TraceField label="Snapshot" value={report?.snapshot_path} />
          </div>

          <TraceEvidence trace={trace} />
          <WorldDeltaEvidence trace={trace} />
          <MediaReferenceList references={trace.media_references} />
          {review ? <ReviewScores review={review} /> : null}
          {review?.issues.length ? <IssueList issues={review.issues} /> : null}
          <DiagnosticList diagnostics={trace.diagnostics} />
          <ErrorList errors={trace.errors} />
        </div>
      ) : (
        <div className="mt-4 border-t border-ink/10 pt-4 text-sm text-ink/55">
          Trace output appears after a playtest turn.
        </div>
      )}
    </section>
  );
}

function TraceEvidence({ trace }: { trace: PlayOnceReport["trace"] }) {
  const intent = trace.action_intent;
  const rule = trace.rule_result;
  const planner = trace.planner_result;
  const reproducibility = trace.reproducibility;

  return (
    <div>
      <h4 className="text-sm font-semibold">Trace Evidence</h4>
      <div className="mt-2 grid gap-3 lg:grid-cols-4">
        <EvidenceGroup title="Action Intent">
          <TraceField label="Status" value={intent?.status} />
          <TraceField label="Choice" value={intent?.choice_id} />
          <TraceField label="Action" value={intent?.action_type} />
          <TraceField
            label="Matched terms"
            value={
              intent?.matched_terms.length
                ? intent.matched_terms.join(", ")
                : null
            }
          />
          <TraceField label="Reason" value={intent?.reason} />
        </EvidenceGroup>

        <EvidenceGroup title="Rule Result">
          <TraceField label="Action" value={rule?.action_type} />
          <TraceField
            label="Committed"
            value={rule ? booleanText(rule.state_committed) : null}
          />
          <TraceField
            label="Delta empty"
            value={rule ? booleanText(rule.delta_empty) : null}
          />
          <TraceField label="Error code" value={rule?.error?.code} />
          <TraceField label="Error" value={rule?.error?.message} />
        </EvidenceGroup>

        <EvidenceGroup title="Planner Result">
          <TraceField
            label="Requested"
            value={planner?.requested_action_type}
          />
          <TraceField label="Scene" value={planner?.scene_key} />
          <TraceField
            label="Fallback"
            value={planner ? booleanText(planner.fallback_used) : null}
          />
          <TraceField label="Error code" value={planner?.error?.code} />
          <TraceField label="Error" value={planner?.error?.message} />
        </EvidenceGroup>

        <EvidenceGroup title="Reproducibility">
          <TraceField label="Run seed" value={reproducibility.run_seed} />
          <TraceField
            label="Prompt version"
            value={reproducibility.prompt_version}
          />
          <TraceField
            label="Model version"
            value={reproducibility.model_version}
          />
          <TraceField
            label="Provider config hash"
            value={reproducibility.provider_config_hash}
          />
          <TraceField label="Trace id" value={reproducibility.trace_id} />
          <TraceField label="Snapshot id" value={reproducibility.snapshot_id} />
        </EvidenceGroup>
      </div>
    </div>
  );
}

function EvidenceGroup({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="min-w-0">
      <h5 className="text-xs font-semibold uppercase text-ink/45">{title}</h5>
      <div className="mt-2 grid gap-2">{children}</div>
    </div>
  );
}

function ReviewScores({
  review,
}: {
  review: NonNullable<PlayOnceReport["trace"]["narrative_review"]>;
}) {
  const scores = [
    ["Hook", review.hook_score],
    ["Pacing", review.pacing_score],
    ["Character", review.character_consistency_score],
    ["Payoff", review.payoff_score],
    ["Choice", review.choice_meaningfulness_score],
    ["AI slop", review.ai_slop_risk],
  ];

  return (
    <div>
      <h4 className="text-sm font-semibold">Narrative Review</h4>
      <div className="mt-2 grid gap-2 sm:grid-cols-3">
        {scores.map(([label, value]) => (
          <div key={label} className="border-t border-ink/10 pt-2">
            <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
            <p className="mt-1 text-lg font-semibold text-ink">{value}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function WorldDeltaEvidence({ trace }: { trace: PlayOnceReport["trace"] }) {
  const delta = trace.world_state_delta;
  const resourceChanges = Object.entries(delta.resource_changes).map(
    ([key, amount]) => `${key}: ${formatSigned(amount)}`,
  );
  const resourceSets = Object.entries(delta.resource_sets).map(
    ([key, value]) => `${key}: set ${value}`,
  );
  const flags = Object.entries(delta.flags).map(([key, value]) => {
    return `flag ${key}: ${String(value)}`;
  });
  const events = delta.triggered_events.map((event) => `event: ${event}`);
  const lines = [...resourceChanges, ...resourceSets, ...flags, ...events];

  return (
    <div>
      <h4 className="text-sm font-semibold">State Delta</h4>
      <DeltaList lines={lines} />
    </div>
  );
}

function MediaReferenceList({
  references,
}: {
  references: PlayOnceReport["trace"]["media_references"];
}) {
  if (references.length === 0) {
    return (
      <div className="border-t border-ink/10 pt-3 text-sm text-ink/55">
        No media references
      </div>
    );
  }

  return (
    <div>
      <h4 className="text-sm font-semibold">Media References</h4>
      <div className="mt-2 grid gap-2">
        {references.map((reference) => (
          <div
            key={`${reference.reference.reference_kind}:${reference.reference.reference_id}:${reference.reference.slot}`}
            className="border-t border-ink/10 pt-2 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone="neutral">{reference.reference.reference_kind}</Badge>
              <code className="text-xs text-ink/55">
                {reference.reference.slot}
              </code>
            </div>
            <p className="mt-1 text-ink/75">{reference.reference.reference_id}</p>
            <p className="mt-0.5 break-all text-xs text-ink/50">
              {reference.project_path}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}

function IssueList({ issues }: { issues: NarrativeIssue[] }) {
  return (
    <div>
      <h4 className="text-sm font-semibold">Review Issues</h4>
      <div className="mt-2 grid gap-2">
        {issues.map((issue) => (
          <div
            key={`${issue.kind}:${issue.message}`}
            className="border-t border-ink/10 pt-2 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone={issue.severity === "error" ? "error" : "fallback"}>
                {issue.severity}
              </Badge>
              <code className="text-xs text-ink/55">{issue.kind}</code>
            </div>
            <p className="mt-1 text-ink/75">{issue.message}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function DiagnosticList({
  diagnostics,
}: {
  diagnostics: RuntimeTraceDiagnostic[];
}) {
  return (
    <div>
      <h4 className="text-sm font-semibold">Diagnostics</h4>
      <div className="mt-2 grid gap-2">
        {diagnostics.map((diagnostic) => (
          <div
            key={`${diagnostic.stage}:${diagnostic.status}:${diagnostic.message}`}
            className="border-t border-ink/10 pt-2 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone={diagnostic.status}>{diagnostic.status}</Badge>
              <code className="text-xs text-ink/55">{diagnostic.stage}</code>
            </div>
            <p className="mt-1 text-ink/75">{diagnostic.message}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function ErrorList({ errors }: { errors: PlayOnceReport["trace"]["errors"] }) {
  if (errors.length === 0) {
    return (
      <div className="flex items-center gap-2 border-t border-ink/10 pt-3 text-sm text-jade">
        <CheckCircle2 aria-hidden size={16} />
        No runtime errors
      </div>
    );
  }

  return (
    <div>
      <h4 className="text-sm font-semibold">Errors</h4>
      <div className="mt-2 grid gap-2">
        {errors.map((error) => (
          <div key={error.code} className="border-t border-ink/10 pt-2 text-sm">
            <code className="text-xs font-semibold text-signal">
              {error.code}
            </code>
            <p className="mt-1 text-ink/75">{error.message}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function DeltaList({ lines }: { lines: string[] }) {
  if (lines.length === 0) {
    return <p className="text-sm text-ink/55">No world delta</p>;
  }

  return (
    <div>
      <h4 className="text-sm font-semibold">World Delta</h4>
      <ul className="mt-2 grid gap-1 text-sm text-ink/70">
        {lines.map((line) => (
          <li key={line}>
            <code>{line}</code>
          </li>
        ))}
      </ul>
    </div>
  );
}

function TraceField({
  label,
  value,
  detail,
}: {
  label: string;
  value?: string | number | null;
  detail?: string;
}) {
  return (
    <div className="border-t border-ink/10 pt-2">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">
        {value ?? "none"}
      </p>
      {detail ? <p className="mt-0.5 text-xs text-ink/50">{detail}</p> : null}
    </div>
  );
}

function booleanText(value: boolean) {
  return value ? "true" : "false";
}

function formatSigned(value: number) {
  return value > 0 ? `+${value}` : String(value);
}

function Badge({
  tone,
  children,
}: {
  tone: "completed" | "fallback" | "error" | "neutral";
  children: string | number;
}) {
  const classes = {
    completed: "border-jade/30 bg-jade/10 text-jade",
    fallback: "border-brass/30 bg-brass/10 text-brass",
    error: "border-signal/30 bg-signal/10 text-signal",
    neutral: "border-ink/15 bg-ink/5 text-ink/60",
  }[tone];

  return (
    <span
      className={`inline-flex min-h-6 items-center rounded-sm border px-2 py-0.5 text-xs font-semibold uppercase ${classes}`}
    >
      {children}
    </span>
  );
}
