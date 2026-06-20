import {
  Boxes,
  FileText,
  GitBranch,
  PackageCheck,
  Play,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import type { CreatorProjectSummary } from "./projectSummary";
import { StudioButton, StudioStatusChip } from "./studioUi";
import type { AssetCatalog } from "./studioModel";
import type {
  PlayOnceReport,
  SourceFileSummary,
  StaticExportReport,
} from "./tauriBridge";

interface ArtifactReviewViewProps {
  projectSummary: CreatorProjectSummary | null;
  loadedPath: string;
  sourceFiles: SourceFileSummary[];
  assetCatalog: AssetCatalog;
  playtestReport: PlayOnceReport | null;
  playtesting: boolean;
  playtestError: string | null;
  exportReport: StaticExportReport | null;
  assetMaintenance: ReactNode;
  onOpenTrace(): void;
  onRunPlayableProof(): void;
}

export function ArtifactReviewView({
  projectSummary,
  loadedPath,
  sourceFiles,
  assetCatalog,
  playtestReport,
  playtesting,
  playtestError,
  exportReport,
  assetMaintenance,
  onOpenTrace,
  onRunPlayableProof,
}: ArtifactReviewViewProps) {
  const editableFiles = sourceFiles.filter((file) => file.editable);
  const assetRecordCount = assetCatalog.items.filter(
    (item) => item.source === "record",
  ).length;
  const traceId = playtestReport?.trace.id ?? "not captured";
  const exportedFileCount = exportReport?.files_written.length ?? 0;

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
                No build run interface
              </h3>
              <p className="mt-1 text-sm leading-6 text-canvas-200/65">
                Real Studio commands expose source files, assets, runtime proof,
                and export reports. They do not expose an agent build queue yet.
              </p>
            </div>
            <StudioStatusChip tone="danger">not implemented</StudioStatusChip>
          </div>

          <EvidenceCard title="Source Files" value={String(sourceFiles.length)}>
            {`${editableFiles.length} editable surfaces loaded from the project.`}
          </EvidenceCard>
          <EvidenceCard title="Runtime Trace" value={traceId}>
            {playtestReport
              ? `${playtestReport.delta_summary.length} state deltas produced.`
              : "Run a playable proof to generate trace evidence."}
          </EvidenceCard>
          <EvidenceCard title="Export Report" value={String(exportedFileCount)}>
            {exportReport
              ? `${exportReport.archived_files.length} files archived.`
              : "Run a static export to inspect package evidence."}
          </EvidenceCard>
        </aside>

        <div className="grid gap-4">
          <div className="rounded-lg border border-amber-500/35 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-amber-400">
                  Artifact Review
                </p>
                <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                  {projectSummary?.title ?? loadedPath}
                </h3>
                <p className="mt-2 max-w-3xl text-sm leading-6 text-canvas-200/65">
                  This view now reviews artifacts returned by real Studio
                  commands. Agent-generated patch bundles are hidden until a
                  real persisted artifact interface exists.
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="health">project source</StudioStatusChip>
                <StudioStatusChip tone="neutral">{loadedPath}</StudioStatusChip>
              </div>
            </div>

            <div className="mt-4 grid gap-3 md:grid-cols-4">
              <BundleFact label="Source files" value={String(sourceFiles.length)} />
              <BundleFact label="Editable" value={String(editableFiles.length)} />
              <BundleFact label="Asset records" value={String(assetRecordCount)} />
              <BundleFact label="Trace" value={traceId} />
            </div>

            <div className="mt-4 rounded-md border border-amber-500/25 bg-amber-500/10 px-3 py-3">
              <div className="flex gap-2">
                <ShieldCheck
                  aria-hidden
                  size={18}
                  className="mt-0.5 shrink-0 text-amber-400"
                />
                <p className="text-sm leading-6 text-canvas-50">
                  No approval action is available because there is no real
                  approval queue or persisted proposal bundle contract.
                </p>
              </div>
            </div>
          </div>

          <section aria-label="Current Source Artifacts" className="grid gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h3 className="text-base font-semibold text-ink">
                Current Source Artifacts
              </h3>
              <StudioStatusChip tone="health">{sourceFiles.length} files</StudioStatusChip>
            </div>
            <div className="grid gap-3 xl:grid-cols-2">
              {sourceFiles.slice(0, 8).map((file) => (
                <article
                  key={file.path}
                  className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel"
                >
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-xs font-semibold uppercase text-graphite-700/45">
                        {file.kind}
                      </p>
                      <h4 className="mt-1 truncate text-base font-semibold">
                        {file.path}
                      </h4>
                    </div>
                    <StudioStatusChip tone={file.editable ? "health" : "neutral"}>
                      {file.editable ? "editable" : "read only"}
                    </StudioStatusChip>
                  </div>
                  <EvidenceBlock label="Bytes" value={String(file.bytes)} />
                  <EvidenceBlock
                    label="Source"
                    value="Loaded through StudioDataSource list_source_files"
                  />
                </article>
              ))}
            </div>
          </section>

          <div className="grid gap-4 xl:grid-cols-[1fr_0.9fr]">
            <section className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-base font-semibold">Runtime Impact</h3>
                <StudioStatusChip tone={playtestReport ? "health" : "neutral"}>
                  {traceId}
                </StudioStatusChip>
              </div>
              <div className="mt-3 grid gap-2">
                {(playtestReport?.delta_summary ?? []).map((line) => (
                  <p
                    key={line}
                    className="rounded-md border border-ink/10 bg-canvas-100 px-3 py-2 text-sm leading-6 text-ink/70"
                  >
                    {line}
                  </p>
                ))}
                {!playtestReport ? (
                  <p className="rounded-md border border-ink/10 bg-canvas-100 px-3 py-2 text-sm leading-6 text-ink/55">
                    No runtime proof has been run for this session.
                  </p>
                ) : null}
                {playtestError ? (
                  <p className="text-sm text-signal">{playtestError}</p>
                ) : null}
              </div>
            </section>

            <section className="rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 text-ink shadow-studio-panel">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <h3 className="text-base font-semibold">Export Evidence</h3>
                <StudioStatusChip tone={exportReport ? "health" : "neutral"}>
                  {exportReport ? "available" : "not run"}
                </StudioStatusChip>
              </div>
              <div className="mt-3 grid gap-2">
                <ExportFact
                  label="Output"
                  value={exportReport?.output_dir ?? "not exported"}
                />
                <ExportFact
                  label="Archive"
                  value={exportReport?.archive_path ?? "not archived"}
                />
                <ExportFact
                  label="Files"
                  value={String(exportReport?.files_written.length ?? 0)}
                />
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
                Real command outputs
              </h3>
            </div>
            <StudioStatusChip tone="health">visible</StudioStatusChip>
          </div>

          <ValidationLine
            icon={FileText}
            label="Project source"
            detail={`${sourceFiles.length} listed files from storage adapter.`}
          />
          <ValidationLine
            icon={Boxes}
            label="Asset registry"
            detail={`${assetRecordCount} asset records from plotforge-media/storage.`}
          />
          <ValidationLine
            icon={GitBranch}
            label="Runtime trace"
            detail={playtestReport ? playtestReport.trace_path : "No trace captured yet."}
          />
          <ValidationLine
            icon={PackageCheck}
            label="Static package"
            detail={
              exportReport
                ? `${exportReport.files_found.length} files found after export.`
                : "No export report captured yet."
            }
          />

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <p className="text-sm font-semibold text-canvas-50">
              Available Actions
            </p>
            <div className="mt-3 grid gap-2">
              <StudioButton onClick={onRunPlayableProof} disabled={playtesting}>
                <Play aria-hidden size={16} />
                {playtesting ? "Running proof" : "Run proof"}
              </StudioButton>
              <StudioButton onClick={onOpenTrace}>
                <GitBranch aria-hidden size={16} />
                View trace
              </StudioButton>
            </div>
          </div>
        </aside>
      </div>

      {assetMaintenance}
    </section>
  );
}

function EvidenceCard({
  title,
  value,
  children,
}: {
  title: string;
  value: string;
  children: ReactNode;
}) {
  return (
    <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-semibold text-canvas-50">{title}</p>
        <StudioStatusChip tone="neutral">{value}</StudioStatusChip>
      </div>
      <p className="mt-2 text-xs leading-5 text-canvas-200/60">{children}</p>
    </div>
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

function ExportFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 justify-between gap-3 border-t border-ink/10 pt-2 first:border-t-0 first:pt-0">
      <span className="text-sm text-ink/45">{label}</span>
      <span className="truncate text-sm font-semibold text-ink">{value}</span>
    </div>
  );
}

function ValidationLine({
  icon: Icon,
  label,
  detail,
}: {
  icon: LucideIcon;
  label: string;
  detail: string;
}) {
  return (
    <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
      <div className="flex items-start gap-2">
        <Icon aria-hidden size={16} className="mt-0.5 shrink-0 text-health-400" />
        <div className="min-w-0">
          <p className="text-sm font-semibold text-canvas-50">{label}</p>
          <p className="mt-1 text-xs leading-5 text-canvas-200/60">{detail}</p>
        </div>
      </div>
    </div>
  );
}
