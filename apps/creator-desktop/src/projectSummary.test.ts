import { describe, expect, it } from "vitest";
import { summarizeProject, type CreatorProjectInput } from "./projectSummary";

describe("summarizeProject", () => {
  it("derives Studio metrics from generated PlotForge contracts", () => {
    const project: CreatorProjectInput = {
      game: {
        id: "starter-project",
        title: "Starter Project",
        version: "0.1.0",
        description: "A council crisis demo.",
        entry_scene: "opening-scene",
        run_seed: 7,
      },
      scenes: [
        {
          key: "opening-scene",
          title: "Memorials at Dawn",
          location: "Civic Hall",
          dramatic_purpose: "Expose the first crisis.",
          hook: "The treasury report contradicts the war ledger.",
          background_asset: "assets/generated/opening-scene.png",
          audio_refs: [],
          character_ids: ["censor"],
          plot_thread_updates: {},
          beats: [],
        },
      ],
      characters: [
        {
          id: "censor",
          name: "Auditor Liu",
          role: "Civic watchdog",
          traits: ["unyielding"],
          visual_card: "ink portrait",
          voice_card: "precise",
        },
      ],
      rules: [
        {
          id: "inspect-ledger",
          action_type: "inspect",
          conditions: [],
          effects: [],
        },
      ],
      story_craft: {
        active_promises: [
          {
            id: "missing-silver",
            text: "The missing silver will return as political leverage.",
            status: "active",
            introduced_at: "opening-scene",
          },
          {
            id: "old-favor",
            text: "An old favor has already been paid off.",
            status: "paid_off",
            introduced_at: "opening-scene",
          },
        ],
        plot_threads: [
          {
            id: "famine-ledger",
            title: "The Famine Ledger",
            promise: "The report hides a factional bargain.",
            thread_type: "political",
            status: "escalating",
            introduced_at: "opening-scene",
            related_characters: ["censor"],
            related_world_flags: [],
            last_update: "opening-scene",
          },
          {
            id: "border-payoff",
            title: "Border Payoff",
            promise: "The border threat receives a payoff.",
            thread_type: "survival",
            status: "resolved",
            introduced_at: "opening-scene",
            related_characters: [],
            related_world_flags: [],
            last_update: "opening-scene",
          },
        ],
        review_notes: [
          {
            id: "weak-choice",
            scene_key: "opening-scene",
            severity: "warning",
            message: "The first choice needs clearer stakes.",
            resolved: false,
          },
          {
            id: "hook-fixed",
            scene_key: "opening-scene",
            severity: "info",
            message: "Opening hook tightened.",
            resolved: true,
          },
        ],
      },
    };

    expect(summarizeProject(project)).toEqual({
      title: "Starter Project",
      entryScene: "opening-scene",
      sceneCount: 1,
      characterCount: 1,
      ruleCount: 1,
      openThreadCount: 1,
      activePromiseCount: 1,
      unresolvedReviewCount: 1,
    });
  });
});
