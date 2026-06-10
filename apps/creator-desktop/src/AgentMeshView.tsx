import {
  Boxes,
  CheckCircle2,
  Circle,
  Code2,
  FileText,
  GitBranch,
  Lock,
  Network,
  PackageCheck,
  Play,
  ShieldCheck,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import type {
  LocalPreviewCapability,
  LocalPreviewState,
  LocalPreviewWorker,
} from "./localPreviewModel";
import { localPreviewSummary } from "./localPreviewModel";
import type { CreatorProjectSummary } from "./projectSummary";
import type { PlayOnceReport } from "./tauriBridge";
import { StudioButton, StudioStatusChip } from "./studioUi";

interface AgentMeshViewProps {
  projectSummary: CreatorProjectSummary | null;
  loadedPath: string;
  localPreviewState: LocalPreviewState;
  playtestReport: PlayOnceReport | null;
  onOpenTrace(): void;
  onRunPlayableProof(): void;
}

export function AgentMeshView({
  projectSummary,
  loadedPath,
  localPreviewState,
  playtestReport,
  onOpenTrace,
  onRunPlayableProof,
}: AgentMeshViewProps) {
  const summary = localPreviewSummary(localPreviewState);
  const externalWorkers = localPreviewState.workers.filter(
    (worker) => worker.kind === "external-worker",
  );
  const subagents = localPreviewState.workers.filter(
    (worker) => worker.kind === "plotforge-subagent",
  );
  const directorWorker = localPreviewState.workers.find(
    (worker) => worker.kind === "director",
  );
  const selectedCapability = localPreviewState.capabilities[0] ?? null;
  const approvalQueue = localPreviewState.approvals.filter(
    (approval) => approval.state === "pending-local-review",
  );
  const proofLabel = playtestReport
    ? `${playtestReport.trace.id} / ${playtestReport.delta_summary.length} deltas`
    : "waiting for playable proof";

  return (
    <section aria-label="Agent Mesh Workspace" className="grid gap-5">
      <div className="grid gap-4 2xl:grid-cols-[280px_minmax(0,1fr)_320px]">
        <aside
          aria-label="ACP Bridge Setup"
          className="grid content-start gap-4 rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-acp-400">
                ACP Bridge Setup
              </p>
              <h3 className="mt-1 text-lg font-semibold text-canvas-50">
                Local capability bridge
              </h3>
            </div>
            <StudioStatusChip tone="agent">
              {localPreviewState.source}
            </StudioStatusChip>
          </div>

          <div className="grid gap-2 rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <BridgeFact
              label="Network"
              value={localPreviewState.networkEnabled ? "enabled" : "disabled"}
            />
            <BridgeFact
              label="Project truth"
              value={
                localPreviewState.authoritativeProjectState
                  ? "authoritative"
                  : "not authoritative"
              }
            />
            <BridgeFact
              label="StudioDataSource"
              value={
                localPreviewState.usesStudioDataSourcePort
                  ? "agent port"
                  : "not an agent port"
              }
            />
          </div>

          <PanelTitle label="Connected Workers" count={externalWorkers.length} />
          <div className="grid gap-2">
            {externalWorkers.map((worker) => (
              <WorkerCard key={worker.id} worker={worker} tone="external" />
            ))}
          </div>

          <PanelTitle label="PlotForge SubAgents" count={subagents.length} />
          <div className="grid gap-2">
            {subagents.map((worker) => (
              <WorkerCard key={worker.id} worker={worker} tone="subagent" />
            ))}
          </div>

          <div className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <p className="text-sm font-semibold text-canvas-50">
                Permission Policy
              </p>
              <StudioStatusChip tone="action">approval gated</StudioStatusChip>
            </div>
            <ul className="mt-3 grid gap-2 text-xs leading-5 text-canvas-200/70">
              {localPreviewState.capabilityPolicy.boundaries.map((boundary) => (
                <li key={boundary} className="flex gap-2">
                  <ShieldCheck
                    aria-hidden
                    size={14}
                    className="mt-0.5 shrink-0 text-health-400"
                  />
                  <span>{boundary}</span>
                </li>
              ))}
            </ul>
          </div>
        </aside>

        <div className="grid gap-4">
          <div
            aria-label="Mesh Map"
            className="rounded-lg border border-graphite-700/15 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-acp-400">
                  Mesh Map
                </p>
                <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                  {projectSummary?.title ?? "No project loaded"}
                </h3>
                <p className="mt-1 max-w-3xl text-sm leading-6 text-canvas-200/60">
                  Director intent fans into mock external workers, PlotForge
                  subAgents, reviewable artifacts, and playable proof.
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="acp">
                  {summary.externalWorkerCount} external workers
                </StudioStatusChip>
                <StudioStatusChip tone="agent">
                  {summary.plotforgeSubagentCount} subAgents
                </StudioStatusChip>
                <StudioStatusChip tone="health">
                  {summary.capabilityCount} capabilities
                </StudioStatusChip>
              </div>
            </div>

            <div className="mt-5 grid gap-4 xl:grid-cols-[minmax(0,0.8fr)_minmax(0,1fr)_minmax(0,1fr)]">
              <MeshColumn
                title="Director Intent"
                icon={GitBranch}
                tone="border-acp-400/45 bg-acp-500/15"
              >
                <MeshNode
                  title={directorWorker?.label ?? "Director Agent"}
                  detail={
                    directorWorker?.role ??
                    "Frames creator intent and validates playable proof"
                  }
                  accent="text-acp-400"
                />
              </MeshColumn>

              <MeshColumn
                title="External Workers"
                icon={Code2}
                tone="border-agent-400/45 bg-agent-500/15"
              >
                {externalWorkers.map((worker) => (
                  <MeshNode
                    key={worker.id}
                    title={worker.label}
                    detail={worker.capabilityIds.join(" / ")}
                    accent="text-agent-400"
                  />
                ))}
              </MeshColumn>

              <MeshColumn
                title="PlotForge SubAgents"
                icon={Network}
                tone="border-health-400/45 bg-health-500/15"
              >
                {subagents.map((worker) => (
                  <MeshNode
                    key={worker.id}
                    title={worker.label}
                    detail={worker.capabilityIds.join(" / ")}
                    accent="text-health-400"
                  />
                ))}
              </MeshColumn>
            </div>

            <div className="mt-5 grid gap-3 md:grid-cols-3">
              <ArtifactNode
                icon={FileText}
                title="Narrative Docs"
                detail="chapters/*.md"
              />
              <ArtifactNode
                icon={Boxes}
                title="Assets"
                detail="assets/* / manifest.json"
              />
              <ArtifactNode
                icon={PackageCheck}
                title="Build Package"
                detail="package.zip / checksums.txt"
              />
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
                  Exposed local/mock capabilities
                </h3>
              </div>
              <StudioStatusChip tone="agent">
                {summary.approvalRequiredCapabilityCount} approval required
              </StudioStatusChip>
            </div>

            <div className="overflow-x-auto">
              <table className="min-w-[760px] w-full border-collapse text-left text-sm">
                <thead className="bg-parchment text-xs uppercase text-ink/50">
                  <tr>
                    <th className="px-4 py-3 font-semibold">Capability</th>
                    <th className="px-4 py-3 font-semibold">Worker</th>
                    <th className="px-4 py-3 font-semibold">SubAgent</th>
                    <th className="px-4 py-3 font-semibold">Inputs</th>
                    <th className="px-4 py-3 font-semibold">Outputs</th>
                    <th className="px-4 py-3 font-semibold">Approval</th>
                    <th className="px-4 py-3 font-semibold">Trace</th>
                  </tr>
                </thead>
                <tbody>
                  {localPreviewState.capabilities.map((capability) => (
                    <CapabilityRow
                      key={capability.id}
                      capability={capability}
                      worker={findWorker(
                        localPreviewState,
                        capability.providerWorkerId,
                      )}
                      subagent={findWorker(
                        localPreviewState,
                        capability.plotforgeAgentId,
                      )}
                    />
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
              Selected Capability
            </p>
            <h3 className="mt-1 text-lg font-semibold text-ink">
              {selectedCapability?.label ?? "No capability"}
            </h3>
            <p className="mt-1 text-sm leading-6 text-ink/60">
              {selectedCapability?.description ??
                "Capability metadata is loaded from local preview state."}
            </p>
          </div>

          {selectedCapability ? (
            <div className="grid gap-2 rounded-md border border-ink/10 bg-parchment px-3 py-3">
              <EvidenceLine
                label="Input artifacts"
                value={selectedCapability.inputArtifacts.join(" / ")}
              />
              <EvidenceLine
                label="Output contract"
                value={selectedCapability.outputArtifacts.join(" / ")}
              />
              <EvidenceLine
                label="Approval required"
                value={selectedCapability.approvalRequired ? "yes" : "no"}
              />
              <EvidenceLine
                label="Trace level"
                value={selectedCapability.traceLevel}
              />
            </div>
          ) : null}

          <div className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold text-ink">Safety & Evidence</h3>
              <StudioStatusChip tone="health">trace visible</StudioStatusChip>
            </div>
            <div className="mt-3 grid gap-3">
              <PathBlock
                title="Allowed paths"
                icon={CheckCircle2}
                paths={localPreviewState.capabilityPolicy.allowedPaths}
                tone="text-health-500"
              />
              <PathBlock
                title="Blocked paths"
                icon={Lock}
                paths={localPreviewState.capabilityPolicy.blockedPaths}
                tone="text-signal"
              />
            </div>
          </div>

          <div className="rounded-md border border-ink/10 bg-parchment px-3 py-3">
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold text-ink">Approval Queue</h3>
              <StudioStatusChip tone="agent">{approvalQueue.length}</StudioStatusChip>
            </div>
            <div className="mt-3 grid gap-2">
              {localPreviewState.approvals.map((approval) => (
                <div
                  key={approval.id}
                  className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2"
                >
                  <div className="flex flex-wrap items-start justify-between gap-2">
                    <p className="text-sm font-semibold text-ink">
                      {approval.title}
                    </p>
                    <span className="text-xs font-semibold text-amber-600">
                      {approval.state}
                    </span>
                  </div>
                  <p className="mt-1 truncate text-xs text-ink/50">
                    {approval.evidenceIds.join(" / ")}
                  </p>
                </div>
              ))}
            </div>
          </div>

          <div className="rounded-md border border-ink/10 bg-graphite-950 px-3 py-3 text-canvas-50">
            <h3 className="text-sm font-semibold">Bridge Evidence</h3>
            <div className="mt-3 grid gap-2">
              <BridgeFact label="Loaded path" value={loadedPath} />
              <BridgeFact label="Playable proof" value={proofLabel} />
              <BridgeFact
                label="Trace id"
                value={playtestReport?.trace.id ?? "not captured"}
              />
              <BridgeFact
                label="Run seed"
                value={String(playtestReport?.trace.reproducibility.run_seed ?? 7)}
              />
              <BridgeFact label="Tests passing" value="local preview only" />
            </div>
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

function findWorker(state: LocalPreviewState, workerId: string) {
  return state.workers.find((worker) => worker.id === workerId) ?? null;
}

function PanelTitle({ label, count }: { label: string; count: number }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <p className="text-sm font-semibold text-canvas-50">{label}</p>
      <StudioStatusChip tone="neutral">{count}</StudioStatusChip>
    </div>
  );
}

function WorkerCard({
  worker,
  tone,
}: {
  worker: LocalPreviewWorker;
  tone: "external" | "subagent";
}) {
  return (
    <article className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold text-canvas-50">
            {worker.label}
          </p>
          <p className="mt-1 text-xs leading-5 text-canvas-200/55">
            {worker.role}
          </p>
        </div>
        <StudioStatusChip tone={tone === "external" ? "acp" : "agent"}>
          {worker.connectionState}
        </StudioStatusChip>
      </div>
      <p className="mt-2 truncate text-xs text-canvas-200/45">
        {worker.capabilityIds.join(" / ")}
      </p>
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

function MeshNode({
  title,
  detail,
  accent,
}: {
  title: string;
  detail: string;
  accent: string;
}) {
  return (
    <div className="rounded-md border border-canvas-200/10 bg-graphite-850 px-3 py-3">
      <p className="text-sm font-semibold text-canvas-50">{title}</p>
      <p className={`mt-1 text-xs leading-5 ${accent}`}>{detail}</p>
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

function CapabilityRow({
  capability,
  worker,
  subagent,
}: {
  capability: LocalPreviewCapability;
  worker: LocalPreviewWorker | null;
  subagent: LocalPreviewWorker | null;
}) {
  return (
    <tr className="border-t border-ink/10 align-top">
      <td className="px-4 py-3">
        <p className="font-semibold text-ink">{capability.label}</p>
        <p className="mt-1 text-xs leading-5 text-ink/55">
          {capability.description}
        </p>
      </td>
      <td className="px-4 py-3 text-ink/70">
        {worker?.label ?? capability.providerWorkerId}
      </td>
      <td className="px-4 py-3 text-ink/70">
        {subagent?.label ?? capability.plotforgeAgentId}
      </td>
      <td className="px-4 py-3">
        <ChipList values={capability.inputArtifacts} />
      </td>
      <td className="px-4 py-3">
        <ChipList values={capability.outputArtifacts} />
      </td>
      <td className="px-4 py-3">
        <span className="inline-flex items-center gap-2 text-sm font-semibold text-ink/70">
          {capability.approvalRequired ? (
            <CheckCircle2 aria-hidden size={16} className="text-amber-500" />
          ) : (
            <Circle aria-hidden size={16} className="text-health-500" />
          )}
          {capability.approvalRequired ? "Required" : "Review only"}
        </span>
      </td>
      <td className="px-4 py-3">
        <span className="rounded-sm border border-ink/10 bg-parchment px-2 py-1 text-xs font-semibold text-ink/60">
          {capability.traceLevel}
        </span>
      </td>
    </tr>
  );
}

function ChipList({ values }: { values: string[] }) {
  return (
    <div className="flex max-w-56 flex-wrap gap-1">
      {values.map((value) => (
        <span
          key={value}
          className="rounded-sm border border-ink/10 bg-parchment px-2 py-1 text-xs text-ink/60"
        >
          {value}
        </span>
      ))}
    </div>
  );
}

function PathBlock({
  title,
  icon: Icon,
  paths,
  tone,
}: {
  title: string;
  icon: LucideIcon;
  paths: string[];
  tone: string;
}) {
  return (
    <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2">
      <div className="flex items-center gap-2">
        <Icon aria-hidden size={15} className={tone} />
        <p className="text-xs font-semibold uppercase text-ink/45">{title}</p>
      </div>
      <div className="mt-2 grid gap-1">
        {paths.map((path) => (
          <code key={path} className="truncate text-xs text-ink/65">
            {path}
          </code>
        ))}
      </div>
    </div>
  );
}

function EvidenceLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[120px_minmax(0,1fr)] gap-3 text-sm">
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
