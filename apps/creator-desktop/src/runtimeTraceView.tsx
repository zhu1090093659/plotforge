import {
  AlertTriangle,
  Bug,
  CheckCircle2,
  Network,
  Loader2,
  Play,
  ShieldCheck,
} from "lucide-react";
import type {
  AiSafetyPolicy,
  ExportProfile,
  NarrativeIssue,
  RuntimeTraceDiagnostic,
} from "../../../contracts/plotforge";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";
import type { PlayOnceReport, StaticExportReport } from "./tauriBridge";
import { ScenePreviewPlaceholder } from "./studioUi";

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
  selectedExportProfile?: ExportProfile | null;
  exportReport?: StaticExportReport | null;
  aiSafetyPolicy?: AiSafetyPolicy | null;
  loadedPath?: string | null;
  projectId?: string | null;
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

export function RuntimeTracePanel({
  report,
  error,
  selectedExportProfile = null,
  exportReport = null,
  aiSafetyPolicy = null,
  loadedPath = null,
  projectId = null,
}: RuntimeTracePanelProps) {
  const trace = report?.trace ?? null;
  const review = trace?.narrative_review ?? null;
  const beat = resolveSceneBeat(
    report?.scene,
    trace?.story_state_after.current_beat_id,
  );
  const sceneImage = resolveScenePreviewImage({
    scene: report?.scene,
    projectId,
    loadedPath,
  });
  const packageReady =
    Boolean(
      selectedExportProfile &&
        exportReport &&
        !selectedExportProfile.includes_provider_config &&
        !selectedExportProfile.includes_private_traces &&
        sameStringSet(exportReport.allowed_files, exportReport.files_found),
    );

  return (
    <section aria-label="Proof And Trace Workspace" className="grid gap-5">
      <div className="grid gap-4 2xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid gap-4">
          <section
            aria-label="Playable Proof"
            className="overflow-hidden rounded-lg border border-graphite-700/15 bg-graphite-950 text-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3 border-b border-canvas-200/10 px-4 py-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-health-400">
                  Playable Proof
                </p>
                <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                  {report?.scene.title ?? "Run a playtest turn to create proof"}
                </h3>
                <p className="mt-1 text-sm leading-6 text-canvas-200/60">
                  {trace
                    ? "Playable result, state deltas, run evidence, artifact diff, and package readiness are shown from the current local run."
                    : "Trace output appears after a playtest turn."}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Badge tone={trace?.fallback_used ? "fallback" : "completed"}>
                  {trace?.fallback_used ? "Fallback" : trace ? "Committed" : "Waiting"}
                </Badge>
                <Badge tone={trace?.errors.length ? "error" : "completed"}>
                  {trace ? `${trace.errors.length} errors` : "no run"}
                </Badge>
                <Badge tone="neutral">{trace?.id ?? "no trace"}</Badge>
              </div>
            </div>

            <div className="relative min-h-[430px] overflow-hidden bg-graphite-900">
              {sceneImage ? (
                <img
                  src={sceneImage}
                  alt=""
                  className="absolute inset-0 h-full w-full object-cover opacity-60"
                />
              ) : (
                <ScenePreviewPlaceholder
                  assetPath={report?.scene.background_asset ?? null}
                />
              )}
              <div className="absolute inset-0 bg-gradient-to-t from-graphite-950 via-graphite-950/35 to-graphite-950/10" />
              <div className="relative flex min-h-[430px] flex-col justify-end p-4">
                <div className="mx-auto w-full max-w-3xl rounded-lg border border-amber-500/40 bg-graphite-950/90 px-4 py-4 shadow-studio-panel">
                  <p className="text-xs font-semibold uppercase text-amber-400">
                    {report
                      ? `${report.scene.key} / ${report.scene.location}`
                      : "No playable result"}
                  </p>
                  <p className="mt-2 text-base leading-7 text-canvas-50">
                    {beat?.text ??
                      "Run a proof turn to inspect the player-facing result."}
                  </p>
                  <div className="mt-4 grid gap-2">
                    {(beat?.choices ?? []).slice(0, 4).map((choice, index) => (
                      <div
                        key={choice.id}
                        className="flex min-h-11 items-center gap-3 rounded-md border border-amber-500/35 bg-canvas-50/5 px-3 text-canvas-50"
                      >
                        <span className="grid h-7 w-7 shrink-0 place-items-center rounded-sm border border-amber-400/60 text-xs text-amber-400">
                          {index + 1}
                        </span>
                        <span className="min-w-0 truncate text-sm">
                          {choice.label}
                        </span>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            </div>

            <div className="grid gap-3 border-t border-canvas-200/10 px-4 py-3 md:grid-cols-4">
              <ProofFact label="Trace ID" value={trace?.id ?? "none"} />
              <ProofFact
                label="Run seed"
                value={trace?.reproducibility.run_seed ?? "none"}
              />
              <ProofFact
                label="State deltas"
                value={String(report?.delta_summary.length ?? 0)}
              />
              <ProofFact
                label="Package readiness"
                value={packageReady ? "local ready" : "review required"}
              />
            </div>
          </section>

          {error ? (
            <div className="rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
              {error}
            </div>
          ) : null}

          <div className="grid gap-4 xl:grid-cols-[1fr_0.9fr]">
            <section className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-base font-semibold">State Delta</h3>
                <Badge tone="neutral">{report?.delta_summary.length ?? 0}</Badge>
              </div>
              <DeltaList lines={report?.delta_summary ?? []} />
            </section>

            <section className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-base font-semibold">Run Evidence</h3>
                <Bug aria-hidden className="text-signal" size={20} />
              </div>
              <div className="mt-3 grid gap-2 sm:grid-cols-2">
                <TraceField label="Selected choice" value={trace?.selected_choice} />
                <TraceField
                  label="Intent"
                  value={trace?.action_intent?.action_type ?? "none"}
                />
                <TraceField
                  label="Prompt version"
                  value={trace?.reproducibility.prompt_version}
                />
                <TraceField label="Snapshot" value={report?.snapshot_path} />
              </div>
            </section>
          </div>

          <section
            aria-label="Trace Debug"
            className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-acp-500">
                  Trace Debug
                </p>
                <h3 className="mt-1 text-lg font-semibold">
                  Redaction-safe causality
                </h3>
              </div>
              {trace?.errors.length ? (
                <AlertTriangle aria-hidden className="text-signal" size={22} />
              ) : (
                <Network aria-hidden className="text-acp-500" size={22} />
              )}
            </div>

            {trace ? (
              <div className="mt-4 grid gap-5">
                <CausalityGraph trace={trace} />
                <TraceEvidence trace={trace} />
                <ToolMetadata trace={trace} exportReport={exportReport} />
                <WorldDeltaEvidence trace={trace} />
                <MediaReferenceList references={trace.media_references} />
                {review ? <ReviewScores review={review} /> : null}
                {review?.issues.length ? <IssueList issues={review.issues} /> : null}
                <DiagnosticList diagnostics={trace.diagnostics} />
                <ErrorList errors={trace.errors} />
              </div>
            ) : (
              <div className="mt-4 border-t border-ink/10 pt-4 text-sm text-ink/55">
                Run a playtest turn to inspect trace evidence.
              </div>
            )}
          </section>
        </div>

        <aside
          aria-label="Proof Evidence Panel"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
        >
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-xs font-semibold uppercase text-health-400">
                Agent Evidence
              </p>
              <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                {trace?.id ?? "No trace selected"}
              </h3>
            </div>
            <ShieldCheck aria-hidden className="text-health-400" size={22} />
          </div>

          <div className="grid gap-2 rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <ProofSideFact label="Trace ID" value={trace?.id ?? "none"} />
            <ProofSideFact
              label="Run Seed"
              value={trace?.reproducibility.run_seed ?? "none"}
            />
            <ProofSideFact
              label="Prompt Version"
              value={trace?.reproducibility.prompt_version ?? "none"}
            />
            <ProofSideFact
              label="Provider Config Hash"
              value={trace?.reproducibility.provider_config_hash ?? "none"}
            />
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Package Evidence Summary
            </p>
            <div className="mt-3 grid gap-2">
              {exportReport ? (
                [
                  ["Files written", String(exportReport.files_written.length)],
                  ["Archived files", String(exportReport.archived_files.length)],
                  ["Allowed files", String(exportReport.allowed_files.length)],
                ].map(([label, value]) => (
                    <div
                      key={label}
                      className="flex items-center justify-between gap-3 border-t border-canvas-200/10 pt-2 text-sm first:border-t-0 first:pt-0"
                    >
                      <span className="truncate text-canvas-50">{label}</span>
                      <span className="shrink-0 text-health-400">{value}</span>
                    </div>
                  ))
              ) : (
                <p className="text-sm text-canvas-200/55">
                  No export report captured.
                </p>
              )}
            </div>
          </div>

          <DisclosureDraft
            title="AI Usage Disclosure"
            body={
              aiSafetyPolicy
                ? aiSafetyPolicy.moderation_policy
                : "AI usage disclosure appears after policy load."
            }
          />
          <DisclosureDraft
            title="Content Warning Draft"
            body={
              aiSafetyPolicy
                ? `Content kinds: ${aiSafetyPolicy.content_kinds.join(", ")}. Human review: ${booleanText(aiSafetyPolicy.human_review_required)}.`
                : "Content warning draft appears after policy load."
            }
          />

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-sm font-semibold text-canvas-50">
                  Local Static Web Package
                </p>
                <p className="mt-1 text-xs text-canvas-200/55">
                  {selectedExportProfile?.id ?? "No profile selected"}
                </p>
              </div>
              <Badge tone={packageReady ? "completed" : "fallback"}>
                {packageReady ? "Ready" : "Review"}
              </Badge>
            </div>
            <div className="mt-3 grid gap-2 text-sm">
              <ProofSideFact
                label="Archive"
                value={exportReport?.archive_path ?? "not exported"}
              />
              <ProofSideFact
                label="Files"
                value={exportReport?.archived_files.length ?? 0}
              />
              <ProofSideFact
                label="Audit"
                value={
                  exportReport
                    ? sameStringSet(
                        exportReport.allowed_files,
                        exportReport.files_found,
                      )
                      ? "matched"
                      : "mismatch"
                    : "pending"
                }
              />
            </div>
          </div>
        </aside>
      </div>
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

function ProofFact({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="min-w-0 rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-2">
      <p className="text-xs font-semibold uppercase text-canvas-200/45">
        {label}
      </p>
      <p className="mt-1 truncate text-sm font-semibold text-canvas-50">
        {value}
      </p>
    </div>
  );
}

function ProofSideFact({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3">
      <p className="text-xs font-medium uppercase text-canvas-200/45">{label}</p>
      <p className="truncate text-xs font-semibold text-canvas-50">{value}</p>
    </div>
  );
}

function DisclosureDraft({ title, body }: { title: string; body: string }) {
  return (
    <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
      <div className="flex items-start justify-between gap-3">
        <p className="text-sm font-semibold text-canvas-50">{title}</p>
        <Badge tone="fallback">Local draft only</Badge>
      </div>
      <p className="mt-3 text-sm leading-6 text-canvas-200/65">{body}</p>
    </div>
  );
}

function CausalityGraph({ trace }: { trace: PlayOnceReport["trace"] }) {
  const steps = [
    {
      id: "intent",
      label: "Director Intent",
      detail: trace.action_intent?.action_type ?? "unsupported",
      tone: "border-acp-500/35 bg-acp-500/10 text-acp-600",
    },
    {
      id: "rule",
      label: "Rules.patch",
      detail: trace.rule_result?.state_committed ? "committed" : "not committed",
      tone: "border-health-500/35 bg-health-500/10 text-health-600",
    },
    {
      id: "planner",
      label: "Runtime.playtest",
      detail: trace.planner_result?.scene_key ?? "none",
      tone: "border-agent-500/35 bg-agent-500/10 text-agent-600",
    },
    {
      id: "diagnostics",
      label: "Diagnostics",
      detail: `${trace.diagnostics.length} steps`,
      tone: "border-amber-500/35 bg-amber-500/10 text-amber-600",
    },
  ];

  return (
    <div>
      <h4 className="text-sm font-semibold">Causality Graph</h4>
      <div className="mt-3 grid gap-3 lg:grid-cols-4">
        {steps.map((step, index) => (
          <div key={step.id} className="min-w-0">
            <div className={`rounded-md border px-3 py-3 ${step.tone}`}>
              <p className="text-sm font-semibold">{step.label}</p>
              <p className="mt-1 truncate text-xs opacity-75">{step.detail}</p>
            </div>
            {index < steps.length - 1 ? (
              <p className="mt-2 text-center text-xs font-semibold text-ink/35">
                flows to
              </p>
            ) : null}
          </div>
        ))}
      </div>
    </div>
  );
}

function ToolMetadata({
  trace,
  exportReport,
}: {
  trace: PlayOnceReport["trace"];
  exportReport: StaticExportReport | null;
}) {
  return (
    <div>
      <h4 className="text-sm font-semibold">Tool Metadata</h4>
      <div className="mt-2 grid gap-3 sm:grid-cols-4">
        <TraceField label="Prompt hash" value={trace.reproducibility.trace_id} />
        <TraceField
          label="Prompt version"
          value={trace.reproducibility.prompt_version}
        />
        <TraceField label="Tool calls" value={trace.diagnostics.length} />
        <TraceField
          label="Export files"
          value={exportReport?.files_written.length ?? 0}
        />
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

function sameStringSet(left: string[], right: string[]) {
  if (left.length !== right.length) {
    return false;
  }

  const rightValues = new Set(right);
  return left.every((value) => rightValues.has(value));
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
