import {
  CheckCircle2,
  FileText,
  FlaskConical,
  Loader2,
  Play,
  RefreshCcw,
  ShieldCheck,
  Sparkles,
  Target,
} from "lucide-react";
import type { ProjectData } from "../../../contracts/plotforge";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";
import type { PlayOnceReport } from "./tauriBridge";
import {
  ScenePreviewPlaceholder,
  StudioButton,
  StudioStatusChip,
  studioUiClassNames,
} from "./studioUi";

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
  const queueItems = directionQueue(report);
  const activityItems = [
    {
      id: "project-source",
      label: "Project source",
      title: projectData ? "Folder project loaded" : "No project loaded",
      body: projectData
        ? `${projectData.scenes.length} scenes, ${projectData.rules.length} rules, ${projectData.characters.length} characters`
        : "Open a project before running a playable turn.",
      detail: loadedPath,
    },
    {
      id: "runtime-proof",
      label: "Runtime proof",
      title: report ? "Latest playtest committed" : "No playtest run yet",
      body: report
        ? `${report.delta_summary.length} visible state deltas from ${report.scene.title}`
        : "Run turn calls the Studio runtime command and writes a trace.",
      detail: report?.trace.id ?? "not captured",
    },
  ];

  return (
    <section aria-label="Director Mode Workspace" className="grid gap-5">
      <div
        data-testid="director-mode-layout"
        className="grid gap-4 2xl:grid-cols-[240px_minmax(0,1fr)_320px]"
      >
        <aside
          aria-label="Activity Stream"
          className="grid content-start gap-3 rounded-lg border border-graphite-700/15 bg-graphite-950 p-3 text-canvas-50 shadow-studio-panel"
        >
          <div className="flex items-center justify-between gap-3">
            <h3 className="text-sm font-semibold">Activity Stream</h3>
            <StudioStatusChip tone="health">Live</StudioStatusChip>
          </div>
          {activityItems.map((item) => (
            <article
              key={item.id}
              className="rounded-md border border-canvas-200/10 bg-canvas-50/5 px-3 py-3"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate text-sm font-semibold text-canvas-50">
                    {item.label}
                  </p>
                  <p className="mt-1 text-xs text-canvas-200/45">real data</p>
                </div>
                <CheckCircle2
                  aria-hidden
                  size={16}
                  className="mt-0.5 shrink-0 text-health-400"
                />
              </div>
              <p className="mt-3 text-sm font-semibold text-canvas-50">
                {item.title}
              </p>
              <p className="mt-1 text-xs leading-5 text-canvas-200/60">
                {item.body}
              </p>
              <p className="mt-2 truncate text-xs text-health-400">
                {item.detail}
              </p>
            </article>
          ))}
        </aside>

        <div className="grid gap-4">
          <div className="rounded-lg border border-amber-500/35 bg-graphite-950 p-4 text-canvas-50 shadow-studio-panel">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex min-w-0 items-start gap-3">
                <div className="grid h-11 w-11 shrink-0 place-items-center rounded-md border border-amber-400/50 bg-amber-500/15 text-amber-400">
                  <Target aria-hidden size={22} />
                </div>
                <div className="min-w-0">
                  <p className="text-xs font-semibold uppercase text-amber-400">
                    Creative Goal
                  </p>
                  <h3 className="mt-1 text-xl font-semibold text-canvas-50">
                    {input.trim() || "Frame the next playable change."}
                  </h3>
                </div>
              </div>
              <StudioButton onClick={onOpenStory}>
                <FileText aria-hidden size={16} />
                Refine goal
              </StudioButton>
            </div>
          </div>

          <div
            aria-label="Playable Scene Preview"
            className="overflow-hidden rounded-lg border border-graphite-700/15 bg-graphite-950 text-canvas-50 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3 border-b border-canvas-200/10 px-4 py-3">
              <div className="flex min-w-0 items-center gap-2">
                <span className="h-2.5 w-2.5 rounded-full bg-health-400" />
                <p className="truncate text-sm font-semibold">Playable Scene</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <StudioButton onClick={onRun} disabled={running}>
                  {running ? (
                    <Loader2 aria-hidden size={16} className="animate-spin" />
                  ) : (
                    <Play aria-hidden size={16} />
                  )}
                  Play
                </StudioButton>
                <StudioButton onClick={() => onInputChange(input)}>
                  <RefreshCcw aria-hidden size={16} />
                  Reload
                </StudioButton>
              </div>
            </div>

            <div className="relative min-h-[460px] overflow-hidden bg-graphite-900">
              {sceneImage ? (
                <img
                  src={sceneImage}
                  alt=""
                  className="absolute inset-0 h-full w-full object-cover opacity-70"
                />
              ) : (
                <ScenePreviewPlaceholder assetPath={scene?.background_asset ?? null} />
              )}
              <div className="absolute inset-0 bg-gradient-to-t from-graphite-950 via-graphite-950/25 to-graphite-950/5" />
              <div className="relative flex min-h-[460px] flex-col justify-end p-4">
                <div className="mx-auto w-full max-w-3xl rounded-lg border border-amber-500/40 bg-graphite-950/90 px-4 py-4 shadow-studio-panel">
                  <p className="text-xs font-semibold uppercase text-amber-400">
                    {scene ? `${scene.title} / ${scene.location}` : loadedPath}
                  </p>
                  <p className="mt-2 text-base leading-7 text-canvas-50">
                    {beat?.text ??
                      "Open a project to preview the player-facing scene."}
                  </p>
                  <div className="mt-4 grid gap-2">
                    {(beat?.choices ?? []).slice(0, 4).map((choice, index) => (
                      <div
                        key={choice.id}
                        className="flex min-h-11 items-center gap-3 rounded-md border border-amber-500/35 bg-canvas-50/5 px-3 text-canvas-50"
                      >
                        <span className="grid h-7 w-7 shrink-0 place-items-center rounded-sm border border-amber-400/60 text-xs text-amber-400">
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

            <div className="grid gap-3 border-t border-canvas-200/10 px-4 py-3 text-sm sm:grid-cols-3">
              <CanvasFact label="Scene" value={scene?.key ?? "none"} />
              <CanvasFact
                label="Tension"
                value={trace?.narrative_review ? `${trace.narrative_review.score}/100` : "ready"}
              />
              <CanvasFact
                label="Branching"
                value={`${beat?.choices.length ?? 0} choices`}
              />
            </div>
          </div>

          <div
            aria-label="Direction Bar"
            className="rounded-lg border border-amber-500/30 bg-canvas-50 p-4 shadow-studio-panel"
          >
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-xs font-semibold uppercase text-amber-600">
                  Direction Bar
                </p>
                <h3 className="mt-1 text-lg font-semibold text-ink">
                  Run a runtime turn
                </h3>
              </div>
              <StudioButton onClick={onOpenTrace}>
                <ShieldCheck aria-hidden size={16} />
                Trace evidence
              </StudioButton>
            </div>

            <textarea
              aria-label="Playtest input"
              value={input}
              onChange={(event) => onInputChange(event.target.value)}
              className={`${studioUiClassNames.textarea} mt-3 min-h-24 border-amber-500/45 bg-canvas-100 text-base leading-7`}
            />

            <div className="mt-3 grid gap-3 lg:grid-cols-[1fr_1fr_auto]">
              <label className="grid min-w-0 gap-1">
                <span className="text-xs font-medium uppercase text-ink/45">
                  Save ID
                </span>
                <input
                  aria-label="Playtest save id"
                  value={saveId}
                  onChange={(event) => onSaveIdChange(event.target.value)}
                  className={studioUiClassNames.input}
                />
              </label>
              <label className="grid min-w-0 gap-1">
                <span className="text-xs font-medium uppercase text-ink/45">
                  Restore ID
                </span>
                <input
                  aria-label="Playtest restore id"
                  value={restoreId}
                  disabled={restoreLatest}
                  onChange={(event) => onRestoreIdChange(event.target.value)}
                  className={`${studioUiClassNames.input} disabled:cursor-not-allowed disabled:bg-ink/5 disabled:text-ink/35`}
                />
              </label>
              <label className="flex min-h-10 items-center gap-2 self-end rounded-md border border-ink/10 px-3 text-sm font-medium text-ink/70">
                <input
                  type="checkbox"
                  aria-label="Restore latest save"
                  checked={restoreLatest}
                  onChange={(event) =>
                    onRestoreLatestChange(event.target.checked)
                  }
                  className="h-4 w-4 accent-ink"
                />
                Restore latest
              </label>
            </div>

            <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
              <div className="flex flex-wrap gap-2">
                {suggestedDirections.map((direction) => (
                  <button
                    type="button"
                    key={direction}
                    onClick={() => onInputChange(direction)}
                    className="inline-flex min-h-9 items-center rounded-md border border-graphite-700/20 bg-canvas-100 px-3 text-sm font-semibold text-ink transition hover:border-amber-500/45"
                  >
                    {direction}
                  </button>
                ))}
              </div>
              <StudioButton variant="primary" aria-label="Run turn" onClick={onRun} disabled={running}>
                {running ? (
                  <Loader2 aria-hidden size={16} className="animate-spin" />
                ) : (
                  <Sparkles aria-hidden size={16} />
                )}
                Run turn
              </StudioButton>
            </div>

            {error ? (
              <div className="mt-3 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
                {error}
              </div>
            ) : null}
          </div>
        </div>

        <aside
          aria-label="Decision Queue"
          className="grid content-start gap-3 rounded-lg border border-graphite-700/15 bg-canvas-50 p-3 shadow-studio-panel"
        >
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-xs font-semibold uppercase text-amber-600">
                Runtime
              </p>
              <h3 className="text-base font-semibold text-ink">Decision Queue</h3>
            </div>
            <StudioStatusChip tone="agent">{queueItems.length}</StudioStatusChip>
          </div>

          {queueItems.length ? (
            queueItems.map((item) => (
              <article
                key={item.id}
                className="rounded-md border border-ink/10 bg-parchment px-3 py-3"
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
                  <QueueFact label="Trace path" value={item.filesChanged} />
                  <QueueFact label="Evidence" value={item.evidence} />
                </div>
                <div className="mt-3 grid gap-2 sm:grid-cols-2">
                  <StudioButton onClick={onOpenTrace}>
                    <FlaskConical aria-hidden size={16} />
                    Open trace
                  </StudioButton>
                  <StudioButton onClick={onOpenTrace}>
                    <CheckCircle2 aria-hidden size={16} />
                    Review evidence
                  </StudioButton>
                </div>
              </article>
            ))
          ) : (
            <div className="rounded-md border border-ink/10 bg-parchment px-3 py-3 text-sm leading-6 text-ink/55">
              No decision queue is available. Run a turn to create runtime
              evidence; agent approval queues are not implemented.
            </div>
          )}
        </aside>
      </div>
    </section>
  );
}

const suggestedDirections = [
  "raise stakes",
  "add clue",
  "make choice consequence visible",
  "test alternate ending",
];

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

function directionQueue(report: PlayOnceReport | null): Array<{
  id: string;
  title: string;
  body: string;
  impact: string;
  filesChanged: string;
  evidence: string;
  tone: "action" | "health" | "acp" | "agent";
}> {
  if (!report) {
    return [];
  }

  return [
    {
      id: "playtest-result",
      title: "Playtest result",
      body: `${report.scene.title} produced ${report.delta_summary.length} visible state deltas.`,
      impact: report.trace.errors.length ? "Needs review" : "Informational",
      filesChanged: report.trace_path,
      evidence: report.trace.id,
      tone: report.trace.errors.length ? "action" : "health",
    },
  ];
}

function CanvasFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-semibold uppercase text-canvas-200/45">
        {label}
      </p>
      <p className="mt-1 truncate text-sm font-semibold text-canvas-50">
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
