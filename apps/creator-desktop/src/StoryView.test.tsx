import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StoryView, type StoryViewProps } from "./StoryView";
import type { StoryCraftEditDocument } from "../../../contracts/plotforge";

afterEach(() => {
  cleanup();
});

const demoStoryCraftDoc: StoryCraftEditDocument = {
  story_bible_markdown: "# Story Bible\n\nThe city crisis unfolds.\n",
  style_guide_markdown: "# Style Guide\n\nConsequence-first choices.\n",
  story_craft: {
    bible: {
      genre_promise: "Political survival drama",
      central_question: "Who pays for the city?",
      target_emotions: ["tension", "dread"],
      core_foreshadowing: ["The seal is broken"],
      emotional_contract: [],
      pacing_profile: {
        escalation_interval_scenes: 3,
        target_tension_curve: [3, 5, 7],
      },
      hook_strategy: {
        primary_hook: "The seal is broken",
        recurring_hook_patterns: [],
      },
      banned_cliches: [],
      reference_modules: [],
    },
    plot_threads: [
      {
        id: "thread-tax",
        title: "Tax Crisis",
        promise: "The treasury will run dry.",
        thread_type: "political",
        status: "open",
        introduced_at: "opening-scene",
        related_characters: [],
        related_world_flags: [],
        last_update: "scene-001",
      },
    ],
    active_promises: [],
    emotional_arc: [],
    character_arcs: [],
    review_notes: [],
  },
};

function defaultProps(overrides: Partial<StoryViewProps> = {}): StoryViewProps {
  return {
    storyCraftEditDocument: demoStoryCraftDoc,
    saving: false,
    formStatus: null,
    storyGenerationConcept:
      "Generate three linked council pressures.",
    onStoryGenerationConceptChange: vi.fn(),
    onSave: vi.fn(),
    onGenerateStoryCraft: vi.fn(),
    onUpdateStoryBible: vi.fn(),
    onUpdateStoryCraftDocument: vi.fn(),
    ...overrides,
  };
}

describe("StoryView", () => {
  it("renders story bible and style guide fields", () => {
    render(<StoryView {...defaultProps()} />);

    expect(screen.getByLabelText("Story bible markdown")).toBeTruthy();
    expect(screen.getByLabelText("Style guide markdown")).toBeTruthy();
    expect(
      screen.getByDisplayValue(/The city crisis unfolds/),
    ).toBeTruthy();
    expect(
      screen.getByDisplayValue(/Consequence-first choices/),
    ).toBeTruthy();
  });

  it("shows plot thread count in subtitle", () => {
    render(<StoryView {...defaultProps()} />);
    expect(screen.getByText("1 plot threads")).toBeTruthy();
  });

  it("renders plot thread cards", () => {
    render(<StoryView {...defaultProps()} />);

    expect(screen.getByText("Tax Crisis")).toBeTruthy();
    expect(screen.getByText("The treasury will run dry.")).toBeTruthy();
  });

  it("keeps AI generation concept visible in main area (not collapsed)", () => {
    render(<StoryView {...defaultProps()} />);

    // The Story generation concept textarea should be immediately visible
    expect(screen.getByLabelText("Story generation concept")).toBeTruthy();
    expect(
      screen.getByDisplayValue(/Generate three linked council pressures/),
    ).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Generate StoryCraft" }),
    ).toBeTruthy();
  });

  it("calls onSave when Save Story Craft is clicked", () => {
    const onSave = vi.fn();
    render(<StoryView {...defaultProps({ onSave })} />);

    fireEvent.click(screen.getByRole("button", { name: "Save Story Craft" }));
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it("calls onGenerateStoryCraft when Generate StoryCraft is clicked", () => {
    const onGenerateStoryCraft = vi.fn();
    render(<StoryView {...defaultProps({ onGenerateStoryCraft })} />);

    fireEvent.click(
      screen.getByRole("button", { name: "Generate StoryCraft" }),
    );
    expect(onGenerateStoryCraft).toHaveBeenCalledTimes(1);
  });

  it("calls onStoryGenerationConceptChange when concept input changes", () => {
    const onStoryGenerationConceptChange = vi.fn();
    render(
      <StoryView {...defaultProps({ onStoryGenerationConceptChange })} />,
    );

    fireEvent.change(screen.getByLabelText("Story generation concept"), {
      target: { value: "Generate a pressure arc." },
    });
    expect(onStoryGenerationConceptChange).toHaveBeenCalledWith(
      "Generate a pressure arc.",
    );
  });

  it("calls onUpdateStoryBible when genre promise changes", () => {
    const onUpdateStoryBible = vi.fn();
    render(<StoryView {...defaultProps({ onUpdateStoryBible })} />);

    fireEvent.change(screen.getByLabelText("Genre promise"), {
      target: { value: "A sharper political survival story." },
    });
    expect(onUpdateStoryBible).toHaveBeenCalledWith({
      genre_promise: "A sharper political survival story.",
    });
  });

  it("calls onUpdateStoryCraftDocument when story bible markdown changes", () => {
    const onUpdateStoryCraftDocument = vi.fn();
    render(
      <StoryView {...defaultProps({ onUpdateStoryCraftDocument })} />,
    );

    fireEvent.change(screen.getByLabelText("Story bible markdown"), {
      target: { value: "# Story Bible\n\nUpdated content.\n" },
    });
    expect(onUpdateStoryCraftDocument).toHaveBeenCalledWith({
      story_bible_markdown: "# Story Bible\n\nUpdated content.\n",
    });
  });

  it("shows empty state when storyCraftEditDocument is null", () => {
    render(
      <StoryView {...defaultProps({ storyCraftEditDocument: null })} />,
    );
    expect(
      screen.getByText("Story Craft edit document not loaded."),
    ).toBeTruthy();
    expect(screen.queryByLabelText("Story bible markdown")).toBeNull();
  });

  it("shows success form status message for story section", () => {
    render(
      <StoryView
        {...defaultProps({
          formStatus: {
            section: "story",
            tone: "success",
            message: "Story Craft saved.",
          },
        })}
      />,
    );
    expect(screen.getByText("Story Craft saved.")).toBeTruthy();
  });

  it("does not show form status for a different section", () => {
    render(
      <StoryView
        {...defaultProps({
          formStatus: {
            section: "world",
            tone: "success",
            message: "World Bible saved.",
          },
        })}
      />,
    );
    expect(screen.queryByText("World Bible saved.")).toBeNull();
  });

  it("disables Save Story Craft button while saving", () => {
    render(<StoryView {...defaultProps({ saving: true })} />);
    expect(
      screen.getByRole("button", { name: "Save Story Craft" }).hasAttribute("disabled"),
    ).toBe(true);
  });

  it("has an Advanced collapsible section for extra details", () => {
    render(<StoryView {...defaultProps()} />);

    // Advanced section should exist
    expect(
      screen.getByRole("button", { name: /Advanced/ }),
    ).toBeTruthy();
  });
});
