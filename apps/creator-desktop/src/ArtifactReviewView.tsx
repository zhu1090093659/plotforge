import {
  CheckCircle2,
  Circle,
  Clock3,
  FileText,
  GitBranch,
  PackageCheck,
  Play,
  ShieldCheck,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import type { LocalPreviewState } from "./localPreviewModel";
import type { CreatorProjectSummary } from "./projectSummary";
import { StudioButton, StudioStatusChip } from "./studioUi";
import type { AssetCatalog } from "./studioModel";
import type { PlayOnceReport } from "./tauriBridge";

interface ArtifactReviewViewProps {
  projectSummary: CreatorProjectSummary | null;
  loadedPath: string;
  assetCatalog: AssetCatalog;
  localPreviewState: LocalPreviewState;
  playtestReport: PlayOnceReport | null;
  playtesting: boolean;
  playtestError: string | null;
  assetMaintenance: ReactNode;
  onOpenTrace(): void;
  onRunPlayableProof(): void;
}

export function ArtifactReviewView({
  projectSummary,
  loadedPath,
  assetCatalog,
  localPreviewState,
  playtestReport,
  playtesting,
  playtestError,
  assetMaintenance,
  onOpenTrace,
  onRunPlayableProof,
}: ArtifactReviewViewProps) {
  const [localDecision, setLocalDecision] = useState<string | null>(null);
  const { buildRun, artifactBundle } = localPreviewState;
  const pendingApprovals = localPreviewState.approvals.filter(
    (approval) => approval.state === "pending-local-review",
  );
  const changedFiles = artifactBundle.changes.flatMap((change) =>
    change.files.map((file) => ({ ...file, changeTitle: change.title })),
  );
  const traceId = playtestReport?.trace.id ?? "waiting for proof run";
  const stateDeltaCount = playtestReport?.delta_summary.length ?? 0;
  const assetRecordCount = assetCatalog.items.filter(
    (item) => item.source === "record",
  ).length;

  return (
    <section aria-label="Artifact Review Workspace" className="grid gap-5">
      <div className="grid gap-4 2xl:grid-cols-[300px_minmax(0,1fr)_340px]">
        <aside
          aria-label="Live Build Room"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
        >
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-acp-400">
                Live Build Room
              </p>
              <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                {buildRun.id}
              </h3>
              <p className="mt-1 text-sm leading-6 text-canvas-200/65">
                {buildRun.intent}
              </p>
            </div>
            <StudioStatusChip tone="action">{buildRun.status}</StudioStatusChip>
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Run Timeline
            </p>
            <div className="mt-3 grid gap-3">
              {buildRun.timeline.map((step) => (
                <div key={step.id} className="flex gap-3">
                  <TimelineIcon state={step.state} />
                  <div className="min-w-0 flex-1 border-b border-canvas-200/10 pb-3 last:border-b-0 last:pb-0">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <p className="text-sm font-semibold text-canvas-50">
                        {step.label}
                      </p>
                      <span className="text-xs font-semibold text-canvas-200/45">
                        {step.duration}
                      </span>
                    </div>
                    <p className="mt-1 text-xs leading-5 text-canvas-200/60">
                      {step.detail}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Agent Work Status
            </p>
            <div className="mt-3 grid gap-2">
              {buildRun.agentStatuses.map((status) => {
                const worker = localPreviewState.workers.find(
                  (candidate) => candidate.id === status.workerId,
                );
                return (
                  <div
                    key={status.workerId}
                    className="rounded-md border border-canvas-200/10 bg-graphite-950/60 px-3 py-2"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <p className="min-w-0 truncate text-sm font-semibold text-canvas-50">
                        {worker?.label ?? status.workerId}
                      </p>
                      <span className="shrink-0 text-xs font-semibold text-health-400">
                        {status.status}
                      </span>
                    </div>
                    <p className="mt-1 text-xs leading-5 text-canvas-200/60">
                      {status.currentTask}
                    </p>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Approval Queue
            </p>
            <div className="mt-3 grid gap-2">
              {localPreviewState.approvals.map((approval) => (
                <div
                  key={approval.id}
                  className="rounded-md border border-canvas-200/10 bg-graphite-950/60 px-3 py-2"
                >
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <p className="text-sm font-semibold text-canvas-50">
                      {approval.title}
                    </p>
                    <span className="text-xs font-semibold text-amber-400">
                      {approval.state}
                    </span>
                  </div>
                  <p className="mt-1 truncate text-xs text-canvas-200/55">
                    {approval.evidenceIds.join(" / ")}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </aside>

        <div className="grid gap-4">
          <div className="rounded-lg border border-amber-500/35 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-amber-400">
                  Artifact Review
                </p>
                <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                  {artifactBundle.title}
                </h3>
                <p className="mt-2 max-w-3xl text-sm leading-6 text-canvas-200/65">
                  {artifactBundle.summary}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="action">
                  {artifactBundle.state}
                </StudioStatusChip>
                <StudioStatusChip tone="acp">
                  {pendingApprovals.length} pending approvals
                </StudioStatusChip>
              </div>
            </div>

            <div className="mt-4 grid gap-3 md:grid-cols-4">
              <BundleFact label="Project" value={projectSummary?.title ?? loadedPath} />
              <BundleFact label="Scope" value={artifactBundle.scope} />
              <BundleFact label="Created" value={artifactBundle.createdAt} />
              <BundleFact
                label="Asset records"
                value={
                  assetCatalog.source === "records"
                    ? String(assetRecordCount)
                    : "scene background fallback"
                }
              />
            </div>

            <div className="mt-4 rounded-md border border-amber-500/25 bg-amber-500/10 px-3 py-3">
              <div className="flex gap-2">
                <ShieldCheck
                  aria-hidden
                  size={18}
                  className="mt-0.5 shrink-0 text-amber-400"
                />
                <p className="text-sm leading-6 text-canvas-50">
                  {artifactBundle.approvalRequirement}
                </p>
              </div>
              {localDecision ? (
                <p className="mt-2 rounded-md border border-canvas-200/10 bg-graphite-950/50 px-3 py-2 text-sm text-health-400">
                  {localDecision}
                </p>
              ) : null}
            </div>
          </div>

          <section aria-label="Proposed Changes" className="grid gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h3 className="text-base font-semibold text-ink">
                Proposed Changes
              </h3>
              <StudioStatusChip tone="agent">
                {artifactBundle.changes.length} artifact groups
              </StudioStatusChip>
            </div>
            <div className="grid gap-3 xl:grid-cols-2">
              {artifactBundle.changes.map((change) => (
                <article
                  key={change.id}
                  className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel"
                >
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-xs font-semibold uppercase text-graphite-700/45">
                        {change.capabilityId}
                      </p>
                      <h4 className="mt-1 text-base font-semibold">
                        {change.title}
                      </h4>
                    </div>
                    <StudioStatusChip tone="agent">
                      {change.files.length} files
                    </StudioStatusChip>
                  </div>
                  <EvidenceBlock label="What changed" value={change.summary} />
                  <EvidenceBlock label="Why" value={change.reason} />
                  <EvidenceBlock label="Validation" value={change.validation} />
                  <EvidenceBlock
                    label="Playable impact"
                    value={change.playableImpact}
                  />
                  <div className="mt-3 grid gap-2">
                    {change.files.map((file) => (
                      <FileRow key={file.path} file={file} />
                    ))}
                  </div>
                </article>
              ))}
            </div>
          </section>

          <div className="grid gap-4 xl:grid-cols-[1fr_0.9fr]">
            <section className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-base font-semibold">Files Changed</h3>
                <StudioStatusChip tone="neutral">
                  {changedFiles.length} files
                </StudioStatusChip>
              </div>
              <div className="mt-3 grid gap-2">
                {changedFiles.map((file) => (
                  <div
                    key={`${file.changeTitle}:${file.path}`}
                    className="grid gap-2 rounded-md border border-ink/10 bg-canvas-100 px-3 py-2 sm:grid-cols-[minmax(0,1fr)_auto]"
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-semibold">
                        {file.path}
                      </p>
                      <p className="mt-1 text-xs text-ink/50">
                        {file.changeTitle} / {file.status}
                      </p>
                    </div>
                    <p className="text-sm font-semibold">
                      <span className="text-health-500">
                        +{file.additions}
                      </span>
                      <span className="mx-1 text-ink/25">/</span>
                      <span className="text-signal">-{file.deletions}</span>
                    </p>
                  </div>
                ))}
              </div>
            </section>

            <section className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-base font-semibold">Playable Impact</h3>
                <StudioStatusChip tone="health">{traceId}</StudioStatusChip>
              </div>
              <div className="mt-3 grid gap-2">
                {artifactBundle.playableImpact.map((impact) => (
                  <p
                    key={impact}
                    className="rounded-md border border-ink/10 bg-canvas-100 px-3 py-2 text-sm leading-6 text-ink/70"
                  >
                    {impact}
                  </p>
                ))}
              </div>
              <div className="mt-3 rounded-md border border-ink/10 bg-canvas-100 px-3 py-2">
                <p className="text-xs font-medium uppercase text-ink/45">
                  Playtest output
                </p>
                <div className="mt-2 grid gap-1">
                  {(playtestReport?.delta_summary ?? buildRun.playtestOutput).map(
                    (line) => (
                      <p key={line} className="text-sm leading-6 text-ink/70">
                        {line}
                      </p>
                    ),
                  )}
                  {playtestError ? (
                    <p className="text-sm text-signal">{playtestError}</p>
                  ) : null}
                  <p className="text-sm font-semibold text-ink">
                    State deltas: {stateDeltaCount}
                  </p>
                </div>
              </div>
            </section>
          </div>
        </div>

        <aside
          aria-label="Validation Evidence"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
        >
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-health-400">
                Validation Evidence
              </p>
              <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                {buildRun.validationSummary}
              </h3>
            </div>
            <StudioStatusChip tone="health">
              {artifactBundle.validationEvidence.length} checks
            </StudioStatusChip>
          </div>

          <div className="grid gap-2">
            {artifactBundle.validationEvidence.map((evidence) => (
              <div
                key={evidence.id}
                className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3"
              >
                <div className="flex items-start gap-2">
                  <CheckCircle2
                    aria-hidden
                    size={16}
                    className="mt-0.5 shrink-0 text-health-400"
                  />
                  <div className="min-w-0">
                    <div className="flex flex-wrap items-center gap-2">
                      <p className="text-sm font-semibold text-canvas-50">
                        {evidence.label}
                      </p>
                      <span className="text-xs font-semibold text-health-400">
                        {evidence.state}
                      </span>
                    </div>
                    <p className="mt-1 text-xs leading-5 text-canvas-200/60">
                      {evidence.detail}
                    </p>
                    <code className="mt-2 block truncate text-xs text-acp-400">
                      {evidence.evidenceId}
                    </code>
                  </div>
                </div>
              </div>
            ))}
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Trace Metadata
            </p>
            <div className="mt-3 grid gap-2 text-sm">
              <TraceFact label="Trace ID" value={traceId} />
              <TraceFact
                label="Run seed"
                value={
                  playtestReport?.trace.reproducibility.run_seed ??
                  "seed-872314"
                }
              />
              <TraceFact
                label="Prompt version"
                value={
                  playtestReport?.trace.reproducibility.prompt_version ??
                  "plotforge-local-preview-v1"
                }
              />
              <TraceFact
                label="Model version"
                value={
                  playtestReport?.trace.reproducibility.model_version ??
                  "local-preview"
                }
              />
            </div>
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Approval Actions
            </p>
            <p className="mt-1 text-xs leading-5 text-canvas-200/60">
              Approval actions are local preview controls. They do not commit,
              write project files, export packages, or call providers.
            </p>
            <div className="mt-3 grid gap-2">
              <StudioButton onClick={onRunPlayableProof} disabled={playtesting}>
                <Play aria-hidden size={16} />
                {playtesting ? "Running proof" : "Request test"}
              </StudioButton>
              <StudioButton onClick={onOpenTrace}>
                <GitBranch aria-hidden size={16} />
                View trace
              </StudioButton>
              <StudioButton
                variant="primary"
                onClick={() =>
                  setLocalDecision(
                    "Approve Bundle recorded locally; no files committed or persisted.",
                  )
                }
              >
                <PackageCheck aria-hidden size={16} />
                Approve Bundle
              </StudioButton>
              <StudioButton
                onClick={() =>
                  setLocalDecision(
                    "Request revision recorded locally; artifact bundle remains uncommitted.",
                  )
                }
              >
                <FileText aria-hidden size={16} />
                Request revision
              </StudioButton>
            </div>
          </div>
        </aside>
      </div>

      {assetMaintenance}
    </section>
  );
}

function TimelineIcon({
  state,
}: {
  state: LocalPreviewState["buildRun"]["timeline"][number]["state"];
}) {
  if (state === "complete") {
    return (
      <CheckCircle2
        aria-hidden
        size={20}
        className="mt-0.5 shrink-0 text-health-400"
      />
    );
  }
  if (state === "active") {
    return (
      <Clock3
        aria-hidden
        size={20}
        className="mt-0.5 shrink-0 text-amber-400"
      />
    );
  }
  return (
    <Circle
      aria-hidden
      size={20}
      className="mt-0.5 shrink-0 text-canvas-200/35"
    />
  );
}

function BundleFact({ label, value }: { label: string; value: string }) {
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

function EvidenceBlock({ label, value }: { label: string; value: string }) {
  return (
    <div className="mt-3 rounded-md border border-ink/10 bg-canvas-100 px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 text-sm leading-6 text-ink/70">{value}</p>
    </div>
  );
}

function FileRow({
  file,
}: {
  file: LocalPreviewState["artifactBundle"]["changes"][number]["files"][number];
}) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3 rounded-md border border-ink/10 bg-canvas-100 px-3 py-2">
      <div className="min-w-0">
        <p className="truncate text-sm font-semibold">{file.path}</p>
        <p className="mt-0.5 text-xs text-ink/45">{file.status}</p>
      </div>
      <p className="shrink-0 text-xs font-semibold">
        <span className="text-health-500">+{file.additions}</span>
        <span className="mx-1 text-ink/25">/</span>
        <span className="text-signal">-{file.deletions}</span>
      </p>
    </div>
  );
}

function TraceFact({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3">
      <p className="text-xs font-medium uppercase text-canvas-200/45">{label}</p>
      <p className="truncate text-xs font-semibold text-canvas-50">
        {value}
      </p>
    </div>
  );
}
