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
  const projectTitle = projectSummary?.title ?? t("app.noProjectLoaded");
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
  const recentRuns = recentRunRows(playtestReport, projectTitle, t);
  const staticProfile =
    exportProfiles.find((profile) => profile.target === "static_web") ?? null;
  const exportProfile = staticProfile;
  const proofReady = Boolean(playtestReport && !playtestError);
  const exportReady = Boolean(staticProfile);
  const sourceSummary = `${sourceFiles.length} files`;

  return (
    <div className="grid gap-4">
      <section
        aria-label={t("commandCenter.aria.projectLaunchpad")}
        className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_300px] xl:grid-cols-[minmax(0,1.25fr)_minmax(320px,0.75fr)] 2xl:grid-cols-[minmax(0,1.35fr)_minmax(360px,0.65fr)]"
      >
        <div className="grid content-start gap-3">
          <div className="rounded-lg border border-canvas-200 bg-graphite-950 p-3 text-ink shadow-studio-panel">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase text-violet-600">
                  {t("commandCenter.projectLaunchpad")}
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
                  {projectSummary ? t("commandCenter.projectLoaded") : t("commandCenter.noProject")}
                </StudioStatusChip>
                <StudioStatusChip tone={dirty ? "action" : "health"}>
                  {dirty ? t("app.unsavedSource") : t("app.workspaceSynced")}
                </StudioStatusChip>
              </div>
            </div>

            <div className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
              {metrics.map((metric) => (
                <StatusTile
                  key={metric.labelKey}
                  label={t(metric.labelKey)}
                  value={metric.value}
                  detail={metricDetail(metric.labelKey, projectSummary, t)}
                />
              ))}
            </div>

            <div className="mt-3 grid gap-2 lg:grid-cols-2">
              <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
                <p className="text-xs font-semibold uppercase text-graphite-700/55">
                  {t("commandCenter.playableProofStatus")}
                </p>
                <div className="mt-2 flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p className="text-base font-semibold text-ink">
                      {proofReady ? t("commandCenter.proofCaptured") : t("commandCenter.readyToRunProof")}
                    </p>
                    <p className="mt-0.5 text-xs text-graphite-700/65">
                      {playtestReport
                        ? t("common.traceDeltas", { trace: playtestReport.trace.id, count: playtestReport.delta_summary.length })
                        : t("commandCenter.noTurnRun")}
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
                    {t("commandCenter.runProof")}
                  </StudioButton>
                </div>
              </div>

              <div className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-2">
                <p className="text-xs font-semibold uppercase text-graphite-700/55">
                  {t("commandCenter.exportReadiness")}
                </p>
                <div className="mt-2 flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p className="text-base font-semibold text-ink">
                      {exportReady ? t("commandCenter.staticPackageAvailable") : t("commandCenter.noStaticProfile")}
                    </p>
                    <p className="mt-0.5 text-xs text-graphite-700/65">
                      {exportProfile?.id ?? t("commandCenter.exportProfileNotLoaded")}
                    </p>
                  </div>
                  {exportProfile ? (
                    <StudioButton
                      onClick={() => onOpenExportProfile(exportProfile.id)}
                    >
                      <PackageCheck aria-hidden size={16} />
                      {t("commandCenter.exportPackage")}
                    </StudioButton>
                  ) : null}
                </div>
              </div>
            </div>
          </div>

          <div
            aria-label={t("commandCenter.aria.liveCanvas")}
            className="overflow-hidden rounded-lg border border-canvas-200 bg-graphite-950 text-ink shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-canvas-200 px-4 py-2.5">
              <div className="flex min-w-0 items-center gap-2">
                <span className="h-2.5 w-2.5 rounded-full bg-health-400" />
                <p className="truncate text-sm font-semibold">{t("commandCenter.liveCanvas")}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioButton onClick={() => onOpenSection("playtest")}>
                  <Play aria-hidden size={16} />
                  {t("commandCenter.playtest")}
                </StudioButton>
                <StudioButton onClick={() => onOpenSection("debugger")}>
                  <TerminalSquare aria-hidden size={16} />
                  {t("commandCenter.runtimeTrace")}
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
                    {entryScene?.title ?? t("commandCenter.scenePreview")}
                  </p>
                  <p className="mt-1 line-clamp-3 text-sm leading-6">
                    {entryBeat?.text ?? t("commandCenter.openProjectPreview")}
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
            aria-label={t("commandCenter.aria.directorCommandInput")}
            className="rounded-lg border border-violet-500/30 bg-canvas-50 p-3 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-xs font-semibold uppercase text-violet-600">
                  {t("commandCenter.commandCenter")}
                </p>
                <h3 className="mt-1 text-base font-semibold text-ink">
                  {t("commandCenter.describeChange")}
                </h3>
              </div>
              <StudioButton onClick={() => onOpenSection("story")}>
                <FileText aria-hidden size={16} />
                {t("commandCenter.storyCraft")}
              </StudioButton>
            </div>

            <textarea
              aria-label={t("commandCenter.aria.directorIntent")}
              value={playtestInput}
              onChange={(event) => onIntentChange(event.target.value)}
              className={`${studioUiClassNames.textarea} mt-3 min-h-24 border-violet-500/45 bg-canvas-100 text-sm leading-6`}
            />
            <div className="mt-3 grid gap-3">
              <p className="text-sm leading-5 text-graphite-700/65">
                {t("commandCenter.sendsThroughRuntime")}
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
                {t("commandCenter.applyAsProofRun")}
              </StudioButton>
            </div>
            {playtestError ? (
              <div className="mt-3 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
                {playtestError}
              </div>
            ) : null}
          </div>

          <StudioTabs
            ariaLabel={t("commandCenter.aria.sidePanels")}
            items={[
              {
                id: "director",
                label: t("commandCenter.directorTab"),
                children: (
                  <div className="grid gap-3">
                    <Panel title={t("commandCenter.directorBrief")}>
                      <p className="text-sm leading-6 text-ink/70">
                        {projectData?.game.description ??
                          t("commandCenter.loadProjectToDirect")}
                      </p>
                      <div className="mt-3 grid gap-2">
                        <MiniFact label={t("commandCenter.loadedPath")} value={loadedPath} />
                        <MiniFact label={t("commandCenter.sourceArtifact")} value={selectedFile?.path ?? sourceSummary} />
                        <MiniFact label={t("commandCenter.entryScene")} value={projectSummary?.entryScene ?? t("common.none")} />
                      </div>
                    </Panel>

                    <Panel title={t("commandCenter.recentRuns")}>
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
                label: t("commandCenter.backendTab"),
                badge: t("common.wired"),
                children: (
                  <div className="grid gap-3">
                    <Panel
                      title={t("commandCenter.backendCommands")}
                      action={
                        <StudioStatusChip tone="health">{t("common.wired")}</StudioStatusChip>
                      }
                    >
                      <div className="grid gap-3">
                        {[
                          {
                            id: "project",
                            label: t("commandCenter.projectSourceLabel"),
                            role: t("commandCenter.projectSourceRole"),
                            state: t("commandCenter.folderFilesState"),
                          },
                          {
                            id: "runtime",
                            label: t("commandCenter.runtimeProofLabel"),
                            role: t("commandCenter.runtimeProofRole"),
                            state: proofReady ? t("common.captured") : t("commandCenter.readyState"),
                          },
                          {
                            id: "export",
                            label: t("commandCenter.staticExportLabel"),
                            role: t("commandCenter.staticExportRole"),
                            state: exportReady ? t("commandCenter.profileLoadedState") : t("commandCenter.notLoadedState"),
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
                      title={t("commandCenter.unavailableAgentInterfaces")}
                      action={<StudioStatusChip tone="danger">{t("common.notImplemented")}</StudioStatusChip>}
                    >
                      <div className="grid gap-2 text-sm leading-6 text-ink/65">
                        <p>{t("commandCenter.piAgentConnected")}</p>
                        <p>{t("commandCenter.approvalQueuesHidden")}</p>
                        <p>{t("commandCenter.providerBackedMock")}</p>
                      </div>
                    </Panel>
                  </div>
                ),
              },
              {
                id: "evidence",
                label: t("commandCenter.evidenceTab"),
                children: (
                  <Panel title={t("commandCenter.evidenceSnapshot")}>
                    <div className="grid grid-cols-2 gap-2">
                      <EvidenceCard
                        icon={Network}
                        label={t("commandCenter.backend")}
                        value={t("commandCenter.studio")}
                      />
                      <EvidenceCard
                        icon={ShieldCheck}
                        label={t("commandCenter.projectTruth")}
                        value={t("commandCenter.folderFilesState")}
                      />
                      <EvidenceCard
                        icon={CheckCircle2}
                        label={t("commandCenter.proof")}
                        value={proofReady ? t("common.captured") : t("common.notRun")}
                      />
                      <EvidenceCard
                        icon={Boxes}
                        label={t("commandCenter.artifacts")}
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
  t: (key: string, params?: Record<string, string | number>) => string,
): string {
  if (!summary) {
    return t("commandCenter.notLoaded");
  }
  if (label === "metrics.openThreads") {
    return t("commandCenter.openThreadsDetail", { count: summary.activePromiseCount });
  }
  return t("commandCenter.reviewNotesDetail", { count: summary.unresolvedReviewCount });
}

function recentRunRows(
  report: PlayOnceReport | null,
  projectTitle: string,
  t: (key: string, params?: Record<string, string | number>) => string,
): Array<{ id: string; detail: string; when: string; tone: "success" | "partial" }> {
  if (report) {
    return [
      {
        id: report.trace.id,
        detail: `${projectTitle} / ${report.scene.title}`,
        when: t("common.current"),
        tone: report.trace.errors.length > 0 ? "partial" : "success",
      },
    ];
  }

  return [
    {
      id: "trace-ready",
      detail: t("common.projectWaitingProof", { project: projectTitle }),
      when: t("common.ready"),
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
