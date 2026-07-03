import {
  Boxes,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  FileText,
  Lock,
  Network,
  PackageCheck,
  Play,
  ShieldCheck,
  TerminalSquare,
  XCircle,
  type LucideIcon,
} from "lucide-react";
import { useState, type ReactNode } from "react";
import type { PiAgentCapability } from "../../../contracts/plotforge";
import type { CreatorProjectSummary } from "./projectSummary";
import type { PlayOnceReport, SourceFileSummary } from "./tauriBridge";
import { Collapsible, StudioButton, StudioStatusChip } from "./studioUi";
import { useStudioI18n } from "./i18n";

interface AgentMeshViewProps {
  projectSummary: CreatorProjectSummary | null;
  loadedPath: string;
  runtimeName: string;
  sourceFiles: SourceFileSummary[];
  assetRecordCount: number;
  exportProfileCount: number;
  playtestReport: PlayOnceReport | null;
  piAgentCapabilities: PiAgentCapability[];
  onOpenTrace(): void;
  onRunPlayableProof(): void;
}

interface Capability {
  id: string;
  label: string;
  source: string;
  evidence: string;
  status: "wired" | "not-implemented";
}

const realCapabilities: Capability[] = [
  {
    id: "project-open-check",
    label: "mesh.cap.projectOpenCheck",
    source: "mesh.cap.projectOpenCheckSource",
    evidence: "mesh.cap.projectOpenCheckEvidence",
    status: "wired",
  },
  {
    id: "structured-editing",
    label: "mesh.cap.structuredEditing",
    source: "mesh.cap.projectOpenCheckSource",
    evidence: "mesh.cap.structuredEditingEvidence",
    status: "wired",
  },
  {
    id: "source-files",
    label: "mesh.cap.sourceFileReadWrite",
    source: "mesh.cap.sourceFileReadWriteSource",
    evidence: "mesh.cap.sourceFileReadWriteEvidence",
    status: "wired",
  },
  {
    id: "runtime-proof",
    label: "mesh.cap.runtimeProof",
    source: "mesh.cap.runtimeProofSource",
    evidence: "mesh.cap.runtimeProofEvidence",
    status: "wired",
  },
  {
    id: "static-export",
    label: "mesh.cap.staticExportZip",
    source: "mesh.cap.staticExportZipSource",
    evidence: "mesh.cap.staticExportZipEvidence",
    status: "wired",
  },
];

function piAgentCapabilityItems(
  piAgentCapabilities: PiAgentCapability[],
): Capability[] {
  return piAgentCapabilities.map((capability) => ({
    id: capability.id,
    label: capability.label,
    source: capability.source,
    evidence: capability.evidence,
    status: capability.status === "wired" ? "wired" : "not-implemented",
  }));
}

export function AgentMeshView({
  projectSummary,
  loadedPath,
  runtimeName,
  sourceFiles,
  assetRecordCount,
  exportProfileCount,
  playtestReport,
  piAgentCapabilities,
  onOpenTrace,
  onRunPlayableProof,
}: AgentMeshViewProps) {
  const { t } = useStudioI18n();
  const capabilities = [
    ...realCapabilities,
    ...piAgentCapabilityItems(piAgentCapabilities),
  ];
  const wiredCount = capabilities.filter(
    (capability) => capability.status === "wired",
  ).length;
  const proofLabel = playtestReport
    ? t("common.traceDeltas", {
        trace: playtestReport.trace.id,
        count: playtestReport.delta_summary.length,
      })
    : t("common.notCaptured");

  return (
    <section aria-label={t("mesh.aria.workspace")} className="grid gap-4">
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px] 2xl:grid-cols-[minmax(0,1fr)_340px]">
        <aside
          aria-label={t("mesh.aria.backendBridge")}
          className="grid content-start gap-3 rounded-lg border border-canvas-200 bg-graphite-950 p-3 text-ink shadow-studio-panel lg:col-start-2 lg:row-start-1"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-accent-400">
                {t("mesh.backendBridgeLabel")}
              </p>
              <h3 className="mt-1 text-lg font-semibold text-ink">
                {t("mesh.realStudioCommands")}
              </h3>
            </div>
            <StudioStatusChip tone="health">{runtimeName}</StudioStatusChip>
          </div>

          <div className="grid gap-2 rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
            <BridgeFact label={t("mesh.projectTruth")} value={t("mesh.folderSourceFiles")} />
            <BridgeFact label={t("mesh.commandSource")} value={t("mesh.plotforgeStudio")} />
            <BridgeFact label={t("mesh.piAgentRuntime")} value={t("mesh.wiredLocal")} />
            <BridgeFact label={t("mesh.providerCalls")} value={t("common.notImplemented")} />
          </div>

          <DarkCollapsible label={t("mesh.removedFakeSurfaces")} badge={t("common.disabled")}>
            <ul className="grid gap-2 text-xs leading-5 text-graphite-700/70">
              <BoundaryLine icon={Lock}>{t("mesh.noMockWorkers")}</BoundaryLine>
              <BoundaryLine icon={Lock}>{t("mesh.noLocalApprovalQueue")}</BoundaryLine>
              <BoundaryLine icon={Lock}>{t("mesh.noGeneratedBundle")}</BoundaryLine>
            </ul>
          </DarkCollapsible>
        </aside>

        <div className="grid content-start gap-4 lg:col-start-1 lg:row-span-2 lg:row-start-1">
          <div
            aria-label={t("mesh.aria.commandBoundaryMap")}
            className="rounded-lg border border-canvas-200 bg-graphite-950 p-4 text-ink shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-accent-400">
                  {t("mesh.commandBoundaryMap")}
                </p>
                <h3 className="mt-1 text-xl font-semibold text-ink">
                  {projectSummary?.title ?? t("app.noProjectLoaded")}
                </h3>
                <p className="mt-1 max-w-3xl text-sm leading-6 text-graphite-700/65">
                  {t("mesh.uiBackedByStudio")}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="health">
                  {t("common.wiredCapabilities", { count: wiredCount })}
                </StudioStatusChip>
                <StudioStatusChip tone="neutral">
                  {t("common.sourceFiles", { count: sourceFiles.length })}
                </StudioStatusChip>
              </div>
            </div>

            <div className="mt-4 grid gap-3 xl:grid-cols-3">
              <MeshColumn
                title={t("mesh.projectSource")}
                icon={FileText}
                tone="border-health-400/45 bg-health-500/15"
              >
                <MeshNode title={loadedPath} detail={t("mesh.folderFilesWin")} />
              </MeshColumn>
              <MeshColumn
                title={t("mesh.runtimeEvidence")}
                icon={Play}
                tone="border-accent-400/45 bg-accent-500/15"
              >
                <MeshNode title={proofLabel} detail={t("mesh.generatedByPlayOnce")} />
              </MeshColumn>
              <MeshColumn
                title={t("mesh.exportEvidence")}
                icon={PackageCheck}
                tone="border-violet-400/45 bg-violet-500/15"
              >
                <MeshNode
                  title={t("common.profiles", { count: exportProfileCount })}
                  detail={t("mesh.onlyStaticWebExecutable")}
                />
              </MeshColumn>
            </div>

            <div className="mt-4 grid gap-3 md:grid-cols-3">
              <ArtifactNode icon={FileText} title={t("mesh.sourceFiles")} detail={String(sourceFiles.length)} />
              <ArtifactNode icon={Boxes} title={t("mesh.assetRecords")} detail={String(assetRecordCount)} />
              <ArtifactNode icon={Network} title={t("mesh.externalAgents")} detail={t("common.notImplemented")} />
            </div>
          </div>

          <div
            aria-label={t("mesh.aria.capabilityMatrix")}
            className="rounded-lg border border-canvas-200 bg-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-ink/10 px-4 py-3">
              <div>
                <p className="text-xs font-semibold uppercase text-accent-500">
                  {t("mesh.capabilityMatrix")}
                </p>
                <h3 className="mt-1 text-lg font-semibold text-ink">
                  {t("mesh.studioBackedCapabilities")}
                </h3>
              </div>
              <StudioStatusChip tone="health">{t("common.wiredCount", { count: wiredCount })}</StudioStatusChip>
            </div>

            <ul className="grid gap-3 p-3 sm:grid-cols-2 xl:grid-cols-3">
              {capabilities.map((capability) => (
                <li key={capability.id}>
                  <CapabilityCard capability={capability} />
                </li>
              ))}
            </ul>
          </div>
        </div>

        <aside
          aria-label={t("mesh.aria.bridgeEvidence")}
          className="grid content-start gap-3 rounded-lg border border-canvas-200 bg-canvas-50 p-3 shadow-studio-panel lg:col-start-2 lg:row-start-2"
        >
          <div>
            <p className="text-xs font-semibold uppercase text-accent-500">
              {t("mesh.bridgeEvidence")}
            </p>
            <h3 className="mt-1 text-lg font-semibold text-ink">
              {t("mesh.currentBackendFacts")}
            </h3>
          </div>

          <div className="grid gap-2 rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
            <EvidenceLine label={t("mesh.runtime")} value={runtimeName} />
            <EvidenceLine label={t("mesh.loadedPath")} value={loadedPath} />
            <EvidenceLine label={t("mesh.playableProof")} value={proofLabel} />
            <EvidenceLine
              label={t("mesh.traceId")}
              value={playtestReport?.trace.id ?? t("common.notCaptured")}
            />
          </div>

          <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold text-ink">{t("mesh.safetyBoundary")}</h3>
              <StudioStatusChip tone="health">{t("common.explicit")}</StudioStatusChip>
            </div>
            <div className="mt-3 grid gap-2 text-sm text-ink/65">
              <p>{t("mesh.noProviderCredentials")}</p>
              <p>{t("mesh.noExternalAgentConnection")}</p>
              <p>{t("mesh.unsupportedDisabled")}</p>
            </div>
          </div>

          <div className="rounded-md border border-ink/10 bg-graphite-950 px-3 py-3 text-ink">
            <h3 className="text-sm font-semibold">{t("mesh.actions")}</h3>
            <div className="mt-3 grid gap-2 sm:grid-cols-2">
              <StudioButton onClick={onOpenTrace}>
                <TerminalSquare aria-hidden size={16} />
                {t("mesh.reviewTrace")}
              </StudioButton>
              <StudioButton variant="primary" onClick={onRunPlayableProof}>
                <Play aria-hidden size={16} />
                {t("mesh.runProof")}
              </StudioButton>
            </div>
          </div>
        </aside>
      </div>
    </section>
  );
}

function BoundaryLine({
  icon: Icon,
  children,
}: {
  icon: LucideIcon;
  children: string;
}) {
  return (
    <li className="flex gap-2">
      <Icon aria-hidden size={14} className="mt-0.5 shrink-0 text-signal" />
      <span>{children}</span>
    </li>
  );
}

/**
 * Dark-themed collapsible for the "Removed Fake Surfaces" technical section.
 * Collapsed by default to keep the surface focused on creator-relevant facts;
 * the boundary detail is available on demand for technical reviewers.
 */
function DarkCollapsible({
  label,
  badge,
  children,
}: {
  label: string;
  badge: string;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(false);
  const slug = label.toLowerCase().replace(/\s+/g, "-");
  const headingId = `dark-collapsible-heading-${slug}`;
  const regionId = `dark-collapsible-region-${slug}`;
  return (
    <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
      <button
        type="button"
        id={headingId}
        aria-expanded={open}
        aria-controls={regionId}
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center justify-between gap-3 text-left"
      >
        <span className="flex items-center gap-2 text-sm font-semibold text-ink">
          {open ? (
            <ChevronDown aria-hidden size={15} className="shrink-0 text-graphite-700/60" />
          ) : (
            <ChevronRight aria-hidden size={15} className="shrink-0 text-graphite-700/60" />
          )}
          {label}
        </span>
        <StudioStatusChip tone="danger">{badge}</StudioStatusChip>
      </button>
      {open ? (
        <div
          id={regionId}
          role="region"
          aria-labelledby={headingId}
          className="mt-3"
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}

/**
 * Single responsive capability card replacing the fixed-width matrix table.
 * Stacks source/evidence as labeled fields so the matrix reads without
 * horizontal scrolling on narrow screens.
 */
function CapabilityCard({ capability }: { capability: Capability }) {
  const { t } = useStudioI18n();
  const wired = capability.status === "wired";
  const StatusIcon = wired ? CheckCircle2 : XCircle;
  return (
    <article className="flex h-full flex-col gap-2 rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
      <div className="flex items-start justify-between gap-2">
        <h4 className="text-sm font-semibold text-ink">{t(capability.label)}</h4>
        <StatusIcon
          aria-hidden
          size={16}
          className={wired ? "shrink-0 text-health-500" : "shrink-0 text-signal"}
        />
      </div>
      <dl className="grid gap-1.5 text-xs leading-5">
        <div className="grid grid-cols-[88px_minmax(0,1fr)] gap-2">
          <dt className="font-semibold uppercase text-ink/45">{t("mesh.realSource")}</dt>
          <dd className="text-ink/70">{t(capability.source)}</dd>
        </div>
        <div className="grid grid-cols-[88px_minmax(0,1fr)] gap-2">
          <dt className="font-semibold uppercase text-ink/45">{t("mesh.evidence")}</dt>
          <dd className="text-ink/70">{t(capability.evidence)}</dd>
        </div>
      </dl>
      <StudioStatusChip tone={wired ? "health" : "danger"}>
        {t(wired ? "mesh.statusWired" : "mesh.statusNotImplemented")}
      </StudioStatusChip>
    </article>
  );
}

function MeshColumn({
  title,
  icon: Icon,
  tone,
  children,
}: {
  title: string;
  icon: LucideIcon;
  tone: string;
  children: ReactNode;
}) {
  return (
    <div className={`rounded-lg border p-3 ${tone}`}>
      <div className="flex items-center gap-2">
        <Icon aria-hidden size={18} />
        <h4 className="text-sm font-semibold text-ink">{title}</h4>
      </div>
      <div className="mt-3 grid gap-2">{children}</div>
    </div>
  );
}

function MeshNode({ title, detail }: { title: string; detail: string }) {
  return (
    <div className="rounded-md border border-canvas-200 bg-graphite-850 px-3 py-3">
      <p className="truncate text-sm font-semibold text-ink">{title}</p>
      <p className="mt-1 text-xs leading-5 text-graphite-700/65">{detail}</p>
    </div>
  );
}

function ArtifactNode({
  icon: Icon,
  title,
  detail,
}: {
  icon: LucideIcon;
  title: string;
  detail: string;
}) {
  return (
    <div className="rounded-md border border-violet-500/35 bg-violet-500/10 px-3 py-3">
      <Icon aria-hidden size={18} className="text-violet-600" />
      <p className="mt-2 text-sm font-semibold text-ink">{title}</p>
      <p className="mt-1 truncate text-xs text-graphite-700/65">{detail}</p>
    </div>
  );
}

function EvidenceLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[110px_minmax(0,1fr)] gap-3 text-sm">
      <span className="text-ink/45">{label}</span>
      <span className="truncate font-medium text-ink">{value}</span>
    </div>
  );
}

function BridgeFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-current/10 pb-2 last:border-b-0 last:pb-0">
      <span className="text-xs font-semibold uppercase opacity-55">{label}</span>
      <span className="truncate text-sm font-semibold">{value}</span>
    </div>
  );
}
