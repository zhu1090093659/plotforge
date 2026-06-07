import type {
  NarrativeReviewNote,
  PlotThread,
  ProjectData,
  StoryPromise,
} from "../../../contracts/plotforge";

export type CreatorProjectInput = Pick<
  ProjectData,
  "characters" | "game" | "rules" | "scenes"
> & {
  story_craft: {
    active_promises: StoryPromise[];
    plot_threads: PlotThread[];
    review_notes: NarrativeReviewNote[];
  };
};

export interface CreatorProjectSummary {
  title: string;
  entryScene: string;
  sceneCount: number;
  characterCount: number;
  ruleCount: number;
  openThreadCount: number;
  activePromiseCount: number;
  unresolvedReviewCount: number;
}

export function summarizeProject(
  project: CreatorProjectInput,
): CreatorProjectSummary {
  return {
    title: project.game.title,
    entryScene: project.game.entry_scene,
    sceneCount: project.scenes.length,
    characterCount: project.characters.length,
    ruleCount: project.rules.length,
    openThreadCount: project.story_craft.plot_threads.filter(
      (thread) => thread.status !== "resolved" && thread.status !== "paid_off",
    ).length,
    activePromiseCount: project.story_craft.active_promises.filter(
      (promise) => promise.status === "active",
    ).length,
    unresolvedReviewCount: project.story_craft.review_notes.filter(
      (note) => !note.resolved,
    ).length,
  };
}
