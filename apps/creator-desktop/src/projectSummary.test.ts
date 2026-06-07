import { describe, expect, it } from "vitest";
import { summarizeProject, type CreatorProjectInput } from "./projectSummary";

describe("summarizeProject", () => {
  it("derives Studio metrics from generated PlotForge contracts", () => {
    const project: CreatorProjectInput = {
      game: {
        id: "dynasty-embers",
        title: "Dynasty Embers",
        version: "0.1.0",
        description: "A court crisis demo.",
        entry_scene: "court-crisis-001",
        run_seed: 7,
      },
      scenes: [
        {
          key: "court-crisis-001",
          title: "Memorials at Dawn",
          location: "Forbidden City",
          dramatic_purpose: "Expose the first crisis.",
          hook: "The treasury report contradicts the war ledger.",
          background_asset: "assets/generated/court-crisis-001.png",
          character_ids: ["censor"],
          plot_thread_updates: {},
          beats: [],
        },
      ],
      characters: [
        {
          id: "censor",
          name: "Censor Liu",
          role: "Court watchdog",
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
            introduced_at: "court-crisis-001",
          },
          {
            id: "old-favor",
            text: "An old favor has already been paid off.",
            status: "paid_off",
            introduced_at: "court-crisis-001",
          },
        ],
        plot_threads: [
          {
            id: "famine-ledger",
            title: "The Famine Ledger",
            promise: "The report hides a factional bargain.",
            thread_type: "political",
            status: "escalating",
            introduced_at: "court-crisis-001",
            related_characters: ["censor"],
            related_world_flags: [],
            last_update: "court-crisis-001",
          },
          {
            id: "border-payoff",
            title: "Border Payoff",
            promise: "The border threat receives a payoff.",
            thread_type: "survival",
            status: "resolved",
            introduced_at: "court-crisis-001",
            related_characters: [],
            related_world_flags: [],
            last_update: "court-crisis-001",
          },
        ],
        review_notes: [
          {
            id: "weak-choice",
            scene_key: "court-crisis-001",
            severity: "warning",
            message: "The first choice needs clearer stakes.",
            resolved: false,
          },
          {
            id: "hook-fixed",
            scene_key: "court-crisis-001",
            severity: "info",
            message: "Opening hook tightened.",
            resolved: true,
          },
        ],
      },
    };

    expect(summarizeProject(project)).toEqual({
      title: "Dynasty Embers",
      entryScene: "court-crisis-001",
      sceneCount: 1,
      characterCount: 1,
      ruleCount: 1,
      openThreadCount: 1,
      activePromiseCount: 1,
      unresolvedReviewCount: 1,
    });
  });
});
