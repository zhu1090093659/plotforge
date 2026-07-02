import {
  Boxes,
  CheckCircle2,
  FileText,
  Loader2,
  Network,
  PackageCheck,
  Play,
  ShieldCheck,
  Sparkles,
  TerminalSquare,
  type LucideIcon,
} from "lucide-react";
import type { ExportProfile, ProjectData } from "../../../contracts/plotforge";
import type { CreatorProjectSummary } from "./projectSummary";
import type { StudioMetric } from "./useStudioWorkspace";
import type {
  PlayOnceReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";
import { useStudioI18n } from "./i18n";
import {
  ScenePreviewPlaceholder,
  StudioButton,
  StudioStatusChip,
  StudioTabs,
  studioUiClassNames,
} from "./studioUi";

interface CommandCenterViewProps {
  projectSummary: CreatorProjectSummary | null;
  projectData: ProjectData | null;
  loadedPath: string;
  metrics: StudioMetric[];
  sourceFiles: SourceFileSummary[];
  selectedFile: SourceFileContent | null;
  playtestInput: string;
  playtesting: boolean;
  playtestReport: PlayOnceReport | null;
  playtestError: string | null;
  exportProfiles: ExportProfile[];
  dirty: boolean;
  onIntentChange(intent: string): void;
  onRunPlayableProof(): void;
  onOpenSection(section: "world" | "story" | "playtest" | "debugger" | "assets"): void;
  onOpenExportProfile(profileId: string): void;
}

export function CommandCenterView({
  projectSummary,
  projectData,
  loadedPath,
  metrics,
  sourceFiles,
  selectedFile,
  playtestInput,
  playtesting,
  playtestReport,
  playtestError,
  exportProfiles,
  dirty,
  onIntentChange,
  onRunPlayableProof,
  onOpenSection,
  onOpenExportProfile,
}: CommandCenterViewProps) {
  const { t } = useStudioI18n();
  const projectTitle = projectSummary?.title ?? "No project loaded";
  const entryScene = resolveEntryScene(projectData);
  const entryBeat = resolveSceneBeat(
    entryScene,
    projectData?.story_state.current_beat_id,
  );
  const sceneImage = resolveScenePreviewImage({
    scene: entryScene,
    projectId: projectData?.game.id,
    loadedPath,
  });
  const recentRuns = recentRunRows(playtestReport, projectTitle);
  const staticProfile =
    exportProfiles.find((profile) => profile.target === "static_web") ??
    null;
  const exportProfile = staticProfile;
  const proofReady = Boolean(playtestReport && !playtestError);
  const exportReady = Boolean(staticProfile);
  const sourceSummary = `${sourceFiles.length} files`;

  return (
    <div className="grid gap-4">
      <section
        aria-label="Project Launchpad"
        className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_300px] xl:grid-cols-[minmax(0,1.25fr)_minmax(320px,0.75fr)] 2xl:grid-cols-[minmax(0,1.35fr)_minmax(360px,0.65fr)]"
      >
        <div className="grid content-start gap-3">
          <div className="rounded-lg border border-canvas-200 bg-graphite-950 p-3 text-ink shadow-studio-panel">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-violet-600">
                  Project Launchpad
                </p>
                <h3 className="mt-1 text-xl font-semibold text-ink">
                  {projectTitle}
                </h3>
                <p className="mt-1 max-w-2xl text-sm leading-5 text-graphite-700/70">
                  {entryScene
                    ? `${entryScene.title} / ${entryScene.location}`
                    : loadedPath}
                </p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioStatusChip tone="health">
                  {projectSummary ? "Project loaded" : "No project"}
                </StudioStatusChip>
                <StudioStatusChip tone={dirty ? "action" : "health"}>
                  {dirty ? "Unsaved source" : "Workspace synced"}
                </StudioStatusChip>
              </div>
            </div>

            <div className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
              {metrics.map((metric) => (
                <StatusTile
                  key={metric.label}
                  label={metric.label}
                  value={metric.value}
                  detail={metricDetail(metric.label, projectSummary)}
                />
              ))}
            </div>

            <div className="mt-3 grid gap-2 lg:grid-cols-2">
              <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
                <p className="text-xs font-semibold uppercase text-graphite-700/55">
                  Playable Proof Status
                </p>
                <div className="mt-2 flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p className="text-base font-semibold text-ink">
                      {proofReady ? "Playable proof captured" : "Ready to run proof"}
                    </p>
                    <p className="mt-0.5 text-xs text-graphite-700/65">
                      {playtestReport
                        ? `${playtestReport.trace.id} / ${playtestReport.delta_summary.length} deltas`
                        : "No turn has been run in this session."}
                    </p>
                  </div>
                  <StudioButton
                    variant="primary"
                    onClick={onRunPlayableProof}
                    disabled={playtesting}
                  >
                    {playtesting ? (
                      <Loader2 aria-hidden size={16} className="animate-spin" />
                    ) : (
                      <Play aria-hidden size={16} />
                    )}
                    Run proof
                  </StudioButton>
                </div>
              </div>

              <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
                <p className="text-xs font-semibold uppercase text-graphite-700/55">
                  Export Readiness
                </p>
                <div className="mt-2 flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p className="text-base font-semibold text-ink">
                      {exportReady ? "Static package available" : "No static package profile"}
                    </p>
                    <p className="mt-0.5 text-xs text-graphite-700/65">
                      {exportProfile?.id ?? "Export profile not loaded"}
                    </p>
                  </div>
                  {exportProfile ? (
                    <StudioButton
                      onClick={() => onOpenExportProfile(exportProfile.id)}
                    >
                      <PackageCheck aria-hidden size={16} />
                      Export Package
                    </StudioButton>
                  ) : null}
                </div>
              </div>
            </div>
          </div>

          <div
            aria-label="Live Game Canvas"
            className="overflow-hidden rounded-lg border border-canvas-200 bg-graphite-950 text-ink shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-canvas-200 px-4 py-2.5">
              <div className="flex min-w-0 items-center gap-2">
                <span className="h-2.5 w-2.5 rounded-full bg-health-400" />
                <p className="truncate text-sm font-semibold">Live Game Canvas</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioButton onClick={() => onOpenSection("playtest")}>
                  <Play aria-hidden size={16} />
                  Playtest
                </StudioButton>
                <StudioButton onClick={() => onOpenSection("debugger")}>
                  <TerminalSquare aria-hidden size={16} />
                  Runtime Trace
                </StudioButton>
              </div>
            </div>

            <div className="relative min-h-[220px] overflow-hidden bg-graphite-900">
              {sceneImage ? (
                <img
                  src={sceneImage}
                  alt=""
                  className="absolute inset-0 h-full w-full object-cover opacity-70"
                />
              ) : (
                <ScenePreviewPlaceholder
                  assetPath={entryScene?.background_asset ?? null}
                />
              )}
              <div className="absolute inset-0 bg-gradient-to-t from-graphite-950 via-graphite-950/30 to-graphite-950/10" />
              <div className="relative flex min-h-[220px] flex-col justify-end p-3">
                <div className="max-w-3xl rounded-lg border border-canvas-200 bg-canvas-100 px-3 py-3 text-ink shadow-studio-panel">
                  <p className="text-xs font-semibold uppercase text-graphite-700/55">
                    {entryScene?.title ?? "Scene preview"}
                  </p>
                  <p className="mt-1 line-clamp-3 text-sm leading-6">
                    {entryBeat?.text ??
                      "Open a project to preview the playable scene and proof loop."}
                  </p>
                  <div className="mt-2 grid gap-2">
                    {(entryBeat?.choices ?? []).slice(0, 3).map((choice, index) => (
                      <div
                        key={choice.id}
                        className="flex min-h-9 items-center gap-3 rounded-md border border-canvas-200 bg-graphite-950 px-3 text-ink"
                      >
                        <span className="grid h-6 w-6 shrink-0 place-items-center rounded-sm border border-violet-400/60 text-xs text-violet-600">
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
          </div>

        </div>

        <aside className="grid content-start gap-3">
          <div
            aria-label="Director Command Input"
            className="rounded-lg border border-violet-500/30 bg-canvas-50 p-3 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-xs font-semibold uppercase text-violet-600">
                  Command Center
                </p>
                <h3 className="mt-1 text-base font-semibold text-ink">
                  Describe the game change you want
                </h3>
              </div>
              <StudioButton onClick={() => onOpenSection("story")}>
                <FileText aria-hidden size={16} />
                Story Craft
              </StudioButton>
            </div>

            <textarea
              aria-label="Director intent"
              value={playtestInput}
              onChange={(event) => onIntentChange(event.target.value)}
              className={`${studioUiClassNames.textarea} mt-3 min-h-24 border-violet-500/45 bg-canvas-100 text-sm leading-6`}
            />
            <div className="mt-3 grid gap-3">
              <p className="text-sm leading-5 text-graphite-700/65">
                {t("Sends this through the local runtime proof command; agent proposal workflows are not implemented.")}
              </p>
              <StudioButton
                variant="primary"
                onClick={onRunPlayableProof}
                disabled={playtesting}
                className="w-full justify-center"
              >
                {playtesting ? (
                  <Loader2 aria-hidden size={16} className="animate-spin" />
                ) : (
                  <Sparkles aria-hidden size={16} />
                )}
                Apply as proof run
              </StudioButton>
            </div>
            {playtestError ? (
              <div className="mt-3 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
                {playtestError}
              </div>
            ) : null}
          </div>

          <StudioTabs
            ariaLabel="Command center side panels"
            items={[
              {
                id: "director",
                label: "Director",
                children: (
                  <div className="grid gap-3">
                    <Panel title="Director Brief">
                      <p className="text-sm leading-6 text-ink/70">
                        {projectData?.game.description ??
                          "Load a folder project to direct the next playable change."}
                      </p>
                      <div className="mt-3 grid gap-2">
                        <MiniFact label="Loaded path" value={loadedPath} />
                        <MiniFact label="Source artifact" value={selectedFile?.path ?? sourceSummary} />
                        <MiniFact label="Entry scene" value={projectSummary?.entryScene ?? "none"} />
                      </div>
                    </Panel>

                    <Panel title="Recent Runs">
                      <div className="grid gap-2">
                        {recentRuns.map((run) => (
                          <div
                            key={run.id}
                            className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3 rounded-md border border-ink/10 bg-canvas-50 px-3 py-2"
                          >
                            <CheckCircle2
                              aria-hidden
                              className={run.tone === "success" ? "text-sage" : "text-plum"}
                              size={16}
                            />
                            <div className="min-w-0">
                              <p className="truncate text-sm font-semibold">{run.id}</p>
                              <p className="truncate text-xs text-ink/50">{run.detail}</p>
                            </div>
                            <span className="text-xs font-semibold text-ink/45">
                              {run.when}
                            </span>
                          </div>
                        ))}
                      </div>
                    </Panel>
                  </div>
                ),
              },
              {
                id: "backend",
                label: "Backend",
                badge: "wired",
                children: (
                  <div className="grid gap-3">
                    <Panel
                      title="Backend Commands"
                      action={
                        <StudioStatusChip tone="health">wired</StudioStatusChip>
                      }
                    >
                      <div className="grid gap-3">
                        {[
                          {
                            id: "project",
                            label: "Project source",
                            role: "open_project, check_project, source file read/write",
                            state: "folder files",
                          },
                          {
                            id: "runtime",
                            label: "Runtime proof",
                            role: "play_once_project writes redaction-safe trace evidence",
                            state: proofReady ? "captured" : "ready",
                          },
                          {
                            id: "export",
                            label: "Static export",
                            role: "export_static_project_zip writes a whitelisted package",
                            state: exportReady ? "profile loaded" : "not loaded",
                          },
                        ].map((item) => (
                          <article
                            key={item.id}
                            className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3"
                          >
                            <div className="flex flex-wrap items-start justify-between gap-2">
                              <div className="min-w-0">
                                <p className="truncate text-sm font-semibold">
                                  {item.label}
                                </p>
                                <p className="mt-1 text-xs leading-5 text-ink/55">
                                  {item.role}
                                </p>
                              </div>
                              <StudioStatusChip tone="health">{item.state}</StudioStatusChip>
                            </div>
                          </article>
                        ))}
                      </div>
                    </Panel>

                    <Panel
                      title="Unavailable Agent Interfaces"
                      action={<StudioStatusChip tone="danger">not implemented</StudioStatusChip>}
                    >
                      <div className="grid gap-2 text-sm leading-6 text-ink/65">
                        <p>pi-Agent runtime is connected via Studio command pi_agent_run.</p>
                        <p>Approval queues are hidden until persisted proposal contracts exist.</p>
                        <p>Provider-backed generation remains explicit local mock/runtime logic.</p>
                      </div>
                    </Panel>
                  </div>
                ),
              },
              {
                id: "evidence",
                label: "Evidence",
                children: (
                  <Panel title="Evidence Snapshot">
                    <div className="grid grid-cols-2 gap-2">
                      <EvidenceCard
                        icon={Network}
                        label="Backend"
                        value="Studio"
                      />
                      <EvidenceCard
                        icon={ShieldCheck}
                        label="Project truth"
                        value="folder files"
                      />
                      <EvidenceCard
                        icon={CheckCircle2}
                        label="Proof"
                        value={proofReady ? "captured" : "not run"}
                      />
                      <EvidenceCard
                        icon={Boxes}
                        label="Artifacts"
                        value={sourceSummary}
                      />
                    </div>
                  </Panel>
                ),
              },
            ]}
          />
        </aside>
      </section>
    </div>
  );
}

function resolveEntryScene(projectData: ProjectData | null) {
  if (!projectData) {
    return null;
  }
  return (
    projectData.scenes.find(
      (scene) => scene.key === projectData.game.entry_scene,
    ) ??
    projectData.scenes[0] ??
    null
  );
}

function metricDetail(
  label: string,
  summary: CreatorProjectSummary | null,
): string {
  if (!summary) {
    return "not loaded";
  }
  if (label === "Open Threads") {
    return `${summary.activePromiseCount} promises`;
  }
  return `${summary.unresolvedReviewCount} review notes`;
}

function recentRunRows(
  report: PlayOnceReport | null,
  projectTitle: string,
): Array<{ id: string; detail: string; when: string; tone: "success" | "partial" }> {
  if (report) {
    return [
      {
        id: report.trace.id,
        detail: `${projectTitle} / ${report.scene.title}`,
        when: "current",
        tone: report.trace.errors.length > 0 ? "partial" : "success",
      },
    ];
  }

  return [
    {
      id: "trace-ready",
      detail: `${projectTitle} / waiting for first proof run`,
      when: "ready",
      tone: "partial",
    },
  ];
}

function StatusTile({
  label,
  value,
  detail,
}: {
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
      <p className="text-xs font-semibold uppercase text-graphite-700/55">
        {label}
      </p>
      <p className="mt-1 text-xl font-semibold text-ink">{value}</p>
      <p className="mt-1 truncate text-xs text-graphite-700/60">{detail}</p>
    </div>
  );
}

function Panel({
  title,
  action,
  children,
}: {
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-lg border border-canvas-200 bg-canvas-50 p-3 shadow-studio-panel">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h3 className="text-base font-semibold text-ink">{title}</h3>
        {action}
      </div>
      <div className="mt-2">{children}</div>
    </section>
  );
}

function MiniFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 rounded-md border border-ink/10 bg-canvas-50 px-3 py-2">
      <p className="text-xs font-semibold uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}

function EvidenceCard({
  icon: Icon,
  label,
  value,
}: {
  icon: LucideIcon;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3">
      <Icon aria-hidden size={18} className="text-ink/55" />
      <p className="mt-2 text-xs font-semibold uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}
