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
import { StudioButton, StudioStatusChip, StudioTabs } from "./studioUi";
import type { AssetCatalog } from "./assetCatalog";
import type {
  PlayOnceReport,
  SourceFileSummary,
  StaticExportReport,
} from "./tauriBridge";
import { useStudioI18n } from "./i18n";

export interface ArtifactReviewViewProps {
  projectSummary: CreatorProjectSummary | null;
  loadedPath: string;
  sourceFiles: SourceFileSummary[];
  assetCatalog: AssetCatalog;
  playtestReport: PlayOnceReport | null;
  playtesting: boolean;
  playtestError: string | null;
  exportReport: StaticExportReport | null;
  onOpenTrace(): void;
  onRunPlayableProof(): void;
  /** Optional slot for additional content rendered below the main grid (e.g. AssetMaintenanceView). */
  children?: ReactNode;
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
  onOpenTrace,
  onRunPlayableProof,
  children,
}: ArtifactReviewViewProps) {
  const { t } = useStudioI18n();
  const editableFiles = sourceFiles.filter((file) => file.editable);
  const assetRecordCount = assetCatalog.items.filter(
    (item) => item.source === "record",
  ).length;
  const traceId = playtestReport?.trace.id ?? t("common.notCaptured");
  const exportedFileCount = exportReport?.files_written.length ?? 0;

  return (
    <section aria-label={t("artifacts.aria.workspace")} className="grid gap-4">
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px] 2xl:grid-cols-[minmax(0,1fr)_340px]">
        <aside
          aria-label={t("artifacts.aria.liveBuildRoom")}
          className="grid content-start gap-3 rounded-lg border border-canvas-200 bg-graphite-950 p-3 text-ink shadow-studio-panel lg:col-start-2 lg:row-start-1"
        >
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-accent-400">
                {t("artifacts.liveBuildRoom")}
              </p>
              <h3 className="mt-1 text-lg font-semibold text-ink">
                {t("artifacts.noBuildRunInterface")}
              </h3>
              <p className="mt-1 text-sm leading-6 text-graphite-700/70">
                {t("artifacts.studioCommandsExpose")}
              </p>
            </div>
            <StudioStatusChip tone="danger">{t("common.notImplemented")}</StudioStatusChip>
          </div>

          <div className="grid gap-2 sm:grid-cols-3 lg:grid-cols-1">
            <EvidenceCard title={t("artifacts.sourceFiles")} value={String(sourceFiles.length)}>
              {t("common.editableSurfaces", { count: editableFiles.length })}
            </EvidenceCard>
            <EvidenceCard title={t("artifacts.runtimeTrace")} value={traceId}>
              {playtestReport
                ? t("common.stateDeltasProduced", {
                    count: playtestReport.delta_summary.length,
                  })
                : t("artifacts.runProofToGenerate")}
            </EvidenceCard>
            <EvidenceCard title={t("artifacts.exportReport")} value={String(exportedFileCount)}>
              {exportReport
                ? t("common.filesArchived", {
                    count: exportReport.archived_files.length,
                  })
                : t("artifacts.runExportToInspect")}
            </EvidenceCard>
          </div>
        </aside>

        <div className="grid content-start gap-4 lg:col-start-1 lg:row-span-2 lg:row-start-1">
          <div className="rounded-lg border border-violet-500/35 bg-graphite-950 p-4 text-ink shadow-studio-panel">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-violet-600">
                  {t("artifacts.artifactReview")}
                </p>
                <h3 className="mt-1 text-xl font-semibold text-ink">
                  {projectSummary?.title ?? loadedPath}
                </h3>
                <p className="mt-2 max-w-3xl text-sm leading-6 text-graphite-700/70">
                  {t("artifacts.artifactReviewDesc")}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="health">{t("artifacts.projectSource")}</StudioStatusChip>
                <StudioStatusChip tone="neutral">{loadedPath}</StudioStatusChip>
              </div>
            </div>

            <div className="mt-4 grid gap-3 md:grid-cols-4">
              <BundleFact label={t("artifacts.sourceFilesLabel")} value={String(sourceFiles.length)} />
              <BundleFact label={t("artifacts.editableLabel")} value={String(editableFiles.length)} />
              <BundleFact label={t("artifacts.assetRecordsLabel")} value={String(assetRecordCount)} />
              <BundleFact label={t("artifacts.traceLabel")} value={traceId} />
            </div>

            <div className="mt-4 rounded-md border border-violet-500/25 bg-violet-500/10 px-3 py-3">
              <div className="flex gap-2">
                <ShieldCheck
                  aria-hidden
                  size={18}
                  className="mt-0.5 shrink-0 text-violet-600"
                />
                <p className="text-sm leading-6 text-ink">
                  {t("artifacts.noApprovalAction")}
                </p>
              </div>
            </div>
          </div>

          <section aria-label={t("artifacts.aria.currentSourceArtifacts")} className="grid gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <h3 className="text-base font-semibold text-ink">
                {t("artifacts.currentSourceArtifacts")}
              </h3>
              <StudioStatusChip tone="health">{t("common.files", { count: sourceFiles.length })}</StudioStatusChip>
            </div>
            <div className="grid gap-3 xl:grid-cols-2">
              {sourceFiles.slice(0, 8).map((file) => (
                <article
                  key={file.path}
                  className="rounded-lg border border-canvas-200 bg-canvas-50 p-4 text-ink shadow-studio-panel"
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
                      {file.editable ? t("common.editable") : t("common.readOnly")}
                    </StudioStatusChip>
                  </div>
                  <EvidenceBlock label={t("artifacts.bytes")} value={String(file.bytes)} />
                  <EvidenceBlock
                    label={t("artifacts.source")}
                    value={t("artifacts.loadedThroughStorage")}
                  />
                </article>
              ))}
            </div>
          </section>

          <StudioTabs
            ariaLabel={t("artifacts.aria.runtimeExportEvidence")}
            items={[
              {
                id: "runtime-impact",
                label: t("artifacts.runtimeImpact"),
                badge: playtestReport?.delta_summary.length,
                children: (
                  <section className="rounded-lg border border-canvas-200 bg-canvas-50 p-4 text-ink shadow-studio-panel">
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <h3 className="text-base font-semibold">{t("artifacts.runtimeImpact")}</h3>
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
                          {t("artifacts.noRuntimeProof")}
                        </p>
                      ) : null}
                      {playtestError ? (
                        <p className="text-sm text-signal">{playtestError}</p>
                      ) : null}
                    </div>
                  </section>
                ),
              },
              {
                id: "export-evidence",
                label: t("artifacts.exportEvidence"),
                badge: exportReport ? exportReport.files_written.length : undefined,
                children: (
                  <section className="rounded-lg border border-canvas-200 bg-canvas-50 p-4 text-ink shadow-studio-panel">
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <h3 className="text-base font-semibold">{t("artifacts.exportEvidence")}</h3>
                      <StudioStatusChip tone={exportReport ? "health" : "neutral"}>
                        {exportReport ? t("common.available") : t("common.notRun")}
                      </StudioStatusChip>
                    </div>
                    <div className="mt-3 grid gap-2">
                      <ExportFact
                        label={t("artifacts.output")}
                        value={exportReport?.output_dir ?? t("common.notExported")}
                      />
                      <ExportFact
                        label={t("artifacts.archive")}
                        value={exportReport?.archive_path ?? t("common.notArchived")}
                      />
                      <ExportFact
                        label={t("artifacts.files")}
                        value={String(exportReport?.files_written.length ?? 0)}
                      />
                    </div>
                  </section>
                ),
              },
            ]}
          />
        </div>

        <aside
          aria-label={t("artifacts.aria.validationEvidence")}
          className="grid content-start gap-3 rounded-lg border border-canvas-200 bg-graphite-950 p-3 text-ink shadow-studio-panel lg:col-start-2 lg:row-start-2"
        >
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-health-400">
                {t("artifacts.validationEvidence")}
              </p>
              <h3 className="mt-1 text-lg font-semibold text-ink">
                {t("artifacts.realCommandOutputs")}
              </h3>
            </div>
            <StudioStatusChip tone="health">{t("common.visible")}</StudioStatusChip>
          </div>

          <ValidationLine
            icon={FileText}
            label={t("artifacts.projectSourceVal")}
            detail={t("common.editableSurfacesFromStorage", { count: sourceFiles.length })}
          />
          <ValidationLine
            icon={Boxes}
            label={t("artifacts.assetRegistry")}
            detail={t("common.assetRecordsFromMedia", { count: assetRecordCount })}
          />
          <ValidationLine
            icon={GitBranch}
            label={t("artifacts.runtimeTraceLabel")}
            detail={playtestReport ? playtestReport.trace_path : t("artifacts.noTraceCaptured")}
          />
          <ValidationLine
            icon={PackageCheck}
            label={t("artifacts.staticPackage")}
            detail={
              exportReport
                ? t("common.filesFoundAfterExport", {
                    count: exportReport.files_found.length,
                  })
                : t("artifacts.noExportReportCaptured")
            }
          />

          <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
            <p className="text-sm font-semibold text-ink">
              {t("artifacts.availableActions")}
            </p>
            <div className="mt-3 grid gap-2">
              <StudioButton onClick={onRunPlayableProof} disabled={playtesting}>
                <Play aria-hidden size={16} />
                {playtesting ? t("artifacts.runningProof") : t("artifacts.runProof")}
              </StudioButton>
              <StudioButton onClick={onOpenTrace}>
                <GitBranch aria-hidden size={16} />
                {t("artifacts.viewTrace")}
              </StudioButton>
            </div>
          </div>
        </aside>
      </div>

      {children}
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
    <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-semibold text-ink">{title}</p>
        <StudioStatusChip tone="neutral">{value}</StudioStatusChip>
      </div>
      <p className="mt-2 text-xs leading-5 text-graphite-700/65">{children}</p>
    </div>
  );
}

function BundleFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
      <p className="text-xs font-semibold uppercase text-graphite-700/55">
        {label}
      </p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">
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
    <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
      <div className="flex items-start gap-2">
        <Icon aria-hidden size={16} className="mt-0.5 shrink-0 text-health-400" />
        <div className="min-w-0">
          <p className="text-sm font-semibold text-ink">{label}</p>
          <p className="mt-1 text-xs leading-5 text-graphite-700/65">{detail}</p>
        </div>
      </div>
    </div>
  );
}
