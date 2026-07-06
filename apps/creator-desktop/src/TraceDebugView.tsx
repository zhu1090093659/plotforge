import {
  AlertTriangle,
  Bug,
  CheckCircle2,
  Network,
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
import {
  ScenePreviewImage,
  Collapsible,
  CollapsibleSection,
  Reveal,
  studioUiClassNames,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

export interface TraceDebugViewProps {
  report: PlayOnceReport | null;
  error: string | null;
  selectedExportProfile?: ExportProfile | null;
  exportReport?: StaticExportReport | null;
  aiSafetyPolicy?: AiSafetyPolicy | null;
  loadedPath?: string | null;
  projectId?: string | null;
  /** Snapshot save id (shared with the playtest workspace). */
  saveId?: string;
  /** Snapshot restore id (shared with the playtest workspace). */
  restoreId?: string;
  /** Whether to restore from the latest snapshot instead of a specific id. */
  restoreLatest?: boolean;
  /** Update the snapshot save id. */
  onSaveIdChange?(value: string): void;
  /** Update the snapshot restore id. */
  onRestoreIdChange?(value: string): void;
  /** Toggle restoring from the latest snapshot. */
  onRestoreLatestChange?(value: boolean): void;
}

export interface NarrativeReviewPanelProps {
  review: NonNullable<PlayOnceReport["trace"]["narrative_review"]>;
}

export function TraceDebugView({
  report,
  error,
  selectedExportProfile = null,
  exportReport = null,
  aiSafetyPolicy = null,
  loadedPath = null,
  projectId = null,
  saveId = "",
  restoreId = "",
  restoreLatest = false,
  onSaveIdChange,
  onRestoreIdChange,
  onRestoreLatestChange,
}: TraceDebugViewProps) {
  const { t } = useStudioI18n();
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
    <Reveal as="section" ariaLabel={t("trace.aria.workspace")} className="grid gap-4">
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_280px] 2xl:grid-cols-[minmax(0,1fr)_340px]">
        <div className="grid gap-4">
          <section
            aria-label={t("trace.aria.playableProof")}
            className="overflow-hidden rounded-lg border border-canvas-200 bg-graphite-950 text-ink shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3 border-b border-canvas-200 px-4 py-3">
              <div className="min-w-0">
                <p className="eyebrow eyebrow--copper">
                  {t("trace.playableProof")}
                </p>
                <h3 className="font-display mt-1 text-2xl font-semibold tracking-display-tight text-ink">
                  {report?.scene.title ?? t("trace.runToCreateProof")}
                </h3>
                <p className="mt-1 text-sm leading-6 text-graphite-700/65">
                  {trace
                    ? t("trace.playableResultDesc")
                    : t("trace.traceOutputAfterTurn")}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Badge tone={trace?.fallback_used ? "fallback" : "completed"}>
                  {trace?.fallback_used
                    ? t("common.fallback")
                    : trace
                      ? t("common.committed")
                      : t("common.waiting")}
                </Badge>
                <Badge tone={trace?.errors.length ? "error" : "completed"}>
                  {trace
                    ? t("common.errors", { count: trace.errors.length })
                    : t("common.notRun")}
                </Badge>
                <Badge tone="neutral">{trace?.id ?? t("common.notRun")}</Badge>
              </div>
            </div>

            <div className="relative min-h-[clamp(200px,35vh,380px)] overflow-hidden bg-graphite-900">
              <ScenePreviewImage
                src={sceneImage}
                assetPath={report?.scene.background_asset ?? null}
                className="absolute inset-0 h-full w-full object-cover opacity-60"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-graphite-950 via-graphite-950/35 to-graphite-950/10" />
              <div className="relative flex min-h-[clamp(200px,35vh,380px)] flex-col justify-end p-4">
                <div className="mx-auto w-full max-w-3xl rounded-lg border border-violet-500/40 bg-graphite-950/90 px-4 py-4 shadow-studio-panel">
                  <p className="text-xs font-semibold uppercase text-violet-600">
                    {report
                      ? `${report.scene.key} / ${report.scene.location}`
                      : t("trace.noPlayableResult")}
                  </p>
                  <p className="mt-2 text-base leading-7 text-ink">
                    {beat?.text ?? t("trace.runProofToInspect")}
                  </p>
                  <div className="mt-4 grid gap-2">
                    {(beat?.choices ?? []).slice(0, 4).map((choice, index) => (
                      <div
                        key={choice.id}
                        className="flex min-h-11 items-center gap-3 rounded-md border border-violet-500/35 bg-ink/5 px-3 text-ink transition ease-expo hover:border-violet-500/60"
                      >
                        <span className="grid h-7 w-7 shrink-0 place-items-center rounded-sm border border-copper-500/55 text-xs text-copper-500">
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

            <div className="grid gap-3 border-t border-canvas-200 px-4 py-3 md:grid-cols-4">
              <ProofFact label={t("trace.traceId")} value={trace?.id ?? t("common.none")} />
              <ProofFact
                label={t("trace.runSeed")}
                value={trace?.reproducibility.run_seed ?? t("common.none")}
              />
              <ProofFact
                label={t("trace.stateDeltas")}
                value={t("common.stateDeltas", { count: report?.delta_summary.length ?? 0 })}
              />
              <ProofFact
                label={t("trace.packageReadiness")}
                value={packageReady ? t("trace.localReady") : t("trace.reviewRequired")}
              />
            </div>
          </section>

          {error ? (
            <div className="rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
              {error}
            </div>
          ) : null}

          <Collapsible
            label={t("trace.runResultSummary")}
            defaultOpen={false}
            badge={report?.delta_summary.length}
          >
            <div className="mt-3 grid gap-3 lg:grid-cols-[1fr_0.9fr]">
              <section className="rounded-lg border border-canvas-200 bg-canvas-50 p-4 text-ink shadow-studio-panel">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <h3 className="text-base font-semibold">{t("trace.stateDelta")}</h3>
                  <Badge tone="neutral">
                    {report?.delta_summary.length ?? 0}
                  </Badge>
                </div>
                <DeltaList lines={report?.delta_summary ?? []} />
              </section>

              <section className="rounded-lg border border-canvas-200 bg-canvas-50 p-4 text-ink shadow-studio-panel">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <h3 className="text-base font-semibold">{t("trace.runEvidence")}</h3>
                  <Bug aria-hidden className="text-signal" size={20} />
                </div>
                <div className="mt-3 grid gap-2 sm:grid-cols-2">
                  <TraceField
                    label={t("trace.selectedChoice")}
                    value={trace?.selected_choice}
                  />
                  <TraceField
                    label={t("trace.intent")}
                    value={trace?.action_intent?.action_type ?? t("common.none")}
                  />
                  <TraceField
                    label={t("trace.promptVersion")}
                    value={trace?.reproducibility.prompt_version}
                  />
                  <TraceField label={t("trace.snapshot")} value={report?.snapshot_path} />
                </div>
              </section>
            </div>

            <div className="mt-3">
              <ErrorList errors={trace?.errors ?? []} />
            </div>
          </Collapsible>

          {trace ? (
            <CollapsibleSection
              title={t("trace.technicalDetails")}
              defaultOpen={false}
              badge={trace.errors.length > 0 ? trace.errors.length : undefined}
            >
              <section aria-label={t("trace.aria.traceDebug")} className="grid gap-5">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="min-w-0">
                    <p className="text-xs font-semibold uppercase text-accent-500">
                      {t("trace.traceDebug")}
                    </p>
                    <h3 className="mt-1 text-lg font-semibold">
                      {t("trace.redactionSafeCausality")}
                    </h3>
                  </div>
                  {trace.errors.length ? (
                    <AlertTriangle
                      aria-hidden
                      className="text-signal"
                      size={22}
                    />
                  ) : (
                    <Network aria-hidden className="text-accent-500" size={22} />
                  )}
                </div>

                <div className="grid gap-5">
                  <CausalityGraph trace={trace} />
                  <TraceEvidence trace={trace} />
                  <ToolMetadata
                    trace={trace}
                    exportReport={exportReport}
                  />
                  <WorldDeltaEvidence trace={trace} />
                  <MediaReferenceList
                    references={trace.media_references}
                  />
                  {review ? <NarrativeReviewPanel review={review} /> : null}
                  {review?.issues.length ? (
                    <IssueList issues={review.issues} />
                  ) : null}
                  <DiagnosticList diagnostics={trace.diagnostics} />
                </div>
              </section>
            </CollapsibleSection>
          ) : (
            <div className="rounded-md border border-ink/10 bg-canvas-50 p-4 text-sm text-ink/55">
              {t("trace.runToInspectTrace")}
            </div>
          )}

          <Collapsible
            label={t("trace.advancedSnapshotControls")}
            defaultOpen={false}
            id="trace-snapshot-controls"
            className="rounded-md border border-canvas-200 bg-canvas-50 p-3"
          >
            <div className="mt-2 grid gap-3 lg:grid-cols-[1fr_1fr_auto]">
              <label className="grid min-w-0 gap-1">
                <span className="text-xs font-medium uppercase text-ink/45">
                  {t("trace.saveId")}
                </span>
                <input
                  aria-label={t("trace.aria.playtestSaveId")}
                  value={saveId}
                  onChange={(event) => onSaveIdChange?.(event.target.value)}
                  className={studioUiClassNames.input}
                />
              </label>
              <label className="grid min-w-0 gap-1">
                <span className="text-xs font-medium uppercase text-ink/45">
                  {t("trace.restoreId")}
                </span>
                <input
                  aria-label={t("trace.aria.playtestRestoreId")}
                  value={restoreId}
                  disabled={restoreLatest}
                  onChange={(event) => onRestoreIdChange?.(event.target.value)}
                  className={`${studioUiClassNames.input} disabled:cursor-not-allowed disabled:bg-ink/5 disabled:text-ink/35`}
                />
              </label>
              <label className="flex min-h-10 items-center gap-2 self-end rounded-md border border-ink/10 px-3 text-sm font-medium text-ink/70">
                <input
                  type="checkbox"
                  aria-label={t("trace.aria.restoreLatestSave")}
                  checked={restoreLatest}
                  onChange={(event) => onRestoreLatestChange?.(event.target.checked)}
                  className="h-4 w-4 accent-ink"
                />
                {t("trace.restoreLatest")}
              </label>
            </div>
            <p className="mt-3 text-xs text-ink/55">
              {t("trace.snapshotHint")}
            </p>
          </Collapsible>
        </div>

        <aside
          aria-label={t("trace.aria.proofEvidencePanel")}
          className="grid content-start gap-4 rounded-lg border border-canvas-200 bg-graphite-950 p-4 text-ink shadow-studio-panel lg:col-start-2 lg:row-start-1"
        >
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-xs font-semibold uppercase text-health-400">
                {t("trace.agentEvidence")}
              </p>
              <h3 className="mt-1 text-lg font-semibold text-ink">
                {t("trace.proofPackage")}
              </h3>
            </div>
            <ShieldCheck aria-hidden className="text-health-400" size={22} />
          </div>

          <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
            <p className="text-sm font-semibold text-ink">
              {t("trace.packageEvidenceSummary")}
            </p>
            <div className="mt-3 grid gap-2">
              {exportReport ? (
                [
                  [t("trace.filesWritten"), String(exportReport.files_written.length)],
                  [
                    t("trace.archivedFiles"),
                    String(exportReport.archived_files.length),
                  ],
                  [t("trace.allowedFiles"), String(exportReport.allowed_files.length)],
                ].map(([label, value]) => (
                  <div
                    key={label}
                    className="flex items-center justify-between gap-3 border-t border-canvas-200 pt-2 text-sm first:border-t-0 first:pt-0"
                  >
                    <span className="truncate text-ink">{label}</span>
                    <span className="shrink-0 text-health-400">{value}</span>
                  </div>
                ))
              ) : (
                <p className="text-sm text-graphite-700/60">
                  {t("trace.noExportReport")}
                </p>
              )}
            </div>
          </div>

          <DisclosureDraft
            title={t("trace.aiUsageDisclosure")}
            body={
              aiSafetyPolicy
                ? aiSafetyPolicy.moderation_policy
                : t("trace.aiUsageAfterPolicy")
            }
          />
          <DisclosureDraft
            title={t("trace.contentWarningDraft")}
            body={
              aiSafetyPolicy
                ? t("trace.contentKindsBody", {
                    kinds: aiSafetyPolicy.content_kinds.join(", "),
                    review: aiSafetyPolicy.human_review_required ? t("common.true") : t("common.false"),
                  })
                : t("trace.contentWarningAfterPolicy")
            }
          />

          <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-sm font-semibold text-ink">
                  {t("trace.localStaticWebPackage")}
                </p>
                <p className="mt-1 text-xs text-graphite-700/60">
                  {selectedExportProfile?.id ?? t("trace.noProfileSelected")}
                </p>
              </div>
              <Badge tone={packageReady ? "completed" : "fallback"}>
                {packageReady ? t("trace.badgeReady") : t("common.review")}
              </Badge>
            </div>
            <div className="mt-3 grid gap-2 text-sm">
              <ProofSideFact
                label={t("trace.archive")}
                value={exportReport?.archive_path ?? t("common.notExported")}
              />
              <ProofSideFact
                label={t("trace.files")}
                value={exportReport?.archived_files.length ?? 0}
              />
              <ProofSideFact
                label={t("trace.audit")}
                value={
                  exportReport
                    ? sameStringSet(
                        exportReport.allowed_files,
                        exportReport.files_found,
                      )
                      ? t("common.matched")
                      : t("common.mismatch")
                    : t("common.pending")
                }
              />
            </div>
          </div>
        </aside>
      </div>
    </Reveal>
  );
}

export function NarrativeReviewPanel({
  review,
}: NarrativeReviewPanelProps) {
  const { t } = useStudioI18n();
  const scores: Array<[string, number]> = [
    [t("trace.hook"), review.hook_score],
    [t("trace.pacing"), review.pacing_score],
    [t("trace.character"), review.character_consistency_score],
    [t("trace.payoff"), review.payoff_score],
    [t("trace.choice"), review.choice_meaningfulness_score],
    [t("trace.aiSlop"), review.ai_slop_risk],
  ];

  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.narrativeReview")}</h4>
      <div className="mt-2 grid gap-2 sm:grid-cols-3">
        {scores.map(([label, value]) => (
          <div key={label} className="border-t border-ink/10 pt-2">
            <p className="text-xs font-medium uppercase text-ink/45">
              {label}
            </p>
            <p className="mt-1 text-lg font-semibold text-ink">{value}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function TraceEvidence({
  trace,
}: {
  trace: PlayOnceReport["trace"];
}) {
  const { t } = useStudioI18n();
  const intent = trace.action_intent;
  const rule = trace.rule_result;
  const planner = trace.planner_result;
  const reproducibility = trace.reproducibility;

  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.traceEvidence")}</h4>
      <div className="mt-2 grid gap-3 lg:grid-cols-4">
        <EvidenceGroup title={t("trace.actionIntent")}>
          <TraceField label={t("trace.status")} value={intent?.status} />
          <TraceField label={t("trace.choiceLabel")} value={intent?.choice_id} />
          <TraceField label={t("trace.action")} value={intent?.action_type} />
          <TraceField
            label={t("trace.matchedTerms")}
            value={
              intent?.matched_terms.length
                ? intent.matched_terms.join(", ")
                : null
            }
          />
          <TraceField label={t("trace.reason")} value={intent?.reason} />
        </EvidenceGroup>

        <EvidenceGroup title={t("trace.ruleResult")}>
          <TraceField label={t("trace.action")} value={rule?.action_type} />
          <TraceField
            label={t("trace.committed")}
            value={rule ? (rule.state_committed ? t("common.true") : t("common.false")) : null}
          />
          <TraceField
            label={t("trace.deltaEmpty")}
            value={rule ? (rule.delta_empty ? t("common.true") : t("common.false")) : null}
          />
          <TraceField label={t("trace.errorCode")} value={rule?.error?.code} />
          <TraceField label={t("trace.error")} value={rule?.error?.message} />
        </EvidenceGroup>

        <EvidenceGroup title={t("trace.plannerResult")}>
          <TraceField
            label={t("trace.requested")}
            value={planner?.requested_action_type}
          />
          <TraceField label={t("trace.scene")} value={planner?.scene_key} />
          <TraceField
            label={t("common.fallback")}
            value={planner ? (planner.fallback_used ? t("common.true") : t("common.false")) : null}
          />
          <TraceField label={t("trace.errorCode")} value={planner?.error?.code} />
          <TraceField label={t("trace.error")} value={planner?.error?.message} />
        </EvidenceGroup>

        <EvidenceGroup title={t("trace.reproducibility")}>
          <TraceField label={t("trace.runSeed")} value={reproducibility.run_seed} />
          <TraceField
            label={t("trace.promptVersion")}
            value={reproducibility.prompt_version}
          />
          <TraceField
            label={t("trace.modelVersion")}
            value={reproducibility.model_version}
          />
          <TraceField
            label={t("trace.providerConfigHash")}
            value={reproducibility.provider_config_hash}
          />
          <TraceField
            label={t("trace.mcpToolCallHash")}
            value={reproducibility.mcp_tool_call_hash ?? null}
          />
          <TraceField label={t("trace.traceIdField")} value={reproducibility.trace_id} />
          <TraceField label={t("trace.snapshotId")} value={reproducibility.snapshot_id} />
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
    <div className="min-w-0 rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
      <p className="text-xs font-semibold uppercase tracking-eyebrow text-copper-500/80">
        {label}
      </p>
      <p className="font-display mt-1 truncate text-sm font-semibold tracking-tightish text-ink">
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
      <p className="text-xs font-medium uppercase text-graphite-700/55">
        {label}
      </p>
      <p className="truncate text-xs font-semibold text-ink">{value}</p>
    </div>
  );
}

function DisclosureDraft({
  title,
  body,
}: {
  title: string;
  body: string;
}) {
  const { t } = useStudioI18n();
  return (
    <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
      <div className="flex items-start justify-between gap-3">
        <p className="text-sm font-semibold text-ink">{title}</p>
        <Badge tone="fallback">{t("trace.localDraftOnly")}</Badge>
      </div>
      <p className="mt-3 text-sm leading-6 text-graphite-700/70">{body}</p>
    </div>
  );
}

function CausalityGraph({
  trace,
}: {
  trace: PlayOnceReport["trace"];
}) {
  const { t } = useStudioI18n();
  const steps = [
    {
      id: "intent",
      label: t("trace.directorIntent"),
      detail: trace.action_intent?.action_type ?? t("trace.unsupported"),
      tone: "border-accent-500/35 bg-accent-500/10 text-accent-400",
    },
    {
      id: "rule",
      label: t("trace.rulesPatch"),
      detail: trace.rule_result?.state_committed
        ? t("common.committed")
        : t("common.notCommitted"),
      tone: "border-health-500/35 bg-health-500/10 text-health-400",
    },
    {
      id: "planner",
      label: t("trace.runtimePlaytest"),
      detail: trace.planner_result?.scene_key ?? t("common.none"),
      tone: "border-agent-500/35 bg-agent-500/10 text-agent-400",
    },
    {
      id: "diagnostics",
      label: t("trace.diagnostics"),
      detail: t("common.steps", { count: trace.diagnostics.length }),
      tone: "border-violet-500/35 bg-violet-500/10 text-violet-600",
    },
  ];

  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.causalityGraph")}</h4>
      <div className="mt-3 grid gap-3 lg:grid-cols-4">
        {steps.map((step, index) => (
          <div key={step.id} className="min-w-0">
            <div className={`rounded-md border px-3 py-3 ${step.tone}`}>
              <p className="text-sm font-semibold">{step.label}</p>
              <p className="mt-1 truncate text-xs opacity-75">
                {step.detail}
              </p>
            </div>
            {index < steps.length - 1 ? (
              <p className="mt-2 text-center text-xs font-semibold text-ink/35">
                {t("trace.flowsTo")}
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
  const { t } = useStudioI18n();
  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.toolMetadata")}</h4>
      <div className="mt-2 grid gap-3 sm:grid-cols-4">
        <TraceField
          label={t("trace.promptHash")}
          value={trace.reproducibility.trace_id}
        />
        <TraceField
          label={t("trace.promptVersion")}
          value={trace.reproducibility.prompt_version}
        />
        <TraceField label={t("trace.toolCalls")} value={trace.diagnostics.length} />
        <TraceField
          label={t("trace.exportFiles")}
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

function WorldDeltaEvidence({
  trace,
}: {
  trace: PlayOnceReport["trace"];
}) {
  const { t } = useStudioI18n();
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
      <h4 className="text-sm font-semibold">{t("trace.stateDelta")}</h4>
      <DeltaList lines={lines} />
    </div>
  );
}

function MediaReferenceList({
  references,
}: {
  references: PlayOnceReport["trace"]["media_references"];
}) {
  const { t } = useStudioI18n();
  if (references.length === 0) {
    return (
      <div className="border-t border-ink/10 pt-3 text-sm text-ink/55">
        {t("trace.noMediaReferences")}
      </div>
    );
  }

  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.mediaReferences")}</h4>
      <div className="mt-2 grid gap-2">
        {references.map((reference) => (
          <div
            key={`${reference.reference.reference_kind}:${reference.reference.reference_id}:${reference.reference.slot}`}
            className="border-t border-ink/10 pt-2 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Badge tone="neutral">
                {reference.reference.reference_kind}
              </Badge>
              <code className="text-xs text-ink/55">
                {reference.reference.slot}
              </code>
            </div>
            <p className="mt-1 text-ink/75">
              {reference.reference.reference_id}
            </p>
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
  const { t } = useStudioI18n();
  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.reviewIssues")}</h4>
      <div className="mt-2 grid gap-2">
        {issues.map((issue) => (
          <div
            key={`${issue.kind}:${issue.message}`}
            className="border-t border-ink/10 pt-2 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Badge
                tone={issue.severity === "error" ? "error" : "fallback"}
              >
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
  const { t } = useStudioI18n();
  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.diagnosticsLabel")}</h4>
      <div className="mt-2 grid gap-2">
        {diagnostics.map((diagnostic) => (
          <div
            key={`${diagnostic.stage}:${diagnostic.status}:${diagnostic.message}`}
            className="border-t border-ink/10 pt-2 text-sm"
          >
            <div className="flex flex-wrap items-center gap-2">
              <Badge
                tone={
                  diagnostic.status === "skipped"
                    ? "neutral"
                    : diagnostic.status
                }
              >
                {diagnostic.status}
              </Badge>
              <code className="text-xs text-ink/55">
                {diagnostic.stage}
              </code>
            </div>
            <p className="mt-1 text-ink/75">{diagnostic.message}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function ErrorList({
  errors,
}: {
  errors: PlayOnceReport["trace"]["errors"];
}) {
  const { t } = useStudioI18n();
  if (errors.length === 0) {
    return (
      <div className="flex items-center gap-2 border-t border-ink/10 pt-3 text-sm text-sage">
        <CheckCircle2 aria-hidden size={16} />
        {t("trace.noRuntimeErrors")}
      </div>
    );
  }

  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.errors")}</h4>
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
  const { t } = useStudioI18n();
  if (lines.length === 0) {
    return <p className="text-sm text-ink/55">{t("trace.noWorldDelta")}</p>;
  }

  return (
    <div>
      <h4 className="text-sm font-semibold">{t("trace.worldDelta")}</h4>
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
  const { t } = useStudioI18n();
  return (
    <div className="border-t border-ink/10 pt-2">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">
        {value ?? t("common.none")}
      </p>
      {detail ? (
        <p className="mt-0.5 text-xs text-ink/50">{detail}</p>
      ) : null}
    </div>
  );
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
    completed: "border-sage/30 bg-sage/10 text-sage",
    fallback: "border-plum-500/30 bg-plum-500/10 text-plum-500",
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
