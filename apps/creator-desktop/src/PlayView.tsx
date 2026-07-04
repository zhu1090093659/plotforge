import type { ProjectData } from "../../../contracts/plotforge";
import { resolveSceneBeat, resolveScenePreviewImage } from "./scenePreview";
import type { PlayOnceReport } from "./tauriBridge";
import {
  Reveal,
  ScenePreviewImage,
  StudioStatusChip,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// PlayView — the single read-only scene preview surface.
//
// The creator directs the story through the right-side Agent chat rail (the
// only "describe a change / run a turn" entry point). This view is the single
// place where the resulting scene is previewed: the background image, the
// current beat text, and the choice cards. Clicking a choice submits it as
// the next turn's intent through `onChooseChoice` (the Agent rail consumes
// the shared playtest-input state).
//
// This view is intentionally read-only and contains no "run" button, no
// input textarea, and no evidence/trace panel. Those concerns live in the
// Agent rail and the Trace view respectively.
// ---------------------------------------------------------------------------

export interface PlayViewProps {
  projectData: ProjectData | null;
  loadedPath: string;
  report: PlayOnceReport | null;
  running: boolean;
  onChooseChoice(choiceLabel: string): void;
}

export function PlayView({
  projectData,
  loadedPath,
  report,
  running,
  onChooseChoice,
}: PlayViewProps) {
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

  return (
    <Reveal as="section" ariaLabel={t("play.aria.workspace")} className="grid gap-4">
      <section
        aria-label={t("play.aria.scene")}
        className="overflow-hidden rounded-lg border border-canvas-200 bg-graphite-950 text-ink shadow-studio-panel"
      >
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-canvas-200 px-4 py-3">
          <div className="flex min-w-0 items-center gap-2">
            <span
              className={`h-2.5 w-2.5 rounded-full transition ease-expo ${
                running ? "bg-violet-400 animate-pulse" : "bg-health-400"
              }`}
            />
            <p className="eyebrow">{t("play.scene")}</p>
          </div>
          <div className="flex flex-wrap gap-2">
            <StudioStatusChip tone={running ? "accent" : "health"}>
              {running ? t("play.running") : t("play.ready")}
            </StudioStatusChip>
          </div>
        </div>

        <div className="relative min-h-[clamp(220px,40vh,420px)] overflow-hidden bg-graphite-900">
          <ScenePreviewImage
            src={sceneImage}
            assetPath={scene?.background_asset ?? null}
            className="absolute inset-0 h-full w-full object-cover opacity-70"
          />
          <div className="absolute inset-0 bg-gradient-to-t from-graphite-950 via-graphite-950/25 to-graphite-950/5" />
          <div className="relative flex min-h-[clamp(220px,40vh,420px)] flex-col justify-end p-4">
            <div className="mx-auto w-full max-w-3xl rounded-lg border border-violet-500/40 bg-graphite-950/90 px-4 py-4 shadow-studio-panel">
              <p className="eyebrow eyebrow--copper">
                {scene ? `${scene.title} / ${scene.location}` : loadedPath}
              </p>
              <p className="mt-2 text-base leading-7 text-ink">
                {beat?.text ?? t("play.openProjectToPreview")}
              </p>
              <div className="mt-4 grid gap-2">
                {(beat?.choices ?? []).map((choice, index) => (
                  <button
                    key={choice.id}
                    type="button"
                    onClick={() => onChooseChoice(choice.label)}
                    disabled={running}
                    className="group flex min-h-11 items-center gap-3 rounded-md border border-violet-500/35 bg-ink/5 px-3 text-left text-ink transition ease-expo hover:border-violet-500/65 hover:bg-violet-500/10 active:translate-y-px disabled:cursor-not-allowed disabled:opacity-50"
                  >
                    <span className="grid h-7 w-7 shrink-0 place-items-center rounded-sm border border-copper-500/55 font-mono text-xs font-semibold text-copper-500 transition ease-expo group-hover:border-copper-500 group-hover:text-copper-600">
                      {index + 1}
                    </span>
                    <span className="min-w-0 truncate text-sm">
                      {choice.label}
                    </span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>

        <div className="grid gap-3 border-t border-canvas-200 px-4 py-3 text-sm sm:grid-cols-3">
          <CanvasFact label={t("play.sceneLabel")} value={scene?.key ?? t("common.none")} />
          <CanvasFact
            label={t("play.tension")}
            value={
              trace?.narrative_review
                ? t("common.scores", { score: trace.narrative_review.score })
                : t("common.ready")
            }
          />
          <CanvasFact
            label={t("play.branching")}
            value={t("common.choices", { count: beat?.choices.length ?? 0 })}
          />
        </div>
      </section>

      <p className="text-sm italic text-copper-600/85">
        {t("play.directHint")}
      </p>
    </Reveal>
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

function CanvasFact({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-semibold uppercase tracking-eyebrow text-copper-500/80">
        {label}
      </p>
      <p className="mt-1 truncate font-display text-sm font-semibold tracking-tightish text-ink">{value}</p>
    </div>
  );
}
