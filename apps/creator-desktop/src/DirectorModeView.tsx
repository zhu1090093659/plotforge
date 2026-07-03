import {
  CheckCircle2,
  FileText,
  FlaskConical,
  Loader2,
  Play,
  ShieldCheck,
  Sparkles,
  Target,
} from "lucide-react";
import type { ProjectData } from "../../../contracts/plotforge";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";
import type { PlayOnceReport } from "./tauriBridge";
import {
  Collapsible,
  ScenePreviewImage,
  StudioButton,
  StudioStatusChip,
  studioUiClassNames,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

interface DirectorModeViewProps {
  projectData: ProjectData | null;
  loadedPath: string;
  input: string;
  running: boolean;
  report: PlayOnceReport | null;
  error: string | null;
  saveId: string;
  restoreId: string;
  restoreLatest: boolean;
  onInputChange(input: string): void;
  onSaveIdChange(saveId: string): void;
  onRestoreIdChange(restoreId: string): void;
  onRestoreLatestChange(restoreLatest: boolean): void;
  onRun(): void;
  onOpenStory(): void;
  onOpenTrace(): void;
}

export function DirectorModeView({
  projectData,
  loadedPath,
  input,
  running,
  report,
  error,
  saveId,
  restoreId,
  restoreLatest,
  onInputChange,
  onSaveIdChange,
  onRestoreIdChange,
  onRestoreLatestChange,
  onRun,
  onOpenStory,
  onOpenTrace,
}: DirectorModeViewProps) {
  const { t } = useStudioI18n();
  const scene = report?.scene ?? resolveEntryScene(projectData);
  const beat = resolveSceneBeat(
    scene,
    report?.trace.story_state_after.current_beat_id ??
      projectData?.story_state.current_beat_id,
  );
  const sceneImage = resolveScenePreviewImage({
    scene,
    projectId: projectData?.game.id,
    loadedPath,
  });
  const trace = report?.trace ?? null;
  const queueItems = directionQueue(report, t);
  const suggestedDirections = [
    t("director.suggestedRaiseStakes"),
    t("director.suggestedAddClue"),
    t("director.suggestedMakeConsequence"),
    t("director.suggestedTestAlternate"),
  ];
  const activityItems = [
    {
      id: "project-source",
      label: t("director.projectSource"),
      title: projectData
        ? t("director.folderProjectLoaded")
        : t("director.noProjectLoaded"),
      body: projectData
        ? t("common.scenesRulesCharacters", {
            count: projectData.scenes.length,
            rules: projectData.rules.length,
            characters: projectData.characters.length,
          })
        : t("director.openProjectToRun"),
      detail: loadedPath,
    },
    {
      id: "runtime-proof",
      label: t("director.runtimeProof"),
      title: report
        ? t("director.latestPlaytestCommitted")
        : t("director.noPlaytestRun"),
      body: report
        ? t("common.visibleStateDeltasFromScene", {
            count: report.delta_summary.length,
            scene: report.scene.title,
          })
        : t("director.runTurnWritesTrace"),
      detail: report?.trace.id ?? t("common.notCaptured"),
    },
  ];

  return (
    <section aria-label={t("director.aria.workspace")} className="grid gap-5">
      <div
        data-testid="director-mode-layout"
        className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px] 2xl:grid-cols-[minmax(0,1fr)_380px]"
      >
        <div className="grid content-start gap-4">
          <aside
            aria-label={t("director.aria.activityStream")}
            className="rounded-lg border border-canvas-200 bg-graphite-950 p-3 text-ink shadow-studio-panel"
          >
            <div className="flex items-center justify-between gap-3">
              <h3 className="text-sm font-semibold">{t("director.activityStream")}</h3>
              <StudioStatusChip tone="health">{t("director.live")}</StudioStatusChip>
            </div>
            <div className="mt-3 grid gap-3 sm:grid-cols-2">
              {activityItems.map((item) => (
                <article
                  key={item.id}
                  className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="truncate text-sm font-semibold text-ink">
                        {item.label}
                      </p>
                      <p className="mt-1 text-xs text-graphite-700/55">{t("director.realData")}</p>
                    </div>
                    <CheckCircle2
                      aria-hidden
                      size={16}
                      className="mt-0.5 shrink-0 text-health-400"
                    />
                  </div>
                  <p className="mt-2 truncate text-sm font-semibold text-ink">
                    {item.title}
                  </p>
                  <p className="mt-1 line-clamp-2 text-xs leading-5 text-graphite-700/65">
                    {item.body}
                  </p>
                  <p className="mt-2 truncate text-xs text-health-400">
                    {item.detail}
                  </p>
                </article>
              ))}
            </div>
          </aside>

          <div
            aria-label={t("director.aria.playableScenePreview")}
            className="overflow-hidden rounded-lg border border-canvas-200 bg-graphite-950 text-ink shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-canvas-200 px-4 py-3">
              <div className="flex min-w-0 items-center gap-2">
                <span className="h-2.5 w-2.5 rounded-full bg-health-400" />
                <p className="truncate text-sm font-semibold">{t("director.playableScene")}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioButton onClick={onRun} disabled={running}>
                  {running ? (
                    <Loader2 aria-hidden size={16} className="animate-spin" />
                  ) : (
                    <Play aria-hidden size={16} />
                  )}
                  {t("director.play")}
                </StudioButton>
              </div>
            </div>

            <div className="relative min-h-[320px] overflow-hidden bg-graphite-900">
              <ScenePreviewImage
                src={sceneImage}
                assetPath={scene?.background_asset ?? null}
                className="absolute inset-0 h-full w-full object-cover opacity-70"
              />
              <div className="absolute inset-0 bg-gradient-to-t from-graphite-950 via-graphite-950/25 to-graphite-950/5" />
              <div className="relative flex min-h-[320px] flex-col justify-end p-4">
                <div className="mx-auto w-full max-w-3xl rounded-lg border border-violet-500/40 bg-graphite-950/90 px-4 py-4 shadow-studio-panel">
                  <p className="text-xs font-semibold uppercase text-violet-600">
                    {scene ? `${scene.title} / ${scene.location}` : loadedPath}
                  </p>
                  <p className="mt-2 text-base leading-7 text-ink">
                    {beat?.text ?? t("director.openProjectToPreview")}
                  </p>
                  <div className="mt-4 grid gap-2">
                    {(beat?.choices ?? []).slice(0, 4).map((choice, index) => (
                      <div
                        key={choice.id}
                        className="flex min-h-11 items-center gap-3 rounded-md border border-violet-500/35 bg-ink/5 px-3 text-ink"
                      >
                        <span className="grid h-7 w-7 shrink-0 place-items-center rounded-sm border border-violet-400/60 text-xs text-violet-600">
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

            <div className="grid gap-3 border-t border-canvas-200 px-4 py-3 text-sm sm:grid-cols-3">
              <CanvasFact label={t("director.scene")} value={scene?.key ?? t("common.none")} />
              <CanvasFact
                label={t("director.tension")}
                value={
                  trace?.narrative_review
                    ? t("common.scores", {
                        score: trace.narrative_review.score,
                      })
                    : t("common.ready")
                }
              />
              <CanvasFact
                label={t("director.branching")}
                value={t("common.choices", { count: beat?.choices.length ?? 0 })}
              />
            </div>
          </div>
        </div>

        <aside className="grid content-start gap-4">
          <div className="rounded-lg border border-violet-500/35 bg-graphite-950 p-4 text-ink shadow-studio-panel">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 items-start gap-3">
                <div className="grid h-11 w-11 shrink-0 place-items-center rounded-md border border-violet-400/50 bg-violet-500/15 text-violet-600">
                  <Target aria-hidden size={22} />
                </div>
                <div className="min-w-0">
                  <p className="text-xs font-semibold uppercase text-violet-600">
                    {t("director.creativeGoal")}
                  </p>
                  <h3 className="mt-1 text-xl font-semibold text-ink">
                    {input.trim() || t("director.frameNextChange")}
                  </h3>
                </div>
              </div>
              <StudioButton onClick={onOpenStory}>
                <FileText aria-hidden size={16} />
                {t("director.refineGoal")}
              </StudioButton>
            </div>
          </div>

          <div
            aria-label={t("director.aria.directionBar")}
            className="rounded-lg border border-violet-500/30 bg-canvas-50 p-3 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-xs font-semibold uppercase text-violet-600">
                  {t("director.directionBar")}
                </p>
                <h3 className="mt-1 text-lg font-semibold text-ink">
                  {t("director.runRuntimeTurn")}
                </h3>
              </div>
              <StudioButton onClick={onOpenTrace}>
                <ShieldCheck aria-hidden size={16} />
                {t("director.traceEvidence")}
              </StudioButton>
            </div>

            <textarea
              aria-label={t("director.aria.playtestInput")}
              value={input}
              onChange={(event) => onInputChange(event.target.value)}
              className={`${studioUiClassNames.textarea} mt-3 min-h-20 border-violet-500/45 bg-canvas-100 text-sm leading-6`}
            />

            <Collapsible
              label={t("director.advancedSnapshotControls")}
              defaultOpen={false}
              id="director-snapshot-controls"
              className="mt-3"
            >
              <div className="mt-2 grid gap-3 lg:grid-cols-[1fr_1fr_auto]">
                <label className="grid min-w-0 gap-1">
                  <span className="text-xs font-medium uppercase text-ink/45">
                    {t("director.saveId")}
                  </span>
                  <input
                    aria-label={t("director.aria.playtestSaveId")}
                    value={saveId}
                    onChange={(event) => onSaveIdChange(event.target.value)}
                    className={studioUiClassNames.input}
                  />
                </label>
                <label className="grid min-w-0 gap-1">
                  <span className="text-xs font-medium uppercase text-ink/45">
                    {t("director.restoreId")}
                  </span>
                  <input
                    aria-label={t("director.aria.playtestRestoreId")}
                    value={restoreId}
                    disabled={restoreLatest}
                    onChange={(event) => onRestoreIdChange(event.target.value)}
                    className={`${studioUiClassNames.input} disabled:cursor-not-allowed disabled:bg-ink/5 disabled:text-ink/35`}
                  />
                </label>
                <label className="flex min-h-10 items-center gap-2 self-end rounded-md border border-ink/10 px-3 text-sm font-medium text-ink/70">
                  <input
                    type="checkbox"
                    aria-label={t("director.aria.restoreLatestSave")}
                    checked={restoreLatest}
                    onChange={(event) =>
                      onRestoreLatestChange(event.target.checked)
                    }
                    className="h-4 w-4 accent-ink"
                  />
                  {t("director.restoreLatest")}
                </label>
              </div>
            </Collapsible>

            <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
              <div className="flex flex-wrap gap-2">
                {projectData && suggestedDirections.length > 0
                  ? suggestedDirections.map((direction) => (
                      <button
                        type="button"
                        key={direction}
                        onClick={() => onInputChange(direction)}
                        className="inline-flex min-h-9 items-center rounded-md border border-canvas-200 bg-canvas-100 px-3 text-sm font-semibold text-ink transition hover:border-violet-500/45"
                      >
                        {direction}
                      </button>
                    ))
                  : null}
              </div>
              <StudioButton
                variant="primary"
                aria-label={t("director.aria.runTurn")}
                onClick={onRun}
                disabled={running}
                className="w-full justify-center"
              >
                {running ? (
                  <Loader2 aria-hidden size={16} className="animate-spin" />
                ) : (
                  <Sparkles aria-hidden size={16} />
                )}
                {t("director.runTurn")}
              </StudioButton>
            </div>

            {error ? (
              <div className="mt-3 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
                {error}
              </div>
            ) : null}
          </div>
          <div
            aria-label={t("director.aria.decisionQueue")}
            className="grid content-start gap-3 rounded-lg border border-canvas-200 bg-canvas-50 p-3 shadow-studio-panel"
          >
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-xs font-semibold uppercase text-violet-600">
                {t("director.runtime")}
              </p>
              <h3 className="mt-1 text-base font-semibold text-ink">{t("director.decisionQueue")}</h3>
            </div>
            <StudioStatusChip tone="agent">{queueItems.length}</StudioStatusChip>
          </div>

          {queueItems.length ? (
            queueItems.map((item) => (
              <article
                key={item.id}
                className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3"
              >
                <div className="flex flex-wrap items-start justify-between gap-2">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-semibold text-ink">
                      {item.title}
                    </p>
                    <p className="mt-1 text-xs leading-5 text-ink/55">
                      {item.body}
                    </p>
                  </div>
                  <StudioStatusChip tone={item.tone}>
                    {item.impact}
                  </StudioStatusChip>
                </div>
                <div className="mt-3 grid gap-2 text-xs">
                  <QueueFact label={t("director.tracePath")} value={item.filesChanged} />
                  <QueueFact label={t("director.evidence")} value={item.evidence} />
                </div>
                <div className="mt-3 grid gap-2 sm:grid-cols-2">
                  <StudioButton onClick={onOpenTrace}>
                    <FlaskConical aria-hidden size={16} />
                    {t("director.openTrace")}
                  </StudioButton>
                  <StudioButton onClick={onOpenTrace}>
                    <CheckCircle2 aria-hidden size={16} />
                    {t("director.reviewEvidence")}
                  </StudioButton>
                </div>
              </article>
            ))
          ) : (
            <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-3 text-sm leading-6 text-ink/55">
              {t("director.noQueue")}
            </div>
          )}
          </div>
        </aside>
      </div>
    </section>
  );
}

function resolveEntryScene(projectData: ProjectData | null) {
  if (!projectData) {
    return null;
  }
  return (
    projectData.scenes.find(
      (scene) => scene.key === projectData.story_state.current_scene_key,
    ) ??
    projectData.scenes.find(
      (scene) => scene.key === projectData.game.entry_scene,
    ) ??
    projectData.scenes[0] ??
    null
  );
}

function directionQueue(
  report: PlayOnceReport | null,
  t: (key: string, params?: Record<string, string | number>) => string,
): Array<{
  id: string;
  title: string;
  body: string;
  impact: string;
  filesChanged: string;
  evidence: string;
  tone: "action" | "health" | "accent" | "agent";
}> {
  if (!report) {
    return [];
  }

  return [
    {
      id: "playtest-result",
      title: t("director.playtestResult"),
      body: t("common.visibleStateDeltasFrom", {
        scene: report.scene.title,
        count: report.delta_summary.length,
      }),
      impact: report.trace.errors.length
        ? t("common.needsReview")
        : t("common.informational"),
      filesChanged: report.trace_path,
      evidence: report.trace.id,
      tone: report.trace.errors.length ? "action" : "health",
    },
  ];
}

function CanvasFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-semibold uppercase text-graphite-700/55">
        {label}
      </p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">
        {value}
      </p>
    </div>
  );
}

function QueueFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 justify-between gap-3 border-t border-ink/10 pt-2">
      <span className="text-ink/45">{label}</span>
      <span className="truncate font-semibold text-ink">{value}</span>
    </div>
  );
}
