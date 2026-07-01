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
import type { CreatorProjectSummary } from "./projectSummary";
import type { PlayOnceReport, SourceFileSummary } from "./tauriBridge";
import { Collapsible, StudioButton, StudioStatusChip } from "./studioUi";

interface AgentMeshViewProps {
  projectSummary: CreatorProjectSummary | null;
  loadedPath: string;
  runtimeName: string;
  sourceFiles: SourceFileSummary[];
  assetRecordCount: number;
  exportProfileCount: number;
  playtestReport: PlayOnceReport | null;
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
    label: "Project open/check",
    source: "plotforge-studio -> plotforge-storage",
    evidence: "Folder project files and schema validation",
    status: "wired",
  },
  {
    id: "structured-editing",
    label: "Structured editing",
    source: "plotforge-studio -> plotforge-storage",
    evidence: "World, StoryCraft, Characters, State, Rules documents",
    status: "wired",
  },
  {
    id: "source-files",
    label: "Source file read/write",
    source: "plotforge-studio source-file adapter",
    evidence: "Editable source list and file content",
    status: "wired",
  },
  {
    id: "runtime-proof",
    label: "Runtime proof",
    source: "plotforge-runtime + trace storage",
    evidence: "PlayOnceReport, RuntimeTrace, optional snapshot",
    status: "wired",
  },
  {
    id: "static-export",
    label: "Static export zip",
    source: "plotforge-export",
    evidence: "Whitelisted package files and archive report",
    status: "wired",
  },
  {
    id: "agent-pi-agent-runtime",
    label: "pi-Agent runtime",
    source: "No schema-backed Studio command",
    evidence: "Not exposed by Tauri or HTTP dev bridge",
    status: "not-implemented",
  },
];

export function AgentMeshView({
  projectSummary,
  loadedPath,
  runtimeName,
  sourceFiles,
  assetRecordCount,
  exportProfileCount,
  playtestReport,
  onOpenTrace,
  onRunPlayableProof,
}: AgentMeshViewProps) {
  const wiredCount = realCapabilities.filter(
    (capability) => capability.status === "wired",
  ).length;
  const proofLabel = playtestReport
    ? `${playtestReport.trace.id} / ${playtestReport.delta_summary.length} deltas`
    : "not captured";

  return (
    <section aria-label="Agent Mesh Workspace" className="grid gap-5">
      <div className="grid gap-4 2xl:grid-cols-[300px_minmax(0,1fr)_320px]">
        <aside
          aria-label="Studio Backend Bridge"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-accent-400">
                Backend Bridge
              </p>
              <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                Real Studio commands
              </h3>
            </div>
            <StudioStatusChip tone="health">{runtimeName}</StudioStatusChip>
          </div>

          <div className="grid gap-2 rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <BridgeFact label="Project truth" value="folder source files" />
            <BridgeFact label="Command source" value="plotforge-studio" />
            <BridgeFact label="pi-Agent network boundary (local-only)" value="not implemented" />
            <BridgeFact label="Provider calls" value="not implemented" />
          </div>

          <DarkCollapsible label="Removed Fake Surfaces" badge="disabled">
            <ul className="grid gap-2 text-xs leading-5 text-canvas-200/70">
              <BoundaryLine icon={Lock}>
                No mock external workers or mock connected state.
              </BoundaryLine>
              <BoundaryLine icon={Lock}>
                No local approval queue unless a real command exists.
              </BoundaryLine>
              <BoundaryLine icon={Lock}>
                No generated artifact bundle without persisted evidence.
              </BoundaryLine>
            </ul>
          </DarkCollapsible>
        </aside>

        <div className="grid gap-4">
          <div
            aria-label="Command Boundary Map"
            className="rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-accent-400">
                  Command Boundary Map
                </p>
                <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                  {projectSummary?.title ?? "No project loaded"}
                </h3>
                <p className="mt-1 max-w-3xl text-sm leading-6 text-canvas-200/60">
                  The UI is now backed by Studio command results. pi-Agent
                  runtime is wired locally; external agent boundaries stay
                  unavailable until schema-backed ports are added.
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="health">
                  {wiredCount} wired capabilities
                </StudioStatusChip>
                <StudioStatusChip tone="neutral">
                  {sourceFiles.length} source files
                </StudioStatusChip>
              </div>
            </div>

            <div className="mt-5 grid gap-4 xl:grid-cols-3">
              <MeshColumn
                title="Project Source"
                icon={FileText}
                tone="border-health-400/45 bg-health-500/15"
              >
                <MeshNode title={loadedPath} detail="Folder files win over cache" />
              </MeshColumn>
              <MeshColumn
                title="Runtime Evidence"
                icon={Play}
                tone="border-accent-400/45 bg-accent-500/15"
              >
                <MeshNode title={proofLabel} detail="Generated only by play_once" />
              </MeshColumn>
              <MeshColumn
                title="Export Evidence"
                icon={PackageCheck}
                tone="border-amber-400/45 bg-amber-500/15"
              >
                <MeshNode
                  title={`${exportProfileCount} profiles`}
                  detail="Only static web has an executable Studio command"
                />
              </MeshColumn>
            </div>

            <div className="mt-5 grid gap-3 md:grid-cols-3">
              <ArtifactNode icon={FileText} title="Source Files" detail={String(sourceFiles.length)} />
              <ArtifactNode icon={Boxes} title="Asset Records" detail={String(assetRecordCount)} />
              <ArtifactNode icon={Network} title="External Agents" detail="not implemented" />
            </div>
          </div>

          <div
            aria-label="Capability Matrix"
            className="rounded-lg border border-graphite-700/15 bg-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-ink/10 px-4 py-3">
              <div>
                <p className="text-xs font-semibold uppercase text-accent-500">
                  Capability Matrix
                </p>
                <h3 className="mt-1 text-lg font-semibold text-ink">
                  Studio-backed capabilities
                </h3>
              </div>
              <StudioStatusChip tone="health">{wiredCount} wired</StudioStatusChip>
            </div>

            <ul className="grid gap-3 p-4 sm:grid-cols-2 xl:grid-cols-3">
              {realCapabilities.map((capability) => (
                <li key={capability.id}>
                  <CapabilityCard capability={capability} />
                </li>
              ))}
            </ul>
          </div>
        </div>

        <aside
          aria-label="Bridge Evidence"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 shadow-studio-panel"
        >
          <div>
            <p className="text-xs font-semibold uppercase text-accent-500">
              Bridge Evidence
            </p>
            <h3 className="mt-1 text-lg font-semibold text-ink">
              Current backend facts
            </h3>
          </div>

          <div className="grid gap-2 rounded-md border border-ink/10 bg-parchment px-3 py-3">
            <EvidenceLine label="Runtime" value={runtimeName} />
            <EvidenceLine label="Loaded path" value={loadedPath} />
            <EvidenceLine label="Playable proof" value={proofLabel} />
            <EvidenceLine
              label="Trace id"
              value={playtestReport?.trace.id ?? "not captured"}
            />
          </div>

          <div className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold text-ink">Safety Boundary</h3>
              <StudioStatusChip tone="health">explicit</StudioStatusChip>
            </div>
            <div className="mt-3 grid gap-2 text-sm text-ink/65">
              <p>No provider credentials are read by this UI surface.</p>
              <p>No external agent connection is started from the browser.</p>
              <p>Unsupported capabilities are disabled instead of simulated.</p>
            </div>
          </div>

          <div className="rounded-md border border-ink/10 bg-graphite-950 px-3 py-3 text-canvas-50">
            <h3 className="text-sm font-semibold">Actions</h3>
            <div className="mt-3 grid gap-2 sm:grid-cols-2">
              <StudioButton onClick={onOpenTrace}>
                <TerminalSquare aria-hidden size={16} />
                Review trace
              </StudioButton>
              <StudioButton variant="primary" onClick={onRunPlayableProof}>
                <Play aria-hidden size={16} />
                Run proof
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
    <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
      <button
        type="button"
        id={headingId}
        aria-expanded={open}
        aria-controls={regionId}
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center justify-between gap-3 text-left"
      >
        <span className="flex items-center gap-2 text-sm font-semibold text-canvas-50">
          {open ? (
            <ChevronDown aria-hidden size={15} className="shrink-0 text-canvas-200/55" />
          ) : (
            <ChevronRight aria-hidden size={15} className="shrink-0 text-canvas-200/55" />
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
  const wired = capability.status === "wired";
  const StatusIcon = wired ? CheckCircle2 : XCircle;
  return (
    <article className="flex h-full flex-col gap-2 rounded-md border border-ink/10 bg-parchment px-3 py-3">
      <div className="flex items-start justify-between gap-2">
        <h4 className="text-sm font-semibold text-ink">{capability.label}</h4>
        <StatusIcon
          aria-hidden
          size={16}
          className={wired ? "shrink-0 text-health-500" : "shrink-0 text-signal"}
        />
      </div>
      <dl className="grid gap-1.5 text-xs leading-5">
        <div className="grid grid-cols-[88px_minmax(0,1fr)] gap-2">
          <dt className="font-semibold uppercase text-ink/45">Real source</dt>
          <dd className="text-ink/70">{capability.source}</dd>
        </div>
        <div className="grid grid-cols-[88px_minmax(0,1fr)] gap-2">
          <dt className="font-semibold uppercase text-ink/45">Evidence</dt>
          <dd className="text-ink/70">{capability.evidence}</dd>
        </div>
      </dl>
      <StudioStatusChip tone={wired ? "health" : "danger"}>
        {capability.status}
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
        <h4 className="text-sm font-semibold text-canvas-50">{title}</h4>
      </div>
      <div className="mt-3 grid gap-2">{children}</div>
    </div>
  );
}

function MeshNode({ title, detail }: { title: string; detail: string }) {
  return (
    <div className="rounded-md border border-canvas-200/10 bg-graphite-850 px-3 py-3">
      <p className="truncate text-sm font-semibold text-canvas-50">{title}</p>
      <p className="mt-1 text-xs leading-5 text-canvas-200/60">{detail}</p>
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
    <div className="rounded-md border border-amber-500/35 bg-amber-500/10 px-3 py-3">
      <Icon aria-hidden size={18} className="text-amber-400" />
      <p className="mt-2 text-sm font-semibold text-canvas-50">{title}</p>
      <p className="mt-1 truncate text-xs text-canvas-200/60">{detail}</p>
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
