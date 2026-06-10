import {
  Boxes,
  CheckCircle2,
  FileText,
  Lock,
  Network,
  PackageCheck,
  Play,
  ShieldCheck,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import type { CreatorProjectSummary } from "./projectSummary";
import type { PlayOnceReport, SourceFileSummary } from "./tauriBridge";
import { StudioButton, StudioStatusChip } from "./studioUi";

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
    id: "agent-acp-bridge",
    label: "ACP / external agent bridge",
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
              <p className="text-xs font-semibold uppercase text-acp-400">
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
            <BridgeFact label="Network ACP" value="not implemented" />
            <BridgeFact label="Provider calls" value="not implemented" />
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <p className="text-sm font-semibold text-canvas-50">
                Removed Fake Surfaces
              </p>
              <StudioStatusChip tone="danger">disabled</StudioStatusChip>
            </div>
            <ul className="mt-3 grid gap-2 text-xs leading-5 text-canvas-200/70">
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
          </div>
        </aside>

        <div className="grid gap-4">
          <div
            aria-label="Command Boundary Map"
            className="rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-acp-400">
                  Command Boundary Map
                </p>
                <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                  {projectSummary?.title ?? "No project loaded"}
                </h3>
                <p className="mt-1 max-w-3xl text-sm leading-6 text-canvas-200/60">
                  The UI is now backed by Studio command results. Agent and ACP
                  concepts stay visible only as unavailable boundaries until
                  schema-backed ports are added.
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
                tone="border-acp-400/45 bg-acp-500/15"
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
            className="overflow-hidden rounded-lg border border-graphite-700/15 bg-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-ink/10 px-4 py-3">
              <div>
                <p className="text-xs font-semibold uppercase text-acp-500">
                  Capability Matrix
                </p>
                <h3 className="mt-1 text-lg font-semibold text-ink">
                  Studio-backed capabilities
                </h3>
              </div>
              <StudioStatusChip tone="health">{wiredCount} wired</StudioStatusChip>
            </div>

            <div className="overflow-x-auto">
              <table className="min-w-[760px] w-full border-collapse text-left text-sm">
                <thead className="bg-parchment text-xs uppercase text-ink/50">
                  <tr>
                    <th className="px-4 py-3 font-semibold">Capability</th>
                    <th className="px-4 py-3 font-semibold">Real source</th>
                    <th className="px-4 py-3 font-semibold">Evidence</th>
                    <th className="px-4 py-3 font-semibold">Status</th>
                  </tr>
                </thead>
                <tbody>
                  {realCapabilities.map((capability) => (
                    <tr key={capability.id} className="border-t border-ink/10 align-top">
                      <td className="px-4 py-3 font-semibold text-ink">
                        {capability.label}
                      </td>
                      <td className="px-4 py-3 text-ink/70">{capability.source}</td>
                      <td className="px-4 py-3 text-ink/70">{capability.evidence}</td>
                      <td className="px-4 py-3">
                        <StudioStatusChip
                          tone={capability.status === "wired" ? "health" : "danger"}
                        >
                          {capability.status}
                        </StudioStatusChip>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </div>

        <aside
          aria-label="Bridge Evidence"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-canvas-50 p-4 shadow-studio-panel"
        >
          <div>
            <p className="text-xs font-semibold uppercase text-acp-500">
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
