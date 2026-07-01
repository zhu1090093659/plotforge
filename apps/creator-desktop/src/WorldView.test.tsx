import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { WorldView, type WorldViewProps } from "./WorldView";
import type { WorldEditDocument } from "../../../contracts/plotforge";

afterEach(() => {
  cleanup();
});

const demoWorldDoc: WorldEditDocument = {
  world_bible_markdown: "# World\n\nCity under pressure.\n",
  canon_markdown: "# Canon\n\nThe city fell.\n",
  forbidden_facts: ["No secret heir", "No undead mayor"],
};

function defaultProps(overrides: Partial<WorldViewProps> = {}): WorldViewProps {
  return {
    worldEditDocument: demoWorldDoc,
    saving: false,
    formStatus: null,
    worldExpansionGoal: "Expand northern border canon.",
    onWorldExpansionGoalChange: vi.fn(),
    onSave: vi.fn(),
    onGenerateWorldExpansion: vi.fn(),
    onUpdateWorldDocument: vi.fn(),
    ...overrides,
  };
}

describe("WorldView", () => {
  it("renders world bible, canon, and forbidden facts fields", () => {
    render(<WorldView {...defaultProps()} />);

    expect(screen.getByLabelText("World bible markdown")).toBeTruthy();
    expect(screen.getByLabelText("Canon markdown")).toBeTruthy();
    expect(screen.getByLabelText("Forbidden facts")).toBeTruthy();
    expect(
      screen.getByDisplayValue(/City under pressure/),
    ).toBeTruthy();
    expect(screen.getByDisplayValue(/The city fell/)).toBeTruthy();
    expect(screen.getByDisplayValue(/No secret heir/)).toBeTruthy();
  });

  it("shows forbidden facts count in subtitle", () => {
    render(<WorldView {...defaultProps()} />);
    expect(screen.getByText("2 forbidden facts")).toBeTruthy();
  });

  it("calls onSave when Save World Bible is clicked", () => {
    const onSave = vi.fn();
    render(<WorldView {...defaultProps({ onSave })} />);

    fireEvent.click(screen.getByRole("button", { name: "Save World Bible" }));
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it("calls onUpdateWorldDocument when world bible text changes", () => {
    const onUpdateWorldDocument = vi.fn();
    render(<WorldView {...defaultProps({ onUpdateWorldDocument })} />);

    fireEvent.change(screen.getByLabelText("World bible markdown"), {
      target: { value: "# World\n\nNew content.\n" },
    });
    expect(onUpdateWorldDocument).toHaveBeenCalledWith({
      world_bible_markdown: "# World\n\nNew content.\n",
    });
  });

  it("calls onUpdateWorldDocument when forbidden facts change", () => {
    const onUpdateWorldDocument = vi.fn();
    render(<WorldView {...defaultProps({ onUpdateWorldDocument })} />);

    fireEvent.change(screen.getByLabelText("Forbidden facts"), {
      target: { value: "No secret heir\nNo resurrection" },
    });
    expect(onUpdateWorldDocument).toHaveBeenCalledWith({
      forbidden_facts: ["No secret heir", "No resurrection"],
    });
  });

  it("collapses AI expansion goal into Advanced section by default", () => {
    render(<WorldView {...defaultProps()} />);

    // The Advanced collapsible button should be visible
    expect(
      screen.getByRole("button", { name: /Advanced/ }),
    ).toBeTruthy();
    // The World generation goal input should NOT be visible until expanded
    expect(screen.queryByLabelText("World generation goal")).toBeNull();
  });

  it("reveals AI expansion goal after expanding Advanced section", () => {
    render(<WorldView {...defaultProps()} />);

    fireEvent.click(screen.getByRole("button", { name: /Advanced/ }));
    expect(screen.getByLabelText("World generation goal")).toBeTruthy();
  });

  it("calls onGenerateWorldExpansion when Generate World Expansion is clicked after expanding", () => {
    const onGenerateWorldExpansion = vi.fn();
    render(<WorldView {...defaultProps({ onGenerateWorldExpansion })} />);

    fireEvent.click(screen.getByRole("button", { name: /Advanced/ }));
    fireEvent.click(
      screen.getByRole("button", { name: "Generate World Expansion" }),
    );
    expect(onGenerateWorldExpansion).toHaveBeenCalledTimes(1);
  });

  it("calls onWorldExpansionGoalChange when expansion goal input changes", () => {
    const onWorldExpansionGoalChange = vi.fn();
    render(
      <WorldView {...defaultProps({ onWorldExpansionGoalChange })} />,
    );

    fireEvent.click(screen.getByRole("button", { name: /Advanced/ }));
    fireEvent.change(screen.getByLabelText("World generation goal"), {
      target: { value: "Expand faction conflict in the east." },
    });
    expect(onWorldExpansionGoalChange).toHaveBeenCalledWith(
      "Expand faction conflict in the east.",
    );
  });

  it("shows empty state when worldEditDocument is null", () => {
    render(
      <WorldView {...defaultProps({ worldEditDocument: null })} />,
    );
    expect(screen.getByText("World edit document not loaded.")).toBeTruthy();
    expect(screen.queryByLabelText("World bible markdown")).toBeNull();
  });

  it("shows success form status message for world section", () => {
    render(
      <WorldView
        {...defaultProps({
          formStatus: {
            section: "world",
            tone: "success",
            message: "World Bible saved.",
          },
        })}
      />,
    );
    expect(screen.getByText("World Bible saved.")).toBeTruthy();
  });

  it("does not show form status for a different section", () => {
    render(
      <WorldView
        {...defaultProps({
          formStatus: {
            section: "story",
            tone: "success",
            message: "Story Craft saved.",
          },
        })}
      />,
    );
    expect(screen.queryByText("Story Craft saved.")).toBeNull();
  });

  it("disables Save World Bible button while saving", () => {
    render(<WorldView {...defaultProps({ saving: true })} />);
    expect(
      screen.getByRole("button", { name: "Save World Bible" }).hasAttribute("disabled"),
    ).toBe(true);
  });
});
